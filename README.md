# Rust as first-class citizen for gRPC ecosystem

`proto_rs` makes Rust the source of truth for your Protobuf and gRPC definitions, providing macros that handle all proto-related work so you don't need to touch `.proto` files at all. The crate ships a reverse encoder, a modular trait system for encoding/decoding, and a rich catalog of built-in wrappers so your Rust types map cleanly onto auto-generated Protobuf schemas.

## Motivation

0. I hate to do conversion after conversion for conversion
1. I love to see Rust only as first-class citizen for all my stuff
2. I hate bloat, so no protoc (shoutout to PewDiePie debloat trend)
3. I don't want to touch .proto files at all

## What can you build with `proto_rs`?

* **Pure-Rust schema definitions.** Use `#[proto_message]`, `#[proto_rpc]`, and `#[proto_dump]` to declare every message and service in idiomatic Rust while the derive machinery keeps `.proto` files in sync for external consumers.
* **Tailored encoding pipelines.** The Shadow pattern lets you bolt custom serialization logic onto any message, target multiple domain "sun" types, and keep performance-sensitive conversions entirely under your control.
* **Reverse encoder.** The single-pass reverse writer avoids precomputing lengths and keeps payload emission deterministic with reverse field order.
* **Workspace-wide schema registries.** The build-time inventory collects every emitted `.proto`, making it easy to materialize or lint schemas from a single crate.

For fellow proto <-> native type conversions enjoyers <=0.5.0 versions of this crate implement different approach

## Key capabilities

- **Message derivation** – `#[proto_message]` turns a Rust struct or enum into a fully featured Protobuf message, emitting the corresponding `.proto` definition and implementing the encoding/decoding traits so the type can be serialized without extra glue code.
- **RPC generation** – `#[proto_rpc]` projects a Rust trait into a complete Tonic service and/or client. Service traits stay idiomatic while still interoperating with non-Rust consumers through the generated `.proto` artifacts.
- **Attribute-level control** – Fine-tune your schema surface with `treat_as` for ad-hoc type substitutions, `transparent` for single-field wrappers, concrete `sun` targets (including generic forms like `Sun<T>`), and optional tags/import paths. Built-in wrappers cover `ArcSwap`, `CachePadded`, atomics, mutexes, and other common containers so everyday Rust types round-trip without extra glue.
- **On-demand schema dumps** – `#[proto_dump]` and `inject_proto_import!` let you register standalone definitions or imports when you need to compose more complex schemas.
- **Workspace-wide schema registry** – With the `build-schemas` feature enabled you can aggregate every proto that was emitted by your dependency tree and write it to disk via [`proto_rs::schemas::write_all`](src/lib.rs). The helper deduplicates inputs and writes canonical packages derived from the file path.
- **Opt-in `.proto` emission** – Proto files are written only when you ask for them via the `emit-proto-files` cargo feature or the `PROTO_EMIT_FILE=1` environment variable, making it easy to toggle between codegen and incremental development.


Define your messages and services using the derive macros with native Rust types:

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
    async fn zero_copy_ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;
    async fn just_ping(&self, request: Request<RizzPing>) -> Result<GoonPong, Status>;
    async fn infallible_just_ping(&self, request: Request<RizzPing>) -> GoonPong;
    async fn infallible_zero_copy_ping(&self, request: Request<RizzPing>) -> Response<GoonPong>;
    async fn infallible_ping(&self, request: Request<RizzPing>) -> Response<GoonPong>;
    async fn rizz_ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;
    type RizzUniStream: Stream<Item = Result<FooResponse, Status>> + Send;
    async fn rizz_uni(&self, request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status>;
}
```

Yep, all types here are Rust types. We can then implement the server just like a normal Tonic service, and the `.proto` schema is generated for you whenever emission is enabled.


## Core Trait Architecture

The encoding/decoding system is built around a modular trait hierarchy that separates concerns cleanly:

### Marker and Kind

```rust
pub trait ProtoExt {
    const KIND: ProtoKind;  // Message, Primitive, SimpleEnum, Bytes, String, Repeated
}
```

`ProtoKind` categorizes how a type is encoded on the wire and determines the wire type.

### Encoding Traits

```rust
pub trait ProtoEncode {
    type Shadow<'a>: ProtoArchive + ProtoExt + ProtoShadowEncode<'a, Self>;

