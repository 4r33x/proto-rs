# Rust as first-class citizen for gRPC ecosystem

`proto_rs` makes Rust the source of truth for your Protobuf and gRPC definitions, providing 4 macros that will handle all proto-related work, so you don't need to touch .proto files at all. The crate ships batteries-included attribute support, zero-copy RPC helpers, and a rich catalog of built-in wrappers so your Rust types map cleanly onto auto-generated Protobuf schemas.

## Motivation

0. I hate to do conversion after conversion for conversion
1. I love to see Rust only as first-class citizen for all my stuff
2. I hate bloat, so no protoc (shoutout to PewDiePie debloat trend)
3. I don't want to touch .proto files at all

## What can you build with `proto_rs`?

* **Pure-Rust schema definitions.** Use `#[proto_message]`, `#[proto_rpc]`, and `#[proto_dump]` to declare every message and service in idiomatic Rust while the derive machinery keeps `.proto` files in sync for external consumers.
* **Tailored encoding pipelines.** `ProtoShadow` lets you bolt custom serialization logic onto any message, opt into multiple domain "suns", and keep performance-sensitive conversions entirely under your control.
* **Zero-copy Tonic integration.** Codegen support borrowed request/response wrappers, and `ToZeroCopy*` traits so RPC handlers can run without cloning payloads.
* **Workspace-wide schema registries.** The build-time inventory collects every emitted `.proto`, making it easy to materialize or lint schemas from a single crate.

For fellow proto <-> native typeconversions enjoyers <=0.5.0 versions of this crate implement different approach

## Key capabilities

- **Message derivation** – `#[proto_message]` turns a Rust struct or enum into a fully featured Protobuf message, emitting the corresponding `.proto` definition and implementing [`ProtoExt`](src/message.rs) so the type can be encoded/decoded without extra glue code. The generated codec now reuses internal helpers to avoid redundant buffering and unnecessary copies.
- **RPC generation** – `#[proto_rpc]` projects a Rust trait into a complete Tonic service and/or client. Service traits stay idiomatic while still interoperating with non-Rust consumers through the generated `.proto` artifacts, and the macro avoids needless boxing/casting in the conversion layer.
- **Attribute-level control** – Fine-tune your schema surface with `treat_as` for ad-hoc type substitutions, `transparent` for single-field wrappers, concrete `sun` targets (including generic forms like `Sun<T>`), and optional tags/import paths. Built-in wrappers cover `ArcSwap`, `CachePadded`, atomics, and other common containers so everyday Rust types round-trip without extra glue.
- **On-demand schema dumps** – `#[proto_dump]` and `inject_proto_import!` let you register standalone definitions or imports when you need to compose more complex schemas.
- **Workspace-wide schema registry** – With the `build-schemas` feature enabled you can aggregate every proto that was emitted by your dependency tree and write it to disk via [`proto_rs::schemas::write_all`](src/lib.rs). The helper deduplicates inputs and writes canonical packages derived from the file path.
- **Opt-in `.proto` emission** – Proto files are written only when you ask for them via the `emit-proto-files` cargo feature or the `PROTO_EMIT_FILE=1` environment variable, making it easy to toggle between codegen and incremental development.


Define your messages and services using the derive macros with native rust types:

```rust
#[proto_message(proto_path = "protos/gen_proto/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FooResponse;

#[proto_message(proto_path = "protos/gen_proto/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BarSub;

#[proto_rpc(rpc_package = "sigma_rpc", rpc_server = true, rpc_client = true, proto_path = "protos/gen_complex_proto/sigma_rpc.proto")]
#[proto_imports(rizz_types = ["BarSub", "FooResponse"], goon_types = ["RizzPing", "GoonPong"] )]
pub trait SigmaRpc {
    async fn zero_copy_ping(&self, request: Request<RizzPing>) -> Result<ZeroCopyResponse<GoonPong>, Status>;
    async fn just_ping(&self, request: Request<RizzPing>) -> Result<GoonPong, Status>;
    async fn infallible_just_ping(&self, request: Request<RizzPing>) -> GoonPong;
    async fn infallible_zero_copy_ping(&self, request: Request<RizzPing>) -> ZeroCopyResponse<GoonPong>;
    async fn infallible_ping(&self, request: Request<RizzPing>) -> Response<GoonPong>;
    async fn rizz_ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;
    type RizzUniStream: Stream<Item = Result<ZeroCopyResponse<FooResponse>, Status>> + Send;
    async fn rizz_uni(&self, request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status>;
}
```

Yep, all types here are Rust types. We can then implement the server just like a normal Tonic service, and the `.proto` schema is generated for you whenever emission is enabled.


## Advanced Features

Macros support all prost types, imports, skipping with default and custom functions, custom conversions, support for native Rust enums (like `Status` below) and prost enumerations (TestEnum in this example, see more in prosto_proto).

