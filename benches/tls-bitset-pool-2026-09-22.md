# Automatic TLS slot pooling — 2026-09-22

Historical implementation and measurements. The later [pool optimization](pool-optimization-2026-09-22.md)
removes the separate Arc lifetime counter while preserving the public TLS API.

This supersedes the explicit [array-queue pool](array-pool-2026-09-22.md).
One lazy pool belongs to each encoding thread; all services and encoding paths
on that thread use it automatically. No service, client, codec or sender stores
a pool handle. Standalone snapshots now use TLS too.

```rust
proto_rs::configure_encode_pool(proto_rs::EncodePoolConfig {
    max_buffers: 8,
    max_buffer_capacity: 1024 * 1024,
}).expect("configure once, before encoding starts on any thread");
```

Configuration is process-wide and immutable after this call or the first pooled
encode. Defaults are eight slots and a 32 MiB retained capacity per slot.
Either zero disables retention. Payload allocation is lazy. Limits are **per
thread**, not per process, and exclude live temporary allocations and metadata.
They do not limit message size or change speculative preallocation limits.

## Implementation and safety

The fixed slot array uses separate cache-padded `AtomicU64` availability masks.
Small pools spread across up to four masks; larger masks cover at most 64 slots
each. Unused bits are never set. A non-atomic TLS cursor remembers the last
successful mask; probes advance only when a mask is busy or a CAS races.
Checkout tries one Acquire CAS per nonempty mask, then falls back to an unpooled
allocation if all probes are busy or race. There is no eviction or waiting loop.

The buffer moves out of its slot into a lease. Final owner release clears and
trims it, restores that same slot and publishes one Release `fetch_or` bit update.
If shrinking cannot meet the cap, it frees the allocation and restores an empty
slot. Trimming can reallocate, and the lease still incurs Arc lifetime operations;
only the pool-state publication is a single atomic bit operation.

The `UnsafeCell` access invariant is exclusive ownership after successful CAS,
with restoration before release. All mask updates after construction are RMWs,
preserving release sequences across other bits' updates. Shared references into
slots never escape. An internal Arc lease keeps the originating pool alive after
TLS/thread destruction, until final snapshot/Bytes/transport release. Return never
accesses TLS. Acquisition during TLS teardown uses temporary storage.

Synchronous destination encoding uses exactly the same slots, returning its
lease before exit/unwind without creating Bytes ownership metadata. Caller-owned
raw encoding retains its existing destination semantics. There is no separate
scratch cache, queue, mutex, configuration LRU, byte counter, retirement flag,
owner-count test, or per-service configuration. Padding reduces false sharing;
it does not eliminate contention or guarantee a speedup.

Migration: replace explicit `SnapshotPool` usage with `EncodedSnapshot::new` or
`with_max_preallocation`, remove `with_encode_pool` builders, and configure once
at startup. `PrepareRequest::prepare_request` now takes only the reservation cap.
The Crossbeam queue dependency is removed; Crossbeam's cache padding remains.

## Validation

Workspace tests with `linux-zerocopy,tonic-gzip`, all-feature workspace tests and
all-target checks, minimal library tests, plain-Tonic feature checks, library
Clippy and pool-benchmark Clippy passed. Existing manifest and unrelated const-fn
warnings remain. The plain-Tonic check uses the workspace's patched dependency;
it is not a fresh external-registry compatibility test.

Tests cover partial masks and 64-bit boundaries, complete slot exhaustion,
no simultaneous ownership, cross-thread returns, per-slot trimming, panic
recovery, startup configuration, multiple services without pool handles,
reentrant encoding, and snapshots outliving their thread. Allocation tests still
require only one Bytes owner-record allocation per warm owned encode.

Miri passed the five pool ownership tests. A second run with schedule seed 17
passed all 11 minimal-feature encode tests, including snapshot and reentrant
encoding paths. The final locality adjustment passed the same 11 tests with
seed 29, plus another complete workspace/feature/Clippy validation matrix.
The final feature-enabled library suite passed 53 tests. The cold preallocation
allocation test holds every TLS slot so warm capacity cannot mask regressions.
Miri checks exercised executions, not an exhaustive concurrency proof.

Artifacts and source backups: `target/tls-bitset.Pcu3qz/`.

## Benchmark method

Focused Criterion runs are sequential, after validation and compilation.
Affinity is CPU 2 for Tonic bodies and CPUs 2,3 for pool/channel cases. Pool cases
use 30 samples, one-second warmup and three-second measurement; body cases use
50 samples with the same times. Release settings and hardware match the preceding
array-queue experiment. This is not a rerun of unrelated varint/protobuf suites.

