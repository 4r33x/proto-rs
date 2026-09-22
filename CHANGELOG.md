# Changelog

## [0.12.0]

### Transport abstraction and traits

- Add the transport-neutral `proto_rs::grpc` API: `Request`, `Response`, `Status`, `Code`, `MetadataMap`, and `Extensions`.
- Add `GrpcTransport`, a client-side trait supporting unary, client-streaming, server-streaming, and bidirectional RPCs. Generated `<service>_transport_client` modules accept custom implementations without requiring Tonic.
- Add `GrpcService`, `MethodDescriptor`, `RpcKind`, and `MessageStream`. Generated `<service>_service` routers handle protobuf decoding, validation, typed dispatch, and response encoding; transport implementations supply framing and I/O.
- Provide `TonicTransport` and response-stream adapters behind the `tonic` feature. `tonic-transport` enables network transport and generated `connect` helpers; disable default features to use only a custom transport.
- Add experimental Linux-only `linux-zerocopy`: isolated Hyper/h2 forks preserve owned HTTP/2 output buffers through `MSG_ZEROCOPY`. `ZeroCopyChannel` and `serve_connection` support generated Tonic services with plain HTTP/2 and gzip; HTTPS is rejected. Track partial sends, completion ranges, cancellation-safe ownership, backpressure, and kernel copy fallbacks. Includes allocation-identity/lifetime tests, forced-loopback 64 MiB benchmarks, and a two-host runner. This is opt-in, has per-connection completion threads and exceptional-failure buffer quarantine, and requires publishing the vendored fork packages before a crates.io release; it is not a measured physical-network zero-copy speedup.
- Make transport-neutral response encoding mode-aware through `GrpcEncode<Mode>` and `ProtoResponse`, including zero-copy responses and Box/Arc dereferencing.
- Extend `ProtoExt` with `WRAP_ROOT` for standalone framing and `IS_BYTE` plus safe byte-access hooks for byte-container specialization. Add state-aware decoder hooks to preserve fixed-array cursors across field occurrences.

### Breaking changes and migration

- Replace `ZeroCopy`, `ZeroCopyPool`, and `to_zero_copy` with `EncodedSnapshot` and `to_encoded_snapshot`; allocation reuse is automatic through TLS. Remove the legacy `ProtoRequest` adapter, duplicate `SunByVal` mode (use `SunByRef`), and snapshot mutable-writer/archive conversions. Use `PrepareRequest` for generated requests and `clone()`/`into_bytes()` for shared snapshots. Snapshot clones share one immutable allocation until final snapshot/Bytes/transport release; ordinary borrowed RPCs do not need a wrapper.
- Generated Tonic unary/server-streaming methods prepare borrowed requests synchronously and return request-independent futures. `&T` and `Request<&T>` no longer borrow input across await; custom request adapters implement `PrepareRequest<T>`. Context interception now precedes readiness polling; transports/interceptors and transport futures must be Send. Nightly futures remain unboxed; stable uses one box for this interface.
- All Tonic output encoding and eager snapshots automatically share one lazy TLS slot pool per thread, including synchronous destination copying. Remove explicit `SnapshotPool` handles and service/codec/encoder `with_encode_pool` builders. Set slot count and per-slot trimming capacity once at startup through `configure_encode_pool(EncodePoolConfig { max_buffers, max_buffer_capacity })`, before any thread's first pooled encode. `PrepareRequest::prepare_request` no longer takes a pool argument. Limits are per thread; standalone snapshots now reuse TLS slots.
- Add `AutoChannel`/generated `connect_auto` with once-per-channel fallback warnings, preserving TLS and configured endpoint policies. Add borrowed `TonicTransport` calls and `EncodedSender` for owned streaming requests. Compression and TLS record allocation are unchanged.
- Replace the owned channel's single, non-reconnecting sender with Tonic's existing channel manager and an alternate HTTP/2 factory. Preserve buffering, backpressure, reconnects, deadlines/limits, origin/user agent, DNS/Happy Eyeballs, socket/HTTP2 tuning, and custom executors. Add lazy connection and AutoChannel static/dynamic endpoint balancing with encrypted fallback. Drain existing RPCs across GOAWAY without replaying possibly executed requests. Channel metrics now aggregate across replacements; fallback warnings span clones/reconnects and adaptive fallback is remembered per endpoint. Kernel resource costs and the exceptional completion-quarantine policy remain.

