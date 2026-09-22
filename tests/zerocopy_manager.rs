#![cfg(all(feature = "linux-zerocopy", target_os = "linux"))]
#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use proto_rs::grpc::AutoChannel;
use proto_rs::grpc::ChannelOptions;
use proto_rs::grpc::Request;
use proto_rs::grpc::Response;
use proto_rs::grpc::Status;
use proto_rs::grpc::zerocopy::ZeroCopyChannel;
use proto_rs::grpc::zerocopy::ZeroCopyConfig;
use proto_rs::grpc::zerocopy::ZeroCopyIo;
use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tokio::sync::Notify;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tonic::codegen::Service;
use tonic::transport::Endpoint;

#[proto_message]
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    id: u64,
    data: Vec<u8>,
}

#[proto_rpc(rpc_package = "manager_test", rpc_client = true, rpc_server = true)]
pub trait Echo {
    async fn echo(&self, request: Request<Message>) -> Result<Response<Message>, Status>;
}

#[derive(Clone, Default)]
struct Handler {
    calls: Arc<AtomicUsize>,
    entered: Arc<Notify>,
    release: Arc<Notify>,
}
impl Echo for Handler {
    async fn echo(&self, request: Request<Message>) -> Result<Response<Message>, Status> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let value = request.into_inner();
        if value.id == 42 {
            self.entered.notify_one();
            self.release.notified().await;
        }
        Ok(Response::new(value))
    }
}

struct Control {
    abort: tokio::task::AbortHandle,
    graceful: oneshot::Sender<()>,
}
struct Server {
    task: tokio::task::JoinHandle<()>,
    controls: mpsc::UnboundedReceiver<Control>,
    requests: mpsc::UnboundedReceiver<(http::Uri, http::HeaderMap)>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn config() -> ZeroCopyConfig {
    ZeroCopyConfig {
        allow_fallback: false,
        adaptive_fallback: false,
        ..Default::default()
    }
}

fn start(listener: tokio::net::TcpListener, handler: Handler, max_streams: u32) -> Server {
    let (tx, controls) = mpsc::unbounded_channel();
    let (request_tx, requests) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                accept = ZeroCopyIo::accept(&listener, config()) => {
                    let (io, _) = accept.unwrap();
                    let service = echo_server::EchoServer::new(handler.clone());
                    let request_tx = request_tx.clone();
                    let service = hyper_zerocopy::service::service_fn(move |request: http::Request<hyper_zerocopy::body::Incoming>| {
                        let _ = request_tx.send((request.uri().clone(), request.headers().clone()));
                        let mut service = service.clone();
                        async move { service.call(request).await }
                    });
                    let (graceful, stop) = oneshot::channel();
                    let abort = connections.spawn(async move {
                        let mut builder = hyper_zerocopy::server::conn::http2::Builder::new(Executor);
                        builder.max_concurrent_streams(max_streams);
                        let connection = builder.serve_connection(io, service);
                        tokio::pin!(connection);
                        tokio::select! {
                            result = &mut connection => { let _ = result; }
                            result = stop => {
                                if result.is_ok() {
                                    connection.as_mut().graceful_shutdown();
                                    let _ = connection.await;
                                } else {
                                    let _ = connection.await;
                                }
                            }
                        }
                    });
                    if tx.send(Control { abort, graceful }).is_err() { break; }
                }
                _ = connections.join_next(), if !connections.is_empty() => {}
            }
        }
    });
    Server { task, controls, requests }
}
#[derive(Clone)]
struct Executor;
impl<F: std::future::Future + Send + 'static> hyper_zerocopy::rt::Executor<F> for Executor
where
    F::Output: Send + 'static,
{
    fn execute(&self, future: F) {
        tokio::spawn(future);
    }
}