`tls_warm` replaces `shared_warm`. `exhausted` holds every TLS slot before timing
so new outputs must allocate temporary storage; it still includes TLS mask probes
and is not identical to the former unpooled control. `parallel_tls` now gives
each of four workers its own TLS pool, unlike the old contended shared-pool test.
Each timed iteration is 64 messages per worker, or 256 messages per batch.
`remote_return` includes bounded-channel handoff and final release by a consumer
thread; its 32-message queue can exhaust the default eight slots.

`cold_pool` is replaced by `exhausted_pool` in the Tonic body benchmark: newly
constructed encoders now reuse TLS and are not cold pools. `tls_copy` replaces
`tls_scratch`; synchronous copying now uses the common slot cache. Duplicate
pooled/standalone snapshot variants were removed because both now use TLS.

Separate runs set an eight-byte watermark to force trim/regrow on every return,
and 64 slots to explore remote-return and parallel capacity effects. No runtime
reconfiguration or benchmark-only pool API is added to the library.

## Results

All 42 final focused cases completed: 14 pool, 16 Tonic-body, six loopback,
two forced-trim and four 64-slot cases. Labels are `tls-bitset-locality-20260922`,
`tls-bitset-locality-trim-20260922` and `tls-bitset-locality-slots64-20260922`.
Logs are in `target/tls-bitset.Pcu3qz/locality/`; JSON estimates named by label
are in its parent directory. Numbers below are arithmetic means from Criterion,
not the terminal's slope estimates. Comparisons are sequential runs, not
simultaneous A/B trials.

### Pool costs

| Case | Payload | Earlier array queue | Final TLS bitset |
| --- | ---: | ---: | ---: |
| tls_warm | 16 B | 42.18 ns | 45.43 ns |
| tls_warm | 16 KiB | 91.13 ns | 123.20 ns |
| overlapping_owner | 16 B | 41.96 ns | 45.40 ns |
| overlapping_owner | 16 KiB | 91.34 ns | 99.10 ns |
| remote_return | 16 B | 175.74 ns | 219.09 ns |
| remote_return | 16 KiB | 253.02 ns | 257.07 ns |
| parallel_tls | 16 B | 20.32 µs/batch | 6.38 µs/batch |
| parallel_tls | 16 KiB | 32.37 µs/batch | 13.48 µs/batch |

The parallel row changes ownership topology: one shared queue versus independent
TLS pools. Its improvement cannot be attributed solely to bitsets. Final
parallel exhausted-slot controls took 3.23 and 12.50 µs/batch
respectively; allocation can still be cheaper than pool synchronization.
Uncontended warm reuse regressed versus the queue, and tiny remote returns also
regressed. The larger remote-return difference is below the 4% practical noise
threshold. TLS removes inter-service pool setup, not all allocation or contention.

The first TLS implementation rotated masks after every checkout
(`tls-bitset-20260922`). Retaining the successful mask instead improved the
16 KiB overlapping-owner case from 131.56 to 99.10 ns,
but plain warm reuse remained around 121–123 ns. Do not conflate those fixtures
or assume every change comes from synchronization rather than buffer locality.

### Tonic and loopback

| Direct Tonic body | Earlier array queue | Final TLS bitset | Change |
| --- | ---: | ---: | ---: |
| small_unary | 99.49 ns | 94.69 ns | -4.8% |
| large_unary | 156.62 ns | 166.10 ns | 6.1% |
| small_stream | 4212.17 ns | 4234.03 ns | 0.5% |
| oversized_stream | 37959.58 ns | 29551.33 ns | -22.2% |

The oversized-stream case improved, but large unary regressed and small-stream
change is below the configured 4% practical noise threshold. The copied-destination
path now pays common-pool synchronization, unlike the former separate scratch
cache; raw previous-writer and exhausted-slot controls are included in the JSON.

| Loopback payload | Ordinary Tonic | Owned/adaptive |
| --- | ---: | ---: |
| 16 B | 20.38 µs | 20.43 µs |
| 16384 B | 26.18 µs | 26.29 µs |
| 262144 B | 130.05 µs | 128.47 µs |

Loopback/adaptive uses copying after fallback, not physical-network kernel
zero-copy. These mixed results do not establish a blanket RPC speedup.

### Capacity and trimming

| Case | 16 B | 16 KiB |
| --- | ---: | ---: |
| Eight-byte watermark, trim/regrow every output | 53.46 ns | 141.53 ns |
| 64-slot remote return | 160.74 ns | 258.07 ns |
| 64-slot parallel TLS | 5.84 µs/batch | 12.26 µs/batch |

The watermark changes retained capacity only. Set it to fit normal outputs to
avoid repeated growth/shrink costs; increasing slots also raises per-thread
retention and is not universally faster.
