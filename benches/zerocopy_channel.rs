//! Matched localhost unary RPCs: regular Tonic versus the managed owned backend.
//! Loopback adaptive fallback measures transport overhead, not NIC zero-copy.
#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use std::time::Duration;
use std::time::Instant;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use proto_rs::grpc::AutoChannel;
use proto_rs::grpc::ChannelOptions;
use proto_rs::grpc::Request;
use proto_rs::grpc::Response;
use proto_rs::grpc::Status;
use proto_rs::proto_message;
use proto_rs::proto_rpc;

#[proto_message]
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    data: Vec<u8>,
}
#[proto_message]
#[derive(Clone, Debug, PartialEq)]
pub struct Ack {
    len: u64,
}
#[proto_rpc(rpc_package = "channel_bench", rpc_client = true, rpc_server = true)]
pub trait Upload {
    async fn upload(&self, request: Request<Message>) -> Result<Response<Ack>, Status>;
}
struct Handler;
impl Upload for Handler {
    async fn upload(&self, request: Request<Message>) -> Result<Response<Ack>, Status> {
        let value = request.into_inner();
        assert!(value.data.iter().all(|b| *b == 42));
        Ok(Response::new(Ack {
            len: value.data.len() as u64,
        }))
    }
}

fn bench(c: &mut Criterion) {
    use tokio_stream::StreamExt;
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let (endpoint, stop, server) = runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = tonic::transport::Endpoint::from_shared(format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener).map(|accepted| {
            accepted.and_then(|stream| {
                stream.set_nodelay(true)?;
                Ok(stream)
            })
        });
        let server = tokio::spawn(
            tonic::transport::Server::builder().add_service(upload_server::UploadServer::new(Handler)).serve_with_incoming_shutdown(
                incoming,
                async {
                    let _ = stopped.await;
                },
            ),
        );
        (endpoint, stop, server)
    });
    let mut group = c.benchmark_group("zerocopy_channel");
    for size in [16, 16384, 262144] {
        let value = Message { data: vec![42; size] };
        group.throughput(Throughput::Bytes(size as u64));
        for owned in [false, true] {
            let channel = runtime
                .block_on(AutoChannel::connect(
                    endpoint.clone(),
                    ChannelOptions {
                        kernel_zero_copy: owned,
                        ..Default::default()
                    },
                ))
                .unwrap();
            let metrics = channel.kernel_metrics();
            let mut client = upload_client::UploadClient::new(channel);
            // Establish streams/pools, validate bytes, and settle adaptive fallback.
            runtime.block_on(async {
                for _ in 0..8 {
                    assert_eq!(client.upload(&value).await.unwrap().into_inner().len, size as u64);
                }
                if let Some(metrics) = &metrics {
                    metrics.wait_for_idle().await.unwrap();
                }
            });
            group.bench_function(BenchmarkId::new(if owned { "owned_adaptive" } else { "tonic" }, size), |b| {
                b.iter_custom(|iterations| {
                    runtime.block_on(async {
                        let start = Instant::now();
                        for _ in 0..iterations {
                            assert_eq!(client.upload(&value).await.unwrap().into_inner().len, size as u64);
                        }
                        start.elapsed()
                    })
                });
            });
            if let Some(metrics) = metrics {
                runtime.block_on(metrics.wait_for_idle()).unwrap();
                assert_eq!(metrics.connections(), 1);
            }
        }
    }
    group.finish();
    stop.send(()).unwrap();
    runtime.block_on(server).unwrap().unwrap();
}

criterion_group! { name = benches; config = Criterion::default().sample_size(30).warm_up_time(Duration::from_secs(1)).measurement_time(Duration::from_secs(3)); targets = bench }
criterion_main!(benches);
