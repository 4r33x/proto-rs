#![cfg(all(feature = "linux-zerocopy", target_os = "linux"))]
#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use std::sync::Arc;
use std::time::Duration;

use proto_rs::grpc::Request;
use proto_rs::grpc::Response;
use proto_rs::grpc::Status;
use proto_rs::grpc::zerocopy::ZeroCopyChannel;
use proto_rs::grpc::zerocopy::ZeroCopyConfig;
use proto_rs::grpc::zerocopy::ZeroCopyIo;
use proto_rs::grpc::zerocopy::serve_connection;
use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tonic::codec::CompressionEncoding;

#[proto_message]
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    a: u64,
    b: u64,
    c: Option<u64>,
    d: Arc<Vec<u8>>,
}

#[proto_rpc(rpc_package = "zero_copy_test", rpc_client = true, rpc_server = true)]
pub trait Echo {
    async fn echo(&self, request: Request<Vec<Record>>) -> Result<Response<Vec<Record>>, Status>;
    async fn echo_snapshot(&self, request: Request<Vec<Record>>) -> Result<Response<proto_rs::EncodedSnapshot<Vec<Record>>>, Status>;
}

struct EchoImpl;
impl Echo for EchoImpl {
    async fn echo(&self, request: Request<Vec<Record>>) -> Result<Response<Vec<Record>>, Status> {
        Ok(Response::new(request.into_inner()))
    }
    async fn echo_snapshot(&self, request: Request<Vec<Record>>) -> Result<Response<proto_rs::EncodedSnapshot<Vec<Record>>>, Status> {
        Ok(Response::new(proto_rs::EncodedSnapshot::with_max_preallocation(
            &request.into_inner(),
            8 * 1024 * 1024,
        )))
    }
}

fn config() -> ZeroCopyConfig {
    ZeroCopyConfig {
        min_send_bytes: 4096,
        max_in_flight_sends: 2,
        max_in_flight_bytes: 32 * 1024,
        allow_fallback: false,
        adaptive_fallback: false,
    }
}

