# Eager ZeroCopy snapshots: owned Tonic handoff

The current API is `EncodedSnapshot<T>` with automatic TLS pooling; explicit
pool handles, old compatibility names and mutable-writer conversions were removed.
Snapshots finalize the header and create
their atomic shared owner once, at construction. Clones and Tonic handoff are
allocation-free, and the last snapshot/Bytes/transport owner returns the buffer.
See [current shared-snapshot
semantics and profiling](unified-owned-encoding.md#shared-snapshots-for-fan-out).
The implementation discussion and allocation/timing measurements below are
historical; they predate shared snapshots.

Historical snapshot-handoff measurements. Ordinary messages and default eager
snapshots use [unified owned encoding](unified-owned-encoding.md). Current owned
pooling is thread-local and shared across services, including standalone snapshots.
The timings below predate those changes.

`ZeroCopy<T>` remains an eagerly encoded snapshot. It does not borrow the original
message, defer encoding until polling, or observe later mutation of the input.

## Original owned-handoff path

Plain: input → reverse encode into an owned buffer with five bytes of headroom →
prepend the gRPC flag/length in that headroom → yield the same allocation as an
HTTP body frame. Tonic checks send limits before yielding. `as_bytes()`,
`into_bytes()` and `into_inner()` still expose protobuf only, never gRPC framing.

The writer preserves header headroom even when hints are inaccurate and it grows.
Exact fitting hints avoid growth; oversized/inexact messages can still allocate
and copy during growth, but are encoded only once. Converting an independently
constructed legacy `ArchivedProtoMessage` can require growth to add headroom.

Gzip: input → snapshot → compressed output. Tonic's compressor reads the snapshot
slice directly; there is no snapshot-to-uncompressed-Tonic-buffer copy. A separate
compressed allocation and compressor workspace remain necessary. Compression
overrides, message-size policy, stream ordering and server trailers are preserved.

Normal TLS still encrypts into rustls record storage. This optimization does not
remove encryption copies or provide TLS kernel zero-copy. On the experimental
plain Linux transport, large snapshot frames can continue through the existing
owned Hyper/h2 path and remain retained until kernel completion.

## Downstream setup — important distribution constraint

The implementation extends Tonic 0.14.6's encoder/body contract. This repository
uses `[patch.crates-io]` so proto_rs, tonic-prost and generated services share the
same Tonic types. **Cargo does not inherit patches from dependencies.** Every
downstream workspace root must include the patch, for example with a checkout:

```toml
[dependencies]
proto_rs = { path = "../proto-rs" }

[patch.crates-io]
tonic = { path = "../proto-rs/vendor/tonic" }
```

The `proto-rs-owned` marker feature makes resolution fail rather than silently
selecting unpatched upstream Tonic. Tonic retains its upstream package name and
version; do not publish this fork as upstream Tonic. A distribution/upstreaming
strategy is required before publishing proto_rs with this mandatory dependency.
The renamed Hyper/h2 forks have their own publishing requirements too. License,
upstream revision and patch scope are recorded in [vendor/README.md](../vendor/README.md).

## Allocation and high-water-mark reuse

```rust,ignore
use proto_rs::{ZeroCopy, ZeroCopyPool};

// Normal eager snapshot; cap controls speculative preallocation only.
let snapshot = ZeroCopy::with_max_preallocation(&message, 128 * 1024 * 1024);
client.echo(snapshot).await?;

// Keep the pool across calls. Limits cover idle retained capacity, not in-flight memory.
let pool = ZeroCopyPool::new(256 * 1024 * 1024, 4);
let snapshot = pool.encode_with_max_preallocation(&message, 128 * 1024 * 1024);
client.echo(snapshot).await?;
```

`to_zero_copy()`/`ZeroCopy::new` default to a 1 MiB speculative cap plus header
headroom, using bounded transport output hints (including flat message batches).
This replaces the earlier snapshot's 64 KiB capped ordinary hint. Tonic send and
receive limits remain separate; changing the codec's cap after creating a
snapshot cannot resize that snapshot retroactively.

The optional pool takes the smallest fitting idle buffer, or grows an available
smaller buffer before encoding. It retains high-water capacities within explicit
byte/count limits. Buffers return only after the last owner releases them, even
if HTTP body or kernel-send ownership outlives the RPC. Cancellation and encoder
panic return releasable buffers safely. `into_inner()` transfers the writer out
of the pool. In-flight allocations are not capped by the pool: admission control
and backpressure remain application responsibilities. No memory zeroization is
promised; only the newly initialized slice is exposed on reuse.

The pool is an opt-in mutex-protected, bounded best-fit collection, not a lock-free
or fixed-size-class allocator. It can cost more than allocation for tiny messages.
`Bytes::from_owner` still allocates small ownership metadata. Therefore:

- A sufficiently preallocated, non-pooled snapshot has one payload allocation.
- Plain handoff has no second payload allocation/copy, but allocates Bytes metadata.
- A warmed, fitting pooled snapshot can encode with zero allocations; ownership
  handoff still allocates that metadata. Pool startup/growth can allocate too.

## Verification

The allocation regression uses 20 × 128 KiB payloads: exactly one allocation to
create the snapshot, one metadata allocation to create and drain its plain Tonic
body, and zero allocations to encode/drop a warmed pooled snapshot. Pointer
identity verifies that payload bytes do not move during Tonic handoff.

Tests cover empty/root framing, inaccurate hints and growth without reencoding,
panic recovery, last-owner retention and reuse, mixed buffered/owned ordering,
body cancellation, size-limit rejection, server trailers/compression override,
plain/TLS/gzip round trips and snapshots over the Linux owned-buffer transport.
Three writer/pool tests and seven in-memory owned-body tests pass under Miri.

The ordinary encoder/byte-container fallback remains available. Small owned
messages are yielded separately instead of being copied into Tonic's coalescing
buffer; streaming frame/poll overhead is a tradeoff, not an end-to-end speedup claim.

## Focused benchmarks

Snapshot cases include eager encoding inside timing. `snapshot_copy` uses the new
snapshot allocator but deliberately copies into Tonic, isolating the handoff from
the sizing improvement. `snapshot_owned` transfers ownership; `snapshot_pooled`
also reuses released snapshot buffers. These are body-only measurements, not RPC
or physical network throughput. Timed targets run sequentially, with ±4% noise band.

```sh
cargo bench -p proto_rs --bench tonic_encode --features linux-zerocopy,tonic-gzip \
  --locked --offline -- \
  '^tonic_encode/(small_unary|large_unary|oversized_unary)/snapshot_' --noise-threshold 0.04

PROTO_RS_BENCH_NO_NETWORK=1 PROTO_RS_BENCH_DATA=compressible \
  cargo bench -p proto_rs --bench bytes_pipeline --features linux-zerocopy,tonic-gzip \
  --locked --offline -- 'body/(plain|gzip)/(snapshot_|direct)' --noise-threshold 0.04
```

September 22, 2026; Ryzen AI MAX+ 395, nightly rustc, fat LTO. Tonic target:
50 samples, 1 s warmup, 3 s measurement. Point estimates per unary body:

| Byte field | Snapshot + copy | Owned snapshot | Pooled owned snapshot |
| --- | ---: | ---: | ---: |
| 16 B | 74.985 ns | 54.891 ns | 83.870 ns |
| 16 KiB | 345.18 ns | 134.28 ns | 143.51 ns |
| 256 KiB | 4.6054 µs | 2.7900 µs | 2.4390 µs |

Pooling is not the default: the small-message measurements show its synchronization
cost. No claim of exactly one allocation for the entire RPC is made.

64 MiB total payload in 20 records, repetitive data, zlib-rs, 10 samples,
100 ms warmup, 1 s requested measurement:

| Body | Snapshot + copy | Owned snapshot | Pooled owned snapshot | Ordinary direct |
| --- | ---: | ---: | ---: | ---: |
| Plain | 7.4909 ms | 3.7518 ms | 3.2309 ms | 4.0167 ms |
| Gzip | 20.682 ms | 16.826 ms | 16.577 ms | 16.774 ms |

Owned handoff halves the plain snapshot path's time; gzip benefits from removing
the extra uncompressed copy. Gzip differences between owned, pooled and ordinary
direct are within the ±4% noise band. The ordinary direct 64 MiB point estimate
was slower than earlier runs around 3.7–3.8 ms; historical comparisons are not
substitutes for these matched same-invocation snapshot comparisons.

The expanded root regression suite passes 184 tests with
`linux-zerocopy,tonic-gzip`; stable library/allocation tests and focused Clippy
also pass. The standalone upstream Tonic unit-suite invocation was blocked
offline by its uncached `quickcheck_macros 1.2.0` development dependency. That
suite is not claimed as validated; the fork is covered by the root integration
and in-memory regression tests described above.

### Ordinary direct encoder: remaining regression finding

The final isolated buffered-path run used the saved September 22 `base`
measurements without overwriting them (`--baseline base --noise-threshold 0.04`):

| Ordinary direct case | Saved point estimate | Final point estimate | Relative-mean comparison |
| --- | ---: | ---: | --- |
| 16 B unary | 68.904 ns | 46.963 ns | Faster |
| 16 B × 128 stream | 2.433 µs | 2.449 µs | +1.32%, noise |
| 16 KiB unary | 177.69 ns | 140.35 ns | Faster |
| 256 KiB unary | 1.950 µs | 2.188 µs | **+12.14%, regression** |

The 256 KiB case also measured 2.24–2.33 µs in earlier repeats, so it cannot be
dismissed under the ±4% noise band. Owned dispatch has been separated from the
original buffered encoding function and ordinary encoders retain eager buffer
allocation, but the remaining historical slowdown is unresolved. These changes
are therefore **not** a blanket encoder no-regression result. The snapshot
copy-elimination and allocation-identity results remain independently verified.