### Attribute quick hits (there are more attributes, view examples and tests)

- Use `treat_as` to replace the encoded representation without changing your Rust field type (for example, for properly treating a type alias).
- Add `transparent` to skip tag and use this type as zerocost newtype wrapper.
- Target multiple `sun` domains, including concrete generics like `Sun<MyType<u64>>`, so a single shadow can serve several variants.
- Mix and match built-in wrappers such as `ArcSwap`, `CachePadded`, and atomic integers; the derive machinery already knows how to serialize them.

### Feature examples

```rust
// Swap a newtype's encoding without changing the Rust field type.
#[derive(Clone)]
pub struct Cents(pub i64);

#[proto_message(proto_path = "protos/payments.proto")]
pub struct Payment {
    #[proto(tag = 1, treat_as = "i64")]
    pub total: Cents,
}

// Forward the inner field directly into the schema for wrapper ergonomics.
#[proto_message(transparent, proto_path = "protos/payments.proto")]
pub struct UserId(pub uuid::Uuid);

// Target concrete generic domains from a single shadow definition.
#[proto_message(proto_path = "protos/order.proto", sun = [SunOrder<u64>, SunOrder<i64>])]
pub struct SunOrderShadow {
    #[proto(tag = 1)]
    pub quantity: u64,
}

// Infallible RPC handler returning zero-copy or boxed responses.
#[proto_rpc(rpc_package = "orders", rpc_server = true, rpc_client = true, proto_path = "protos/orders.proto")]
pub trait OrdersRpc {
    async fn confirm(&self, Request<Order>) -> Response<Box<OrderAck>>;
    async fn infallible_zero_copy(&self, Request<Order>) -> ZeroCopyResponse<Arc<OrderAck>>;
}

// Custom wrappers are understood out of the box.
#[proto_message(proto_path = "protos/runtime.proto")]
pub struct RuntimeState {
    #[proto(tag = 1)]
    pub config: arc_swap::ArcSwap<Arc<Config>>,
    #[proto(tag = 2)]
    pub cached_hits: crossbeam_utils::CachePadded<u64>,
    #[proto(tag = 3)]
    pub concurrent: std::sync::atomic::AtomicUsize,
}
```

### Struct with Advanced Attributes

```rust
#[proto_message(proto_path ="protos/showcase_proto/show.proto")]
pub struct Attr {
    #[proto(skip)]
    id_skip: Vec<i64>,
    id_vec: Vec<String>,
    id_opt: Option<String>,
    status: Status,
    status_opt: Option<Status>,
    status_vec: Vec<Status>,
    #[proto(skip = "compute_hash_for_struct")]
    hash: String,
    #[proto(import_path = "google.protobuf")]
    timestamp: Timestamp,
    #[proto(import_path = "google.protobuf")]
    timestamp_vec: Vec<Timestamp>,
    #[proto(import_path = "google.protobuf")]
    timestamp_opt: Option<Timestamp>,
    #[proto(import_path = "google.protobuf")]
    test_enum: TestEnum,
    #[proto(import_path = "google.protobuf")]
    test_enum_opt: Option<TestEnum>,
    #[proto(import_path = "google.protobuf")]
    test_enum_vec: Vec<TestEnum>,
    #[proto(into = "i64", into_fn = "datetime_to_i64", try_from_fn = "try_i64_to_datetime")]
    pub updated_at: DateTime<Utc>,
}
```

Generated proto:

```proto
message Attr {
  repeated string id_vec = 1;
  optional string id_opt = 2;
  Status status = 3;
  optional Status status_opt = 4;
  repeated Status status_vec = 5;
  google.protobuf.Timestamp timestamp = 6;
  repeated google.protobuf.Timestamp timestamp_vec = 7;
  optional google.protobuf.Timestamp timestamp_opt = 8;
  google.protobuf.TestEnum test_enum = 9;
  optional google.protobuf.TestEnum test_enum_opt = 10;
  repeated google.protobuf.TestEnum test_enum_vec = 11;
  int64 updated_at = 12;
}
```

Use `from_fn` when your conversion is infallible and `try_from_fn` when it needs to return a `Result<T, E>` where `E: Into<DecodeError>`.

### Complex Enums

