# Changelog

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