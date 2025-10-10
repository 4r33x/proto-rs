# Rust as first-class citizen for gRPC ecosystem

This crate provides 4 macros that will handle all proto-related work, so you don't need to touch .proto files at all.

## Motivation

0. I hate to do conversion after conversion for conversion
1. I love to see Rust only as first-class citizen for all my stuff
2. I hate bloat, so no protoc (shoutout to PewDiePie debloat trend)
3. I don't want to touch .proto files at all

## Usage

The `#[proto_rpc]` macro will convert your Rust native trait to tonic and optionally emit .proto file:

```rust
#[proto_rpc(rpc_package = "sigma_rpc", rpc_server = true, rpc_client = true, proto_path = "protos/gen_complex_proto/sigma_rpc.proto")]
#[proto_imports(rizz_types = ["BarSub", "FooResponse"], goon_types = ["RizzPing", "GoonPong"] )]
pub trait SigmaRpc {
    type RizzUniStream: Stream<Item = Result<FooResponse, Status>> + Send;
    async fn rizz_ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;

    async fn rizz_uni(&self, request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status>;
}
```

Yep, all types here are just Rust types. We can then implement the server like this:

```rust
#[tonic::async_trait]
impl SigmaRpc for S {
    type RizzUniStream = Pin<Box<dyn Stream<Item = Result<FooResponse, Status>> + Send>>;
    async fn rizz_ping(&self, _req: Request<RizzPing>) -> Result<Response<GoonPong>, Status> {
        Ok(Response::new(GoonPong {}))
    }
    async fn rizz_uni(&self, _request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(128);

        tokio::spawn(async move {
            for _ in 0..5 {
                if tx.send(Ok(FooResponse {})).await.is_err() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let stream = ReceiverStream::new(rx);
        let boxed_stream: Self::RizzUniStream = Box::pin(stream);

        Ok(Response::new(boxed_stream))
    }
}
```

This is possible because of this trait, that handles all conversions automagically:

```rust
pub trait HasProto {
    type Proto: prost::Message;
    fn to_proto(&self) -> Self::Proto;
    fn from_proto(proto: Self::Proto) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;
}
```

We can derive it (or manually implement) for most types with `#[proto_message]` macro:

```rust
#[proto_message(proto_path ="protos/gen_proto/goon_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct RizzPing;
```

But that's not all — `#[proto_message]` and `#[proto_rpc]` will also create .proto definitions for non-Rust clients.

## Build All .proto Files from Dependencies at once

**Pure Rust Black Magic**

This crate provides a powerful feature to collect and build .proto files from ALL dependencies that use `proto_rs` in a single place. This is incredibly useful for building a centralized proto schema from a multi-crate workspace.

### Usage

In your `build.rs` or `main.rs` (or any crate that has other proto_rs dependent crates):

```rust
use proto_rs::schemas::ProtoSchema;

fn main() {
    proto_rs::schemas::write_all("build_protos").expect("Failed to write proto files");
    
    for schema in inventory::iter::<ProtoSchema> {
        println!("Collected: {}", schema.name);
    }
}
```

This will automatically collect and build all .proto files from all crates in your dependency tree that use `proto_rs` macros!

## Examples

You can see more in examples:

- **proto_gen_example** - simple service with streaming (generated .proto saved here: protos/gen_proto)
- **prosto_proto** - showcase of type possibilities (generated .proto saved here: protos/showcase_proto)
- **tests/proto_build_test** - example of how you can build .proto files only on demand


## .proto Auto-Emission Control

Controls auto-emission of .proto files by macros:
- `"emit-proto-files"` - cargo feature
- `"PROTO_EMIT_FILE"` - env var

### .proto Auto-Emission Behavior

| Feature | Env Var | Result |
|---------|---------|--------|
| none | not set | ❌ No emission |
| none | true | ✅ Emit files |
| none | false | ❌ No emission |
| emit-proto-files | not set | ✅ Emit files |
| emit-proto-files | true | ✅ Emit files |
| emit-proto-files | false | ❌ No emission (override) |
| build-schemas | (any) | ✅ Emit const |

## proto_dump for hand-written types

This crate also provides an auxiliary macro `#[proto_dump(proto_path ="protos/proto_dump.proto")]` that outputs a .proto file. This is helpful for hand-written types.

```rust
#[proto_dump(proto_path ="protos/proto_dump.proto")]
#[derive(prost::Message, Clone, PartialEq)]
pub struct LamportsProto {
    #[prost(uint64, tag = 1)]
    pub amount: u64,
}
```