```rust
#[proto_message(proto_path ="protos/showcase_proto/show.proto")]
pub enum VeryComplex {
    First,
    Second(Address),
    Third {
        id: u64,
        address: Address,
    },
    Repeated {
        id: Vec<u64>,
        address: Vec<Address>,
    },
    Option {
        id: Option<u64>,
        address: Option<Address>,
    },
    Attr {
        #[proto(skip)]
        id_skip: Vec<i64>,
        id_vec: Vec<String>,
        id_opt: Option<String>,
        status: Status,
        status_opt: Option<Status>,
        status_vec: Vec<Status>,
        #[proto(skip = "compute_hash_for_enum")]
        hash: String,
        #[proto(import_path = "google.protobuf")]
        timestamp: Timestamp,
        #[proto(import_path = "google.protobuf")]
        timestamp_vec: Vec<Timestamp>,
        #[proto(import_path = "google.protobuf")]
        timestamp_opt: Option<Timestamp>,
        #[proto(import_path = "google.protobuf")]
        test_enum: TestEnum,
        #[proto(import_path = "google.protobuf")]
        test_enum_opt: Option<TestEnum>,
        #[proto(import_path = "google.protobuf")]
        test_enum_vec: Vec<TestEnum>,
    },
}
```

Generated proto:

```proto
message VeryComplexProto {
  oneof value {
    VeryComplexProtoFirst first = 1;
    Address second = 2;
    VeryComplexProtoThird third = 3;
    VeryComplexProtoRepeated repeated = 4;
    VeryComplexProtoOption option = 5;
    VeryComplexProtoAttr attr = 6;
  }
}

message VeryComplexProtoFirst {}

message VeryComplexProtoThird {
  uint64 id = 1;
  Address address = 2;
}

message VeryComplexProtoRepeated {
  repeated uint64 id = 1;
  repeated Address address = 2;
}

message VeryComplexProtoOption {
  optional uint64 id = 1;
  optional Option address = 2;
}

message VeryComplexProtoAttr {
  repeated string id_vec = 1;
  optional string id_opt = 2;
  Status status = 3;
  optional Status status_opt = 4;
  repeated Status status_vec = 5;
  google.protobuf.Timestamp timestamp = 6;
  repeated google.protobuf.Timestamp timestamp_vec = 7;
  optional google.protobuf.Timestamp timestamp_opt = 8;
  google.protobuf.TestEnum test_enum = 9;
  optional google.protobuf.TestEnum test_enum_opt = 10;
  repeated google.protobuf.TestEnum test_enum_vec = 11;
}
```

## Inject Proto Imports

It's not mandatory to use this macro at all, macros above derive and inject imports from attributes automaticly

But in case you need it, to add custom imports to your generated .proto files use the `inject_proto_import!` macro:

```rust
inject_proto_import!("protos/test.proto", "google.protobuf.timestamp", "common");
```

This will inject the specified import statements into the target .proto file


## Custom encode/decode pipelines with `ProtoShadow`

`ProtoExt` types pair with a companion `Shadow` type that implements [`ProtoShadow`](src/traits.rs). This trait defines how a value is lowered into the bytes that will be sent over the wire and how it is rebuilt during decoding. The default derive covers most standard Rust types, but you can substitute a custom representation when you need to interoperate with an existing protocol or avoid lossy conversions. Default Shadow of type is &Self for complex types and Self for primitive

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

When you need the same wire format to serve multiple domain models, supply several `sun` targets in one go:

```rust
#[proto_message(proto_path = "protos/invoice.proto", sun = [InvoiceLine, AccountingLine])]
pub struct LineShadow {
    #[proto(tag = 1)]
    pub cents: i64,
    #[proto(tag = 2)]
    pub description: String,
}

impl ProtoShadow<InvoiceLine> for LineShadow {
    type Sun<'a> = &'a InvoiceLine;
    type OwnedSun = InvoiceLine;
    type View<'a> = Self;

    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(InvoiceLine::new(self.cents, self.description))
    }

    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        LineShadow { cents: value.total_cents(), description: value.title().to_owned() }
    }
}

impl ProtoShadow<AccountingLine> for LineShadow {
    type Sun<'a> = &'a AccountingLine;
    type OwnedSun = AccountingLine;
    type View<'a> = Self;

    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(AccountingLine::from_parts(self.cents, self.description))
    }

    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        LineShadow { cents: value.cents(), description: value.label().to_owned() }
    }
}
```

Each `sun` entry generates a full `ProtoExt` implementation so the same shadow type can round-trip either domain struct without code duplication.

## Zero-copy server responses

Service handlers produced by `#[proto_rpc]` work with [`ZeroCopyResponse`](src/tonic/resp.rs) to avoid cloning payloads. Any borrowed message (`&T`) can be turned into an owned response buffer via [`ToZeroCopyResponse::to_zero_copy`](src/tonic.rs). The macro accepts infallible method signatures (returning `Response<T>` or the zero-copy variant without `Result`) and understands common wrappers like `Arc<T>` and `Box<T>` so you can return shared or heap-allocated payloads without extra boilerplate. The server example in [`examples/proto_gen_example.rs`](examples/proto_gen_example.rs) demonstrates both patterns:

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

