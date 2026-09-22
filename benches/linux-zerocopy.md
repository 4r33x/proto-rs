# Experimental Linux owned-buffer send path

Enable `linux-zerocopy` for plain HTTP/2, optionally with `tonic-gzip`. This is an
opt-in transport, not a change to the normal Tonic Channel or Server. The isolated
[Hyper/h2 forks](../vendor/README.md) preserve ownership through HTTP/2; Tonic's
ordinary framing and compression policies remain intact. A subsequent separate
[Tonic patch](owned-snapshot.md) adds eager `EncodedSnapshot<T>` handoff into
this ownership-preserving path.

## What is copied

For the benchmark's exact-sized 20-record batch, configure a 128 MiB encode
reservation cap and message limits. Each `Arc<Vec<u8>>` payload is copied once
into Tonic's owned protobuf output allocation. Hyper/h2 then pass slices of that
same allocation, plus separate HTTP/2 headers, to `sendmsg(MSG_ZEROCOPY)`.
Linux can reference those userspace pages instead of copying the payload into
kernel storage. This is not encoding into a kernel-owned allocation.

With gzip, Tonic first encodes protobuf and then produces compressed output.
The transport submits the **compressed** allocation; compression's input/output
transformation and workspace are not eliminated. Small frames may still be
copied/coalesced, and Linux can copy any submission. No end-to-end zero-copy
claim applies to decoding at the receiver.

TLS is **not implemented** on this transport. `https://` is rejected before
connecting, never downgraded. Normal Tonic TLS remains available separately.

## Usage

```rust,ignore
use proto_rs::grpc::zerocopy::{ZeroCopyChannel, ZeroCopyConfig};

let channel = ZeroCopyChannel::connect(
    "http://127.0.0.1:50051".parse()?,
    ZeroCopyConfig::default(),
).await?;
let metrics = channel.metrics();
let client = MyServiceClient::new(channel)
    .max_encoding_message_size(128 * 1024 * 1024)
    .max_decoding_message_size(128 * 1024 * 1024)
    .with_max_encode_preallocation(128 * 1024 * 1024);
```

For a server, accept with `ZeroCopyIo::accept` and pass the I/O and generated
service to `serve_connection`. The [runnable example](../examples/zerocopy_transfer.rs)
shows both sides. The client now uses Tonic's connection manager, not an isolated
single sender. The server helper does not install all Tonic
Server layers. Callers own admission limits, request timeouts and graceful shutdown.

## Managed client lifecycle

`ZeroCopyChannel::connect_endpoint(endpoint, config)` preserves all supported
plaintext Endpoint settings. `connect_lazy` defers connection establishment;
`handshake` uses the supplied socket first and reconnects to its origin afterward.
All clones share Tonic's bounded request buffer and reconnect state. DNS/Happy
Eyeballs and TCP options use the same connector as normal Tonic. The same Tonic
layers apply origin, user agent, deadlines, rate/concurrency limits and executor;
the owned Hyper builder receives the same HTTP/2 window, frame, header and
keepalive settings. Connection replacement never re-encodes a message.

This is Tonic's recovery policy, not transparent RPC retry: failed in-flight
calls return an error, and subsequent calls can reconnect. A possibly executed
RPC is never replayed automatically. GOAWAY can drain existing RPCs while a new
connection serves new calls. Request-buffer capacity is not a total in-flight
memory limit and does not bound already prepared payloads waiting to enqueue.

Use `AutoChannel::balance_list(endpoints, options)` for a fixed list, or
`AutoChannel::balance_channel::<Key>(capacity, options)` for a channel plus a
sender of Tonic `Change::Insert(key, endpoint)` / `Change::Remove(key)` events.
These use Tonic's existing balancer/discovery worker. Each endpoint retains its
configuration; TLS endpoints use normal encrypted transport. The balancing
worker uses Tonic's default executor, while per-endpoint HTTP/2 tasks honor the
endpoint executor. As with Tonic, an empty discovery set waits for endpoints.

Channel `metrics()` now returns `ZeroCopyChannelMetrics`: cumulative totals,
`connections()`, and `wait_for_idle()` including outstanding sends on replaced
connections. Closed/drained entries fold into totals. Raw `ZeroCopyIo::metrics()`
still returns per-socket `ZeroCopyMetrics`. AutoChannel exposes optional
`kernel_metrics()`. Balancing metrics cover owned connections, not TLS fallbacks.
Adaptive downgrade persists across reconnects **per endpoint**; a copied path
does not disable acceleration on other balanced endpoints. One warning state
is shared across the entire channel's clones and connection replacements.
Failure to allocate the completion descriptor/thread falls back before exposing
the socket when fallback is enabled. Failures after submissions still follow
the conservative completion/quarantine rules below.

Connection management does not mean "no downside" on every workload: kernel
pinning/completion overhead, completion threads, possible copies, TLS fallback,
and exceptional failure quarantine still apply. No physical-NIC speedup is claimed.
See the [matched managed-channel benchmarks and regression coverage](managed-channel.md).

## Lifetimes, backpressure, and failure behavior