Generated proto:

```proto
syntax = "proto3";
package proto_dump;

message Lamports {
    uint64 amount = 1;
}
```

## Inject Proto Imports

It's not mandatory to use this macro at all, macros above derive and inject imports from attributes automaticly

But in case you need it, to add custom imports to your generated .proto files use the `inject_proto_import!` macro:

```rust
inject_proto_import!("protos/test.proto", "google.protobuf.timestamp", "common");
```

This will inject the specified import statements into the target .proto file


## Advanced Features

Macros support all prost types, imports, skipping with default and custom functions, custom conversions, support for native Rust enums (like `Status` below) and prost enumerations (TestEnum in this example, see more in prosto_proto).

### Struct with Advanced Attributes

```rust
#[proto_message(proto_path ="protos/showcase_proto/show.proto")]
pub struct Attr {
    #[proto(skip)]
    id_skip: Vec<i64>,
    id_vec: Vec<String>,
    id_opt: Option<String>,
    #[proto(rust_enum)]
    status: Status,
    #[proto(rust_enum)]
    status_opt: Option<Status>,
    #[proto(rust_enum)]
    status_vec: Vec<Status>,
    #[proto(skip = "compute_hash_for_struct")]
    hash: String,
    #[proto(import_path = "google.protobuf")]
    #[proto(message)]
    timestamp: Timestamp,
    #[proto(message)]
    #[proto(import_path = "google.protobuf")]
    timestamp_vec: Vec<Timestamp>,
    #[proto(message)]
    #[proto(import_path = "google.protobuf")]
    timestamp_opt: Option<Timestamp>,
    #[proto(enum)]
    #[proto(import_path = "google.protobuf")]
    test_enum: TestEnum,
    #[proto(enum)]
    #[proto(import_path = "google.protobuf")]
    test_enum_opt: Option<TestEnum>,
    #[proto(enum)]
    #[proto(import_path = "google.protobuf")]
    test_enum_vec: Vec<TestEnum>,
    #[proto(into = "i64", into_fn = "datetime_to_i64", from_fn = "i64_to_datetime")]
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
        #[proto(rust_enum)]
        status: Status,
        #[proto(rust_enum)]
        status_opt: Option<Status>,
        #[proto(rust_enum)]
        status_vec: Vec<Status>,
        #[proto(skip = "compute_hash_for_enum")]
        hash: String,
        #[proto(import_path = "google.protobuf")]
        #[proto(message)]
        timestamp: Timestamp,
        #[proto(message)]
        #[proto(import_path = "google.protobuf")]
        timestamp_vec: Vec<Timestamp>,
        #[proto(message)]
        #[proto(import_path = "google.protobuf")]
        timestamp_opt: Option<Timestamp>,
        #[proto(enum)]
        #[proto(import_path = "google.protobuf")]
        test_enum: TestEnum,
        #[proto(enum)]
        #[proto(import_path = "google.protobuf")]
        test_enum_opt: Option<TestEnum>,
        #[proto(enum)]
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

### Simple Rust Enum

```rust
#[proto_message(proto_path ="protos/showcase_proto/show.proto")]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum Status {
    Pending,
    #[default]
    Active,
    Inactive,
    Completed,
}
```

Generated proto:

```proto
enum Status {
  PENDING = 0;
  ACTIVE = 1;
  INACTIVE = 2;
  COMPLETED = 3;
}
```

## Dependencies

Crate pulled dependencies:

```
01:04:53 ➜ cargo tree
proto_rs v0.2.0
├── prost v0.14.1
│   ├── bytes v1.10.1
│   └── prost-derive v0.14.1 (proc-macro)
│       ├── anyhow v1.0.100
│       ├── itertools v0.14.0
│       │   └── either v1.15.0
│       ├── proc-macro2 v1.0.101
│       │   └── unicode-ident v1.0.19
│       ├── quote v1.0.41
│       │   └── proc-macro2 v1.0.101 (*)
│       └── syn v2.0.106
│           ├── proc-macro2 v1.0.101 (*)
│           ├── quote v1.0.41 (*)
│           └── unicode-ident v1.0.19
└── prosto_derive v0.2.0 (proc-macro) 
    ├── proc-macro2 v1.0.101 (*)
    ├── quote v1.0.41 (*)
    └── syn v2.0.106 (*)
```