# Owned-buffer transport forks

Hyper/h2 forks are used only by the experimental `linux-zerocopy` feature. The
normal Tonic transport continues using registry Hyper/h2; those forks are not
global patches. Tonic is patched at the workspace root for owned eager snapshot
handoff, shared by proto_rs and tonic-prost to preserve their type identity.

The root `tonic-owned` feature selects these Tonic extensions and remains enabled
by default. `default-features = false, features = ["stable", "tonic-transport"]`
instead compiles against unpatched registry Tonic, using a buffered copy adapter.
The `linux-zerocopy` feature implies `tonic-owned`.

| Directory | Published source | Local package |
| --- | --- | --- |
| h2 | h2 0.4.20 | proto-rs-h2 0.4.20-proto.1 |
| hyper | hyper 1.12.0 | proto-rs-hyper 1.12.0-proto.1 |
| tonic | tonic 0.14.9 | tonic 0.14.9, Cargo patch with proto-rs-owned marker feature |

Source copied from the corresponding crates.io packages. Upstream MIT licenses,
original manifests, and available source metadata are retained. The h2 package's
VCS revision is `b0e890842ac16d3c76836fd4fcc06421acb15fa7`; Hyper's is
`7f67699ce32d717081deb7018b4da7d679277ac0`.
Tonic's VCS revision is `4c0f455c264fe2ec9f9ab5d0cc7fd2824d5daf70`.

The November 2026 refresh uses the latest stable crates.io releases checked on
2026-11-07: h2 0.4.20 (already current), Hyper 1.12.0, and Tonic 0.14.9.
Local owned-buffer hooks are merged onto those published sources. Tonic's new
HTTP/2 reset-stream settings are forwarded to both transport engines.

Local changes:

- Tonic exposes alternate HTTP/2 connection-factory hooks that reuse its
  existing reconnect, buffer, policy and balancing/discovery layers. Endpoint
  shares its configured TCP connector, executor and HTTP/2 settings with the
  owned engine. TLS/UDS retain normal transport. The normal registry Hyper/h2
  path is unchanged; only the connection factory differs for owned endpoints.

- Tonic Encoder gains optional `supports_owned`/`encode_owned` hooks.
  `OwnedMessage` accepts a framed allocation or separate header/body/suffix
  segments. Large messages keep their shared payload allocations; ready messages
  below 32 KiB are coalesced, while singleton messages remain owned.
  The body preserves message order and flushes complete messages before errors.
  `with_compression_cache(cache)` accepts an application-defined `CompressionCache`;
  messages are uncached by default. Tonic validates cached frames and applies
  each subscriber's size limit. Cache storage and synchronization live in the
  application. Compression uses the standard flate2/zstd backends and levels;
  their readers consume the protobuf segments without joining them.
  The separate `supports_owned_batch`/`encode_owned_batch` hooks let ordinary
  codecs drain ready inputs into an uncompressed batch with per-message limits.

- Hyper's runtime write trait has an optional owned two-`Bytes` write hook,
  forwarded by I/O adapters. HTTP/2 installs the hook only for supporting I/O.
- h2 handshakes accept that hook; its frame writer retains header/DATA owners
  across partial writes and Pending, without copying a large DATA prefix into
  the header buffer. It restores header staging capacity after split/freeze.
- `Buf::copy_to_bytes` forwarding through Hyper SendBuf and h2 Prioritized/Take
  preserves the backing `Bytes` allocation; focused tests assert identity.
- Original borrowed-write behavior remains the default.
- h2's test-only tokio-rustls dependency selects ring instead of the default
  aws-lc backend. This does not change runtime TLS behavior.

Run focused fork regressions:

```sh
cargo test --manifest-path vendor/h2/Cargo.toml --lib owned_write
cargo test --manifest-path vendor/hyper/Cargo.toml --features full --lib owned_write
```

The published h2 archive does not include its external HPACK fixtures. To run
all locally available unit tests, add `-- --skip hpack::test::fixture` instead of
the `owned_write` filter; an unfiltered run fails on those missing fixture files.

These renamed packages are **not published**. Repository/path builds resolve them
locally; a crates.io release of proto_rs requires publishing the fork packages
first (h2, then Hyper), even though the root dependency is optional. Audit upstream
security fixes and rebase both forks when updating the transport. Updating only
the registry dependencies does not update these sources.

The Tonic patch is also **required at every downstream workspace root**; Cargo
does not inherit dependency patches. See [downstream setup](../benches/owned-snapshot.md).
The `proto-rs-owned` marker feature deliberately prevents silently selecting an
unpatched Tonic. The repository is not ready for ordinary crates.io publication
until the Tonic distribution/upstreaming strategy is resolved as well.
