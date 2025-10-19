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

## Custom encode/decode pipelines with `ProtoShadow`

`ProtoExt` types pair with a companion `Shadow` type that implements [`ProtoShadow`](src/traits.rs). This trait defines how a value is lowered into the bytes that will be sent over the wire and how it is rebuilt during decoding. The default derive covers most standard Rust types, but you can substitute a custom representation when you need to interoperate with an existing protocol or avoid lossy conversions.

The [`fastnum` decimal adapter](src/custom_types/fastnum/signed.rs) shows how to map `fastnum::D128` into a compact integer layout while still exposing ergonomic Rust APIs:

```rust
#[proto_message(proto_path = "protos/fastnum.proto", sun = D128)]
pub struct D128Proto {
    #[proto(tag = 1)]
    pub lo: u64,
    #[proto(tag = 2)]
    pub hi: u64,
    #[proto(tag = 3)]
    pub fractional_digits_count: i32,
    #[proto(tag = 4)]
    pub is_negative: bool,
}

impl ProtoShadow for D128Proto {
    type Sun<'a> = &'a D128;
    type OwnedSun = D128;
    type View<'a> = Self;

    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> { /* deserialize */ }
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> { /* serialize */ }
}
```

By hand-tuning `from_sun` and `to_sun` you can remove redundant allocations, hook into validation logic, or bridge Rust-only types into your RPC surface without ever touching `.proto` definitions directly.

## Zero-copy server responses

Service handlers produced by `#[proto_rpc]` work with [`ZeroCopyResponse`](src/tonic/resp.rs) to avoid cloning payloads. Any borrowed message (`&T`) can be turned into an owned response buffer via [`ToZeroCopyResponse::to_zero_copy`](src/tonic.rs), and the macro also supports infallible method signatures that return a response directly. The server example in [`examples/proto_gen_example.rs`](examples/proto_gen_example.rs) demonstrates both patterns:

```rust
#[proto_rpc(rpc_package = "sigma_rpc", rpc_server = true, rpc_client = true, ...)]
pub trait SigmaRpc {
    async fn zero_copy_ping(&self, Request<RizzPing>) -> Result<ZeroCopyResponse<GoonPong>, Status>;
    async fn infallible_zero_copy_ping(&self, Request<RizzPing>) -> ZeroCopyResponse<GoonPong>;
}

impl SigmaRpc for ServerImpl {
    async fn zero_copy_ping(&self, _: Request<RizzPing>) -> Result<ZeroCopyResponse<GoonPong>, Status> {
        Ok(GoonPong {}.to_zero_copy())
    }
}
```

This approach keeps the encoded bytes around without materializing a fresh `GoonPong` for each call. Compared to Prost-based services—where `&T` data must first be cloned into an owned `T` before encoding—`ZeroCopyResponse` removes at least one allocation and copy per RPC, which shows up as lower tail latencies for large payloads.

## Zero-copy client requests

Clients get the same treatment through [`ZeroCopyRequest`](src/tonic/req.rs). The generated stubs accept any type that implements `ProtoRequest`, so you can hand them an owned message, a `tonic::Request<T>`, or a zero-copy wrapper created from an existing borrow:

```rust
let payload = RizzPing {};
let zero_copy = (&payload).to_zero_copy();
client.rizz_ping(zero_copy).await?;
```

If you already have a configured `tonic::Request<&T>`, call `request.to_zero_copy()` to preserve metadata while still avoiding a clone. The async tests in [`examples/proto_gen_example.rs`](examples/proto_gen_example.rs) and [`tests/rpc_integration.rs`](tests/rpc_integration.rs) show how to mix borrowed and owned requests seamlessly, matching the server-side savings when round-tripping large messages.

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
- `stable` – compile everything on the stable toolchain by boxing async state. See below for trade-offs.

### Stable vs. nightly builds

The crate defaults to the nightly toolchain so it can use `impl Trait` in associated types for zero-cost futures when deriving RPC services. If you need to stay on stable Rust, enable the `stable` feature. Doing so switches the generated service code to heap-allocate and pin boxed futures, which keeps the API identical but introduces one allocation per RPC invocation and a small amount of dynamic dispatch. Disable the feature when you can use nightly to get the leanest possible generated code.

For the full API surface and macro documentation see [docs.rs/proto_rs](https://docs.rs/proto_rs).
