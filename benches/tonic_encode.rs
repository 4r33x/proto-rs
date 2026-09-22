//! Focused comparison of the direct Tonic encoder and its previous scratch+copy path.
use std::hint::black_box;
use std::pin::pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::time::Duration;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use proto_rs::ProtoEncode;
use proto_rs::ProtoEncoder;
use proto_rs::ProtoResponse;
use proto_rs::proto_message;
use tonic::codec::EncodeBody;
use tonic::codec::EncodeBuf;
use tonic::codec::Encoder;
use tonic::codegen::Body;

#[proto_message]
#[derive(Clone)]
struct Message {
    id: u64,
    name: String,
    bytes: Vec<u8>,
    values: Vec<u64>,
}

struct PreviousEncoder;

type ResponseMode = <proto_rs::grpc::Response<Arc<Message>> as ProtoResponse<Message>>::Mode;

impl Encoder for PreviousEncoder {
    type Item = Arc<Message>;
    type Error = tonic::Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        item.as_ref().encode(dst).map_err(|error| tonic::Status::internal(error.to_string()))
    }
}

fn drain(encoder: impl Encoder<Item = Arc<Message>, Error = tonic::Status>, message: &Arc<Message>, count: usize) -> usize {
    let source = tokio_stream::iter((0..count).map(|_| Ok(Arc::clone(message))));
    let mut body = pin!(EncodeBody::new_client(encoder, source, None, None));
    let mut context = Context::from_waker(Waker::noop());
    let mut total = 0;
    loop {
        match body.as_mut().poll_frame(&mut context) {
            Poll::Ready(Some(Ok(frame))) => {
                if let Ok(bytes) = frame.into_data() {
                    total += black_box(bytes).len();
                }
            }
            Poll::Ready(None) => return total,
            Poll::Ready(Some(Err(status))) => panic!("in-memory body failed: {status}"),
            // Tokio's cooperative budget may yield even for an in-memory source.
            Poll::Pending => {}
        }
    }
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("tonic_encode");
    for (name, size, count) in [
        ("small_unary", 16, 1),
        ("small_stream", 16, 128),
        ("large_unary", 16_384, 1),
        ("large_stream", 16_384, 128),
        ("oversized_unary", 262_144, 1),
        ("oversized_stream", 262_144, 16),
    ] {
        let message = Arc::new(Message {
            id: 12345,
            name: "example".into(),
            bytes: vec![42; size],
            values: vec![1, 127, 128, u64::MAX],
        });
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new(name, "previous"), &message, |b, message| {
            b.iter(|| drain(PreviousEncoder, black_box(message), count));
        });
        group.bench_with_input(BenchmarkId::new(name, "direct"), &message, |b, message| {
            b.iter(|| drain(ProtoEncoder::<Arc<Message>, ResponseMode>::default(), black_box(message), count));
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(50).warm_up_time(Duration::from_secs(1)).measurement_time(Duration::from_secs(3));
    targets = bench
}
criterion_main!(benches);
