# Tonic encoding and allocation changes — 2026-09-22

Historical report from before eager snapshot handoff. The subsequent
[owned-snapshot integration](owned-snapshot.md) vendors Tonic and removes the
`ZeroCopy<T>` payload copy; statements below about unmodified Tonic describe this
earlier measurement only.

These compare the new direct-buffer encoder with the previous `ProtoEncode::encode`
scratch-buffer-and-copy path, **not with Prost**. Both run through Tonic 0.14.6's
real `EncodeBody`, including gRPC framing, output-buffer allocation, polling, and
the same per-message Arc handle operations. They do not include sockets, HTTP/2,
compression, TLS, or service dispatch.

## Reproduce

```sh
cargo bench -p proto_rs --bench tonic_encode --locked --offline
# Example narrower filter:
cargo bench -p proto_rs --bench tonic_encode --locked --offline -- oversized
```

Criterion: 50 samples, 1-second warmup, 3-second measurement per case. Default
nightly features; rustc 1.100.0-nightly (bba531001 2026-09-20), fat LTO,
codegen-units=1, AMD RYZEN AI MAX+ 395. The fixture also contains an integer,
string, and packed integer list; sizes below describe its bytes field, not total
wire size. Streaming timings cover the complete batch, not one message.

## Final complete run

| Bytes field | Messages per body | Previous | Direct | Throughput ratio |
| --- | ---: | ---: | ---: | ---: |
| 16 B | 1 | 69.492 ns | 68.040 ns | 1.02× |
| 16 B | 128 | 4.186 µs | 2.433 µs | 1.72× |
| 16 KiB | 1 | 276.39 ns | 177.69 ns | 1.56× |
| 16 KiB | 128 | 29.721 µs | 11.578 µs | 2.57× |
| 256 KiB | 1 | 3.681 µs | 1.950 µs | 1.89× |
| 256 KiB | 16 | 57.812 µs | 42.392 µs | 1.36× |

Timings are Criterion point estimates. Tiny unary performance is effectively
similar. Absolute timings varied between runs/builds, especially the 256 KiB
streaming case (direct runs approximately 28–42 µs); the table uses the final
complete run, not the best result. These measurements are workload-specific and
are not end-to-end gRPC speedup claims. No existing full benchmark suite or varint
benchmark was run for this change.

## Allocation and ownership changes

- Ordinary messages write into `EncodeBuf::chunk_mut()` and commit only fully
  initialized bytes. Exact-size hints can avoid compaction; other fitting
  messages require an in-buffer move.
- Insufficient space spills once into scratch and continues encoding, without
  repeating reads of the message. Scratch is reused between stream items and
  retains at most 64 KiB. Speculative output reservations default to a 1 MiB cap
  (now configurable via `with_max_encode_preallocation`);
  this is not a message-size limit. Encoder clones do not copy scratch capacity.
- The 32-message allocation regression fixture measured **33 → 2 allocations**;
  the test requires at most two allocations on the direct path and at least 31
  fewer than the previous path. `MessageStream::once` and `empty` allocate no
  stream containers. Allocation counts are separate tests, not instrumented
  Criterion timings.
- Generated method adapters take the existing per-call Arc instead of cloning
  it again. Nightly async futures remain unboxed; synchronous handler futures
  use `Ready` on both nightly and stable.
- Tonic response-stream wrappers store `Streaming<T>` inline. Transport-neutral
  stream encoding uses one type-erasure box instead of two boxes and supports
  `!Unpin` source streams through safe pin projection.
- Consuming status conversion moves metadata instead of cloning it. Message and
  details still copy because Tonic exposes them by reference.

Pre-encoded `ZeroCopy<T>` uses the existing byte-copy path. Neither ordinary
encoding nor pre-encoding promises kernel/network zero-copy. Tonic itself is
unchanged and unvendored; compression, framing, and message-size enforcement
stay in Tonic.

## Validation

- Default proto_rs and derive suites, plus focused nightly and stable tests.
- Existing Prost/Tonic interoperability tests, sync/async generated handlers,
  metadata, status details, stream errors, cancellation, gzip, and size limits.
- Miri checks of direct writes, every varint width, spill boundaries, dishonest
  size hints, empty messages, bounded scratch retention, and panic recovery.
- Clippy and formatting checks.

Full workspace/all-features validation was blocked by separate concurrent
Solana dependency upgrades: the matches in `src/custom_types/solana/tx_errors.rs`
at lines 349 and 408 do not cover the upgraded non-exhaustive error enums. Those
unrelated dependency and source changes were not reverted or patched here.
