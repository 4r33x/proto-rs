//! One Vec<BytesBench> per RPC, with 20 independent payload buffers totaling 64 MiB.
#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use std::hint::black_box;
use std::pin::pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::time::Duration;
use std::time::Instant;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use proto_rs::ProtoCodec;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::ProtoEncoder;
use proto_rs::ProtoResponse;
use proto_rs::grpc::Request;
use proto_rs::grpc::Response;
use proto_rs::grpc::Status;
use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tokio_stream::StreamExt;
use tonic::codec::Codec;
use tonic::codec::CompressionEncoding;
use tonic::codec::EncodeBody;
use tonic::codec::EncodeBuf;
use tonic::codec::Encoder;
use tonic::codegen::Body;
use tonic::transport::Certificate;
use tonic::transport::Channel;
use tonic::transport::ClientTlsConfig;
use tonic::transport::Endpoint;
use tonic::transport::Identity;
use tonic::transport::Server;
use tonic::transport::ServerTlsConfig;

const TOTAL_BYTES: usize = 64 * 1024 * 1024;
const COUNT: usize = 20;
const MESSAGE_LIMIT: usize = 128 * 1024 * 1024;

#[proto_message]
#[derive(Clone, Debug, PartialEq)]
pub struct BytesBench {
    a: u64,
    b: u64,
    c: Option<u64>,
    d: Arc<Vec<u8>>,
}

#[proto_message]
#[derive(Clone, Debug)]
pub struct Receipt {
    count: u64,
    bytes: u64,
}

type Batch = Vec<BytesBench>;
type SharedBatch = Arc<Batch>;
type Mode = <Response<SharedBatch> as ProtoResponse<Batch>>::Mode;
type BatchCodec = ProtoCodec<SharedBatch, Receipt, Mode>;

#[proto_rpc(rpc_package = "bytes_pipeline", rpc_server = true, rpc_client = false)]
pub trait Upload {
    async fn send(&self, request: Request<Batch>) -> Result<Response<Receipt>, Status>;
}

struct Receiver {
    expected: SharedBatch,
}

impl Upload for Receiver {
    async fn send(&self, request: Request<Batch>) -> Result<Response<Receipt>, Status> {
        let verify = request.metadata().contains_key("x-verify-payload");
        let values = request.into_inner();
        // Full byte-for-byte verification outside the timed iterations.
        if verify {
            assert_eq!(values, *self.expected);
        }
        Ok(Response::new(Receipt {
            count: values.len() as u64,
            bytes: values.iter().map(|item| item.d.len() as u64).sum(),
        }))
    }
}

// This retains the old scratch+copy implementation in the same benchmark binary.
#[derive(Clone)]
struct ScratchCodec;
struct ScratchEncoder;

struct SnapshotEncoder {
    owned: bool,
}

impl Encoder for SnapshotEncoder {
    fn supports_owned(&self) -> bool {
        true
    }
    type Item = SharedBatch;
    type Error = tonic::Status;
    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        use bytes::BufMut;
        let snapshot = proto_rs::EncodedSnapshot::with_max_preallocation(item.as_ref(), MESSAGE_LIMIT);
        dst.put_slice(snapshot.as_bytes());
        Ok(())
    }
    fn encode_owned(&mut self, item: Self::Item) -> Result<tonic::codec::EncodeResult<Self::Item>, Self::Error> {
        use tonic::codec::EncodeResult;
        if !self.owned {
            return Ok(EncodeResult::Buffered(item));
        }
        let snapshot = proto_rs::EncodedSnapshot::with_max_preallocation(item.as_ref(), MESSAGE_LIMIT);
        match ProtoEncoder::<proto_rs::EncodedSnapshot<Batch>, proto_rs::BytesMode>::default().encode_owned(snapshot)? {
            EncodeResult::Owned(message) => Ok(EncodeResult::Owned(message)),
            EncodeResult::Buffered(_) => unreachable!("snapshot ownership hook was bypassed"),
        }
    }
}

impl Encoder for ScratchEncoder {
    type Item = SharedBatch;
    type Error = tonic::Status;