- Use `proto_rs::grpc::{Request, Response, Status}` in transport-neutral service implementations. Request-scoped validators use `proto_rs::grpc::Extensions`; Tonic integration converts between the corresponding transport types.
- Encode standalone scalars and collections as field 1 of an implicit wrapper message. Message-valued Option, Box, Arc, Mutex and related wrappers now also emit their schema-declared field-1 envelope. This changes previously inconsistent standalone wire representations; raw payload callers can still use `ProtoArchive::archive::<0>`.
- Use repeated-varint encoding for byte-valued sets and wrapped u8 elements rather than treating them as raw byte buffers. Regenerate affected schemas and clients, and coordinate upgrades with peers or stored data using the previous representation.

### Correctness and interoperability

- Replace byte-container layout casts with safe byte access hooks.
- Fix split packed/unpacked fixed arrays, including nested message occurrences, and merge repeated message-valued oneof variants.
- Release mutex encoding guards between fields so aliased mutex fields do not deadlock.
- Emit valid RPC message wrappers for scalars and enums, bytes (not repeated uint32) for byte collections, and inline optional/repeated fields for Rust aliases. Fixed arrays use the corresponding sequence schema; their length constraints remain Rust-side validation.
- Honor Box/Arc response dereferencing in the transport-neutral gRPC encoder, matching the tonic codec and generated response schemas.
- Preserve ArcSwapOption presence for default-valued payloads and merge ArcSwap message occurrences through owned decoding shadows.
- Validate emitted schemas with protoc and compare encoding/decoding against its C++ implementation in interoperability regression tests (requires protoc, or the PROTOC environment variable).

### Performance

- Use fixed TLS slot arrays with cache-padded `AtomicU64` ownership masks. The single TLS producer claims free bits with Acquire `fetch_or`; final release trims oversized allocations, restores the original slot and clears its uniquely owned bit with AcqRel `fetch_sub`. This avoids CAS retries, and one TLS bit plus up to 63 lease bits per shard also manage its lifetime, removing separate pool Arc increments/decrements. Full pools use temporary unpooled allocations. No queue, mutex, eviction, size classes, aggregate byte accounting, configuration LRU, owner-count checks or retirement protocol. Defaults are eight slots capped at 32 MiB each per thread. Leases remain valid across cross-thread release and TLS destruction.
- Consolidate Arc/Box Tonic adapters through `Deref`; remove the redundant Bytes ownership allocation from the required destination-buffer adapter. Retain Tonic's buffered fallback for Prost and caller-provided encoders; it is not an obsolete protobuf encoding path.
- Create shared snapshot ownership metadata once during encoding, eliminating per-recipient handoff allocations. Cloning needs no `T: Clone` and retains no input borrow. Reuse the first shadow for threshold-sized standalone stream messages and consolidate root hint/archive logic with the ordinary borrowed-request writer.
- Coalesce ready ordinary uncompressed stream messages into one pooled owned batch, with one archive per message and no encoded-message concatenation copy. Bounded input staging preserves wire order with the reverse writer and is reused within an RPC. Flush on Pending/end, the 32 KiB size-hint threshold, or 128 messages; enforce individual message limits. Eager snapshots and compression keep their existing paths.

- Preserve eager `EncodedSnapshot<T>` snapshots through an owned-message handoff in a bundled Tonic 0.14.6 fork. Reserve five bytes of gRPC headroom, transfer plain frames without payload copying, and compress directly from owned bytes. Ordinary proto_rs codecs use the same owned writer; byte-container and caller-provided destination interfaces retain copying fallbacks. Tonic users must apply the bundled Cargo patch at their workspace root; this is a distribution constraint, not transparent compatibility with unpatched upstream Tonic.

