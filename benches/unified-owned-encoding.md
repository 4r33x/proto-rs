# Unified owned encoding

Current pool design and measured tradeoffs: [pool optimization, 2026-09-22](pool-optimization-2026-09-22.md).

Ordinary proto_rs Tonic codecs now borrow their inputs and archive once into the
same owned reverse-buffer implementation as eager snapshots. The borrowed Tonic
destination writer, scratch spill, compaction and copy-back paths were removed.
The public `Encoder::encode` compatibility entry point still copies into its
caller-supplied destination; Tonic bodies use the owned hook instead. Raw
`encode(&mut BufMut)` retains its caller-owned destination semantics.

## Borrowed calls

```rust,ignore
// Once at process startup, before any thread encodes:
proto_rs::configure_encode_pool(proto_rs::EncodePoolConfig {
    max_buffers: 8,
    max_buffer_capacity: 32 * 1024 * 1024,
}).expect("configure pooling before its first use");
let mut client = EchoClient::connect("http://127.0.0.1:50051").await?;
let pending = client.echo(&message); // Encodes now, once.
drop(message);                       // The future does not borrow the message.
let reply = pending.await?;
```

Generated unary/server-streaming methods return request-independent futures.
Owned values, `tonic::Request<T>`, `Request<&T>`, Arc/Box and eager snapshots remain
supported. Metadata/extensions move; snapshots are not encoded again. Custom
adapters implement `PrepareRequest<T>`; the legacy `ProtoRequest<T>` conversion
interface has been removed. Generated transports/interceptors and transport futures
must be Send. The future still borrows the client.

Nightly uses unboxed associated opaque future types; stable uses one boxed future
with the same borrow-release behavior. Preparation and configured client-context
interception happen before readiness polling, even if the future is never polled.
Dropping that future releases its frame. Serialization panics happen at invocation.
Payload bytes still need copying into protobuf storage during serialization.

`TonicTransport::unary_ref` and `server_streaming_ref` support neutral Request and
Response containers. The generic GrpcTransport trait retains its owned contract.
For client/bidirectional streaming, `grpc::EncodedSender::<T>::channel(n)` provides
a sender and owned stream. `sender.send(&message)` prepares synchronously; awaiting
waits only for queue space. Use `client_streaming_encoded`/`bidirectional_encoded`
on TonicTransport. Cancelling a pending send does not enqueue it.

## Shared snapshots for fan-out

`EncodedSnapshot<T>` / `ProtoEncode::to_encoded_snapshot` explicitly mean
encode once, share the same immutable allocation among recipients. Allocation
reuse is automatic via TLS. `ZeroCopy`, `ZeroCopyPool`, and `to_zero_copy` have been
removed; the snapshot does not promise kernel zero-copy. Ordinary `&T` RPCs
use the internal writer directly, without constructing a public snapshot wrapper.

The five-byte gRPC header is finalized before publication. Bytes' built-in atomic
reference-counted owner supplies Arc-style lifetime, with no second Arc wrapper.
Cloning a snapshot requires no `T: Clone`, retains no input borrow, and does not
allocate, copy payloads or serialize again. Tonic handoff also allocates nothing.
Snapshot creation uses one shared-owner metadata allocation plus a payload
allocation on a cache miss; a warm pool only needs the metadata allocation.
Final snapshot/Bytes/transport release returns the output to its originating pool.
Compression and TLS still operate separately for each send, as before.

`as_bytes` and `into_bytes` expose protobuf-only views. `into_bytes` moves its view
without another atomic clone/drop. The mutable-writer `into_inner` export and
archive-to-snapshot conversion have been removed. Use `clone()` or `into_bytes()`
to preserve zero-copy sharing.

## Ordinary stream batching

Uncompressed ordinary `Stream<T>` / Arc / Box inputs now drain ready messages
into one pooled output allocation per batch, not one output per message. A batch
ends on source Pending/end/error, a cumulative 32 KiB framed-size **hint**, or
128 messages. There is no timer or wait to fill a batch; an empty poll stays
Pending. Source errors follow any preceding completed batch.

