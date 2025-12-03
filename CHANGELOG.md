# Changelog

## [0.6.16]
- Add sync methods optimisation
- Add #[proto_import_all_from(package_name)] attribute
- Add VecDeque

## [0.6.15]
- Add std Mutex and parking_lot Mutex 
- Add #[proto(getter = &*$.field)] attrubute (view tests/getter_reference.rs)

## [0.6.14]
- Add chrono::TimeDelta

## [0.6.13]
- Change validators signature to &mut value

## [0.6.12]
- Fix maps with Copy values

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
- Introduced `#[proto_transparent]` support for structs and improved transparent decoding.
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