    fn encode(&self, buf: &mut impl BufMut) -> Result<(), EncodeError>;
    fn encode_to_vec(&self) -> Vec<u8>;
    fn to_zero_copy(&self) -> ZeroCopy<Self>;
}

pub trait ProtoShadowEncode<'a, T: ?Sized> {
    fn from_sun(value: &'a T) -> Self;
}

pub trait ProtoArchive {
    fn is_default(&self) -> bool;
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter);
}
```

- **`ProtoEncode`**: Main encoding trait. Associates the type with its encoding shadow.
- **`ProtoShadowEncode`**: Converts a reference to the "sun" type into the encoding shadow.
- **`ProtoArchive`**: Performs the actual reverse-pass encoding into a `RevWriter`.

### Decoding Traits

```rust
pub trait ProtoDecode: Sized {
    type ShadowDecoded: ProtoDecoder + ProtoExt + ProtoShadowDecode<Self>;

    fn decode(buf: impl Buf, ctx: DecodeContext) -> Result<Self, DecodeError>;
}

pub trait ProtoShadowDecode<T> {
    fn to_sun(self) -> Result<T, DecodeError>;
}

pub trait ProtoDecoder: ProtoExt {
    fn proto_default() -> Self;
    fn clear(&mut self);
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;
}
```

- **`ProtoDecode`**: Main decoding trait. Associates the type with its decoding shadow.
- **`ProtoShadowDecode`**: Converts the decoded shadow into the final "sun" type.
- **`ProtoDecoder`**: Low-level field-by-field decoding logic.

### The Shadow Pattern

The "shadow" pattern separates wire representation from your domain types:

- **Sun Type**: Your actual Rust type (e.g., `DateTime<Utc>`, `D128`)
- **Encoding Shadow**: Borrows from the sun for encoding (lifetime-bound, zero-copy friendly)
- **Decoding Shadow**: Owned type accumulated during decode, then converted to sun

This separation allows custom wire formats without polluting your domain model.


## Advanced Features

Macros support all prost types, imports, skipping with default and custom functions, custom conversions, support for native Rust enums and prost enumerations.

### Attribute quick hits

- Use `treat_as` to replace the encoded representation without changing your Rust field type (for example, for properly treating a type alias).
- Add `transparent` to skip tag and use this type as zero-cost newtype wrapper.
- Target multiple `sun` domains, including concrete generics like `Sun<MyType<u64>>`, so a single shadow can serve several variants.
- Mix and match built-in wrappers such as `ArcSwap`, `CachePadded`, `Mutex`, `RwLock`, and atomic integers; the derive machinery already knows how to serialize them.

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
    async fn infallible_ack(&self, Request<Order>) -> Response<Arc<OrderAck>>;
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

It's not mandatory to use this macro at all, macros above derive and inject imports from attributes automatically.

But in case you need it, to add custom imports to your generated .proto files use the `inject_proto_import!` macro:

```rust
inject_proto_import!("protos/test.proto", "google.protobuf.timestamp", "common");
```

This will inject the specified import statements into the target .proto file.


## Custom encode/decode pipelines with Shadows

Types implement `ProtoEncode` and `ProtoDecode` which pair with companion shadow types. The encoding shadow implements `ProtoShadowEncode` to borrow from your type, and `ProtoArchive` to write bytes. The decoding shadow implements `ProtoDecoder` for field-by-field parsing and `ProtoShadowDecode` to convert to your final type.

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

// Encoding: borrow from D128 to create the shadow
impl<'a> ProtoShadowEncode<'a, D128> for D128Proto {
    fn from_sun(value: &'a D128) -> Self { /* extract fields */ }
}

// Decoding: convert shadow back to D128
impl ProtoShadowDecode<D128> for D128Proto {
    fn to_sun(self) -> Result<D128, DecodeError> { /* reconstruct */ }
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

impl<'a> ProtoShadowEncode<'a, InvoiceLine> for LineShadow {
    fn from_sun(value: &'a InvoiceLine) -> Self {
        LineShadow { cents: value.total_cents(), description: value.title().to_owned() }
    }
}

impl ProtoShadowDecode<InvoiceLine> for LineShadow {
    fn to_sun(self) -> Result<InvoiceLine, DecodeError> {
        Ok(InvoiceLine::new(self.cents, self.description))
    }
}

impl<'a> ProtoShadowEncode<'a, AccountingLine> for LineShadow {
    fn from_sun(value: &'a AccountingLine) -> Self {
        LineShadow { cents: value.cents(), description: value.label().to_owned() }
    }
}

impl ProtoShadowDecode<AccountingLine> for LineShadow {
    fn to_sun(self) -> Result<AccountingLine, DecodeError> {
        Ok(AccountingLine::from_parts(self.cents, self.description))
    }
}
```

