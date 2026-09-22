//! Capability-selected transport. Both backends share Tonic's channel manager.
//! TLS always uses the configured encrypted endpoint, never plaintext.
use std::sync::Arc;
use std::sync::OnceLock;
use std::task::Context;
use std::task::Poll;

use tonic::body::Body;
use tonic::codegen::Service;
use tonic::transport::Channel;
use tonic::transport::Endpoint;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone, Copy, Debug, Default)]
pub struct ChannelOptions {
    /// Request owned HTTP/2 for plaintext endpoints. TLS/unsupported builds use
    /// the regular channel; both preserve configured policies and reconnects.
    pub kernel_zero_copy: bool,
    #[cfg(all(feature = "linux-zerocopy", target_os = "linux"))]
    /// AutoChannel always enables ordinary/adaptive fallback.
    pub zero_copy: super::zerocopy::ZeroCopyConfig,
}

/// Cloneable Tonic-managed service with one shared fallback warning.
#[derive(Clone)]
pub struct AutoChannel {
    inner: Channel,
    fallback: Arc<OnceLock<&'static str>>,
    #[cfg(all(feature = "linux-zerocopy", target_os = "linux"))]
    metrics: Option<super::zerocopy::ZeroCopyChannelMetrics>,
}

pub(crate) fn warn_once(state: &OnceLock<&'static str>, reason: &'static str) {
    if state.set(reason).is_ok() {
        tracing::warn!(
            reason,
            "kernel zero-copy unavailable; using ordinary writes (TLS remains enabled when requested)"
        );
    }
}

impl AutoChannel {
    /// Dynamic endpoint discovery using Tonic's own balancer. Each endpoint
    /// keeps its policies and reconnect state; TLS entries use normal transport.
    pub fn balance_channel<K>(
        capacity: usize,
        options: ChannelOptions,
    ) -> (Self, tokio::sync::mpsc::Sender<tonic::transport::channel::Change<K, Endpoint>>)
    where
        K: std::hash::Hash + Eq + Send + Clone + 'static,
    {
        let fallback = Arc::new(OnceLock::new());
        #[cfg(all(feature = "linux-zerocopy", target_os = "linux"))]
        if options.kernel_zero_copy {
            let metrics = super::zerocopy::ZeroCopyChannelMetrics::with_warning(Arc::clone(&fallback));
            let observed = metrics.clone();
            let warning = Arc::clone(&fallback);
            let mut config = options.zero_copy;
            config.allow_fallback = true;
            config.adaptive_fallback = true;
            let (inner, sender) = Channel::balance_channel_with_service(capacity, move |endpoint| {
                if let Some(reason) = endpoint.owned_transport_fallback_reason() {
                    warn_once(&warning, reason);
                    return None;
                }
                if let Ok(factory) = super::zerocopy::Factory::new(endpoint, config, observed.clone()) {
                    Some(factory)
                } else {
                    warn_once(
                        &warning,
                        "owned transport configuration failed; using the configured Tonic endpoint",
                    );
                    None
                }
            });
            return (
                Self {
                    inner,
                    fallback,
                    metrics: Some(metrics),
                },
                sender,
            );
        }
        #[cfg(not(all(feature = "linux-zerocopy", target_os = "linux")))]
        if options.kernel_zero_copy {
            warn_once(&fallback, "owned Linux transport is not enabled in this build");
        }
        let (inner, sender) = Channel::balance_channel(capacity);
        (
            Self {
                inner,
                fallback,
                #[cfg(all(feature = "linux-zerocopy", target_os = "linux"))]
                metrics: None,
            },
            sender,
        )
    }

    /// Lazily balance a fixed endpoint list. Clones share discovery and workers.
    pub fn balance_list(endpoints: impl IntoIterator<Item = Endpoint>, options: ChannelOptions) -> Self {
        let endpoints: Vec<_> = endpoints.into_iter().collect();
        let (channel, sender) = Self::balance_channel(endpoints.len().max(1), options);
        for endpoint in endpoints {
            sender
                .try_send(tonic::transport::channel::Change::Insert(endpoint.uri().clone(), endpoint))
                .expect("new balance channel has enough discovery capacity");
        }
        channel
    }

    pub async fn connect(endpoint: Endpoint, options: ChannelOptions) -> Result<Self, Error> {
        let fallback = Arc::new(OnceLock::new());
        if options.kernel_zero_copy {
            #[cfg(feature = "tonic-owned")]
            let reason = endpoint.owned_transport_fallback_reason();
            #[cfg(not(feature = "tonic-owned"))]
            let reason = Some("owned transport is not enabled in this build");
            if let Some(reason) = reason {
                warn_once(&fallback, reason);
            } else {
                #[cfg(all(feature = "linux-zerocopy", target_os = "linux"))]
                {
                    let mut config = options.zero_copy;
                    config.allow_fallback = true;
                    config.adaptive_fallback = true;
                    match super::zerocopy::ZeroCopyChannel::connect_with_warning(endpoint.clone(), config, Arc::clone(&fallback)).await {
                        Ok(channel) => {
                            let metrics = Some(channel.metrics());
                            return Ok(Self {
                                inner: channel.into_channel(),
                                fallback,
                                metrics,
                            });
                        }
                        Err(_) => warn_once(&fallback, "owned transport setup failed; using the configured Tonic endpoint"),
                    }
                }
                #[cfg(not(all(feature = "linux-zerocopy", target_os = "linux")))]
                warn_once(&fallback, "owned Linux transport is not enabled in this build");
            }
        }
        Ok(Self {
            inner: endpoint.connect().await?,
            fallback,
            #[cfg(all(feature = "linux-zerocopy", target_os = "linux"))]
            metrics: None,
        })
    }

    /// Known setup/runtime fallback reason, shared across clones and reconnects.
    pub fn fallback_reason(&self) -> Option<&'static str> {
        self.fallback.get().copied()
    }

    #[cfg(all(feature = "linux-zerocopy", target_os = "linux"))]
    pub fn kernel_metrics(&self) -> Option<super::zerocopy::ZeroCopyChannelMetrics> {
        self.metrics.clone()
    }
}

impl Service<http::Request<Body>> for AutoChannel {
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    use super::*;

    struct Counter(Arc<AtomicUsize>);
    impl tracing::Subscriber for Counter {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::WARN {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    #[test]
    fn fallback_warning_is_emitted_once_across_clones() {
        let count = Arc::new(AtomicUsize::new(0));
        let _subscriber = tracing::subscriber::set_default(Counter(Arc::clone(&count)));
        let warning = Arc::new(OnceLock::new());
        for reason in ["setup", "setup", "runtime", "runtime"] {
            warn_once(&warning.clone(), reason);
        }
        assert_eq!(count.load(Ordering::Relaxed), 1);
        assert_eq!(warning.get(), Some(&"setup"));
    }
}
