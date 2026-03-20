# proto_rs

Rust-first Protobuf and gRPC. Define messages, enums, and services as native Rust types — `.proto` files are generated for you. No `protoc`, no code generation step, no conversion boilerplate.

```toml
[dependencies]
proto_rs = "0.11"
```

## Why

- Rust structs and enums are the source of truth, not `.proto` files
- Zero conversion boilerplate between your domain types and the wire format
- No `protoc` binary required — everything is pure Rust
- Single-pass reverse encoder that avoids length precomputation
- Wire-compatible* (with regular, protobuf specification compatable rust types) with Prost and any standard Protobuf implementation

## proto_rs vs Prost

### Workflow

| | proto_rs | Prost |
|---|---|---|
| Source of truth | Rust structs and enums | `.proto` files |
| Rust codegen source | Derive macros at compile time | `protoc?` + `prost-build` in build.rs |
| `.proto` files | Auto-generated from Rust (opt-in) | Written by hand, required |
| Type conversions | Zero boilerplate — native types encode directly | Manual `From`/`Into` between generated and domain types |
| External tooling | None | Requires? `protoc` binary |
| Tonic integration | Built-in codec, zero-copy responses | Separate `tonic-build` step |
| Custom types | `sun` / `sun_ir` shadow system | Not supported — hand-written wrappers |

### Performance

proto_rs uses a **single-pass reverse encoder** (upb-style). Fields are written payload-first, then prefixed with tags and lengths — no two-pass measure-then-write like Prost's `encoded_len()` + `encode()`.

Encoding and decoding throughput is **on par with Prost** as far I managed to test it with totally unscientific benches. 

Per-field micro-benchmarks show both libraries trading wins depending on field type — proto_rs is faster on enums, nested messages, and collections; Prost edges ahead on raw bytes and strings. Overall throughput is comparable.

### Zero-copy

`ZeroCopy<T>` pre-encodes a message once from ref. This eliminates cloning, so you can use references in RPC services

| | Prost (clone + encode) | proto_rs (zero_copy) | Speedup |
|---|---:|---:|---|
| Complex message | 122K ops/s | 246K ops/s | **2.01x** |


## Quick start

```rust
use proto_rs::{proto_message, ProtoEncode, ProtoDecode};
use proto_rs::encoding::DecodeContext;

#[proto_message]
#[derive(Debug, PartialEq)]
struct User {
    id: u64,
    name: String,
    tags: Vec<String>,
}

let user = User { id: 42, name: "Alice".into(), tags: vec!["admin".into()] };
let bytes = User::encode_to_vec(&user);
let decoded = User::decode(bytes.as_slice(), DecodeContext::default()).unwrap();
assert_eq!(user, decoded);
```

The `#[proto_message]` macro derives encoding, decoding, and `.proto` schema generation. Tags are assigned automatically but can be overridden with `#[proto(tag = N)]`.

## Table of contents

