# TLS pool optimization — 2026-09-22

This supersedes the implementation in [the first TLS bitset report](tls-bitset-pool-2026-09-22.md).
The public configuration, automatic sharing between services on a thread,
fixed slot count, overflow allocation and per-slot trimming are unchanged.

## Findings and changes

A five-second user-space `perf` sample of the original warm 16-byte snapshot
benchmark attributed 39.0% of cycles to buffer return and 30.6% to checkout.
Disassembly showed four pool atomic updates per lease: claim, Arc increment,
return publication and Arc decrement. Return also divided the slot index by
the mask width. Checkout wrote an empty descriptor into the slot before moving
another descriptor back on return.

The replacement uses the existing bitset for lifetime as well as slot ownership:

- Each independently owned shard has one cache-padded `AtomicU64`: its top bit
  belongs to TLS and its lower 63 bits represent active leases. The operation
  clearing the last bit destroys the shard. There is no separate pool Arc.
- Only the originating TLS thread can acquire slots. Other threads only clear
  bits, so a free bit remains free until that thread claims it. Acquire uses
  `fetch_or(Acquire)`, without CAS failure or retry on unrelated returns.
- Return restores the slot, then uses `fetch_sub(bit, AcqRel)`. The lease
  uniquely owns that set bit: subtraction clears it without borrowing from
  another bit. The old word identifies the final owner. Release-build x86-64
  assembly reduces this to `lock sub` plus a zero-result branch; acquisition
  uses `lock or`. Neither needs a compare-exchange retry loop.
- Slots contain `MaybeUninit<RevVec>` while leased. Checkout moves the descriptor
  out without overwriting it with an empty descriptor. Final return restores
  it before publishing availability. Destruction occurs only with all slots
  initialized. The lease stores the shard pointer and bit, avoiding division
  on return.
- Empty cached buffers reserve the requested capacity directly when too small,
  rather than geometrically growing storage with no live bytes to preserve.
  Trimming stays enabled; actual shrinking is moved out of the hot path.

The first experiment only cleaned up descriptor moves, branching and division;
it did not materially improve tiny snapshots. Removing the separate lifetime
updates produced the measurable warm-path improvement. No per-slot atomic
flags, new public tuning knobs, return-thread lookup, queue, byte counter or
retirement flag were introduced.

The tradeoff is explicit: lifetime management now has a small unsafe ownership
implementation instead of relying on Arc. Each shard allocates its own metadata
and slot array at TLS initialization, and each word covers at most 63 slots,
not 64. This is not a claim that initialization or exhausted scanning is cheaper.

## Safety and validation

The local shard handle is neither cloneable nor shareable across threads. Each
lease uniquely owns a slot bit and can move to another thread. Slot restoration
precedes release; acquisition synchronizes through the same word. All updates
are atomic RMWs, preserving release sequences across changes to other bits.
After releasing a bit, a caller accesses no shard memory unless it was the last
owner. TLS destruction simply releases its own bit; outstanding outputs remain
valid and the final operation frees the shard exactly once.

Validation passed:

- Workspace tests with `linux-zerocopy,tonic-gzip`, and again with all features.
- All-target/all-feature workspace checks, minimal-feature library tests and
  plain-Tonic compilation.
- Library and pool-benchmark Clippy, plus formatting checks. Pre-existing
  manifest and unrelated const-function warnings remain.
- The feature-enabled library suite: 55 tests; minimal library: 42 tests.
- Miri: all 13 minimal-feature encode tests on the final operations with seed
  113. The intermediate lifetime representation also passed seeds 53 and 97.

Tests exercise 63-bit and partial-word boundaries, unique simultaneous leases,
exhaustion, trimming, panic recovery, a producer acquiring while four consumers
return, and four concurrent returners racing TLS-owner destruction. Test-only
destruction probes verify exactly-once shard destruction. Existing snapshot,
reentrant encoding, startup configuration and allocation-counting tests remain.
Miri validates exercised executions, not every possible schedule.

## Measurement method

Host: AMD Ryzen AI MAX+ 395, Linux x86-64, rustc
`1.100.0-nightly (bba531001 2026-09-20)`, release benchmark profile.
Pre-change release executables were saved before editing. The same benchmark
sources and fixtures were used on both sides; no compiler/build/test process
was run alongside timed measurements.

There are 42 focused cases per version: 14 pool cases, 16 selected Tonic-body
cases, six matched localhost RPC cases, two forced-trim cases and four 64-slot
cases. This is the pool/transport regression suite, not every repository
protobuf, varint or compression benchmark. Warmup is one second, measurement
three seconds, with 30 samples (50 for Tonic bodies). Affinity is CPUs 2–3,
except Tonic bodies on CPU 2. The four-worker parallel cases share those two
CPUs and report a batch of 256 snapshots, not a single snapshot's latency.
Remote cases include channel overhead; localhost cases do not establish a
physical-network zero-copy benefit.