    fn encode(&mut self, item: SharedBatch, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        item.as_ref().encode(dst).map_err(|error| tonic::Status::internal(error.to_string()))
    }
}

impl Codec for ScratchCodec {
    type Encode = SharedBatch;
    type Decode = Receipt;
    type Encoder = ScratchEncoder;
    type Decoder = <BatchCodec as Codec>::Decoder;

    fn encoder(&mut self) -> Self::Encoder {
        ScratchEncoder
    }
    fn decoder(&mut self) -> Self::Decoder {
        BatchCodec::default().decoder()
    }
}

fn fixture(compressible: bool) -> SharedBatch {
    let mut state = 0x0123_4567_89ab_cdef_u64;
    let batch: Vec<_> = (0..COUNT)
        .map(|index| {
            let length = TOTAL_BYTES / COUNT + usize::from(index < TOTAL_BYTES % COUNT);
            let mut data = vec![0u8; length];
            for chunk in data.chunks_mut(8) {
                // Deterministic high-entropy bytes, generated once outside timing.
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let bytes = if compressible {
                    (index as u64).to_le_bytes()
                } else {
                    state.to_le_bytes()
                };
                chunk.copy_from_slice(&bytes[..chunk.len()]);
            }
            BytesBench {
                a: index as u64,
                b: u64::MAX - index as u64,
                c: match index % 3 {
                    0 => None,
                    1 => Some(0),
                    _ => Some(u64::MAX),
                },
                d: Arc::new(data),
            }
        })
        .collect();
    assert_eq!(batch.iter().map(|item| item.d.len()).sum::<usize>(), TOTAL_BYTES);
    Arc::new(batch)
}

fn body_bytes(encoder: impl Encoder<Item = SharedBatch, Error = tonic::Status>, batch: &SharedBatch, gzip: bool) -> usize {
    let source = tokio_stream::iter([Ok(Arc::clone(batch))]);
    let compression = gzip.then_some(CompressionEncoding::Gzip);
    let mut body = pin!(EncodeBody::new_client(encoder, source, compression, Some(MESSAGE_LIMIT)));
    let mut context = Context::from_waker(Waker::noop());
    let mut size = 0;
    loop {
        match body.as_mut().poll_frame(&mut context) {
            Poll::Ready(Some(Ok(frame))) => {
                if let Ok(data) = frame.into_data() {
                    size += black_box(data).len();
                }
            }
            Poll::Ready(Some(Err(error))) => panic!("body encoding failed: {error}"),
            Poll::Ready(None) => return size,
            Poll::Pending => {}
        }
    }
}

