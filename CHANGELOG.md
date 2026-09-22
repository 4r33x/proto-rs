# Changelog

## [0.12.0]

### Transport abstraction and traits

- Add the transport-neutral `proto_rs::grpc` API: `Request`, `Response`, `Status`, `Code`, `MetadataMap`, and `Extensions`.
- Add `GrpcTransport`, a client-side trait supporting unary, client-streaming, server-streaming, and bidirectional RPCs. Generated `<service>_transport_client` modules accept custom implementations without requiring Tonic.
- Add `GrpcService`, `MethodDescriptor`, `RpcKind`, and `MessageStream`. Generated `<service>_service` routers handle protobuf decoding, validation, typed dispatch, and response encoding; transport implementations supply framing and I/O.
- Provide `TonicTransport` and response-stream adapters behind the `tonic` feature. `tonic-transport` enables network transport and generated `connect` helpers; disable default features to use only a custom transport.
- Make transport-neutral response encoding mode-aware through `GrpcEncode<Mode>` and `ProtoResponse`, including zero-copy responses and Box/Arc dereferencing.
- Extend `ProtoExt` with `WRAP_ROOT` for standalone framing and `IS_BYTE` plus safe byte-access hooks for byte-container specialization. Add state-aware decoder hooks to preserve fixed-array cursors across field occurrences.

### Breaking changes and migration

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