Baselines: `pool-opt-full-before-20260922` and `pool-opt-final-20260922`, with
`-trim` / `-slots64` suffixes. Trimming uses `PROTO_RS_BENCH_POOL_CAP=8`;
larger pools use `PROTO_RS_BENCH_POOL_SLOTS=64`. Raw logs, saved baseline
executables, validation scripts and the comparison script are in
`target/pool-opt.6RWdii/`; Criterion estimates are in `target/criterion/`.
Point estimates below are sample means. Negative change is faster. Confidence
interval separation alone does not establish repeatability across runs.

## Full matched run

All times below are nanoseconds; parallel rows are per 256-snapshot batch.
"Separate" and "overlap" refer to the two sample-mean 95% confidence intervals,
not to an independent replication test.


### Default configuration

| Case | Before, ns | After, ns | Change | 95% CIs |
|---|---:|---:|---:|---|
| encode_pool/exhausted/16 | 30.56 | 32.32 | +5.7% | separate |
| encode_pool/exhausted/16384 | 102.38 | 111.25 | +8.7% | separate |
| encode_pool/overlapping_owner/16 | 45.45 | 38.74 | -14.8% | separate |
| encode_pool/overlapping_owner/16384 | 96.61 | 93.40 | -3.3% | separate |
| encode_pool/parallel_exhausted/16 | 3130.50 | 3529.36 | +12.7% | separate |
| encode_pool/parallel_exhausted/16384 | 12560.62 | 20228.15 | +61.0% | separate |
| encode_pool/parallel_tls/16 | 6098.95 | 5321.08 | -12.8% | separate |
| encode_pool/parallel_tls/16384 | 13033.16 | 11753.44 | -9.8% | separate |
| encode_pool/remote_exhausted/16 | 249.82 | 268.55 | +7.5% | separate |
| encode_pool/remote_exhausted/16384 | 408.10 | 438.30 | +7.4% | separate |
| encode_pool/remote_return/16 | 222.71 | 235.25 | +5.6% | separate |
| encode_pool/remote_return/16384 | 283.29 | 286.01 | +1.0% | overlap |
| encode_pool/tls_warm/16 | 45.47 | 38.92 | -14.4% | separate |
| encode_pool/tls_warm/16384 | 99.75 | 93.46 | -6.3% | separate |
| tonic_encode/large_unary/direct | 161.29 | 146.43 | -9.2% | separate |
| tonic_encode/large_unary/exhausted_pool | 189.70 | 184.64 | -2.7% | separate |
| tonic_encode/large_unary/previous | 355.93 | 342.19 | -3.9% | overlap |
| tonic_encode/large_unary/tls_copy | 274.67 | 257.47 | -6.3% | separate |
| tonic_encode/oversized_stream/direct | 28551.90 | 29563.17 | +3.5% | separate |
| tonic_encode/oversized_stream/exhausted_pool | 32194.91 | 42348.35 | +31.5% | separate |
| tonic_encode/oversized_stream/previous | 59488.35 | 71459.16 | +20.1% | separate |
| tonic_encode/oversized_stream/tls_copy | 74500.18 | 60155.48 | -19.3% | separate |
| tonic_encode/small_stream/direct | 4242.91 | 4234.78 | -0.2% | overlap |
| tonic_encode/small_stream/exhausted_pool | 4330.52 | 4283.50 | -1.1% | separate |
| tonic_encode/small_stream/previous | 4195.53 | 4221.20 | +0.6% | overlap |
| tonic_encode/small_stream/tls_copy | 8084.68 | 6774.21 | -16.2% | separate |
| tonic_encode/small_unary/direct | 92.52 | 94.85 | +2.5% | separate |
| tonic_encode/small_unary/exhausted_pool | 78.94 | 81.99 | +3.9% | separate |
| tonic_encode/small_unary/previous | 65.08 | 63.75 | -2.0% | separate |
| tonic_encode/small_unary/tls_copy | 107.94 | 107.53 | -0.4% | overlap |
| zerocopy_channel/owned_adaptive/16 | 20552.12 | 20473.63 | -0.4% | overlap |
| zerocopy_channel/owned_adaptive/16384 | 26255.21 | 26702.09 | +1.7% | overlap |
| zerocopy_channel/owned_adaptive/262144 | 129179.87 | 135481.83 | +4.9% | separate |
| zerocopy_channel/tonic/16 | 20792.01 | 20415.40 | -1.8% | overlap |
| zerocopy_channel/tonic/16384 | 26922.66 | 26639.07 | -1.1% | overlap |
| zerocopy_channel/tonic/262144 | 130574.58 | 137711.46 | +5.5% | separate |

