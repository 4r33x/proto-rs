//! Opt-in Linux owned-buffer HTTP/2 transport.
//!
//! Uses isolated Hyper/h2 forks, not a global dependency patch. Large Tonic Bytes
//! allocations reach sendmsg without a payload copy; Linux may still copy, which
//! is reported through completion metrics. Plain HTTP/2 and Tonic gzip are
//! supported. HTTPS is rejected: this transport does NOT bypass TLS or implement
//! ownership-aware rustls/kTLS. Use the normal Tonic transport for HTTPS.
//!
//! Each enabled connection has a completion thread that outlives Tokio shutdown.
//! Queue limits bound submitted ranges, not total backing allocation size. On an
//! unrecoverable completion-reader failure, unresolved owners/descriptors are
//! quarantined until process exit rather than risking premature memory reuse.

mod channel;
mod metrics;
mod socket;
use std::future::Future;

pub(crate) use channel::Factory;
pub use channel::ZeroCopyChannel;
pub use hyper_zerocopy::Error as HttpError;
pub use hyper_zerocopy::body::Incoming;
pub use metrics::ZeroCopyChannelMetrics;
pub use socket::ZeroCopyConfig;
pub use socket::ZeroCopyIo;
pub use socket::ZeroCopyMetrics;
pub use socket::ZeroCopySnapshot;
use tonic::codegen::Service;

pub type ConnectError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
struct TokioExecutor;

impl<F> hyper_zerocopy::rt::Executor<F> for TokioExecutor
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, future: F) {
        tokio::spawn(future);
    }
}

/// Serve a generated Tonic service on one accepted plain HTTP/2 connection.
/// The caller owns listening, connection limits, shutdown, and request timeouts.
/// Canceling this future closes TCP but its completion thread retains outstanding
/// output buffers. This helper is not a replacement for all Tonic Server layers.
pub async fn serve_connection<S>(io: ZeroCopyIo, service: S) -> Result<(), HttpError>
where
    S: Service<http::Request<Incoming>, Response = http::Response<tonic::body::Body>> + Clone + Send + 'static,
    S::Future: Send,
    S::Error: Into<ConnectError> + Send,
{
    let service = hyper_zerocopy::service::service_fn(move |request| {
        let mut service = service.clone();
        async move {
            std::future::poll_fn(|cx| service.poll_ready(cx)).await?;
            service.call(request).await
        }
    });
    hyper_zerocopy::server::conn::http2::Builder::new(TokioExecutor).serve_connection(io, service).await
}