Each `sun` entry generates full `ProtoEncode` and `ProtoDecode` implementations so the same shadow type can round-trip either domain struct without code duplication.

## Reverse encoding

The encoder writes in a single reverse pass, upb-style. Implementations write payload bytes first, then prefix lengths and tags as needed. This has a few implications:

- `TAG == 0` encodes a root payload with **no** field key or length prefix.
- `TAG != 0` encodes a field payload and then prefixes the field key (and length for length-delimited payloads).
- Deterministic output requires message fields (and repeated elements) to be emitted **in reverse order**.
- `RevWriter::finish_tight()` returns the backing buffer without copying; the resulting buffer is compacted with data at offset 0.

The `RevWriter` trait and `RevVec` implementation handle the reverse-writing mechanics:

```rust
pub trait RevWriter {
    fn with_capacity(cap: usize) -> Self;
    fn mark(&self) -> Self::Mark;
    fn written_since(&self, mark: Self::Mark) -> usize;
    fn put_u8(&mut self, b: u8);
    fn put_slice(&mut self, s: &[u8]);
    fn put_varint(&mut self, v: u64);
    fn finish_tight(self) -> Self::TightBuf;
}
```


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

## Built-in Wrapper Support

The crate provides out-of-the-box encoding/decoding for many common Rust types:

### Collections
- `Vec<T>`, `VecDeque<T>` - repeated fields
- `HashMap<K, V>`, `BTreeMap<K, V>` - map fields
- `HashSet<T>`, `BTreeSet<T>` - repeated fields
- Fixed-size arrays `[T; N]`

### Smart Pointers
- `Box<T>`, `Arc<T>` - transparent wrappers
- `Option<T>` - optional fields

### Concurrency Primitives
- `Mutex<T>`, `RwLock<T>` - with `parking_lot` feature for parking_lot variants
- `ArcSwap<T>` - with `arc_swap` feature
- `CachePadded<T>` - with `cache_padded` feature
- Concurrent collections via `papaya` feature

### Atomics
- `AtomicBool`, `AtomicI32`, `AtomicU32`, `AtomicI64`, `AtomicU64`, `AtomicUsize`

### Domain Types (feature-gated)
- `DateTime<Utc>`, `TimeDelta` - with `chrono` feature
- `D128`, `UD128` - with `fastnum` feature
- `Address`, `Keypair`, `Signature`, transaction errors - with `solana` feature

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

| Feature | Description |
|---------|-------------|
| `tonic` *(default)* | gRPC integration layer: codecs, zero-copy request/response wrappers, service/client generators |
| `build-schemas` | Register generated schemas at compile time for later aggregation |
| `emit-proto-files` | Eagerly write `.proto` files during compilation |
| `stable` | Compile on the stable toolchain by boxing async state (see below) |
| `chrono` | `DateTime<Utc>` and `TimeDelta` support |
| `fastnum` | `D128` / `UD128` decimal number support |
| `solana` | Solana blockchain types: `Address`, `Keypair`, `Signature`, transaction errors |
| `solana_address_hash` | Hash randomization for Solana addresses |
| `arc_swap` | `ArcSwap<T>` wrapper support |
| `cache_padded` | `CachePadded<T>` (crossbeam-utils) support |
| `papaya` | Concurrent `HashMap`/`HashSet` via papaya crate |
| `parking_lot` | `Mutex`/`RwLock` backed by parking_lot |
| `block_razor` | Block Razor MEV service client |
| `next_block` | NextBlock RPC client |
| `bloxroute` | Bloxroute service client |
| `jito` | Jito bundle service client |
| `no-recursion-limit` | Disable recursion depth checking during decode |

### Stable vs. nightly builds

The crate defaults to the nightly toolchain so it can use `impl Trait` in associated types for zero-cost futures when deriving RPC services. If you need to stay on stable Rust, enable the `stable` feature. Doing so switches the generated service code to heap-allocate and pin boxed futures, which keeps the API identical but introduces one allocation per RPC invocation and a small amount of dynamic dispatch. Disable the feature when you can use nightly to get the leanest possible generated code.

For the full API surface and macro documentation see [docs.rs/proto_rs](https://docs.rs/proto_rs).