fn fixture() -> Vec<Record> {
    (0..20)
        .map(|i| Record {
            a: i,
            b: u64::MAX - i,
            c: if i % 2 == 0 { None } else { Some(0) },
            d: Arc::new((0..65536u32).map(|n| (n.wrapping_mul(0x9e37_79b9) >> (i % 17)).to_le_bytes()[0]).collect()),
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn grpc_owned_buffers_roundtrip_both_directions_plain_and_gzip() {
    tokio::time::timeout(Duration::from_secs(30), async {
        for gzip in [false, true] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let (metrics_tx, metrics_rx) = tokio::sync::oneshot::channel();
            let server = tokio::spawn(async move {
                let (io, _) = ZeroCopyIo::accept(&listener, config()).await.unwrap();
                let metrics = io.metrics();
                metrics_tx.send(metrics).ok().unwrap();
                let mut service = echo_server::EchoServer::new(EchoImpl)
                    .max_decoding_message_size(8 * 1024 * 1024)
                    .with_max_encode_preallocation(8 * 1024 * 1024);
                if gzip {
                    service = service.accept_compressed(CompressionEncoding::Gzip).send_compressed(CompressionEncoding::Gzip);
                }
                serve_connection(io, service).await
            });
            let channel = ZeroCopyChannel::connect(format!("http://{addr}").parse().unwrap(), config()).await.unwrap();
            let metrics = channel.metrics();
            let server_metrics = metrics_rx.await.unwrap();
            let mut client = echo_client::EchoClient::new(channel)
                .max_decoding_message_size(8 * 1024 * 1024)
                .with_max_encode_preallocation(8 * 1024 * 1024);
            if gzip {
                client = client.send_compressed(CompressionEncoding::Gzip).accept_compressed(CompressionEncoding::Gzip);
            }
            let input = fixture();
            for _ in 0..3 {
                assert_eq!(client.echo(input.clone()).await.unwrap().into_inner(), input);
                let snapshot = proto_rs::EncodedSnapshot::with_max_preallocation(&input, 8 * 1024 * 1024);
                assert_eq!(client.echo_snapshot(snapshot).await.unwrap().into_inner(), input);
            }
            metrics.wait_for_idle().await.unwrap();
            server_metrics.wait_for_idle().await.unwrap();
            for snapshot in [metrics.snapshot(), server_metrics.snapshot()] {
                assert!(snapshot.send_calls > 0, "owned write hook was bypassed");
                assert!(snapshot.submitted_bytes > 4096);
                assert_eq!(snapshot.pending_sends, 0);
                assert_eq!(
                    snapshot.submitted_bytes,
                    snapshot.completed_without_copy_flag + snapshot.completed_with_copy_flag
                );
                assert_eq!(snapshot.completion_errors, 0);
                // Loopback uses deferred copies; do not call this a zero-copy win.
                assert!(snapshot.completed_with_copy_flag > 0);
            }
            drop(client);
            server.abort();
            let _ = server.await;
            metrics.wait_for_idle().await.unwrap();
            server_metrics.wait_for_idle().await.unwrap();
        }
    })
    .await
    .expect("zero-copy RPC/completion timeout");
}

#[tokio::test]
async fn https_is_rejected_without_plaintext_downgrade() {
    let result = ZeroCopyChannel::connect("https://localhost:443".parse().unwrap(), config()).await;
    assert!(result.is_err());
    assert!(result.err().unwrap().to_string().contains("TLS"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_owned_send_keeps_accepted_bytes_alive() {
    use std::pin::Pin;

    use bytes::Buf;
    use bytes::Bytes;
    use hyper_zerocopy::rt::Write;
    use tokio::io::AsyncReadExt;
    tokio::time::timeout(Duration::from_secs(20), async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let receiver = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
            let mut received = Vec::new();
            stream.read_to_end(&mut received).await.unwrap();
            received
        });
        let mut io = ZeroCopyIo::connect(address, config()).await.unwrap();
        let metrics = io.metrics();
        let mut data = Bytes::from(vec![0x71; 512 * 1024]);
        let empty = Bytes::new();
        let sent = std::future::poll_fn(|cx| Pin::new(&mut io).poll_write_owned(cx, &empty, &data)).await.unwrap();
        data.advance(sent);
        drop(data);
        drop(io);
        let output = receiver.await.unwrap();
        assert_eq!(output, vec![0x71; sent]);
        metrics.wait_for_idle().await.unwrap();
        assert_eq!(metrics.snapshot().submitted_bytes, sent as u64);
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn copied_completion_switches_adaptive_mode_to_ordinary_sends() {
    use std::pin::Pin;

    use bytes::Bytes;
    use hyper_zerocopy::rt::Write;
    use tokio::io::AsyncReadExt;
    tokio::time::timeout(Duration::from_secs(10), async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let receiver = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).await.unwrap();
            bytes
        });
        let mut io = ZeroCopyIo::connect(
            address,
            ZeroCopyConfig {
                allow_fallback: true,
                adaptive_fallback: true,
                ..config()
            },
        )
        .await
        .unwrap();
        let metrics = io.metrics();
        let bytes = Bytes::from(vec![42; 16384]);
        let empty = Bytes::new();
        let first = std::future::poll_fn(|cx| Pin::new(&mut io).poll_write_owned(cx, &empty, &bytes)).await.unwrap();
        metrics.wait_for_idle().await.unwrap();
        let stats = metrics.snapshot();
        assert_eq!(stats.send_calls, 1);
        assert!(!stats.enabled, "loopback copied completion should disable zero-copy requests");
        let second = std::future::poll_fn(|cx| Pin::new(&mut io).poll_write_owned(cx, &empty, &bytes)).await.unwrap();
        assert_eq!(metrics.snapshot().send_calls, 1);
        assert_eq!(metrics.snapshot().ordinary_send_bytes, second as u64);
        drop(io);
        assert_eq!(receiver.await.unwrap(), vec![42; first + second]);
    })
    .await
    .unwrap();
}

#[test]
fn runtime_shutdown_preserves_completion_ownership() {
    use std::io::Read;
    use std::pin::Pin;

    use bytes::Bytes;
    use hyper_zerocopy::rt::Write;

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let receiver = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let mut bytes = Vec::new();
        socket.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let (metrics, sent) = runtime.block_on(async {
        let mut io = ZeroCopyIo::connect(address, config()).await.unwrap();
        let metrics = io.metrics();
        let data = Bytes::from(vec![0x73; 512 * 1024]);
        let sent = std::future::poll_fn(|cx| Pin::new(&mut io).poll_write_owned(cx, &Bytes::new(), &data)).await.unwrap();
        tokio::spawn(async move {
            let _owner = io;
            std::future::pending::<()>().await;
        });
        (metrics, sent)
    });
    drop(runtime); // Cancels the task and closes I/O, not the completion owner.
    assert_eq!(receiver.join().unwrap(), vec![0x73; sent]);
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while metrics.snapshot().pending_sends != 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "completion owner did not drain after runtime shutdown"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let stats = metrics.snapshot();
    assert_eq!(stats.submitted_bytes, sent as u64);
    assert_eq!(
        stats.submitted_bytes,
        stats.completed_with_copy_flag + stats.completed_without_copy_flag
    );
    assert_eq!(stats.completion_errors, 0);
}