The reverse writer first stages bounded input values/handles, then archives them
in reverse source order so the final wire order matches the stream. Each message
is serialized once through a reference, including its own five-byte gRPC header.
There is no per-message payload buffer or concatenation copy. The staging Vec
is reused within the RPC, but its initial allocation/growth is additional to the
single output buffer. Shadow construction/size hints do not serialize payloads;
custom serializers must not depend on archive invocation order across messages.

The actual encoded lengths are checked individually against Tonic's send limit;
the batch's total length is not treated as one message. Dishonest/underestimated
hints and reservation caps still grow safely without re-encoding, potentially
moving the already-written suffix. Thus one archive is guaranteed; zero growth
copies requires sufficient capacity. Hints are not a hard batch-byte limit.

The allocation is handed directly to the body and remains owned until its final
Bytes/kernel reference drops. Ready messages from different RPCs are not mixed.
Already-encoded `EncodedSnapshot<T>` / `OwnedMessage` / `EncodedSender` inputs retain
their per-frame ownership handoff: combining those into one contiguous buffer
would require copying existing payloads. Compression remains unchanged.

## Thread-local slot pools

Every encoding thread lazily initializes one pool, shared by all services,
clients, codecs, encoders, TonicTransport, EncodedSender and eager snapshots used
on that thread. Services do not store pool handles. Tasks that migrate between
runtime workers acquire from whichever thread actually encodes; release always
goes to the originating pool, not the thread performing the release.

Configure once at startup, before the first pooled encode on any thread:

```rust,ignore
proto_rs::configure_encode_pool(proto_rs::EncodePoolConfig {
    max_buffers: 8,
    max_buffer_capacity: 1024 * 1024,
}).expect("pool configuration was already set or encoding already started");
let server = EchoServer::new(service); // No per-service pool settings.
```

The process-wide configuration is immutable. Omitting this call fixes defaults
on first use: eight slots and a 32 MiB per-slot retained-capacity cap. A repeated
or late call returns `Err(config)`; it never silently replaces existing pools.
Either zero disables retention. Payload buffers are allocated lazily, not
preallocated to the watermark. The cap includes framing, not allocator metadata.

The pool contains fixed slot arrays and cache-padded `AtomicU64` ownership masks.
Small pools spread across up to four shards; larger shards use at most 63 slots
per mask. The top bit belongs to TLS; lower set bits represent active leases.
Unused bits are excluded from acquisition. A non-atomic TLS cursor
remembers the last successful mask; busy masks advance the probe.
Acquisition loads a mask and claims a free bit with Acquire `fetch_or`.
Only the TLS thread can set lease bits; other threads only clear them, so a bit
observed free stays free until this thread claims it. No CAS retry or allocation
on an unrelated racing return is needed. A full pool falls back to a temporary
allocation, without waiting or evicting a slot.

The lease is held until final snapshot/Bytes/transport release. Return clears
and trims the buffer, restores its original slot and clears its ownership bit with
AcqRel `fetch_sub(bit)`. The lease uniquely owns a set bit, so subtraction clears
only that bit without borrowing. It also returns the previous ownership word,
using a direct atomic instruction on x86 rather than a CAS retry loop. That same
operation releases the shard lifetime: there is no separate pool Arc
increment/decrement. Buffer access
is protected by the lease and a narrowly scoped unsafe implementation, not by
a mutex. Masks can still contend—especially simultaneous remote returns.

The TLS bit and lease bits keep each originating shard alive. Thread exit
clears the TLS bit; outstanding outputs remain valid. The operation clearing
the final bit destroys the shard, after every slot has been restored. Return
never accesses TLS. No retirement flag, thread identity lookup,
configuration LRU, synchronized byte budget or owner-count check is needed.
Encoding during TLS teardown falls back to an unpooled allocation.

Retention is bounded by slot count times per-slot capacity **per thread**.
These settings do not bound in-flight/fallback allocations, compression buffers
or pending futures, and do not limit message size. Oversized outputs can grow
normally and are trimmed only after final release. Shrinking can reallocate;
repeatedly exceeding the watermark repeatedly grows and shrinks. An allocator
that cannot shrink enough causes that allocation to be discarded, leaving an
empty reusable slot. Buffers are not zeroized.