- Add configurable output reservation caps through codecs, encoders, generated clients/servers, and `TonicTransport` (1 MiB default). Bounded top-level message-batch hints can avoid cold-buffer growth for large flat batches; all writes remain checked, including dishonest hints. Reused high-water allocations are not shrunk to this speculative cap.
- Add opt-in `tonic-gzip` using flate2's `zlib-rs` backend. Add the focused `bytes_pipeline` benchmark: one 20-record vector with 64 MiB of distinct `Arc<Vec<u8>>` payloads, plain/gzip body encoding and plain/gzip/TLS/gzip+TLS localhost RPCs, with full untimed payload validation. Direct output remains userspace encoding, not kernel zero-copy.
- Encode ordinary Tonic messages into pooled owned output and transfer the allocation to the body. Preserve framing, compression and message-size checks; pool limits cover idle retained memory, not in-flight allocations.
- Remove the second per-request service Arc clone and use concrete ready futures for synchronous RPC handlers on stable as well as nightly. Keep nightly async handler futures unboxed.
- Store Tonic response streams inline; eliminate double boxing in transport-neutral streaming responses using pin projection, and avoid stream-container allocations for unary/empty messages. Move metadata from owned Tonic errors instead of cloning it.
- Add focused Tonic-body benchmarks, allocation-counting and cancellation/metadata/compression regressions, and Miri-tested ownership, growth and panic-recovery paths. The replacement shared pool has dedicated overlapping-owner and concurrent-return tests.
- Add `EncodeSizeHint` and runtime encoding-size hints with bounded initial preallocation; support reusable reverse-writer buffers through `ArchivedProtoMessage::new_with_buffer`.
- Bound speculative repeated-field decoding allocations to 4 KiB; share repeated-field framing across collections.
- Encode BTreeSet and VecDeque through borrowed views without allocating temporary shadow collections.
- Decode owned byte containers with one copy, avoiding an intermediate `Bytes` allocation. Allocate fixed-array decode cursors lazily and merge unconverted tuple-oneof payloads in place.
- Improve small scalar/string/bytes collection capacity hints and recognize enums whose values all fit in one byte.
- Add `RevWriter::put_bytes` with a compatible default implementation; `RevVec` fuses small length-delimited fields into one reservation. Keep buffer growth out of line and inline nonrecursive encode/decode entry points to avoid aggregate copies.
- Keep `DecodeError` pointer-sized so successful decode results do not carry large error storage; constructing an error now allocates its diagnostic payload, without changing diagnostic text or public methods.

## [0.11.26]
- Gate generated tonic client transport codegen behind `tonic-transport`

## [0.11.25]
- Update deps, and gate tonic transport to support WASM 

## [0.11.24]
- Fix validators

## [0.11.23]
- Fixed RustClientCtx::only_these_modules module overwriting

## [0.11.22]
- Added proto_rs::schemas::write_only_these for building subset of .proto

## [0.11.21]
- Added RustClientCtx::only_these_modules builder

## [0.11.20]
- Fixed NonZero primitives output in rust client and .proto codgen
- Added per module splitting possibility in rust client output 
- Improve README

## [0.11.19]
- Added NonZero primitives
- Added solana_instruction::Instruction, solana_instruction::AccountMeta, solana_hash::Hash
- Updated README

## [0.11.18]
- Fix .proto output for bytes fields

## [0.11.17]
- Use mut ref in enum validators

## [0.11.16]
- Fix enum validators

## [0.11.15]
- Change #[inline(always)] to #[inline] everywhere due to enormous stack usage

## [0.11.14]
- Correct narrowed primitives in rust client output

## [0.11.13]
- Removed Arc wrapping in generated rust client

## [0.11.12]
- Add + Send + 'static bounds to server streaming responses due to impl Trait capturing rules
- 
## [0.11.11]
- Made rpc_client_ctx fallible 

## [0.11.10]
- Fixed: attributes apply in rust client generation

## [0.11.9]
- Added: remove_type_attribute method to rust client builder

## [0.11.8]
- fixed: AtomicPrimitives written as Primitives in rust client output

## [0.11.7]
- support module scoped type_attribute in build system 

## [0.11.6]
- Even better rpc_client_ctx

## [0.11.5]
- Switch rpc_client_ctx to using trait, instead of function

## [0.11.4]
- Add teloxide UserId

## [0.11.3]
- More fixes in build system

## [0.11.2]
- Fixed bugs in build system

## [0.11.1]
- Improved encoding codegen path by binding temporaries 

## [0.11.0]
- Changed decode path codegen for #[proto_message(sun = [Task], sun_ir = TaskRef<'a>)] by using DecodeIrBuilder<T> trait

## [0.10.0]
- Better decode path codegen

## [0.9.2]
- Added sun_ir #[proto_message(sun = [Task], sun_ir = TaskRef<'a>)] for encoding path override 

## [0.9.1]
- Add init impl of jito rpc client
- Add init impl of bloxroute rpc client
- Add init impl of blockrazor rpc client
- Add init impl of nextblock rpc client 

## [0.9.0]
- Introduced new trait design - codegen and encoding/decoding paths changed to new algo
- Encoding path now uses upb-style reverse writing
- Decoding converts to the Shadow IR exactly once
- Both encoding and decoding performance improved significantly
- Since encoding is done in a single pass now, it is now impossible to produce corrupted messages when using atomics or other concurrent types

## [0.8.0]
- Revert to [0.7.6]

## [0.7.7]
- Remove double to shadow conversion in some cases

## [0.7.6]
- Gate validate_with_ext codegen