fn message(id: u64) -> Message {
    Message {
        id,
        data: vec![id as u8; 32768],
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_balancer_adds_removes_and_recovers_endpoints() {
    use tonic::transport::channel::Change;
    tokio::time::timeout(Duration::from_secs(15), async {
        let first = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let second = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let first_endpoint = Endpoint::from_shared(format!("http://{}", first.local_addr().unwrap())).unwrap();
        let second_endpoint = Endpoint::from_shared(format!("http://{}", second.local_addr().unwrap())).unwrap();
        let first_handler = Handler::default();
        let second_handler = Handler::default();
        let _first_server = start(first, first_handler.clone(), 16);
        let _second_server = start(second, second_handler.clone(), 16);
        let (channel, discovery) = AutoChannel::balance_channel(
            4,
            ChannelOptions {
                kernel_zero_copy: true,
                ..Default::default()
            },
        );
        let metrics = channel.kernel_metrics().unwrap();
        discovery.send(Change::Insert(1usize, first_endpoint.clone())).await.unwrap();
        let mut client = echo_client::EchoClient::new(channel);
        client.echo(&message(1)).await.unwrap();
        metrics.wait_for_idle().await.unwrap();
        let first_stats = metrics.snapshot();
        assert!(first_stats.fallbacks > 0);
        discovery.send(Change::Insert(2, second_endpoint)).await.unwrap();
        for _ in 0..64 {
            assert_eq!(client.echo(&message(1)).await.unwrap().into_inner(), message(1));
        }
        assert_eq!(metrics.connections(), 2);
        assert!(first_handler.calls.load(Ordering::Relaxed) > 0);
        assert!(second_handler.calls.load(Ordering::Relaxed) > 0);
        metrics.wait_for_idle().await.unwrap();
        assert!(
            metrics.snapshot().send_calls > first_stats.send_calls,
            "one endpoint's downgrade must not disable another"
        );
        discovery.send(Change::Remove(1)).await.unwrap();
        let before = first_handler.calls.load(Ordering::Relaxed);
        for _ in 0..8 {
            client.echo(&message(2)).await.unwrap();
        }
        assert_eq!(first_handler.calls.load(Ordering::Relaxed), before);
        discovery.send(Change::Remove(2)).await.unwrap();
        discovery.send(Change::Insert(1, first_endpoint)).await.unwrap();
        client.echo(&message(3)).await.unwrap();
        assert_eq!(metrics.connections(), 3);
        metrics.wait_for_idle().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn balanced_tls_endpoint_uses_encrypted_fallback() {
    use tonic::transport::Certificate;
    use tonic::transport::ClientTlsConfig;
    use tonic::transport::Identity;
    use tonic::transport::ServerTlsConfig;
    tokio::time::timeout(Duration::from_secs(15), async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let rcgen::CertifiedKey { cert, signing_key } = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let certificate = cert.pem();
        let endpoint = Endpoint::from_shared(format!("https://{address}"))
            .unwrap()
            .tls_config(ClientTlsConfig::new().domain_name("localhost").ca_certificate(Certificate::from_pem(&certificate)))
            .unwrap();
        let (stop, stopped) = oneshot::channel();
        let server = tokio::spawn(
            tonic::transport::Server::builder()
                .tls_config(ServerTlsConfig::new().identity(Identity::from_pem(&certificate, signing_key.serialize_pem())))
                .unwrap()
                .add_service(echo_server::EchoServer::new(Handler::default()))
                .serve_with_incoming_shutdown(tokio_stream::wrappers::TcpListenerStream::new(listener), async {
                    let _ = stopped.await;
                }),
        );
        let channel = AutoChannel::balance_list(
            [endpoint],
            ChannelOptions {
                kernel_zero_copy: true,
                ..Default::default()
            },
        );
        let mut client = echo_client::EchoClient::new(channel.clone());
        assert_eq!(client.echo(&message(1)).await.unwrap().into_inner(), message(1));
        assert!(channel.fallback_reason().unwrap().contains("TLS"));
        assert_eq!(channel.kernel_metrics().unwrap().connections(), 0);
        drop(client);
        drop(channel);
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
    })
    .await
    .unwrap();
}

async fn retry(client: &mut echo_client::EchoClient<ZeroCopyChannel>, value: &Message) {
    for _ in 0..100 {
        if let Ok(response) = client.echo(value).await {
            assert_eq!(response.into_inner(), *value);
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("channel failed to recover");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clones_share_connection_and_reconnect_after_disconnect() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = Endpoint::from_shared(format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let mut server = start(listener, Handler::default(), 2);
        let channel = ZeroCopyChannel::connect_endpoint(endpoint.buffer_size(2).concurrency_limit(4), config()).await.unwrap();
        let metrics = channel.metrics();
        let control = server.controls.recv().await.unwrap();
        // A completed exchange ensures receipt of the peer's initial SETTINGS;
        // HTTP/2 otherwise permits requests before its stream limit is known.
        echo_client::EchoClient::new(channel.clone()).echo(&message(99)).await.unwrap();
        let mut requests = tokio::task::JoinSet::new();
        for id in 0..16 {
            let mut client = echo_client::EchoClient::new(channel.clone());
            requests.spawn(async move {
                assert_eq!(client.echo(&message(id)).await.unwrap().into_inner(), message(id));
            });
        }
        while let Some(result) = requests.join_next().await {
            result.unwrap();
        }
        assert_eq!(metrics.connections(), 1, "clones must not open independent connections");
        control.abort.abort();
        let mut client = echo_client::EchoClient::new(channel);
        retry(&mut client, &message(20)).await;
        assert_eq!(metrics.connections(), 2);
        let _replacement = server.controls.recv().await.unwrap();
        drop(client);
        metrics.wait_for_idle().await.unwrap();
        let stats = metrics.snapshot();
        assert!(stats.send_calls > 0);
        assert_eq!(stats.pending_sends, 0);
        assert_eq!(
            stats.submitted_bytes,
            stats.completed_with_copy_flag + stats.completed_without_copy_flag
        );
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_inflight_rpc_is_not_replayed_and_channel_recovers() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = Endpoint::from_shared(format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let handler = Handler::default();
        let mut server = start(listener, handler.clone(), 16);
        let channel = ZeroCopyChannel::connect_endpoint(endpoint, config()).await.unwrap();
        let control = server.controls.recv().await.unwrap();
        let mut client = echo_client::EchoClient::new(channel.clone());
        let pending = tokio::spawn(async move { client.echo(&message(42)).await });
        handler.entered.notified().await;
        control.abort.abort();
        assert!(pending.await.unwrap().is_err());
        assert_eq!(
            handler.calls.load(Ordering::Relaxed),
            1,
            "possibly executed RPC must never be replayed"
        );
        let mut client = echo_client::EchoClient::new(channel);
        retry(&mut client, &message(3)).await;
        assert_eq!(handler.calls.load(Ordering::Relaxed), 2);
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lazy_channel_survives_initial_refusal_and_server_restart() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let endpoint = Endpoint::from_shared(format!("http://{address}")).unwrap().connect_timeout(Duration::from_secs(1));
        let channel = ZeroCopyChannel::connect_lazy(endpoint, config()).unwrap();
        let metrics = channel.metrics();
        assert_eq!(metrics.connections(), 0);
        let mut client = echo_client::EchoClient::new(channel);
        assert!(client.echo(&message(1)).await.is_err());
        let listener = tokio::net::TcpListener::bind(address).await.unwrap();
        let server = start(listener, Handler::default(), 16);
        retry(&mut client, &message(2)).await;
        drop(server);
        // Wait until a refused/disconnected call is observed, then restart.
        for _ in 0..100 {
            if client.echo(&message(3)).await.is_err() {
                break;
            }
            tokio::task::yield_now().await;
        }
        let listener = tokio::net::TcpListener::bind(address).await.unwrap();
        let _restarted = start(listener, Handler::default(), 16);
        retry(&mut client, &message(4)).await;
        assert_eq!(metrics.connections(), 2);
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graceful_goaway_drains_old_rpc_while_new_connection_serves_calls() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = Endpoint::from_shared(format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let handler = Handler::default();
        let mut server = start(listener, handler.clone(), 16);
        let channel = ZeroCopyChannel::connect_endpoint(endpoint, config()).await.unwrap();
        let metrics = channel.metrics();
        let control = server.controls.recv().await.unwrap();
        let mut old = echo_client::EchoClient::new(channel.clone());
        let pending = tokio::spawn(async move { old.echo(&message(42)).await });
        handler.entered.notified().await;
        control.graceful.send(()).unwrap();
        let mut client = echo_client::EchoClient::new(channel);
        for _ in 0..100 {
            retry(&mut client, &message(5)).await;
            if metrics.connections() == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(metrics.connections(), 2);
        assert!(!pending.is_finished(), "old stream must remain alive while draining");
        handler.release.notify_one();
        assert_eq!(pending.await.unwrap().unwrap().into_inner(), message(42));
        metrics.wait_for_idle().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_channel_applies_endpoint_policies_without_disabling_owned_transport() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = Endpoint::from_shared(format!("http://{}", listener.local_addr().unwrap()))
            .unwrap()
            .origin("http://virtual.test".parse().unwrap())
            .user_agent("manager-test")
            .unwrap()
            .timeout(Duration::from_millis(300))
            .concurrency_limit(2)
            .rate_limit(100, Duration::from_secs(1))
            .buffer_size(4)
            .connect_timeout(Duration::from_secs(1))
            .tcp_nodelay(false)
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .initial_stream_window_size(65535)
            .initial_connection_window_size(131070)
            .http2_keep_alive_interval(Duration::from_secs(1))
            .keep_alive_timeout(Duration::from_secs(1))
            .keep_alive_while_idle(true)
            .http2_adaptive_window(true)
            .max_frame_size(32768)
            .http2_header_table_size(2048)
            .http2_max_header_list_size(16384)
            .local_address(Some("127.0.0.1".parse().unwrap()));
        let handler = Handler::default();
        let mut server = start(listener, handler.clone(), 16);
        let channel = AutoChannel::connect(
            endpoint,
            ChannelOptions {
                kernel_zero_copy: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let metrics = channel.kernel_metrics().expect("configured endpoint must keep owned transport");
        let mut client = echo_client::EchoClient::new(channel.clone());
        assert_eq!(client.echo(&message(1)).await.unwrap().into_inner(), message(1));
        let (uri, headers) = server.requests.recv().await.unwrap();
        assert_eq!(uri.authority().unwrap(), "virtual.test");
        assert!(headers["user-agent"].to_str().unwrap().starts_with("manager-test"));
        assert!(
            client.echo(&message(42)).await.is_err(),
            "endpoint timeout must cancel a stalled RPC"
        );
        assert_eq!(metrics.connections(), 1);
        // Loopback forces adaptive fallback; its warning/counters survive reconnect.
        metrics.wait_for_idle().await.unwrap();
        assert!(channel.fallback_reason().is_some());
        let old_calls = metrics.snapshot().send_calls;
        server.controls.recv().await.unwrap().abort.abort();
        for _ in 0..100 {
            if client.echo(&message(2)).await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(metrics.connections(), 2);
        assert_eq!(
            metrics.snapshot().send_calls,
            old_calls,
            "reconnect must remember adaptive fallback"
        );
        assert!(!metrics.snapshot().enabled);
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_queued_call_is_not_sent_and_last_clone_closes_connection() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint =
            Endpoint::from_shared(format!("http://{}", listener.local_addr().unwrap())).unwrap().concurrency_limit(1).buffer_size(1);
        let handler = Handler::default();
        let mut server = start(listener, handler.clone(), 1);
        let channel = ZeroCopyChannel::connect_endpoint(endpoint, config()).await.unwrap();
        let metrics = channel.metrics();
        let _control = server.controls.recv().await.unwrap();
        let mut first = echo_client::EchoClient::new(channel.clone());
        let pending = tokio::spawn(async move { first.echo(&message(42)).await });
        handler.entered.notified().await;
        let snapshot = proto_rs::EncodedSnapshot::new(&message(6));
        let mut second = echo_client::EchoClient::new(channel.clone());
        let queued = tokio::spawn(async move { second.echo(snapshot).await });
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(
            handler.calls.load(Ordering::Relaxed),
            1,
            "concurrency limit must backpressure clones"
        );
        queued.abort();
        assert!(queued.await.unwrap_err().is_cancelled());
        handler.release.notify_one();
        pending.await.unwrap().unwrap();
        let mut client = echo_client::EchoClient::new(channel.clone());
        assert_eq!(client.echo(&message(7)).await.unwrap().into_inner(), message(7));
        assert_eq!(handler.calls.load(Ordering::Relaxed), 2, "cancelled queued call must not run");
        // Final-owner recycling, including cross-thread drops, is covered by
        // the TLS pool tests; cancellation must still release this request.
        drop(channel);
        assert!(metrics.snapshot().enabled, "one client clone remains");
        drop(client);
        while metrics.snapshot().enabled {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        metrics.wait_for_idle().await.unwrap();
        assert_eq!(metrics.snapshot().completion_errors, 0);
    })
    .await
    .unwrap();
}