Synchronous `Encoder::encode` copies from the same TLS pool and releases its
lease before returning, including unwinding. It creates no Bytes owner record.
There is no separate one-buffer scratch cache. Caller-owned raw `encode` and
`encode_to_vec` still return/write the caller's storage rather than a pooled lease.

Migration: remove `SnapshotPool` and `with_encode_pool`; call
`EncodedSnapshot::new` or `with_max_preallocation` directly. Configure retention
with the startup function, not a service builder. `PrepareRequest::prepare_request`
now takes only its reservation limit. Byte/count pool-inspection methods are
removed along with the explicit handle.

Single-message bodies still refine output hints before reservation. Growth never
restarts serialization, and framing headroom avoids final compaction.
`Bytes::from_owner` still allocates ownership metadata. The measurements below
describe **superseded implementations**, not the TLS bitset pool.

## Automatic transport fallback

```rust,ignore
let client = EchoClient::connect_auto(
    endpoint,
    proto_rs::grpc::ChannelOptions {
        kernel_zero_copy: true,
        ..Default::default()
    },
).await?;
```

Also available as `grpc::AutoChannel::connect`. Existing `connect` uses normal
Tonic Channel. AutoChannel requests the experimental owned Hyper/h2 backend only
for compatible plaintext endpoints, including configured policies. TLS, Unix
sockets, unavailable build support or setup failure select the configured normal
endpoint. Explicit TLS
settings are honored even on an http URI. TLS is never downgraded.

Unsupported SO_ZEROCOPY/send operations fall back to ordinary writes; copied
completions disable subsequent attempts. Successful partial sends are not replayed.
One tracing WARN is shared across AutoChannel clones and replacement connections. Known reasons are logged
during connect; later kernel fallback also notifies the shared warning state. Install
a tracing subscriber to see logs. No warning if acceleration was not requested.

The owned backend now reuses Tonic's channel manager, including shared buffering,
reconnects and policy layers. Each endpoint multiplexes RPCs over HTTP/2; GOAWAY
can leave a draining old connection alongside its replacement. AutoChannel's
static/dynamic balancing uses Tonic's balancer. No possibly executed request is
automatically replayed. Low-level ZeroCopyChannel remains explicit and rejects HTTPS.
Kernel completion/quarantine rules and downstream Tonic patch requirements remain.
No physical-NIC speedup has been demonstrated.

## Compression and TLS

Their implementation/settings are unchanged. Compression reads owned protobuf
bytes without an intermediate uncompressed copy. Compressed output/workspace and
rustls ciphertext record allocations are not covered by the new protobuf pool.
TLS fallback retains pooled serialization and owned Tonic handoff, not kernel
zero-copy of plaintext.

## Performance and verification

### Follow-up: the apparent 256 KiB regression and 16 KiB unary

The historical 178.83 → 143.34 ns unary row is a **19.8% improvement**, not a
regression. The 256 KiB × 16 stream's +7.4% was not reproduced in a controlled
repeat pinned to CPU 2: **29.360 µs batched versus 29.407 µs unbatched** (−0.16%,
noise). These are pre-cleanup means from `snapshot-before-pin2`, one executable,
sequential 50-sample/1 s warmup/3 s measurement runs. Earlier unpinned runs changed
both magnitude and sign. CPU placement/frequency/cache conditions are plausible
contributors, not a proven attribution of the original seven-percent difference.

A separate CPU-2 `perf record -e cycles:u -F 499` profile of the 256 KiB case
places about **94% of sampled cycles in libc's AVX-512 copy routine** (confirmed
by disassembly). Each body still serializes/copies 4 MiB: these messages exceed
the 32 KiB batching threshold and go out individually. This is not a hidden
batch concatenation copy. For 16 KiB unary, about **46%** is in libc copying,
**14%** in pool checkout and **11%** in buffer return, with the rest spread over
encoding, ownership and the inlined benchmark loop. Sampling percentages are
approximate and do not isolate every operation within inlined code.