## [0.7.5]
- Add AHash Hasher

## [0.7.4]
- Fixed and optimized Mutex code

## [0.7.3]
- Better .proto codegen
- const proto schema validation and proper reflection

## [0.7.2]
- Impl ProtoIdentifiable for AddressHasherBuilder under solana_address_hash feature gate

## [0.7.1]
- Impl ProtoIdentifiable for ZeroCopy<T>

## [0.7.0]
- Impl proper build system for .proto definitions and lightweight rust clients with auto resolving names, imports, attributes
- View tests/proto_build_test for example

## [0.6.24]
- Gate validate_with_ext 

## [0.6.23]
- Fix proto_import_all_from attribute

## [0.6.22]
- Fixed VecDeque .proto definitions

## [0.6.21]
- Relaxed generic bounds in generated code
- Fixed Copy generic types
- Added #[proto(generic_types = [T = [u64, u32]])] attribute for in place .proto generation for types with generics
- Added proper parsing for types with generics for .proto generation 

## [0.6.20]
- Added support for Vec<T> and VecDeque<T> as top-level message
- Added initial support for generics. Types with generics can now be used with proto_message, and concrete generic types can be used in proto_rpc, but types with generics do not emit .proto definitions yet.

## [0.6.19]
- Removed #[cfg(feature = "tonic")] gate on validate_with_ext method

## [0.6.18]
- Added #[proto(validator_with_ext = ...)] attribute

## [0.6.17]
- Fixed multiple streams duplicate assoc. type error

## [0.6.16]
- Added sync methods optimisation
- Added #[proto_import_all_from(package_name)] attribute
- Added VecDeque

## [0.6.15]
- Added std Mutex and parking_lot Mutex 
- Added #[proto(getter = &*$.field)] attrubute (view tests/getter_reference.rs)

## [0.6.14]
- Added chrono::TimeDelta

## [0.6.13]
- Changed validators signature to &mut value

## [0.6.12]
- Fixed maps with Copy values

## [0.6.11]
- Removed SmallVec buffers from zero-copy wrappers and corrected zero-copy encoding/decoding for enums.
- Added support for `Arc` and `Box` response types in `proto_rpc`.

## [0.6.10]
- Added infallible streaming RPC method support on the server side.

## [0.6.9]
- Implemented `CachePadded` wrapper encoding/decoding using reference-based handling.

## [0.6.8]
- Improved transparent `proto_message` syntax and `proto_path` handling.

## [0.6.7]
- Added support for `sun` types with concrete generics in `proto_message`.

## [0.6.6]
- Fixed `proto(skip)` handling for tuple variants in enum proto generation.

## [0.6.5]
- Optimized `prosto_derive` macro generation to reduce code duplication and improve performance.

## [0.6.4]
- Added a `treat_as` attribute for `proto_message` fields to override protobuf mappings.

## [0.6.0] - [0.6.3]
- Introduced `#[transparent]` support for structs and improved transparent decoding.
- Added wrapper and proto generation support for `ArcSwap` types with roundtrip tests.
- Added `CachePadded` wrapper detection and implementations.
- Added support for Rust atomic primitives and SmallVec-backed zero-copy buffers.

## [0.5.0] - [0.6.0] next level design 
- Removed double conversion and prost from design
- Implement protobuf encdoding\decodong from scratch

## [0.5.0]
- Added solana-signature shadow
- Fix solana-address shadow (now properly implements HasProto)
- Relaxed HasProto bounds
- resolve Clippy lints 

## [0.3.0] - [0.4.0] - HUGE REFACTOR 2

- Refactored code to eliminate duplication and improve logic.
- Added support for:
  - Arrays
  - Byte arrays
  - Arrays in tuples
  - Named enum fields
  - Skipping fields in tuple enums
  - Other miscellaneous types
- Added proto shadows for Solana native types under the `solana` feature (currently only `Address`).
- Improved formatting for generated `.proto` files.

## [0.2.0] - HUGE REFACTOR

### Added
- `proto_imports` attribute for any macro
- Changed `file` attribute to `proto_path`
- Add ability to control auto-emission of .proto files with `PROTO_EMIT_FILE` env var and `emit-proto-files` cargo feature
- Add ability to collect and build .proto from single crate from ALL DEPENDENCIES that use proto_rs
- Fastnum proto conversions via feature flag (`D128Proto` and `UD128Proto`)
- stable format to prevent random ordering

## [0.1.1]

### Added
- `#[proto_dump]` macro

### Fixed
- Bug when multiple files with the same name were written to .proto
