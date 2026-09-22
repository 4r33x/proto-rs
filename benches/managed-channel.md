# Managed owned channel — 2026-09-22

The optional kernel path now uses Tonic's existing connection manager, not a
second independently implemented manager. The factory supplies owned Hyper/h2
connections; Tonic supplies reconnect state, request buffering/backpressure,
endpoint policy layers and balancing/discovery. Both AutoChannel backends return
Tonic's own response future, without per-poll metrics snapshots or a backend enum.

For usage and remaining kernel/TLS/resource limitations, see
[Linux owned transport](linux-zerocopy.md#managed-client-lifecycle).

## Focused localhost comparison

Criterion, 30 samples, 1-second warmup, 3-second measurement, CPU affinity 2,3,
two Tokio workers. The same Tonic server receives and validates the whole payload,
then returns its length. Accepted sockets explicitly enable TCP_NODELAY, matching
normal Tonic TCP serving. Both paths synchronously prepare borrowed messages,
reuse encoding allocations and use the same connection manager. Connections and
adaptive kernel fallback are warmed before timing. Units are Criterion sample
means (not its separate fitted-time estimate); ±4% is noise.

| Payload | Regular Tonic | Managed owned/adaptive | Difference |
| --- | ---: | ---: | ---: |
| 16 B | 19.87 µs | 20.53 µs | +3.3%, noise |
| 16 KiB | 26.74 µs | 24.67 µs | −7.8% |
| 256 KiB | 135.33 µs | 120.00 µs | −11.3% |

No regression beyond the specified noise threshold in this subset. These are
sequential unary roundtrips, not concurrency saturation, reconnect latency,
TLS/compression throughput or physical-NIC zero-copy measurements. Loopback
copies: the owned path uses ordinary sends after adaptive downgrade. Advantages
here cannot be attributed to kernel zero-copy. A first unpinned run with a custom
listener that did not set TCP_NODELAY had noisy, millisecond-scale larger RPCs;
it is not used for this comparison.

```sh
taskset -c 2,3 cargo bench -p proto_rs --offline \
  --features linux-zerocopy,tonic-gzip --bench zerocopy_channel -- \
  --save-baseline managed-nodelay-pin23 --noise-threshold 0.04
```

## Verification

Dedicated tests cover shared clones, peer stream limits after SETTINGS,
disconnect recovery, lazy initial refusal/server restart, no replay of failed
in-flight RPCs, GOAWAY draining alongside replacement connections, queued
cancellation/snapshot reclamation, last-clone shutdown, endpoint metadata and
timeout policies, adaptive fallback across reconnects, dynamic endpoint changes,
per-endpoint fallback isolation, and encrypted balanced TLS fallback. Aggregate
metrics tests verify retirement of closed/drained entries and retention of
pending completion statistics. Existing plaintext/gzip/TLS, allocation identity,
kernel completion and runtime-shutdown tests remain enabled.

Final validation: 216 root tests with `linux-zerocopy,tonic-gzip`, 103 focused
stable tests, all-target/minimal/non-kernel builds and library Clippy pass
(pre-existing warnings remain). Miri passes the two aggregate-metrics lifetime
tests and the pre-submission worker-setup fallback model. These model tests do
not execute real kernel syscalls; the socket behavior is covered by integration
tests, not by Miri. The full upstream vendored test suites were not rerun.

This is not a claim of zero downsides for all workloads. Kernel completion threads,
page retention, optional copies and exceptional failure quarantine still have
costs. In-flight RPCs may fail during disconnects, just as with normal Tonic;
the manager does not blindly replay requests or add a new retry/backoff policy.