This cleanup consolidates root hint/archive logic and avoids constructing a
second shadow when the first message already exceeds the batch threshold; a
test asserts one shadow, one archive, and no poll-ahead. The output writer and
pool stay shared with ordinary borrowed requests. For fan-out, sharing one
EncodedSnapshot removes subsequent serialization/payload copies entirely.
For unrelated messages, that source-to-protobuf copy remains necessary under
the contiguous-output design. These measurements predate the local-spare and
cross-thread FIFO queue follow-up below. No new unsafe code, compression changes
or connection manager were added.

Pinned CPU-2 before/after means for this cleanup (±4% is noise):

| Ordinary body case | Before | After | Change |
| --- | ---: | ---: | ---: |
| 16 B unary | 83.43 ns | 80.74 ns | −3.2%, noise |
| 16 B × 128 stream | 4.374 µs | 4.422 µs | +1.1%, noise |
| 16 KiB unary | 137.17 ns | 139.11 ns | +1.4%, noise |
| 256 KiB × 16 stream | 29.360 µs | 29.029 µs | −1.1%, noise |

No measured ordinary-encoder improvement or regression exceeds the noise band
in this subset. The singleton shadow optimization is structurally verified, not
claimed as a measurable large-payload speedup. After cleanup, the unbatched
256 KiB control is 29.195 µs, also effectively equal to the batched dispatcher.

The new `shared_snapshot` fan-out workflow measures **27.13 ns** for one 16 KiB
body-frame handoff and **328.99 ns** for sixteen handoffs of the same 256 KiB
snapshot. Encoding happens once **outside timing**, so these numbers are not
encoder throughput or network/TLS/gzip latency. The comparison demonstrates the
benefit of reusing already-encoded immutable storage. Pointer and allocation
tests verify no clone/handoff allocation, and concurrent-client tests exercise
the shared snapshot with plaintext/TLS and gzip.

Shared-snapshot verification: **193 root tests**, **69 stable tests** (41 library,
12 allocation/integration, 16 non-socket body tests), and **23 Miri tests**
(10 snapshot/cache, 13 body) pass. Focused Clippy and minimal-feature checks
pass with pre-existing warnings. No full benchmark suite or varint benchmarks
were run.

```sh
# Compile without constraining the compiler; pin only timed runs.
cargo bench -p proto_rs --bench tonic_encode --features linux-zerocopy,tonic-gzip --offline --no-run
taskset -c 2 cargo bench -p proto_rs --bench tonic_encode --features linux-zerocopy,tonic-gzip \
  --offline -- '^tonic_encode/(small_unary|small_stream|large_unary|oversized_stream)/(direct|unbatched_owned)$' \
  --baseline snapshot-before-pin2 --noise-threshold 0.04
taskset -c 2 cargo bench -p proto_rs --bench tonic_encode --features linux-zerocopy,tonic-gzip \
  --offline -- '^tonic_encode/(large_unary|oversized_stream)/shared_snapshot$' \
  --save-baseline shared-snapshot-pin2 --noise-threshold 0.04
```

The ordinary uncompressed path now coalesces ready messages. Exactly one metadata
allocation per warmed 32-message batch (no payload or staging allocations) is
checked by an allocation test. Pointer/lifetime, inaccurate-hint, cross-thread return,
count eviction, thread exit, borrowed input release, streaming queue, warning-once,
TLS policy and plain/gzip/TLS/kernel tests cover the new paths.

Focused benchmark results below use the historical `direct` case name to preserve
comparability. ±4% is noise; no blanket no-regression claim is made.

### Batched versus unbatched owned frames

Same-executable sequential comparison after restoring ordinary stream batching,
50 samples, 1 s warmup, 3 s measurement. `unbatched_owned` is a benchmark-only
adapter around the same current ProtoEncoder/pool that disables the batch hook.
Criterion **mean estimates**, with no sockets or compression in these cases:

