# Bounded array allocation pool — 2026-09-22

Historical explicit-pool experiment, superseded by the
[automatic TLS bitset pool](tls-bitset-pool-2026-09-22.md). The measurements below
describe the earlier shared array queue, not current TLS slot leasing.

This replaces the [mutex/size-class experiment](pool-simplification-2026-09-22.md)
with the requested simple queue. Owned output keeps an explicitly shared pool;
synchronous destination encoding keeps its separate single TLS scratch buffer.

## Contract

- One `crossbeam_queue::ArrayQueue<RevVec>`, bounded by `max_buffers`.
- Checkout pops once, growing the returned allocation if necessary.
- Final owner release clears and shrinks an oversized allocation to
  `max_buffer_capacity`, then pushes once. A full queue discards returns without
  evicting existing buffers. If already full, it skips unnecessary shrinking.
- No size classes, mutex, aggregate byte counter, TLS configuration lookup,
  retirement flags, owner-count tests or fences around pool ownership.
- The queue's own atomic retry protocol remains; operations are not wait-free.

```rust,ignore
let server = EchoServer::new(service).with_encode_pool(proto_rs::EncodePoolConfig {
    max_buffers: 8,
    max_buffer_capacity: 1024 * 1024,
});
```

Defaults are eight idle allocations capped at 32 MiB each. Either zero disables
retention. The cap includes framing and is not a message-size limit. Live frames
are never shrunk. Allocator overhead and in-flight allocations are not bounded
by these settings. An allocator that cannot satisfy a shrink causes the buffer
to be discarded. Memory is not zeroized.

`max_retained_bytes` is replaced by the **per-allocation**
`max_buffer_capacity`; the first argument of `SnapshotPool::new(capacity, count)`
has that new meaning too. `retained_buffers()` replaces `retained_bytes()` and
reports only idle count. Clients, servers and pool clones share these limits.

## Verification

Passed workspace tests with `linux-zerocopy,tonic-gzip`, all-feature workspace
tests and all-target checks, minimal library tests, plain-Tonic feature checks,
library Clippy and pool-benchmark Clippy. Existing manifest and unrelated const-fn
warnings remain. The plain-Tonic check uses the workspace's patched dependency;
it is not a new external registry compatibility test.

Regression tests cover FIFO reuse and growth, full-queue drops, independent
allocation caps without a total byte budget, zero/tiny caps, trimming after final
remote release, concurrent count/capacity limits, trim/regrow writer invariants,
panic recovery, overlapping owners and service-creation configuration. Warm
allocation tests still permit only one Bytes owner-record allocation per encode.

Logs and scripts: `target/array-pool.ko0nBR/`. Source backups in `before/` preserve
the implementation at the beginning of this change.

## Measurements

Focused pool, Tonic-body and loopback channel results follow from the completed
sequential run. This is not a rerun of the unrelated protobuf and
varint suites. Criterion label: `array-pool-20260922`; the comparison label is
`simple-pool-20260922` from the earlier mutex experiment.

Pool cases use 30 samples, one-second warmup and three-second measurement;
Tonic-body cases use 50 samples with the same times. Pool/channel affinity is
CPUs 2,3 and body affinity CPU 2. Release build uses fat LTO and one codegen unit;
hardware/toolchain are unchanged from the preceding experiment. Compilation and
tests finish before timed measurements begin.

The new `trim_on_return` cases intentionally set an eight-byte retention cap,
including framing, so every message must grow and shrink. They measure that
policy's cost, not normal warm reuse. Set the cap to fit usual service outputs.

### Pool comparison

Arithmetic means from the saved Criterion estimates (not the terminal's slope
estimate). Percentages compare to the earlier mutex run; these are sequential
experiments, not simultaneous A/B trials.

| Case | Payload | Mutex | Array queue | Change |
| --- | ---: | ---: | ---: | ---: |
| shared_warm | 16 B | 51.81 ns | 42.18 ns | -18.6% |
| shared_warm | 16 KiB | 105.79 ns | 91.13 ns | -13.9% |
| overlapping_owner | 16 B | 50.15 ns | 41.96 ns | -16.3% |
| overlapping_owner | 16 KiB | 106.15 ns | 91.34 ns | -14.0% |
| remote_return | 16 B | 190.61 ns | 175.74 ns | -7.8% |
| remote_return | 16 KiB | 298.75 ns | 253.02 ns | -15.3% |
| contended_shared | 16 B | 27.59 µs/batch | 20.32 µs/batch | -26.3% |
| contended_shared | 16 KiB | 21.25 µs/batch | 32.37 µs/batch | 52.3% |

Contended cases use four workers on two CPUs, 64 messages per worker per timed
iteration (256 messages/batch). They include scheduling and queue contention.
Unpooled controls in that run took 2.87 µs/batch for 16 B and 14.63 µs/batch for
16 KiB: a shared pool is not universally faster than allocation.
Remote cases include bounded-channel handoff to the final-release thread.

A longer repeat (50 samples, two-second warmup, five-second measurement;
`array-pool-repeat-20260922`) measured shared contention at 22.21 µs/batch for
16 B and 33.71 µs/batch for 16 KiB. The unpooled controls were 2.99 and
17.39 µs/batch. The slower large-message contention case reproduced; control
drift also shows scheduler/allocator variability. The mutex baseline was not
rerun in this repeat. Estimates: `target/array-pool.ko0nBR/repeat-results.json`.

The eight-byte-cap `trim_on_return` cases took 79.50 ns (16 B) and 144.43 ns
(16 KiB), compared with 42.18 ns and 91.13 ns for normal warm reuse. These cases
intentionally exercise repeated growth and shrinking, not a typical cap.

### Tonic and loopback

| Direct Tonic body | Mutex | Array queue | Change |
| --- | ---: | ---: | ---: |
| small_unary | 102.62 ns | 99.49 ns | -3.0% |
| large_unary | 162.25 ns | 156.62 ns | -3.5% |
| small_stream | 4284.86 ns | 4212.17 ns | -1.7% |
| oversized_stream | 42354.83 ns | 37959.58 ns | -10.4% |

Three of these changes are below the configured 4% practical noise threshold;
only oversized-stream improvement exceeds it. This does not restore the older
TLS-local-spare timing (about 29.60 µs for that fixture); the explicit-pool
architecture and benchmark lifecycle also differ from that older implementation.
Raw previous-writer, cold-pool and destination-scratch controls were rerun too.

| Loopback payload | Ordinary Tonic | Owned/adaptive |
| --- | ---: | ---: |
| 16 B | 18.90 µs | 19.62 µs |
| 16384 B | 27.70 µs | 26.45 µs |
| 262144 B | 130.30 µs | 132.33 µs |

Loopback changes are mixed: owned/adaptive versus the earlier mutex run changed
by −4.4%, −1.8% and +12.9% respectively. Do not infer a general RPC speedup.
Loopback's adaptive path falls back to copying; these are not physical-network
kernel-zero-copy measurements. All 38 focused cases completed successfully.
Full per-case estimates are saved in `target/array-pool.ko0nBR/results.json`.
