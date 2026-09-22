use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use std::time::Instant;

use hyper_util::client::legacy::connect::HttpConnector;
use tonic::body::Body;
use tonic::codegen::Service;
use tonic::transport::Channel;
use tonic::transport::Endpoint;

use super::ConnectError;
use super::ZeroCopyChannelMetrics;
use super::ZeroCopyConfig;
use super::ZeroCopyIo;
use super::ZeroCopyMetrics;

type BoxFuture<T> = Pin<Box<dyn Future<Output = Result<T, ConnectError>> + Send>>;

/// Owned HTTP/2 with the regular Tonic channel manager: cheap shared clones,
/// bounded request buffering, reconnects, endpoint policies and backpressure.
/// Requests already dispatched are never replayed. TLS requires AutoChannel's
/// normal encrypted fallback. Like one Tonic Endpoint, one HTTP/2 connection
/// multiplexes RPCs and is replaced when closed; this is not a TCP pool per RPC.
#[derive(Clone)]
pub struct ZeroCopyChannel {
    inner: Channel,
    metrics: ZeroCopyChannelMetrics,
}

impl ZeroCopyChannel {
    pub async fn connect(origin: http::Uri, config: ZeroCopyConfig) -> Result<Self, ConnectError> {
        Self::connect_endpoint(Endpoint::from(origin), config).await
    }

    /// Preserve all configured plaintext endpoint policies, including DNS/TCP,
    /// HTTP/2 keepalive/flow control, origin, user agent, limits and executor.
    pub async fn connect_endpoint(endpoint: Endpoint, config: ZeroCopyConfig) -> Result<Self, ConnectError> {
        Self::connect_with_warning(endpoint, config, Arc::new(OnceLock::new())).await
    }

    pub(crate) async fn connect_with_warning(
        endpoint: Endpoint,
        config: ZeroCopyConfig,
        warning: Arc<OnceLock<&'static str>>,
    ) -> Result<Self, ConnectError> {
        let metrics = ZeroCopyChannelMetrics::with_warning(warning);
        let factory = Factory::new(&endpoint, config, metrics.clone())?;
        let inner = Channel::connect_with_service(factory, endpoint).await?;
        Ok(Self { inner, metrics })
    }

    /// Validate immediately, but defer DNS/TCP/HTTP2 setup until first use.
    pub fn connect_lazy(endpoint: Endpoint, config: ZeroCopyConfig) -> Result<Self, ConnectError> {
        let metrics = ZeroCopyChannelMetrics::default();
        let factory = Factory::new(&endpoint, config, metrics.clone())?;
        Ok(Self {
            inner: Channel::new_with_service(factory, endpoint),
            metrics,
        })
    }

    /// Use an already connected owned socket first, then reconnect to `origin`
    /// with the same zero-copy configuration. The descriptor remains private.
    pub async fn handshake(io: ZeroCopyIo, origin: http::Uri) -> Result<Self, ConnectError> {
        let endpoint = Endpoint::from(origin);
        let metrics = ZeroCopyChannelMetrics::default();
        let mut factory = Factory::new(&endpoint, io.config(), metrics.clone())?;
        factory.first = Some(io);
        Ok(Self {
            inner: Channel::connect_with_service(factory, endpoint).await?,
            metrics,
        })
    }

    pub fn metrics(&self) -> ZeroCopyChannelMetrics {
        self.metrics.clone()
    }

    pub(crate) fn into_channel(self) -> Channel {
        self.inner
    }
}

impl Service<http::Request<Body>> for ZeroCopyChannel {
    type Response = http::Response<Body>;
    type Error = tonic::transport::Error;
    type Future = <Channel as Service<http::Request<Body>>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, request: http::Request<Body>) -> Self::Future {
        self.inner.call(request)
    }
}

pub(crate) struct Factory {
    connector: HttpConnector,
    endpoint: Endpoint,
    config: ZeroCopyConfig,
    metrics: ZeroCopyChannelMetrics,
    first: Option<ZeroCopyIo>,
    previous: Arc<Mutex<Option<ZeroCopyMetrics>>>,
}

impl Factory {
    pub(crate) fn new(endpoint: &Endpoint, config: ZeroCopyConfig, metrics: ZeroCopyChannelMetrics) -> Result<Self, ConnectError> {
        config.validate()?;
        Ok(Self {
            connector: endpoint.owned_http_connector()?,
            endpoint: endpoint.clone(),
            config,
            metrics,
            first: None,
            previous: Arc::new(Mutex::new(None)),
        })
    }
}

