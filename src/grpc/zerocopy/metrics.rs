use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

use super::ZeroCopyMetrics;
use super::ZeroCopySnapshot;

/// Channel-wide counters, including replaced connections still draining kernel
/// completions. Closed/drained entries are folded into totals, not kept forever.
#[derive(Clone, Default)]
pub struct ZeroCopyChannelMetrics {
    state: Arc<Mutex<State>>,
    pub(super) warning: Arc<OnceLock<&'static str>>,
}

#[derive(Default)]
struct State {
    connections: u64,
    retired: ZeroCopySnapshot,
    active: Vec<ZeroCopyMetrics>,
}

const fn add(total: &mut ZeroCopySnapshot, next: ZeroCopySnapshot) {
    total.enabled |= next.enabled;
    total.send_calls += next.send_calls;
    total.submitted_bytes += next.submitted_bytes;
    total.completed_without_copy_flag += next.completed_without_copy_flag;
    total.completed_with_copy_flag += next.completed_with_copy_flag;
    total.ordinary_send_bytes += next.ordinary_send_bytes;
    total.fallbacks += next.fallbacks;
    total.completion_errors += next.completion_errors;
    total.pending_sends += next.pending_sends;
    total.pending_bytes += next.pending_bytes;
}

impl State {
    fn collect(&mut self) -> ZeroCopySnapshot {
        let mut live = ZeroCopySnapshot::default();
        self.active.retain(|metrics| {
            let closed = metrics.is_closed();
            let mut snapshot = metrics.snapshot();
            snapshot.enabled &= !closed;
            if closed && snapshot.pending_sends == 0 {
                add(&mut self.retired, snapshot);
                false
            } else {
                add(&mut live, snapshot);
                true
            }
        });
        add(&mut live, self.retired);
        live
    }
}

impl ZeroCopyChannelMetrics {
    pub(crate) fn with_warning(warning: Arc<OnceLock<&'static str>>) -> Self {
        Self {
            warning,
            ..Self::default()
        }
    }

    pub(super) fn register(&self, metrics: ZeroCopyMetrics) {
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        state.collect();
        state.connections += 1;
        state.active.push(metrics);
    }

    /// Total successfully established connections, including replacements.
    pub fn connections(&self) -> u64 {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).connections
    }

    /// Observational totals, not an atomic transaction across connections.
    pub fn snapshot(&self) -> ZeroCopySnapshot {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).collect()
    }

    /// Stop producers first. Includes completions from replaced connections;
    /// wrap in a timeout if the network can stall. This is not delivery ACK.
    pub async fn wait_for_idle(&self) -> std::io::Result<()> {
        let active = {
            let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.collect().completion_errors != 0 {
                return Err(std::io::Error::other(
                    "zero-copy completion reader failed; outstanding buffers quarantined",
                ));
            }
            state.active.clone()
        };
        for metrics in active {
            metrics.wait_for_idle().await?;
        }
        Ok(())
    }
}
