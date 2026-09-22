//! Owned Tonic encoding versus legacy scratch+copy. The historical `direct`
//! case name is retained for comparison with saved borrowed-writer baselines.
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
use proto_rs::BytesMode;
use proto_rs::EncodedSnapshot;
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

// Exercise the required copying adapter and its synchronous TLS scratch cache.
struct ScratchEncoder(ProtoEncoder<Arc<Message>, ResponseMode>);

impl Encoder for ScratchEncoder {
    type Item = Arc<Message>;
    type Error = tonic::Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        self.0.encode(item, dst)
    }
}

struct SnapshotEncoder {
    owned: bool,
}

impl Encoder for SnapshotEncoder {
    fn supports_owned(&self) -> bool {
        true
    }
    type Item = Arc<Message>;
    type Error = tonic::Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        use bytes::BufMut;
        let snapshot = item.as_ref().to_encoded_snapshot();
        dst.put_slice(snapshot.as_bytes());
        Ok(())
    }

    fn encode_owned(&mut self, item: Self::Item) -> Result<tonic::codec::EncodeResult<Self::Item>, Self::Error> {
        use tonic::codec::EncodeResult;
        if !self.owned {
            return Ok(EncodeResult::Buffered(item));
        }
        let snapshot = item.as_ref().to_encoded_snapshot();
        match ProtoEncoder::<EncodedSnapshot<Message>, BytesMode>::default().encode_owned(snapshot)? {
            EncodeResult::Owned(message) => Ok(EncodeResult::Owned(message)),
            EncodeResult::Buffered(_) => unreachable!("snapshot ownership hook was bypassed"),
        }
    }
}

type ResponseMode = <proto_rs::grpc::Response<Arc<Message>> as ProtoResponse<Message>>::Mode;

// Same pooled owned writer, but the pre-batching one-message-per-frame dispatch.
// Keep this benchmark-only control for matched comparisons in one executable.
struct UnbatchedOwned(ProtoEncoder<Arc<Message>, ResponseMode>);

impl Encoder for UnbatchedOwned {
    type Item = Arc<Message>;
    type Error = tonic::Status;
    fn supports_owned(&self) -> bool {
        true
    }
    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        self.0.encode(item, dst)
    }
    fn encode_owned(&mut self, item: Self::Item) -> Result<tonic::codec::EncodeResult<Self::Item>, Self::Error> {
        self.0.encode_owned(item)
    }
}

impl Encoder for PreviousEncoder {
    type Item = Arc<Message>;
    type Error = tonic::Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        item.as_ref().encode(dst).map_err(|error| tonic::Status::internal(error.to_string()))
    }
}

fn drain(encoder: impl Encoder<Item = Arc<Message>, Error = tonic::Status>, message: &Arc<Message>, count: usize) -> usize {
    let source = tokio_stream::iter((0..count).map(|_| Ok(Arc::clone(message))));
    drain_source(encoder, source)
}

fn drain_source<E: Encoder<Error = tonic::Status>>(
    encoder: E,
    source: impl tokio_stream::Stream<Item = Result<E::Item, tonic::Status>>,
) -> usize {
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
        // All independently constructed encoders share this thread's pool.
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new(name, "previous"), &message, |b, message| {
            b.iter(|| drain(PreviousEncoder, black_box(message), count));
        });
        group.bench_with_input(BenchmarkId::new(name, "direct"), &message, |b, message| {
            b.iter(|| drain(ProtoEncoder::<Arc<Message>, ResponseMode>::default(), black_box(message), count));
        });
        group.bench_with_input(BenchmarkId::new(name, "unbatched_owned"), &message, |b, message| {
            b.iter(|| drain(UnbatchedOwned(ProtoEncoder::default()), black_box(message), count));
        });
        group.bench_with_input(BenchmarkId::new(name, "exhausted_pool"), &message, |b, message| {
            let _held: Vec<_> =
                (0..proto_rs::EncodePoolConfig::default().max_buffers).map(|_| message.as_ref().to_encoded_snapshot()).collect();
            b.iter(|| drain(ProtoEncoder::<Arc<Message>, ResponseMode>::default(), black_box(message), count));
        });
        group.bench_with_input(BenchmarkId::new(name, "tls_copy"), &message, |b, message| {
            b.iter(|| drain(ScratchEncoder(ProtoEncoder::default()), black_box(message), count));
        });
        for (variant, owned) in [("snapshot_copy", false), ("snapshot_owned", true)] {
            group.bench_with_input(BenchmarkId::new(name, variant), &message, |b, message| {
                b.iter(|| drain(SnapshotEncoder { owned }, black_box(message), count));
            });
        }
        // Fan-out workflow: encoding is intentionally outside timing. Each send
        // clones the same immutable snapshot; this is not encoder throughput.
        group.bench_with_input(BenchmarkId::new(name, "shared_snapshot"), &message, |b, message| {
            let snapshot = message.as_ref().to_encoded_snapshot();
            b.iter(|| {
                drain_source(
                    ProtoEncoder::<proto_rs::EncodedSnapshot<Message>, BytesMode>::default(),
                    tokio_stream::iter((0..count).map(|_| Ok(black_box(&snapshot).clone()))),
                )
            });
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
