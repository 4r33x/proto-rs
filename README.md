# proto_rs 2.0

`proto_rs` makes Rust the source of truth for your Protobuf and gRPC definitions. Version 2.0 tightens the ergonomics of every macro, removes redundant code paths in the runtime, and makes the crate's `no_std` story first class. The crate ships a set of procedural macros and runtime helpers that derive message encoders/decoders, generate `.proto` files on demand, and wire traits directly into Tonic servers and clients.

## Motivation

0. I hate to do conversion after conversion for conversion
1. I love to see Rust only as first-class citizen for all my stuff
2. I hate bloat, so no protoc (shoutout to PewDiePie debloat trend)
3. I don't want to touch .proto files at all

For fellow proto <-> native typeconversions enjoyers <=0.5.0 versions of this crate implement different approach

## Key capabilities

- **Message derivation** – `#[proto_message]` turns a Rust struct or enum into a fully featured Protobuf message, emitting the corresponding `.proto` definition and implementing [`ProtoExt`](src/message.rs) so the type can be encoded/decoded without extra glue code. The generated codec now reuses internal helpers to avoid redundant buffering and unnecessary copies.
- **RPC generation** – `#[proto_rpc]` projects a Rust trait into a complete Tonic service and/or client. Service traits stay idiomatic while still interoperating with non-Rust consumers through the generated `.proto` artifacts, and the macro avoids needless boxing/casting in the conversion layer.
- **On-demand schema dumps** – `#[proto_dump]` and `inject_proto_import!` let you register standalone definitions or imports when you need to compose more complex schemas.
- **Workspace-wide schema registry** – With the `build-schemas` feature enabled you can aggregate every proto that was emitted by your dependency tree and write it to disk via [`proto_rs::schemas::write_all`](src/lib.rs). The helper deduplicates inputs and writes canonical packages derived from the file path.
- **Opt-in `.proto` emission** – Proto files are written only when you ask for them via the `emit-proto-files` cargo feature or the `PROTO_EMIT_FILE=1` environment variable, making it easy to toggle between codegen and incremental development.
- **`no_std` by default runtime** – Runtime helpers lean entirely on `core` and `alloc`; enabling the `std` feature layers on Tonic integration and filesystem tooling without changing the API.

## Getting started

Add `proto_rs` to your `Cargo.toml` and optionally enable features you need (for example to eagerly emit `.proto` files during development):

```toml
[dependencies]
proto_rs = { version = "0.6", features = ["emit-proto-files"] }
tonic = "0.14"
```

Define your messages and services using the derive macros with native rust types:

```rust
use proto_rs::{proto_message, proto_rpc};

#[proto_message(proto_path = "protos/gen_proto/rpc.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct RizzPing;

#[proto_rpc(
    rpc_package = "sigma_rpc",
    rpc_server = true,
    rpc_client = true,
    proto_path = "protos/gen_proto/sigma_rpc.proto",
)]
pub trait SigmaRpc {
    type Stream: futures_core::Stream<Item = Result<FooResponse, tonic::Status>> + Send;

    async fn rizz_ping(
        &self,
        request: tonic::Request<RizzPing>,
    ) -> Result<tonic::Response<GoonPong>, tonic::Status>;

    async fn rizz_uni(
        &self,
        request: tonic::Request<BarSub>,
    ) -> Result<tonic::Response<Self::Stream>, tonic::Status>;
}
```

Once compiled, the trait can be implemented just like a normal Tonic service, but the `.proto` schema is generated for you whenever emission is enabled.

### Running without `std`

Disable the default feature set if you only need message encoding/decoding in `no_std` contexts:

```toml
[dependencies]
proto_rs = { version = "0.6", default-features = false }
```

All core traits (`ProtoExt`, `MessageField`, wrappers, etc.) remain available. Re-enable the `std` feature (enabled by default) when you want the Tonic codec helpers and RPC generation macros.

## Collecting schemas across a workspace

Enable the `build-schemas` feature for the crate that should aggregate `.proto` files and call the helper at build or runtime:

```rust
fn main() {
    // Typically gated by an env flag to avoid touching disk unnecessarily.
    proto_rs::schemas::write_all("./protos")
        .expect("failed to write generated protos");

    for schema in proto_rs::schemas::all() {
        println!("Registered proto: {}", schema.name);
    }
}
```

This walks the inventory of registered schemas and writes deduplicated `.proto` files with a canonical header and package name derived from the file path.

## Controlling `.proto` emission

`proto_rs` will only touch the filesystem when one of the following is set:

- Enable the `emit-proto-files` cargo feature to always write generated files.
- Set `PROTO_EMIT_FILE=1` (or `true`) to override the default at runtime.
- Set `PROTO_EMIT_FILE=0` (or `false`) to force emission off even if the feature is enabled.

The emission logic is shared by all macros so you can switch behaviours without code changes.

## Examples and tests

Explore the `examples/` directory and the integration tests under `tests/` for end-to-end usage patterns, including schema-only builds and cross-compatibility checks.

To validate changes locally run:

```bash
cargo test
```

The test suite exercises more than 400 codec and integration scenarios to ensure the derived implementations stay compatible with Prost and Tonic.

## Optional features

- `std` *(default)* – pulls in the Tonic dependency tree and enables the RPC helpers.
- `build-schemas` – register generated schemas at compile time so they can be written later.
- `emit-proto-files` – eagerly write `.proto` files during compilation.
- `fastnum`, `solana` – enable extra type support.

For the full API surface and macro documentation see [docs.rs/proto_rs](https://docs.rs/proto_rs).