async fn start_server(
    tls: bool,
    batch: &SharedBatch,
) -> (Channel, http::Uri, tokio::sync::oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut server = Server::builder();
    let mut endpoint =
        Endpoint::from_shared(format!("{}://{address}", if tls { "https" } else { "http" })).unwrap().timeout(Duration::from_secs(120));
    if tls {
        let rcgen::CertifiedKey { cert, signing_key } = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let certificate = cert.pem();
        server = server.tls_config(ServerTlsConfig::new().identity(Identity::from_pem(&certificate, signing_key.serialize_pem()))).unwrap();
        endpoint = endpoint
            .tls_config(ClientTlsConfig::new().domain_name("localhost").ca_certificate(Certificate::from_pem(certificate)))
            .unwrap();
    }
    let service = upload_server::UploadServer::new(Receiver {
        expected: Arc::clone(batch),
    })
    .max_decoding_message_size(MESSAGE_LIMIT)
    .accept_compressed(CompressionEncoding::Gzip);
    let (shutdown, stopped) = tokio::sync::oneshot::channel();
    // serve_with_incoming uses caller-owned sockets, so configure these just
    // like Tonic's normal listener (TCP_NODELAY is enabled there by default).
    // Otherwise Nagle/delayed ACK can dominate the small receipt's latency.
    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener).map(|stream| {
        stream.and_then(|stream| {
            stream.set_nodelay(true)?;
            Ok(stream)
        })
    });
    let task = tokio::spawn(async move {
        server
            .add_service(service)
            .serve_with_incoming_shutdown(incoming, async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    (
        endpoint.connect().await.unwrap(),
        format!("http://{address}").parse().unwrap(),
        shutdown,
        task,
    )
}

async fn upload<C: Codec<Encode = SharedBatch, Decode = Receipt>, T>(
    client: &mut tonic::client::Grpc<T>,
    codec: C,
    batch: &SharedBatch,
    verify: bool,
) where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError> + std::fmt::Debug,
    T::ResponseBody: Body<Data = proto_rs::bytes::Bytes> + Send + 'static,
    <T::ResponseBody as Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    client.ready().await.unwrap();
    let mut request = tonic::Request::new(Arc::clone(batch));
    if verify {
        request.metadata_mut().insert("x-verify-payload", "true".parse().unwrap());
    }
    let reply =
        client.unary(request, http::uri::PathAndQuery::from_static("/bytes_pipeline.Upload/Send"), codec).await.unwrap().into_inner();
    assert_eq!(reply.count, COUNT as u64);
    assert_eq!(reply.bytes, TOTAL_BYTES as u64);
}

#[allow(clippy::too_many_lines)] // keep paired transport setups and timings together
fn bench(c: &mut Criterion) {
    let compressible = std::env::var("PROTO_RS_BENCH_DATA").is_ok_and(|value| value == "compressible");
    let distribution = if compressible { "compressible" } else { "incompressible" };
    let backend = if cfg!(feature = "tonic-gzip") { "zlib-rs" } else { "default-gzip" };
    let batch = fixture(compressible);
    // Pool retention is configured globally, before any worker starts encoding.
    proto_rs::configure_encode_pool(proto_rs::EncodePoolConfig {
        max_buffers: 8,
        max_buffer_capacity: MESSAGE_LIMIT,
    })
    .unwrap();
    // Independent ordinary encoder/decoder validation, outside all timing.
    assert_eq!(
        Batch::decode(batch.as_ref().encode_to_vec().as_slice(), proto_rs::DecodeContext::default()).unwrap(),
        *batch
    );
    let mut body_group = c.benchmark_group(format!("bytes_pipeline/{backend}/{distribution}/body"));
    body_group.throughput(Throughput::Bytes(TOTAL_BYTES as u64));
    for gzip in [false, true] {
        let mode = if gzip { "gzip" } else { "plain" };
        body_group.bench_function(BenchmarkId::new(mode, "scratch"), |b| {
            b.iter(|| body_bytes(ScratchEncoder, black_box(&batch), gzip));
        });
        body_group.bench_function(BenchmarkId::new(mode, "bounded"), |b| {
            b.iter(|| body_bytes(ProtoEncoder::<SharedBatch, Mode>::default(), black_box(&batch), gzip));
        });
        body_group.bench_function(BenchmarkId::new(mode, "direct"), |b| {
            b.iter(|| {
                body_bytes(
                    ProtoEncoder::<SharedBatch, Mode>::default().with_max_encode_preallocation(MESSAGE_LIMIT),
                    black_box(&batch),
                    gzip,
                )
            });
        });
        for (variant, owned) in [("snapshot_copy", false), ("snapshot_owned", true)] {
            body_group.bench_function(BenchmarkId::new(mode, variant), |b| {
                b.iter(|| body_bytes(SnapshotEncoder { owned }, black_box(&batch), gzip));
            });
        }
    }
    body_group.finish();

    // Skip socket setup for body-only filters (useful in sandboxed environments).
    if std::env::var_os("PROTO_RS_BENCH_NO_NETWORK").is_some() {
        return;
    }
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let mut rpc_group = c.benchmark_group(format!("bytes_pipeline/{backend}/{distribution}/rpc"));
    rpc_group.throughput(Throughput::Bytes(TOTAL_BYTES as u64));
    for tls in [false, true] {
        let (channel, origin, shutdown, server) = runtime.block_on(start_server(tls, &batch));
        #[cfg(not(all(feature = "linux-zerocopy", target_os = "linux")))]
        let _ = &origin;
        for gzip in [false, true] {
            let mode = match (tls, gzip) {
                (false, false) => "plain",
                (false, true) => "gzip",
                (true, false) => "tls",
                (true, true) => "gzip_tls",
            };
            let mut client = tonic::client::Grpc::new(channel.clone()).max_encoding_message_size(MESSAGE_LIMIT);
            if gzip {
                client = client.send_compressed(CompressionEncoding::Gzip);
            }
            // Establish HTTP/2/TLS and verify the full-size transfer before timing.
            runtime.block_on(upload(
                &mut client,
                BatchCodec::default().with_max_encode_preallocation(MESSAGE_LIMIT),
                &batch,
                true,
            ));
            runtime.block_on(upload(&mut client, ScratchCodec, &batch, true));
            rpc_group.bench_function(BenchmarkId::new(mode, "scratch"), |b| {
                b.iter_custom(|iterations| {
                    runtime.block_on(async {
                        let start = Instant::now();
                        for _ in 0..iterations {
                            upload(&mut client, ScratchCodec, &batch, false).await;
                        }
                        start.elapsed()
                    })
                });
            });
            rpc_group.bench_function(BenchmarkId::new(mode, "direct"), |b| {
                b.iter_custom(|iterations| {
                    runtime.block_on(async {
                        let start = Instant::now();
                        for _ in 0..iterations {
                            upload(
                                &mut client,
                                BatchCodec::default().with_max_encode_preallocation(MESSAGE_LIMIT),
                                &batch,
                                false,
                            )
                            .await;
                        }
                        start.elapsed()
                    })
                });
            });
            #[cfg(all(feature = "linux-zerocopy", target_os = "linux"))]
            if !tls {
                use proto_rs::grpc::zerocopy::ZeroCopyChannel;
                use proto_rs::grpc::zerocopy::ZeroCopyConfig;
                // Loopback always reports copied completions. Force the request
                // path for overhead/correctness measurement, never a speed claim.
                let config = ZeroCopyConfig {
                    allow_fallback: false,
                    adaptive_fallback: false,
                    ..Default::default()
                };
                let zc = runtime.block_on(ZeroCopyChannel::connect(origin.clone(), config)).unwrap();
                let metrics = zc.metrics();
                let mut client = tonic::client::Grpc::new(zc).max_encoding_message_size(MESSAGE_LIMIT);
                if gzip {
                    client = client.send_compressed(CompressionEncoding::Gzip);
                }
                runtime.block_on(upload(
                    &mut client,
                    BatchCodec::default().with_max_encode_preallocation(MESSAGE_LIMIT),
                    &batch,
                    true,
                ));
                rpc_group.bench_function(BenchmarkId::new(mode, "zerocopy_forced_loopback"), |b| {
                    b.iter_custom(|iterations| {
                        runtime.block_on(async {
                            let start = Instant::now();
                            for _ in 0..iterations {
                                upload(
                                    &mut client,
                                    BatchCodec::default().with_max_encode_preallocation(MESSAGE_LIMIT),
                                    &batch,
                                    false,
                                )
                                .await;
                            }
                            start.elapsed()
                        })
                    });
                });
                runtime.block_on(async {
                    tokio::time::timeout(Duration::from_secs(30), metrics.wait_for_idle()).await.unwrap().unwrap();
                });
                let stats = metrics.snapshot();
                assert!(stats.send_calls > 0, "MSG_ZEROCOPY path not exercised");
                assert_eq!(
                    stats.submitted_bytes,
                    stats.completed_without_copy_flag + stats.completed_with_copy_flag
                );
                eprintln!("{mode} MSG_ZEROCOPY completion stats (loopback is copied): {stats:?}");
            }
        }
        let _ = shutdown.send(());
        runtime.block_on(server).unwrap();
    }
    rpc_group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).warm_up_time(Duration::from_millis(100)).measurement_time(Duration::from_secs(1));
    targets = bench
}
criterion_main!(benches);
