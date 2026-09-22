# 64 MiB batch encoding and transport — 2026-09-22

This records the earlier ordinary/direct encoder measurements. The later
[eager-snapshot integration](owned-snapshot.md) patches Tonic and adds owned and
pooled snapshot variants; the earlier unvendored-Tonic descriptions below are
historical, not the current snapshot implementation.

## Workload and reproduction

One gRPC message contains `Vec<BytesBench>` with 20 records:

```rust
struct BytesBench {
    a: u64,
    b: u64,
    c: Option<u64>,
    d: Arc<Vec<u8>>,
}
```

The 20 distinct byte allocations total exactly **64 MiB (67,108,864 bytes)**,
excluding protobuf/gRPC framing. Optional integers exercise None, Some(0), and
Some(u64::MAX). Payloads are deterministic high-entropy bytes by default; a
second distribution repeats each record's integer index. Fixtures and their
initial encode/decode validation are outside timing. Each timed operation borrows
the batch through an Arc; neither encoder deep-clones the records or payloads.

```sh
# Default gzip backend, all cases in this focused target:
cargo bench -p proto_rs --bench bytes_pipeline --locked --offline
# Opt-in zlib-rs gzip backend:
cargo bench -p proto_rs --bench bytes_pipeline --features tonic-gzip --locked --offline
# Repetitive data, direct encoder only:
PROTO_RS_BENCH_DATA=compressible cargo bench -p proto_rs --bench bytes_pipeline --features tonic-gzip --locked --offline -- direct
# Body-only, no sockets:
PROTO_RS_BENCH_NO_NETWORK=1 cargo bench -p proto_rs --bench bytes_pipeline --locked --offline -- body/plain
```

`scratch` is the previous `ProtoEncode::encode` scratch-and-copy implementation,
frozen in the benchmark. `bounded` uses the current direct encoder with its default
1 MiB reservation cap. `direct` raises that cap to 128 MiB; this permits the actual
hint-sized reservation, not an unconditional 128 MiB allocation. These comparisons
are **not against Prost**. Prost byte compatibility is checked separately in tests.

Body cases include a fresh Tonic `EncodeBody`, protobuf encoding, gRPC framing,
optional gzip, allocation, and draining frames without copying them into a collector.
RPC cases additionally include localhost TCP, HTTP/2, optional rustls encryption,
receiver protobuf decoding, and a small receipt. Connections persist across timed
iterations. Certificate generation, handshake, and full byte-for-byte verification
of both encoders are untimed. TLS verifies a locally trusted localhost certificate;
certificate verification is not disabled. Custom incoming sockets explicitly enable
TCP_NODELAY, matching Tonic's normal listener. No physical NIC throughput is measured.

Criterion uses 10 samples, 100 ms warmup, and a 1-second requested measurement;
slow gzip cases necessarily run longer to collect 10 samples. Hardware: AMD RYZEN
AI MAX+ 395, Linux 7.2.6-arch2-1, rustc 1.100.0-nightly, fat LTO, one codegen unit.
Tonic 0.14.6, rustls 0.23.45 (ring), flate2 1.1.10, optional zlib-rs 0.6.8.
Throughput uses original payload bytes, not compressed bytes on the wire.

## Measured results

Criterion point estimates, milliseconds per complete 64 MiB batch:

| High-entropy workload | Scratch + copy | Direct, 128 MiB cap |
| --- | ---: | ---: |
| Plain body (no socket) | 14.723 | 3.707 |
| Plain RPC | 31.029 | 20.396 |
| TLS RPC | 43.031 | 33.996 |

The default-cap body control was 13.451 ms. The plain RPC intervals were
30.419–31.427 ms versus 20.277–20.613 ms; TLS intervals were 41.528–44.489 ms versus
31.523–36.194 ms. These are localhost workload results, not production latency
guarantees. The small sample count and short warmup warrant caution.

Both columns below use the **same direct encoder with a 128 MiB cap**, isolating
the optional compression-backend change:

| Payload / workload | Default gzip (miniz_oxide) | `tonic-gzip` (zlib-rs) |
| --- | ---: | ---: |
| High entropy / gzip body | 1113.4 | 801.69 |
| High entropy / gzip RPC | 1095.0 | 829.74 |
| High entropy / gzip+TLS RPC | 1106.1 | 838.09 |
| Repetitive / gzip body | 69.375 | 16.721 |
| Repetitive / gzip RPC | 89.680 | 30.948 |
| Repetitive / gzip+TLS RPC | 89.262 | 30.904 |

For high entropy, compression dominates: direct output alone changed default-gzip
RPC latency only from 1105.6 to 1095.0 ms. For repetitive bytes, the faster gzip
backend improved gzip+TLS RPC throughput about 2.9×; the gain is not a zero-copy
effect. Enabling the feature does not automatically enable compression on a call.