- [proto_rs vs Prost](#proto_rs-vs-prost)
- [Messages](#messages)
- [Enums](#enums)
- [Field attributes](#field-attributes)
- [Transparent wrappers](#transparent-wrappers)
- [Generics](#generics)
- [Custom type conversions (sun)](#custom-type-conversions-sun)
- [Zero-copy IR encoding (sun_ir)](#zero-copy-ir-encoding-sun_ir)
- [Getters](#getters)
- [Validation](#validation)
- [RPC services](#rpc-services)
- [Zero-copy encoding](#zero-copy-encoding)
- [Built-in type support](#built-in-type-support)
- [Wrapper types](#wrapper-types)
- [Third-party integrations](#third-party-integrations)
- [Schema registry and emission](#schema-registry-and-emission)
- [Feature flags](#feature-flags)
- [Stable toolchain](#stable-toolchain)
- [Benchmarks](#benchmarks)

## Messages

Structs become Protobuf messages. Fields map to proto fields with auto-assigned tags.

```rust
#[proto_message(proto_path = "protos/orders.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct Order {
    pub id: u64,
    pub item: String,
    pub quantity: u32,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}
```

Generated `.proto`:

```proto
message Order {
  uint64 id = 1;
  string item = 2;
  uint32 quantity = 3;
  optional string notes = 4;
  repeated string tags = 5;
  map<string, string> metadata = 6;
}
```

Nested messages work naturally:

```rust
#[proto_message]
#[derive(Clone, Debug, PartialEq)]
pub struct LineItem {
    pub product_id: u64,
    pub amount: u32,
}

#[proto_message]
#[derive(Clone, Debug, PartialEq)]
pub struct Invoice {
    pub id: u64,
    pub items: Vec<LineItem>,
    pub total: Option<LineItem>,
}
```

## Enums

Rust enums map to Protobuf `oneof`. Unit variants, tuple variants, and struct variants are all supported.

```rust
#[proto_message(proto_path = "protos/events.proto")]
pub enum Event {
    Ping,
    Message(String),
    Transfer {
        from: u64,
        to: u64,
        amount: u64,
    },
    Batch {
        ids: Vec<u64>,
        labels: Vec<String>,
    },
    Optional {
        id: Option<u64>,
        note: Option<String>,
    },
}
```

Generated `.proto`:

```proto
message Event {
  oneof value {
    EventPing ping = 1;
    string message = 2;
    EventTransfer transfer = 3;
    EventBatch batch = 4;
    EventOptional optional = 5;
  }
}

message EventPing {}

message EventTransfer {
  uint64 from = 1;
  uint64 to = 2;
  uint64 amount = 3;
}

message EventBatch {
  repeated uint64 ids = 1;
  repeated string labels = 2;
}

message EventOptional {
  optional uint64 id = 1;
  optional string note = 2;
}
```

## Field attributes

### `#[proto(tag = N)]`

Override auto-assigned field tag:

```rust
#[proto_message]
pub struct Config {
    #[proto(tag = 5)]
    pub name: String,
    #[proto(tag = 10)]
    pub value: u64,
}
```

### `#[proto(skip)]` and `#[proto(skip = "fn_path")]`

Skip a field during encoding. With a function, the field is recomputed on decode:

```rust
#[proto_message]
pub struct Document {
    pub content: String,
    #[proto(skip)]
    pub cached: Vec<u8>,
    #[proto(skip = "recompute_checksum")]
    pub checksum: u32,
}

fn recompute_checksum(doc: &Document) -> u32 {
    doc.content.len() as u32
}
```

### `#[proto(treat_as = "Type")]`

Encode a field using a different type's wire format. Useful for type aliases:

```rust
pub type ComplexMap = std::collections::BTreeMap<u64, u64>;

#[proto_message]
pub struct State {
    #[proto(treat_as = "std::collections::BTreeMap<u64, u64>")]
    pub index: ComplexMap,
}
```

### `#[proto(into)]`, `#[proto(into_fn)]`, `#[proto(from_fn)]`, `#[proto(try_from_fn)]`

Custom field-level type conversions:

```rust
use chrono::{DateTime, Utc};

fn datetime_to_i64(dt: &DateTime<Utc>) -> i64 { dt.timestamp() }
fn i64_to_datetime(ts: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(ts, 0).unwrap()
}

#[proto_message]
pub struct Record {
    #[proto(into = "i64", into_fn = "datetime_to_i64", from_fn = "i64_to_datetime")]
    pub updated_at: DateTime<Utc>,
}
```

Use `try_from_fn` when the conversion can fail (the error type must implement `Into<DecodeError>`).

### `#[proto(import_path = "package")]`

Optional hint for live `.proto` emission — tells the emitter which package to import for an external type. The build-schema system resolves all imports automatically, so this is only needed when using `emit-proto-files` or `PROTO_EMIT_FILE=1`:

```rust
#[proto_message]
pub struct WithTimestamp {
    // Optional: only needed for live .proto emission
    #[proto(import_path = "google.protobuf")]
    pub timestamp: Timestamp,
}
```

## Transparent wrappers

Single-field newtypes can be encoded without additional message framing:

```rust
#[proto_message(transparent)]
#[derive(Debug, PartialEq)]
pub struct UserId(u64);

#[proto_message(transparent)]
#[derive(Debug, PartialEq)]
pub struct Token {
    pub inner: String,
}
```

The wrapper encodes/decodes as the inner type directly — no extra tag overhead on the wire.

## Generics

Generic structs work out of the box:

```rust
#[proto_message]
#[derive(Debug, PartialEq)]
struct Pair<K, V> {
    key: K,
    value: V,
}

#[proto_message]
#[derive(Debug, PartialEq)]
struct Cache<K, V, const CAP: usize> {
    items: VecDeque<Pair<K, V>>,
}

// Encodes/decodes like any other message
let pair = Pair { key: 1u64, value: "hello".to_string() };
let bytes = <Pair<u64, String>>::encode_to_vec(&pair);
```

For `.proto` generation with concrete type substitution:

```rust
#[proto_message(
    proto_path = "protos/order.proto",
    generic_types = [T = [u64, i64]]
)]
pub struct OrderLine<T> {
    pub quantity: T,
    pub label: String,
}
```

## Custom type conversions (sun)

The `sun` attribute maps native Rust types to a proto shadow struct. The shadow handles encoding/decoding while your domain type stays clean.

```rust
use proto_rs::{proto_message, ProtoShadowEncode, ProtoShadowDecode, DecodeError};

// Your domain type — not proto-aware
struct Account {
    balance: std::sync::Mutex<u64>,
    name: String,
}

// The proto shadow — handles wire format
#[proto_message(sun = Account)]
struct AccountProto {
    #[proto(tag = 1)]
    balance: u64,
    #[proto(tag = 2)]
    name: String,
}

impl ProtoShadowDecode<Account> for AccountProto {
    fn to_sun(self) -> Result<Account, DecodeError> {
        Ok(Account {
            balance: std::sync::Mutex::new(self.balance),
            name: self.name,
        })
    }
}

impl<'a> ProtoShadowEncode<'a, Account> for AccountProto {
    fn from_sun(value: &'a Account) -> Self {
        AccountProto {
            balance: *value.balance.lock().unwrap(),
            name: value.name.clone(),
        }
    }
}
```

A single shadow can serve multiple domain types:

```rust
#[proto_message(sun = [InvoiceLine, AccountingLine])]
struct LineShadow {
    #[proto(tag = 1)]
    cents: i64,
    #[proto(tag = 2)]
    description: String,
}

// Implement ProtoShadowEncode + ProtoShadowDecode for each sun target
```

## Zero-copy IR encoding (sun_ir)

For types with expensive-to-clone fields, `sun_ir` provides a reference-based intermediate representation that avoids cloning during encoding:

```rust
use proto_rs::{proto_message, ProtoShadowEncode, ProtoShadowDecode, DecodeIrBuilder, DecodeError};
use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};

// IR struct holds references — no cloning on encode
pub struct InstructionIr<'a> {
    program_id: &'a Address,
    accounts: &'a Vec<AccountMeta>,
    data: &'a Vec<u8>,
}

#[proto_message(
    proto_path = "protos/solana.proto",
    sun = [Instruction],
    sun_ir = InstructionIr<'a>
)]
pub struct InstructionProto {
    #[proto(tag = 1)]
    pub program_id: Address,
    #[proto(tag = 2)]
    pub accounts: Vec<AccountMeta>,
    #[proto(tag = 3)]
    pub data: Vec<u8>,
}

// Encode path: borrows everything, zero allocations
impl<'a> ProtoShadowEncode<'a, Instruction> for InstructionIr<'a> {
    fn from_sun(value: &'a Instruction) -> Self {
        Self {
            program_id: &value.program_id,
            accounts: &value.accounts,
            data: &value.data,
        }
    }
}

// Decode path: moves owned data
impl ProtoShadowDecode<Instruction> for InstructionProto {
    fn to_sun(self) -> Result<Instruction, DecodeError> {
        Ok(Instruction {
            program_id: self.program_id,
            accounts: self.accounts,
            data: self.data,
        })
    }
}


impl DecodeIrBuilder<InstructionProto> for Instruction {
    fn build_ir(&self) -> Result<InstructionProto, DecodeError> {
        Ok(InstructionProto {
            program_id: self.program_id,
            accounts: self.accounts.clone(),
            data: self.data.clone(),
        })
    }
}
```

## Getters

When the IR struct's fields don't map 1:1 to the proto struct, use `getter` to specify how to access values from the IR:

```rust
struct TaskCtx { flags: u32, name: String }
struct TaskRef<'a> { user_id: u64, ctx: &'a TaskCtx }

#[proto_message(sun = [Task], sun_ir = TaskRef<'a>)]
struct TaskProto {
    user_id: u64,
    #[proto(getter = "*$.ctx.flags")]
    flags: u32,
    #[proto(getter = "&*$.ctx.name")]
    name: String,
}
```

The `$` refers to the IR struct instance. Same-name, same-type fields are resolved automatically without a getter.

## Validation

Validate fields or entire messages on decode:

```rust
fn validate_port(port: &u16) -> Result<(), DecodeError> {
    if *port == 0 { return Err(DecodeError::new("port cannot be zero")); }
    Ok(())
}

fn validate_config(cfg: &ServerConfig) -> Result<(), DecodeError> {
    if cfg.max_connections > 100_000 {
        return Err(DecodeError::new("too many connections"));
    }
    Ok(())
}

#[proto_message]
#[proto(validator = validate_config)]
pub struct ServerConfig {
    #[proto(validator = validate_port)]
    pub port: u16,
    pub max_connections: u32,
}
```

Field validators run after each field is decoded. Message validators run after all fields are decoded. Both return `Result<(), DecodeError>`.

With the `tonic` feature, `validator_with_ext` gives access to `tonic::Extensions` for request-scoped validation:

```rust
#[proto_message]
#[proto(validator_with_ext = validate_with_auth)]
pub struct SecureRequest {
    pub payload: String,
}

fn validate_with_auth(req: &SecureRequest, ext: &tonic::Extensions) -> Result<(), DecodeError> {
    // Access request metadata for auth checks
    Ok(())
}
```

## RPC services

Define gRPC services as Rust traits. The macro generates Tonic server and client implementations:

The macro parses your trait methods and generates both the server trait and client struct. Return types are flexible — you can use or omit `Result` and `Response` wrappers depending on what makes sense semantically:

```rust
use proto_rs::{proto_rpc, proto_message, ZeroCopy};
use tonic::{Request, Response, Status};

#[proto_message]
pub struct Ping { pub id: u64 }

#[proto_message]
pub struct Pong { pub id: u64, pub message: String }

#[proto_rpc(
    rpc_package = "echo",
    rpc_server = true,
    rpc_client = true,
    proto_path = "protos/echo.proto"
)]
pub trait EchoService {
    // Standard: Result<Response<T>, Status>
    async fn echo(&self, request: Request<Ping>) -> Result<Response<Pong>, Status>;

    // Infallible: drop Result when the handler never fails
    async fn echo_infallible(&self, request: Request<Ping>) -> Response<Pong>;

    // Bare value: drop both Result and Response for maximum brevity
    async fn echo_bare(&self, request: Request<Ping>) -> Pong;

    // Zero-copy: pre-encoded bytes, avoids re-encoding on send
    async fn echo_fast(&self, request: Request<Ping>) -> Result<Response<ZeroCopy<Pong>>, Status>;

    // Smart pointer responses work too
    async fn echo_boxed(&self, request: Request<Ping>) -> Result<Response<Box<Pong>>, Status>;
    async fn echo_arced(&self, request: Request<Ping>) -> Response<Arc<Pong>>;

    // Server streaming
    type PingStream: Stream<Item = Result<Pong, Status>> + Send;
    async fn ping_stream(&self, request: Request<Ping>) -> Result<Response<Self::PingStream>, Status>;
}
```

The macro unwraps `Result`, `Response`, `Box`, `Arc`, and `ZeroCopy` layers automatically to determine the proto message type for the generated `.proto` definition — you get clean trait signatures without affecting the wire format.

### Server implementation

```rust
struct MyService;

impl EchoService for MyService {
    async fn echo(&self, request: Request<Ping>) -> Result<Response<Pong>, Status> {
        let ping = request.into_inner();
        Ok(Response::new(Pong { id: ping.id, message: "pong".into() }))
    }

    async fn echo_fast(&self, request: Request<Ping>) -> Result<Response<ZeroCopy<Pong>>, Status> {
        let pong = Pong { id: request.into_inner().id, message: "fast".into() };
        Ok(Response::new(pong.to_zero_copy()))
    }

    // ...
}

// Start server
Server::builder()
    .add_service(echo_service_server::EchoServiceServer::new(MyService))
    .serve(addr)
    .await?;
```

### Generated client

The generated client methods accept any type that implements `ProtoRequest<T>` — not just `Request<T>`. This means you can pass:

- **Bare values:** `client.echo(Ping { id: 1 })` — auto-wrapped in `Request`
- **Wrapped requests:** `client.echo(Request::new(Ping { id: 1 }))` — passed through
- **Zero-copy:** `client.echo(ping.to_zero_copy())` — sent as pre-encoded bytes

```rust
let mut client = echo_service_client::EchoServiceClient::connect("http://127.0.0.1:50051").await?;

// All three are equivalent — pass whatever is convenient:
let r1 = client.echo(Ping { id: 1 }).await?;
let r2 = client.echo(Request::new(Ping { id: 1 })).await?;
let r3 = client.echo(Ping { id: 1 }.to_zero_copy()).await?;
```

The generated method signature is:

```rust
pub async fn echo<R>(&mut self, request: R) -> Result<Response<Pong>, Status>
where
    R: ProtoRequest<Ping>,
```

This generic bound is what makes all three call styles work — `ProtoRequest<T>` is implemented for `T`, `Request<T>`, `ZeroCopy<T>`, and `Request<ZeroCopy<T>>`.

### RPC imports

Optional import hints for live `.proto` emission. The build-schema system resolves all imports automatically — `#[proto_imports]` is only needed when using `emit-proto-files` or `PROTO_EMIT_FILE=1`:

```rust
#[proto_rpc(rpc_server = true, rpc_client = true, proto_path = "protos/svc.proto")]
// Optional: only needed for live .proto emission
#[proto_imports(common_types = ["UserId", "Status"], orders = ["Order"])]
pub trait OrderService {
    async fn get_order(&self, request: Request<UserId>) -> Result<Response<Order>, Status>;
}
```

### RPC client interceptors

`rpc_client_ctx` adds a generic `Ctx` parameter to the generated client, enabling per-request middleware (auth tokens, tracing headers, rate limiting, etc.).

Define an interceptor trait:

```rust
pub trait AuthInterceptor: Send + Sync + 'static + Sized {
    type Payload;
    fn intercept<T>(payload: Self::Payload, req: &mut Request<T>) -> Result<(), Status>;
}
```

Wire it to the service:

```rust
#[proto_rpc(rpc_server = true, rpc_client = true, rpc_client_ctx = "AuthInterceptor")]
pub trait SecureService {
    async fn protected(&self, request: Request<Ping>) -> Result<Response<Pong>, Status>;
}
```

The generated client becomes `SecureServiceClient<T, Ctx>` where `Ctx: AuthInterceptor`. Each method gains an extra first parameter for the interceptor payload, with the bound `I: Into<Ctx::Payload>`:

```rust
// Generated signature:
pub async fn protected<R, I>(
    &mut self,
    ctx: I,          // interceptor payload — first argument
    request: R,
) -> Result<Response<Pong>, Status>
where
    R: ProtoRequest<Ping>,
    I: Into<Ctx::Payload>,
    Ctx: AuthInterceptor,
```

This means you can pass any type that converts into the payload — not just the payload type itself:

```rust
struct MyAuth;
impl AuthInterceptor for MyAuth {
    type Payload = String;  // e.g. a bearer token
    fn intercept<T>(token: String, req: &mut Request<T>) -> Result<(), Status> {
        req.metadata_mut().insert("authorization", token.parse().unwrap());
        Ok(())
    }
}

let mut client: SecureServiceClient<_, MyAuth> =
    SecureServiceClient::connect("http://127.0.0.1:50051").await?;

// Pass a String directly:
client.protected(format!("Bearer {token}"), Ping { id: 1 }).await?;

// Or anything that implements Into<String>:
client.protected("Bearer abc123".to_string(), Ping { id: 1 }).await?;
```

Multiple services can share the same interceptor trait with different concrete implementations

## Zero-copy encoding

Pre-encode a message and reuse the bytes:

```rust
use proto_rs::{ProtoEncode, ZeroCopy};

let msg = Pong { id: 1, message: "hello".into() };
let zc: ZeroCopy<Pong> = ProtoEncode::to_zero_copy(&msg);

// Access raw bytes without re-encoding
let bytes: &[u8] = zc.as_bytes();

// Use in Tonic responses — sent without re-encoding
Ok(Response::new(zc))
```

## Built-in type support

### Primitives

`bool`, `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `f32`, `f64`, `usize`, `isize`, `String`, `Vec<u8>`, `bytes::Bytes`

Narrow types (`u8`, `u16`, `i8`, `i16`) are widened on the wire to `uint32`/`int32` with overflow validation on decode.

### Atomics

All `std::sync::atomic` types: `AtomicBool`, `AtomicU8`, `AtomicU16`, `AtomicU32`, `AtomicU64`, `AtomicUsize`, `AtomicI8`, `AtomicI16`, `AtomicI32`, `AtomicI64`, `AtomicIsize`

Atomic types encode and decode using `Ordering::Relaxed`.

### NonZero types

All `core::num::NonZero*` types: `NonZeroU8`, `NonZeroU16`, `NonZeroU32`, `NonZeroU64`, `NonZeroUsize`, `NonZeroI8`, `NonZeroI16`, `NonZeroI32`, `NonZeroI64`, `NonZeroIsize`

Default value is `MAX` (not zero). Decoding zero returns an error.

### Collections

`Vec<T>`, `VecDeque<T>`, `[T; N]`, `HashMap<K, V>`, `BTreeMap<K, V>`, `HashSet<T>`, `BTreeSet<T>`

### Smart pointers

`Box<T>`, `Arc<T>`, `Option<T>`

### Unit type

`()` maps to `google.protobuf.Empty`.

## Wrapper types

Feature-gated wrapper types are encoded transparently:

| Type | Feature | Description |
|------|---------|-------------|
| `ArcSwap<T>` | `arc_swap` | Lock-free atomic pointer |
| `ArcSwapOption<T>` | `arc_swap` | Optional atomic pointer |
| `CachePadded<T>` | `cache_padded` | Cache-line aligned value |
| `parking_lot::Mutex<T>` | `parking_lot` | Fast mutex |
| `parking_lot::RwLock<T>` | `parking_lot` | Fast read-write lock |
| `std::sync::Mutex<T>` | *(always)* | Standard mutex |
| `papaya::HashMap<K,V>` | `papaya` | Lock-free concurrent map |
| `papaya::HashSet<T>` | `papaya` | Lock-free concurrent set |

```rust
use arc_swap::ArcSwap;
use std::sync::Arc;

#[proto_message]
pub struct RuntimeState {
    pub config: ArcSwap<Config>,
    pub counter: std::sync::atomic::AtomicU64,
    pub cache: parking_lot::Mutex<Vec<u8>>,
}
```

## Third-party integrations

### Chrono (`chrono` feature)

`DateTime<Utc>` and `TimeDelta` encode as `(i64 secs, u32 nanos)`:

```rust
#[proto_message]
pub struct Event {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration: chrono::TimeDelta,
}
```

### Fastnum (`fastnum` feature)

`D128`, `D64`, and `UD128` encode as split integer components:

```rust
#[proto_message]
pub struct Price {
    pub amount: fastnum::D128,   // signed 128-bit decimal
    pub total: fastnum::UD128,   // unsigned 128-bit decimal
}
```

### Solana (`solana` feature)

Native support for Solana SDK types:

| Type | Proto representation |
|------|---------------------|
| `Address` | `bytes` (32 bytes) |
| `Signature` | `bytes` (64 bytes) |
| `Hash` | `bytes` (32 bytes) |
| `Keypair` | `bytes` |
| `Instruction` | message with `Address` program_id, repeated `AccountMeta`, `bytes` data |
| `AccountMeta` | message with `Address` pubkey, `bool` is_signer, `bool` is_writable |
| `InstructionError` | `oneof` with all error variants |
| `TransactionError` | `oneof` with all error variants |

`Instruction` uses `sun_ir` for zero-copy encoding — account lists and data are borrowed, not cloned.

### Teloxide (`teloxide` feature)

`teloxide_core::types::UserId` is supported as a primitive.

### Hashers (`ahash` feature)

`ahash::RandomState` and `std::hash::RandomState` are supported for `HashMap`/`HashSet` construction.

## Schema registry and emission

proto\_rs includes a build system that collects all proto schemas at compile time using the `inventory` crate. Every `#[proto_message]` and `#[proto_rpc]` macro invocation automatically registers its schema.  `write_all()` gathers all registered schemas across your entire workspace (and from whole dependency tree!) and generates two outputs:

1. **`.proto` files** — valid proto3 definitions with resolved imports and package structure
2. **Rust client module** — optional generated Rust code with `#[proto_message]` / `#[proto_rpc]` attributes, ready for use by downstream consumers who depend on proto\_rs but don't have access to your original types

### Emitting `.proto` files

Proto files are written only when explicitly enabled:

- **Cargo feature:** `emit-proto-files`
- **Environment variable:** `PROTO_EMIT_FILE=1` (overrides the feature flag)
- **Disable override:** `PROTO_EMIT_FILE=0`

### Build-time schema collection

With the `build-schemas` feature, collect all proto schemas across your workspace and write them to disk:

```rust
use proto_rs::schemas::{write_all, RustClientCtx};

fn main() {
    let count = write_all("./protos", &RustClientCtx::disabled())
        .expect("failed to write generated protos");
    println!("Generated {count} proto files");
}
```

### Rust client generation

`RustClientCtx` controls whether and how a Rust client module is generated alongside `.proto` files. The generated module mirrors your proto package hierarchy as nested Rust `pub mod` blocks, with each type annotated by `#[proto_message]` or `#[proto_rpc]`.

```rust
use proto_rs::schemas::{write_all, RustClientCtx};

fn main() {
    let ctx = RustClientCtx::enabled("src/generated.rs");
    write_all("./protos", &ctx).expect("failed");
}
```

### Import substitution (`with_imports`)

When you provide imports, the build system **auto-substitutes** matching types in the generated client. If a generated type name matches an imported type, it is replaced with the import — the struct definition is omitted and all references use the imported path instead.

This is useful when consumers already have types (like `fastnum::UD128`, `solana_address::Address`, or `chrono::DateTime`) and want the generated client to reference those directly rather than re-generating wrapper structs.

```rust
let ctx = RustClientCtx::enabled("src/client.rs")
    .with_imports(&[
        "fastnum::UD128",
        "solana_address::Address",
        "chrono::DateTime",
        "chrono::Utc",
    ]);
```

**Before** (without import): the build system generates a `pub struct UD128 { ... }` in the client.
**After** (with import): the client emits `use fastnum::UD128;` and references `UD128` directly — no struct generated.

Aliased imports are also supported:

```rust
.with_imports(&["my_crate::MyType as Alias"])
```

### Module type attributes (`type_attribute`)

Apply `#[derive(...)]` or other attributes to all types within a module:

```rust
let ctx = RustClientCtx::enabled("src/client.rs")
    .type_attribute("goon_types".into(), "#[derive(Clone, Debug)]".into())
    .type_attribute("goon_types".into(), "#[derive(Clone, PartialEq)]".into());
```

Duplicate derive entries are automatically merged — the above produces a single `#[derive(Clone, Debug, PartialEq)]` on every type in `goon_types`.

### Per-type and per-field attributes (`add_client_attrs`, `remove_type_attribute`)

Add or remove attributes on individual types, fields, or RPC methods:

```rust
use proto_rs::schemas::{AttrLevel, ClientAttrTarget, ProtoIdentifiable, UserAttr};

let ctx = RustClientCtx::enabled("src/client.rs")
    // Add an attribute to a specific type
    .add_client_attrs(
        ClientAttrTarget::Ident(BuildRequest::PROTO_IDENT),
        UserAttr { level: AttrLevel::Top, attr: "#[allow(dead_code)]".into() },
    )
    // Add an attribute to a specific field
    .add_client_attrs(
        ClientAttrTarget::Ident(BuildResponse::PROTO_IDENT),
        UserAttr {
            level: AttrLevel::Field {
                field_name: "status".into(),
                id: ServiceStatus::PROTO_IDENT,
                variant: None,
            },
            attr: "#[allow(dead_code)]".into(),
        },
    )
    // Add a module-level attribute
    .add_client_attrs(
        ClientAttrTarget::Module("extra_types"),
        UserAttr { level: AttrLevel::Top, attr: "#[allow(clippy::upper_case_acronyms)]".into() },
    )
    // Remove a specific derive from a type (e.g., remove Clone from BuildRequest)
    .remove_type_attribute(
        ClientAttrTarget::Ident(BuildRequest::PROTO_IDENT),
        UserAttr { level: AttrLevel::Top, attr: "#[derive(Clone)]".into() },
    );
```

### Type replacement (`replace_type`)

Replace types in the generated client — useful for substituting proto types with domain-specific types in struct fields or RPC method signatures:

```rust
use proto_rs::schemas::{MethodReplace, TypeReplace};

let ctx = RustClientCtx::enabled("src/client.rs")
    .replace_type(&[
        // Replace a struct field's type
        TypeReplace::Type {
            id: BuildResponse::PROTO_IDENT,
            variant: None,
            field: "status".into(),
            type_name: "::core::atomic::AtomicU32".into(),
        },
        // Replace an RPC method's argument type
        TypeReplace::Trait {
            id: sigma_ident,
            method: "OwnerLookup".into(),
            kind: MethodReplace::Argument("::core::primitive::u64".into()),
            type_name: "::core::atomic::AtomicU64".into(),
        },
        // Replace an RPC method's return type
        TypeReplace::Trait {
            id: sigma_ident,
            method: "Build".into(),
            kind: MethodReplace::Return("::core::primitive::u32".into()),
            type_name: "::core::atomic::AtomicU32".into(),
        },
    ]);
```

### Custom statements (`with_statements`)

Inject arbitrary Rust statements into a specific module:

```rust
let ctx = RustClientCtx::enabled("src/client.rs")
    .with_statements(&[("extra_types", "const MY_CONST: usize = 1337")]);
```

### Split module output (`split_module`)

For large codebases, split specific modules into separate files instead of bundling everything into a single output file:

```rust
let ctx = RustClientCtx::enabled("src/client.rs")
    .split_module("atomic_types", "src/client_atomic_types.rs");
```

The `atomic_types` module is written to `src/client_atomic_types.rs` and excluded from the main `src/client.rs`. All other modules remain in the main output.

### Type handling in generated output

The build system automatically handles special Rust types when generating client code:

| Rust source type | Proto output | Rust client output |
|---|---|---|
| `AtomicBool` | `bool` | `bool` |
| `AtomicU8`, `AtomicU16`, `AtomicU32` | `uint32` | `u8`, `u16`, `u32` |
| `AtomicU64`, `AtomicUsize` | `uint64` | `u64` |
| `AtomicI8`, `AtomicI16`, `AtomicI32` | `int32` | `i8`, `i16`, `i32` |
| `AtomicI64`, `AtomicIsize` | `int64` | `i64` |
| `NonZeroU8`, `NonZeroU16`, `NonZeroU32` | `uint32` | `::core::num::NonZeroU8`, etc. |
| `NonZeroU64`, `NonZeroUsize` | `uint64` | `::core::num::NonZeroU64` |
| `NonZeroI8`, `NonZeroI16`, `NonZeroI32` | `int32` | `::core::num::NonZeroI8`, etc. |
| `NonZeroI64`, `NonZeroIsize` | `int64` | `::core::num::NonZeroI64` |
| `Mutex<T>`, `Arc<T>`, `Box<T>` | inner type | inner type (unwrapped) |
| `Vec<T>`, `VecDeque<T>` | `repeated T` | `Vec<T>` |
| `HashMap<K,V>`, `BTreeMap<K,V>` | `map<K,V>` | `HashMap<K,V>` |
| `Option<T>` | `optional T` | `Option<T>` |

Atomic types are unwrapped to their inner primitives (they are a runtime concern). NonZero types preserve their NonZero semantics in the Rust client since the non-zero constraint is meaningful for downstream consumers.

### Macro import tracking

The build system tracks which macros each module actually uses and emits only the necessary imports. Modules containing only structs/enums import `proto_message`; modules with only services import `proto_rpc`; modules with both import both. No `#[allow(unused_imports)]` suppression is needed.

### Custom proto definitions

`#[proto_dump]` emits standalone proto definitions. `inject_proto_import!` adds import hints to generated `.proto` files. Both are optional — the build-schema system resolves all imports automatically. These are only needed when using live `.proto` emission (`emit-proto-files` or `PROTO_EMIT_FILE=1`):

```rust
inject_proto_import!("protos/service.proto", "google.protobuf.timestamp", "common");
```

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `tonic` | yes | Tonic gRPC integration: codecs, service/client generation |
| `stable` | no | Compile on stable Rust (boxes async futures) |
| `build-schemas` | no | Compile-time schema registry via `inventory` |
| `emit-proto-files` | no | Write `.proto` files during compilation |
| `chrono` | no | `DateTime<Utc>`, `TimeDelta` support |
| `fastnum` | no | `D128`, `D64`, `UD128` decimal support |
| `solana` | no | Solana SDK types (Address, Instruction, errors, etc.) |
| `solana_address_hash` | no | Solana address hasher support |
| `teloxide` | no | Telegram bot types |
| `ahash` | no | AHash hasher for collections |
| `arc_swap` | no | `ArcSwap<T>` wrapper |
| `cache_padded` | no | `CachePadded<T>` wrapper |
| `parking_lot` | no | `parking_lot::Mutex<T>`, `RwLock<T>` |
| `papaya` | no | Lock-free concurrent `HashMap`/`HashSet` |
| `block_razor` | no | Block Razor RPC integration |
| `jito` | no | Jito RPC integration |
| `bloxroute` | no | Bloxroute RPC integration |
| `next_block` | no | NextBlock RPC integration |
| `no-recursion-limit` | no | Disable decode recursion depth checking |

## Stable toolchain

The crate defaults to nightly for `impl Trait` in associated types, giving zero-cost futures in generated RPC services. Enable the `stable` feature to compile on stable Rust — this boxes async futures (one allocation per RPC call) but keeps the API identical:

```toml
[dependencies]
proto_rs = { version = "0.11", features = ["stable"] }
```

## Reverse encoding

The encoder writes in a single reverse pass (upb-style). Fields are emitted payload-first, then prefixed with lengths and tags. This avoids precomputing message sizes and produces deterministic output. The `RevWriter` trait powers this:

- `TAG == 0` encodes a root payload with no field key or length prefix
- `TAG != 0` prefixes the field key (and length for length-delimited payloads)
- Fields and repeated elements are emitted in reverse order
- `RevWriter::finish_tight()` returns the buffer without slack

## Benchmarks

```bash
cargo bench -p bench_runner
```

The Criterion harness under `benches/bench_runner` includes zero-copy vs clone comparisons and encode/decode micro-benchmarks against Prost.

## Testing

```bash
cargo test              # default features
cargo test --all-features  # all 500+ tests
```

The test suite covers codec roundtrips, cross-library compatibility with Prost, RPC integration, validation, and every supported type.

## License

MIT OR Apache-2.0