- Only freshly created/accepted sockets are supported: the transport exclusively
  owns zero-copy submission IDs and the socket error queue.
- Owned `Bytes` slices are retained **before** sending and until their completion
  IDs arrive. Partial writes retain only accepted ranges; coalesced, out-of-order
  and wrapping completion IDs are handled. Borrowed writes never request zero-copy.
- Defaults: 16 KiB minimum send, 256 pending submissions, 8 MiB of pending ranges.
  Reaching a limit applies async backpressure. Slices can keep larger backing
  allocations alive, so these are **not RSS or total memory limits**.
- One dedicated completion thread and duplicated descriptor per enabled
  connection survive connection cancellation and Tokio runtime shutdown.
  Slow peers/teardown can retain allocations longer than the RPC lifetime.
- Unrecoverable completion-reader failure quarantines unresolved buffers and its
  descriptor **until process exit** rather than risking use-after-free. Metrics
  report this failure. This deliberate exceptional-path resource leak and the
  per-connection threads are limitations of the experimental implementation.
- `metrics.wait_for_idle()` waits for release notifications, not application
  receipt. Stop producers first and apply a timeout. Completion counters count
  accepted byte ranges, not retained allocation capacity or NIC DMA operations.
- By default, unsupported operations and resource errors fall back to ordinary
  writes; a copied completion disables future zero-copy requests. Outstanding
  submissions are still retained and drained. Forced mode is for validation,
  not a guarantee that Linux won't copy.

The [Linux MSG_ZEROCOPY documentation](https://www.kernel.org/doc/html/latest/networking/msg_zerocopy.html)
describes the completion contract and copy fallbacks. In particular, loopback
always copies. `completed_without_copy_flag` only reports the absence of the
kernel's copied flag; it is not independent proof of NIC DMA.

## Two-host validation

Run on a trusted test network: this fixture transport is plaintext, not TLS.
Use two physical hosts and their physical-interface addresses, not localhost.
The fixture contains 20 distinct byte allocations totaling exactly 64 MiB;
each echo verifies every field and payload byte. Request setup clones 20 Arc
handles outside timing, not their payload bytes.

```sh
cargo build --release --example zerocopy_transfer --features linux-zerocopy,tonic-gzip
# Host A:
target/release/examples/zerocopy_transfer server HOST_A_IP:50051 --force
# Host B:
target/release/examples/zerocopy_transfer client http://HOST_A_IP:50051 --force
```

Repeat with `--gzip` on both sides to exercise compressed output. Remove `--force`
to test adaptive fallback. Inspect both peers' snapshots after completions drain:
pending counts should reach zero, completion errors should be zero, and submitted
bytes should equal completed-with-copy plus completed-without-copy bytes.
Compare CPU, throughput and tail latency against ordinary Tonic on the same NIC,
kernel and workload before deployment. A two-host test has **not** been run here.

## Focused checks and local benchmark

Tests cover plain/gzip gRPC in both directions, full payload equality, small
in-flight limits, cancellation, adaptive fallback, completion ID wrap/coalescing,
owner drop timing, ancillary parsing, and allocation identity through HTTP/2
partial writes/Pending. The memory-only lifetime/parser tests also run under Miri.

```sh
cargo test -p proto_rs --features linux-zerocopy --lib --test linux_zerocopy
PROTO_RS_BENCH_DATA=compressible cargo bench -p proto_rs \
  --bench bytes_pipeline --features linux-zerocopy,tonic-gzip -- \
  'rpc/(plain|gzip)/(direct|zerocopy)'
```

`zerocopy_forced_loopback` deliberately keeps requesting MSG_ZEROCOPY despite
copied completions to exercise the entire 64 MiB send and completion path. It is
an overhead/correctness check, **not evidence of copy avoidance or speedup**.

September 22, 2026, this host (AMD RYZEN AI MAX+ 395, nightly rustc, fat LTO),
compressible 64 MiB fixture, 10 samples, 100 ms warmup, 1 s requested measurement:

| Upload + receipt RPC | Ordinary direct send | Forced loopback MSG_ZEROCOPY |
| --- | ---: | ---: |
| Plain | 20.648 ms | 24.613 ms |
| Gzip, zlib-rs | 31.476 ms | 31.478 ms |

These are Criterion point estimates from the same invocation, not two-host or
echo timings. Plain forced loopback was approximately 19.2% slower; gzip was
effectively unchanged. All completions reported copying, pending ranges drained
to zero, and completion errors were zero. Forced mode deliberately disables the
default adaptive fallback, so it measures ongoing pin/completion overhead.

Validation: all 170 root tests with `linux-zerocopy,tonic-gzip` pass, including
44 library tests and five socket/gRPC integration tests. Four Miri lifetime/parser
tests, 44 available h2 unit tests and 101 Hyper unit tests also pass.
The crates.io h2 archive omits its external HPACK fixture files: 382 fixture tests
fail with file-not-found if run unfiltered, so the available-suite invocation is
`cargo test --manifest-path vendor/h2/Cargo.toml --lib -- --skip hpack::test::fixture`.
Those fixture tests have not been validated here. Stable builds and focused
Clippy checks also pass (unrelated pre-existing warnings remain).