Body measurements came from complete default/zlib-rs body runs. All RPC numbers
above use the corrected TCP_NODELAY setup. Earlier exploratory RPC measurements
without that socket option were discarded; neither those latencies nor Criterion's
automatic comparisons against them are used here. No other benchmarks or tests
were deliberately run concurrently with these timed measurements.

## Encoding change

Previously, a 20-message vector had an inexact collection hint. Its 64 MiB payload
exceeded Tonic's capped initial reservation, spilled into a growing reverse buffer,
and was copied into Tonic afterward. The initial direct-encoder body measurement
was **13.976 ms**. With bounded top-level batch sizing and the larger reservation,
it was **3.771 ms** in the first post-change run (about **3.7× faster**).

`ProtoArchive::output_size_hint` examines at most 32 top-level message elements,
using ordinary field hints rather than recursively traversing collections. For
this flat batch it obtains an exact size without reading the payload contents.
The encoder reserves once, copies each payload directly into its final protobuf
position, and commits the initialized prefix. No scratch spill or final compaction
is needed for this fixture. Incorrect hints still fall back safely without encoding
the message twice. The default reservation remains bounded to 1 MiB.

Configure a generated Tonic client/server, `ProtoCodec`, `ProtoEncoder`, or
`TonicTransport` with `.with_max_encode_preallocation(128 * 1024 * 1024)` for this
workload. Configure Tonic's sending/receiving message-size limits independently.
The cap is not an admission-control mechanism or a limit on total memory; account
for concurrent RPCs, receive buffers, and compression/encryption buffers too.

## What this does not make zero-copy

- Plain: original payload → Tonic protobuf buffer → ordinary socket/kernel path.
- Gzip: original payload → Tonic uncompressed protobuf buffer → compressed output
  (with compressor work buffers) → socket/kernel path.
- TLS adds rustls record encryption/output buffering; gzip+TLS performs both
  transformations. These stages are not bypassed by the direct protobuf writer.

Only the first arrow is reduced to one payload copy for an exact, fitting hint.
The ordinary transport measured above does **not** encode into a kernel allocation,
install kTLS, or use MSG_ZEROCOPY. Tonic remains unvendored. A separate experimental
`linux-zerocopy` transport now preserves owned output pages through isolated
Hyper/h2 forks and submits them with MSG_ZEROCOPY; see its
[validation and limitations](linux-zerocopy.md). It does not implement TLS.

Linux MSG_ZEROCOPY pins **userspace** pages and requires completion/error-queue
tracking before they can be reused; it can fall back to copying, and loopback
always defers to copying. A normal borrowed `AsyncWrite` buffer does not supply
the necessary lifetime/ownership contract across cancellation and completion.
See the [kernel MSG_ZEROCOPY documentation](https://www.kernel.org/doc/html/latest/networking/msg_zerocopy.html).

kTLS is a different integration: the application still performs the handshake and
installs traffic keys. Its documented zero-copy sendfile optimization requires
device offload and immutable file data, not arbitrary Vec payloads encoded through
rustls. A production kernel-offload path needs an ownership-aware transport and
target-kernel/NIC validation, not just a Tonic codec fork. See the
[kernel TLS documentation](https://www.kernel.org/doc/html/latest/networking/tls.html).

The optional `tonic-gzip` feature selects flate2's `zlib-rs` backend, not a new wire
format. Sending gzip still requires `send_compressed`. Feature unification selects
the backend across the dependency graph; an enabled C backend takes precedence.
See [flate2's backend documentation](https://docs.rs/flate2/1.1.10/flate2/).

## Validation

- Root/derive test suites with `tonic-gzip`, plus stable library and Tonic adapter
  tests, passed. The crate also checks with default features disabled.
- All three protoc interoperability tests passed with `build-schemas`; the new
  batch test additionally compares bytes with Prost for 0/1/20/32/33 records,
  empty payloads, optional-zero presence, and empty-message framing.
- Nine direct-buffer tests passed under Miri, covering exact and inaccurate hints,
  configurable reservation bounds, spill boundaries, panic recovery, and cloning.
- The allocation test uses a smaller **20 × 128 KiB** shared batch to keep routine
  tests inexpensive: raising the cap reduced encoding allocation/reallocation
  calls from **6 to 2**. Decoding verifies every field and payload byte; Arc counts
  return to their original values. Allocation instrumentation is not part of the
  Criterion runs. This is not a 64 MiB peak-memory measurement.
- The small-message control measured 70.492 ns for scratch encoding and 68.904 ns
  for direct encoding; Criterion detected no significant change in direct latency
  versus its previous baseline. Only this control and the new batch target ran;
  the full suite and varint benchmarks were not rerun.
- Formatting checks and scoped Clippy passed (existing manifest and two
  `missing_const_for_fn` warnings in primitive encoders remain).