### Forced trimming (8-byte cap)

| Case | Before, ns | After, ns | Change | 95% CIs |
|---|---:|---:|---:|---|
| encode_pool/tls_warm/16 | 54.85 | 49.05 | -10.6% | separate |
| encode_pool/tls_warm/16384 | 137.57 | 144.79 | +5.3% | separate |

### 64 slots

| Case | Before, ns | After, ns | Change | 95% CIs |
|---|---:|---:|---:|---|
| encode_pool/parallel_tls/16 | 5800.60 | 5251.84 | -9.5% | separate |
| encode_pool/parallel_tls/16384 | 13833.89 | 12461.23 | -9.9% | separate |
| encode_pool/remote_return/16 | 156.81 | 154.21 | -1.7% | separate |
| encode_pool/remote_return/16384 | 254.88 | 256.01 | +0.4% | overlap |

Warm tiny snapshots improve 14–15%, warm 16 KiB snapshots improve 6%, and
parallel warm throughput improves roughly 10–13%. Direct 16 KiB Tonic unary
encoding improves 9%. Small direct Tonic unary is 2.5% slower and the batched
small stream is unchanged; these results do not support a universal Tonic win.
With 64 slots, the measured remote-return change is small (−1.7% / +0.4%), not
evidence of a major contention improvement. Tiny forced trimming improves 11%,
while the larger trim case is 5% slower.

The default exhausted path is slower, and the parallel exhausted 16 KiB case
shows a large +61% regression in the full run. Oversized-stream results also move substantially,
including +20% in the unchanged `previous` control. These cases are repeated
below rather than silently excluded. Most localhost RPC differences are small;
both backends are about 5% slower at 256 KiB, without a demonstrated pool-specific
cause.

Even after this optimization, a warm 16-byte pooled snapshot (38.92 ns) is slower
than the exhausted/unpooled workflow (32.32 ns) in this run. Owned outputs still
allocate a Bytes owner record and perform its lifetime operations; successful
pooling adds two synchronized slot operations while avoiding payload allocation.
Synchronous destination encoding has no Bytes owner record. The measured result
is reduced pooling overhead, not proof that pooling beats allocation at every
size. A final five-second profile attributes 32.1% of cycles to buffer-drop glue,
31.7% to TLS checkout and 23.2% to owned framing/handoff. These are relative
profile shares, not absolute per-operation timing comparisons.

## Isolated repeats

Repeated six pool cases and all four oversized-stream variants with the same
settings, using fresh processes and the labels `pool-opt-repeat-before-20260922`
and `pool-opt-repeat-after-20260922`.

| Case | Before, ns | After, ns | Change |
|---|---:|---:|---:|
| encode_pool/exhausted/16 | 29.64 | 33.16 | 11.9% |
| encode_pool/exhausted/16384 | 102.25 | 108.84 | 6.4% |
| encode_pool/parallel_exhausted/16 | 3322.14 | 3346.95 | 0.7% |
| encode_pool/parallel_exhausted/16384 | 12962.66 | 14018.61 | 8.1% |
| encode_pool/remote_return/16 | 229.04 | 225.50 | -1.5% |
| encode_pool/remote_return/16384 | 256.35 | 273.95 | 6.9% |
| tonic_encode/oversized_stream/direct | 29791.54 | 30558.31 | 2.6% |
| tonic_encode/oversized_stream/exhausted_pool | 29561.38 | 29754.91 | 0.7% |
| tonic_encode/oversized_stream/previous | 63405.92 | 68132.20 | 7.5% |
| tonic_encode/oversized_stream/tls_copy | 61958.59 | 58761.08 | -5.2% |

The full-run +61% parallel-exhaustion result repeated as +8.1%, and the +31.5%
oversized exhausted stream repeated as +0.7%. Their original magnitudes are not
repeatable. However, single-thread exhaustion remains 6–12% slower (about
3.5 ns / 6.6 ns), and remote-return measurements remain mixed. The warm-path
improvement is not grounds for dismissing those regressions.

Remaining optimization targets are full-shard scanning/local metadata locality
and the separate Bytes owner-record allocation on each owned output. Replacing
Arc removed two lifetime updates but does not remove the two slot synchronization
operations. No claim is made that tiny payload pooling is now cheaper than the
allocator, or that all end-to-end workloads improved.