impl Service<http::Uri> for Factory {
    type Response = SendRequest;
    type Error = ConnectError;
    type Future = BoxFuture<SendRequest>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.connector.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, uri: http::Uri) -> Self::Future {
        let first = self.first.take();
        let connect = if first.is_none() { Some(self.connector.call(uri)) } else { None };
        let endpoint = self.endpoint.clone();
        let config = self.config;
        let metrics = self.metrics.clone();
        let previous = Arc::clone(&self.previous);
        Box::pin(async move {
            let io = if let Some(io) = first {
                io
            } else {
                let socket = connect.expect("fresh connection requested when no initial socket exists").await?.into_inner();
                // Remember an adaptive downgrade across reconnects instead
                // of retrying expensive copied completions on every socket.
                let ordinary = config.allow_fallback
                    && config.adaptive_fallback
                    && previous
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .as_ref()
                        .is_some_and(|m| m.snapshot().fallbacks != 0);
                ZeroCopyIo::from_connector(socket, config, ordinary)?
            };
            let socket_metrics = io.metrics();
            socket_metrics.set_fallback_warning(Arc::clone(&metrics.warning));
            if !socket_metrics.snapshot().enabled {
                crate::grpc::channel::warn_once(
                    &metrics.warning,
                    "kernel zero-copy unavailable or disabled after a copied completion",
                );
            }
            let settings = endpoint.owned_http2_config();
            let mut builder = hyper_zerocopy::client::conn::http2::Builder::new(EndpointExecutor(endpoint.clone()));
            builder
                .initial_stream_window_size(settings.stream_window)
                .initial_connection_window_size(settings.connection_window)
                .keep_alive_interval(settings.keep_alive_interval)
                .timer(TokioTimer);
            if let Some(v) = settings.max_frame_size {
                builder.max_frame_size(v);
            }
            if let Some(v) = settings.keep_alive_timeout {
                builder.keep_alive_timeout(v);
            }
            if let Some(v) = settings.keep_alive_while_idle {
                builder.keep_alive_while_idle(v);
            }
            if let Some(v) = settings.adaptive_window {
                builder.adaptive_window(v);
            }
            if let Some(v) = settings.header_table_size {
                builder.header_table_size(v);
            }
            if let Some(v) = settings.max_header_list_size {
                builder.max_header_list_size(v);
            }
            let (sender, connection) = builder.handshake(io).await?;
            *previous.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(socket_metrics.clone());
            metrics.register(socket_metrics);
            endpoint.execute_owned(Box::pin(async move {
                if let Err(error) = connection.await {
                    tracing::debug!(%error, "owned HTTP/2 connection ended");
                }
            }));
            Ok(SendRequest(sender))
        })
    }
}

pub(crate) struct SendRequest(hyper_zerocopy::client::conn::http2::SendRequest<Body>);
impl Service<http::Request<Body>> for SendRequest {
    type Response = http::Response<Body>;
    type Error = ConnectError;
    type Future = BoxFuture<Self::Response>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx).map_err(Into::into)
    }
    fn call(&mut self, request: http::Request<Body>) -> Self::Future {
        let future = self.0.send_request(request);
        Box::pin(async move { Ok(future.await?.map(Body::new)) })
    }
}

#[derive(Clone)]
struct EndpointExecutor(Endpoint);
impl<F: Future<Output = ()> + Send + 'static> hyper_zerocopy::rt::Executor<F> for EndpointExecutor {
    fn execute(&self, future: F) {
        self.0.execute_owned(Box::pin(future));
    }
}

#[derive(Clone)]
struct TokioTimer;
impl hyper_zerocopy::rt::Timer for TokioTimer {
    fn sleep(&self, duration: Duration) -> Pin<Box<dyn hyper_zerocopy::rt::Sleep>> {
        Box::pin(TokioSleep {
            inner: tokio::time::sleep(duration),
        })
    }
    fn sleep_until(&self, deadline: Instant) -> Pin<Box<dyn hyper_zerocopy::rt::Sleep>> {
        Box::pin(TokioSleep {
            inner: tokio::time::sleep_until(deadline.into()),
        })
    }
    fn reset(&self, sleep: &mut Pin<Box<dyn hyper_zerocopy::rt::Sleep>>, deadline: Instant) {
        if let Some(sleep) = sleep.as_mut().downcast_mut_pin::<TokioSleep>() {
            sleep.project().inner.reset(deadline.into());
        } else {
            *sleep = self.sleep_until(deadline);
        }
    }
}
pin_project_lite::pin_project! {
    struct TokioSleep { #[pin] inner: tokio::time::Sleep }
}
impl Future for TokioSleep {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        self.project().inner.poll(cx)
    }
}
impl hyper_zerocopy::rt::Sleep for TokioSleep {}