| Case | Unbatched owned | Batched owned | Time change |
| --- | ---: | ---: | ---: |
| 16 B unary | 88.51 ns | 79.85 ns | −9.8% |
| 16 B × 128 stream | 11.413 µs | 4.394 µs | −61.5% |
| 16 KiB unary | 145.56 ns | 138.72 ns | −4.7% |
| 16 KiB × 128 stream | 19.215 µs | 15.650 µs | −18.6% |
| 256 KiB unary | 2.919 µs | 2.889 µs | −1.0%, noise |
| 256 KiB × 16 stream | 42.253 µs | 45.368 µs | +7.4% |

The 256 KiB stream's isolated repeat reversed direction: 30.195 µs unbatched
versus 28.795 µs batched (−4.6%). Its absolute times also shifted substantially;
do not claim either a consistent improvement or a proven no-regression result
there. These messages already exceed the batching threshold, so they remain
individual frames. The tiny stream is still about 81% slower than the historical
2.429 µs borrowed-buffer baseline below, despite the improvement over owned
per-message handoff. A body frame is not necessarily a socket syscall or packet.

```sh
cargo bench -p proto_rs --bench tonic_encode --features linux-zerocopy,tonic-gzip \
  --offline -- '^tonic_encode/(small_unary|small_stream|large_unary|large_stream|oversized_unary|oversized_stream)/(direct|unbatched_owned)$' \
  --save-baseline owned-batching --noise-threshold 0.04
```

Saved `base` remains untouched; the isolated large-stream repeat is saved as
`owned-batching-repeat`. Only this focused target was benchmarked for batching.

Batching verification: 190 root tests pass with linux-zerocopy/tonic-gzip,
including localhost plaintext/TLS/gzip and the owned kernel transport. Stable
passes 39 library, 12 allocation/integration and 15 non-socket snapshot/body tests.
Miri passes eight snapshot/cache tests and twelve non-compression body tests.
Focused Clippy and minimal-feature checks pass, apart from pre-existing warnings.
New tests cover exactly-once archives despite dishonest hints, wire order,
Pending/error flushes, count/size thresholds, individual limits, malformed batch
headers, mixed buffered/batched ordering, and warmed allocation counts.

### Historical per-message owned path

Historical pre-batching run, September 22, Ryzen AI MAX+ 395, nightly/fat LTO. Only affected body benchmarks
were run, sequentially; no varint or full benchmark suite. Saved `base` was not
overwritten. Criterion **mean estimates** (not the separately printed slopes):

| Case | Saved borrowed-writer mean | Unified owned mean | Change |
| --- | ---: | ---: | ---: |
| 16 B unary | 68.81 ns | 82.52 ns | +19.9% |
| 16 B × 128 stream | 2.429 µs | 10.226 µs | +320.9% |
| 16 KiB unary | 178.83 ns | 143.34 ns | −19.8% |
| 256 KiB unary | 1.943 µs | 2.308 µs | +18.8% |

All four differences exceed the noise band. This table predates the restoration
of ordinary streaming coalescing; it records the motivation, not current batch
performance. The small-stream case is one RPC body with 128 messages, not 128
unary RPCs or a network-throughput measurement.

For the 20-record, 64 MiB repetitive batch, current body point estimates are
**3.206 ms plain** and **17.300 ms gzip**. Earlier direct-body measurements were
4.017 ms and 16.774 ms respectively: the gzip difference is within ±4%, while
plain is faster in this historical comparison. These are not matched simultaneous
A/B runs or physical-network measurements. The pooled snapshot previously measured
3.231 ms plain, effectively matching the new ordinary path within noise.

```sh
cargo bench -p proto_rs --bench tonic_encode --features linux-zerocopy,tonic-gzip \
  --offline -- '^tonic_encode/(small_unary|small_stream|large_unary|oversized_unary)/direct$' \
  --baseline base --noise-threshold 0.04
PROTO_RS_BENCH_NO_NETWORK=1 PROTO_RS_BENCH_DATA=compressible \
  cargo bench -p proto_rs --bench bytes_pipeline --features linux-zerocopy,tonic-gzip \
  --offline -- 'body/(plain|gzip)/direct$' --save-baseline unified-owned --noise-threshold 0.04
```

