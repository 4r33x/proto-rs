# Allocation pool simplification — 2026-09-22

Historical mutex/size-class experiment, superseded by the requested
[bounded array pool](array-pool-2026-09-22.md). Measurements below describe that
earlier implementation, not the current queue or per-allocation capacity limits.

The configuration LRU, local-spare owner-count/fence checks, retirement protocol,
FIFO eviction and synchronized byte counter are removed. Owned buffers now return
to explicit shared pools held by RPC infrastructure; synchronous copying alone
uses a single bounded TLS scratch buffer. The Crossbeam queue dependency is gone.

This is a simplicity change, **not a blanket speedup**. Encoder-only regressions
reproduce, and a highly contended shared pool is slower than the allocator control.
See [current design and migration](unified-owned-encoding.md#thread-local-slot-pools).

## Verification

Passed:

- Workspace tests with `linux-zerocopy,tonic-gzip`.
- Workspace tests and all-target checks with all features (including stable futures).
- Minimal/no-default-feature library tests.
- Library compilation with only `tonic` (without the owned extension).
- Library Clippy and the new pool benchmark's Clippy; existing unrelated warnings remain.
- Formatting checks for edited Rust files and `git diff --check`.

New coverage includes final-owner lifetime, overlapping owners, cross-thread
return, six independent pools, concurrent count/capacity bounds, best-fit lookup,
full-class drops without eviction, extreme/zero limits, scratch panic recovery
and oversized-scratch rejection. Generated client/server/client-clone tests verify
that explicitly supplied pools are used. Warm allocation tests still require
one Bytes owner-record allocation, not another payload allocation.

A cold allocation check exposed a reservation issue masked by old TLS reuse:
single-message bodies must refine their staging hint before allocating. That is
fixed without serializing twice.

## Method

AMD Ryzen AI MAX+ 395, Linux 7.2.6-arch2-1, rustc
1.100.0-nightly (bba531001, 2026-09-20). Optimized bench profile, fat LTO,
one codegen unit; default features plus `linux-zerocopy,tonic-gzip`.
Timed targets ran sequentially. Tonic bodies used CPU 2; pool concurrency and
loopback RPCs used CPUs 2,3. Four contention workers therefore share two CPUs.

Tables use Criterion sample means, not its fitted slope estimates.
A ±4% difference is the historical practical review band, not a statistical test.
Initial labels: `simple-pool-20260922`; repeats:
`simple-pool-repeat-20260922`. Historical baselines were preserved.

The previous full-suite rerun was interrupted for this refactor after varint,
protobuf and Tonic encoding completed. It was not resumed as a full compression/
TLS/byte-pipeline run. This report covers **36 focused cases plus eight repeats**.

## Warm Tonic body comparison

Baseline: the completed pre-change `refactor-20260922-1718` run.
The benchmark now holds an explicit pool outside timing and clones it into each
encoder, modeling client/server reuse across bodies. Previously TLS retained that
reuse implicitly. This compares those lifecycles, not an identical pool API.
Separate cold-pool and synchronous TLS-scratch cases retain their setup costs.

| Payload / messages | Before | First pass | Repeat | Repeat vs before |
| --- | ---: | ---: | ---: | ---: |
| 16 B × 1 | 69.87 ns | 102.62 ns | 102.35 ns | +46.5% |
| 16 B × 128 | 4.165 µs | 4.285 µs | — | +2.9% (first pass) |
| 16 KiB × 1 | 127.21 ns | 162.25 ns | 179.08 ns | +40.8% |
| 256 KiB × 16 | 29.602 µs | 42.355 µs | 41.832 µs | +41.3% |

Unary and oversized-stream slowdowns reproduced. Small streaming stayed within
the practical band. The scratch control also changed: small unary improved about
11%, large unary stayed within 2%, and oversized streaming slowed about 9% on
repeat. Thus these are measured workflow changes, not a claim that every
nanosecond of change comes from the mutex alone.

Tonic: initially 50 samples, 1 s warmup, 3 s measurement; repeats 100 samples,
2 s warmup, 5 s measurement.

## Pool workload controls

Payload is a byte vector, not a Tonic body. Values below are mean wall time per
message. Four-worker results divide each measured 256-message batch by 256:
they express aggregate throughput, not per-thread operation latency.
Remote-return cases include a bounded 32-entry producer/consumer channel and
final release on the consumer thread. Thread creation is outside timing.

| Workflow | 16 B | 16 KiB |
| --- | ---: | ---: |
| Unpooled, one thread | 27.31 ns | 106.58 ns |
| Shared pool, one thread | 51.81 ns | 105.79 ns |
| Shared pool, another snapshot held alive | 50.15 ns | 106.15 ns |
| Unpooled, four workers | 10.93 ns | 48.53 ns |
| One shared pool, four workers | 107.77 ns | 83.03 ns |
| Unpooled cross-thread handoff | 256.83 ns | 407.43 ns |
| Pooled cross-thread handoff | 190.61 ns | 298.75 ns |

Overlapping ownership does not disable reuse. However, single-thread tiny
allocations are cheaper unpooled, and four workers contending on one mutex are
about 9.9×/1.7× slower than the matched unpooled controls for 16 B/16 KiB.
The pooled producer/consumer cases are about 26–27% faster than their unpooled
controls. These new cases have no pre-refactor contention baseline; none of
those comparisons establish improvement over the former queue implementation.

Pool cases: 30 samples, 1 s warmup, 3 s measurement.

## Loopback RPCs

Both paths use the same Tonic connection manager; the owned/adaptive path
downgrades to ordinary sends on loopback. These are not physical-NIC or
kernel-zero-copy speed measurements. Saved baselines match the prior
[CPU 2,3 managed-channel run](managed-channel.md#focused-localhost-comparison).

| Payload | Path | Saved mean | First pass | Repeat |
| --- | --- | ---: | ---: | ---: |
| 16 B | tonic | 19.870 µs | 20.083 µs | — |
| 16 B | owned_adaptive | 20.530 µs | 20.529 µs | — |
| 16 KiB | tonic | 26.743 µs | 25.636 µs | 26.920 µs |
| 16 KiB | owned_adaptive | 24.666 µs | 26.930 µs | 24.832 µs |
| 256 KiB | tonic | 135.328 µs | 126.980 µs | — |
| 256 KiB | owned_adaptive | 119.997 µs | 117.223 µs | — |

The first 16 KiB owned/adaptive mean was +9.2%; its repeat was +0.7%, so the
slowdown did not reproduce. The regular path's repeat was also +0.7%. The 16 B
results were within the practical band; 256 KiB showed no historical slowdown
in this pass. RPC cases: 30 samples, 1 s warmup, 3 s measurement.

## Reproduction and artifacts

```sh
taskset -c 2,3 cargo bench -p proto_rs --bench encode_pool \
  --features linux-zerocopy,tonic-gzip --locked --offline -- \
  --save-baseline simple-pool-20260922 --noise-threshold 0.04

taskset -c 2 cargo bench -p proto_rs --bench tonic_encode \
  --features linux-zerocopy,tonic-gzip --locked --offline -- \
  'tonic_encode/(small_unary|large_unary|small_stream|oversized_stream)/(direct|previous|tls_scratch|cold_pool)$' \
  --save-baseline simple-pool-20260922 --noise-threshold 0.04

taskset -c 2,3 cargo bench -p proto_rs --bench zerocopy_channel \
  --features linux-zerocopy,tonic-gzip --locked --offline -- \
  --save-baseline simple-pool-20260922 --noise-threshold 0.04
```

Local logs, test outputs, pre-change source copies, scripts and consolidated
measurements: `target/pool-simplify.v4nBta/`.
Criterion samples/confidence intervals: `target/criterion/**/simple-pool*-20260922/`.
Old pre-rerun measurements: `target/bench-rerun-20260922-1718/before-criterion/`.
These target-directory artifacts are intentionally untracked.