This approach keeps the encoded bytes around without materializing a fresh `GoonPong` for each call. Compared to Prost-based services—where `&T` data must first be cloned into an owned `T` before encoding—`ZeroCopyResponse` removes at least one allocation and copy per RPC, which shows up as lower tail latencies for large payloads. The Criterion harness ships a dedicated `bench_zero_copy_vs_clone` group that regularly clocks the zero-copy flow at 1.3×–1.7× the throughput of the Prost clone-and-encode baseline, confirming the wins for read-heavy endpoints.

## Zero-copy client requests

Clients get the same treatment through [`ZeroCopyRequest`](src/tonic/req.rs). The generated stubs accept any type that implements `ProtoRequest`, so you can hand them an owned message, a `tonic::Request<T>`, or a zero-copy wrapper created from an existing borrow:

```rust
let payload = RizzPing {};
let zero_copy = (&payload).to_zero_copy();
client.rizz_ping(zero_copy).await?;
```

If you already have a configured `tonic::Request<&T>`, call `request.to_zero_copy()` to preserve metadata while still avoiding a clone. The async tests in [`examples/proto_gen_example.rs`](examples/proto_gen_example.rs) and [`tests/rpc_integration.rs`](tests/rpc_integration.rs) show how to mix borrowed and owned requests seamlessly, matching the server-side savings when round-tripping large messages.

### Performance trade-offs vs. Prost

The runtime exposes both zero-copy and owned-code paths so you can pick the trade-off that matches your workload. Wrapping payloads in `ZeroCopyRequest`/`ZeroCopyResponse` means the encoder works with borrowed data (`SunByRef`) and never materializes owned clones before writing bytes to the socket, which is why the benchmark suite records 70% higher throughput than `prost::Message` when measuring identical service implementations (`bench_zero_copy_vs_clone`).

Owned encoding\decoding is somewhat slower or faster in somecases - see bench section



## Collecting schemas across a workspace

Enable the `build-schemas` feature for the crate that should aggregate `.proto` files and call the helper at build or runtime:

```rust
fn main() {
    // Typically gated by an env flag to avoid touching disk unnecessarily.
    proto_rs::schemas::write_all("./protos", proto_rs::schemas::RustClientCtx::disabled())
        .expect("failed to write generated protos");

    for schema in proto_rs::schemas::all() {
        println!("Registered proto: {}", schema.name);
    }
}
```

This walks the inventory of registered schemas and writes deduplicated `.proto` files with a canonical header and package name derived from the file path.

## Controlling `.proto` emission

`proto_rs` will only touch the filesystem when one of the following is set:

- Enable the `emit-proto-files` cargo feature to write generated files.
- Set `PROTO_EMIT_FILE=1` (or `true`) to turn on emission, overriding emit-proto-files behaviour
- Set `PROTO_EMIT_FILE=0` (or `false`) to turn off emission, overriding emit-proto-files behaviour

The emission logic is shared by all macros so you can switch behaviours without code changes.

## Examples and tests

Explore the `examples/` directory and the integration tests under `tests/` for end-to-end usage patterns, including schema-only builds and cross-compatibility checks.

To validate changes locally run:

```bash
cargo test
```

The test suite exercises more than 400 codec and integration scenarios to ensure the derived implementations stay compatible with Prost and Tonic.

## Benchmarks

The repository bundles a standalone Criterion harness under `benches/bench_runner`. Run the benches with:

```bash
cargo bench -p bench_runner
```

Each run appends a markdown report to `benches/bench.md`, including the `bench_zero_copy_vs_clone` comparison and encode/decode micro-benchmarks that pit `proto_rs` against Prost. Use those numbers to confirm zero-copy changes stay ahead of the Prost baseline and to track regressions on the clone-heavy paths.

## Optional features

- `std` *(default)* – pulls in the Tonic dependency tree and enables the RPC helpers.
- `tonic` *(default)* – compiles the gRPC integration layer, including the drop-in codecs, zero-copy request/response wrappers, and Tonic service/client generators.
- `build-schemas` – register generated schemas at compile time so they can be written later.
- `emit-proto-files` – eagerly write `.proto` files during compilation.
- `stable` – compile everything on the stable toolchain by boxing async state. See below for trade-offs.
- `fastnum`, `solana` – enable extra type support.

### Stable vs. nightly builds

The crate defaults to the nightly toolchain so it can use `impl Trait` in associated types for zero-cost futures when deriving RPC services. If you need to stay on stable Rust, enable the `stable` feature. Doing so switches the generated service code to heap-allocate and pin boxed futures, which keeps the API identical but introduces one allocation per RPC invocation and a small amount of dynamic dispatch. Disable the feature when you can use nightly to get the leanest possible generated code.

For the full API surface and macro documentation see [docs.rs/proto_rs](https://docs.rs/proto_rs).