Pre-batching verification: 184 root tests with linux-zerocopy/tonic-gzip, 38 stable library tests
and 12 stable allocation/integration tests, fourteen Miri snapshot/cache/body tests,
and three real protoc interoperability checks pass. The 11 owned-snapshot tests
also pass without the Linux transport feature, exercising ordinary/TLS fallback.
Focused Clippy passes apart from
pre-existing primitive const-function suggestions and manifest warnings. The full
upstream vendored Tonic test suite is not claimed as validated.

## Historical same-thread spare and FIFO return queue follow-up

The shared retention path now uses `crossbeam-queue` 0.3.14 (`ArrayQueue`),
with FIFO eviction for both count and byte spills. Removed the pool mutex,
shared vector, best-fit/extrema scans and separate explicit-pool eviction policy.
The warm local spare bypasses queue operations entirely. Strong compare-exchange
keeps spurious weak-CAS failures from dropping uncontended returns; Miri caught
this during validation. No new unsafe code was added.

Removed legacy `ZeroCopy`/`ZeroCopyPool`/`to_zero_copy`, `ProtoRequest`, duplicate
`SunByVal`, mutable snapshot conversions and redundant preparation forwarding.
The required Tonic destination-buffer fallback still exists for compatible
encoders, but uses the common writer without allocating temporary Bytes ownership
metadata. Owned transport-neutral unary/server-streaming calls now prepare
synchronously through the borrowed implementation too.

Only the four affected ordinary-body cases were rerun, pinned to CPU 2,
50 samples, 1-second warmup and 3-second measurement. Values below are Criterion
sample means, not its separately reported fitted time estimate. All changes
within ±4% are noise.

| Body case | Before local spare | Local spare + mutex returns | Local spare + FIFO queue | Queue change |
| --- | ---: | ---: | ---: | ---: |
| 16 B unary | 81.36 ns | 69.13 ns | 68.64 ns | −0.7%, noise |
| 16 B × 128 stream | 4.442 µs | 4.408 µs | 4.376 µs | −0.7%, noise |
| 16 KiB unary | 138.52 ns | 130.88 ns | 126.93 ns | −3.0%, noise |
| 256 KiB × 16 stream | 30.115 µs | 28.327 µs | 29.447 µs | +3.95%, noise boundary |

Across the combined local-spare/queue cleanup, unary means improve by 15.6%
and 8.4%; both stream cases remain within noise. The FIFO queue change alone
does not establish a speedup, and these warm-local measurements do not measure
contended cross-thread queue throughput or network throughput.

The first queue run had a 35.874 µs large-stream mean (+26.6% against the
local-spare baseline). This did not reproduce: an isolated repeat was about
29.4 µs, a matched control run reported 29.35 µs direct / 29.26 µs unbatched,
and the final complete subset is reported above. The cause of that first-run
anomaly was not established; it is not being attributed to a code fix.

Baselines: `pool-before-pin2` (before local spare), `pool-before-fifo-pin2`
(preserved final local-spare run), and `fifo-large-control-pin2` (matched control).

```sh
taskset -c 2 cargo bench -p proto_rs --bench tonic_encode \
  --features linux-zerocopy,tonic-gzip --offline -- \
  '^tonic_encode/(small_unary|small_stream|large_unary|oversized_stream)/direct$' \
  --baseline pool-before-fifo-pin2 --noise-threshold 0.04
```

Validation includes concurrent byte/count bounds, FIFO eviction, remote final
release, final-slice lifetimes, thread exit/configuration retirement, disabled
retention, panic recovery, and queue-free warm local reuse. Miri passes 21
snapshot/pool tests and 15 owned-body tests, including compression. The full
root suite passes 205 tests with `linux-zerocopy,tonic-gzip`; 78 focused stable
tests, the all-targets/minimal builds and library Clippy also pass (pre-existing
warnings remain). Compression, TLS and experimental connection management
were not changed.
