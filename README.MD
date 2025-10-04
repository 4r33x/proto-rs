# Rust as First-class citizen for gRPC

This crate provides 2 macros that will handle all proto-related work, so you don't need to touch .proto files at all.

**Motivation:**

0. I hate to do conversion after conversion for conversion 
1. I love to see Rust only as first citizen for all my stuff
2. I hate bloat, so no protoc (shoutout to pewdiepie debloat trend)
3. I don't want to touch .proto files at all

`#[proto_rpc]` macro will convert your Rust native trait to tonic:

```rust
#[proto_rpc(sigma_rpc, server = true, client = true, proto = true, proto_path = "protos/gen_proto/sigma_rpc.proto")]
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
    type Proto: Clone + prost::Message + PartialEq;
    fn to_proto(&self) -> Self::Proto;
    fn from_proto(proto: Self::Proto) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;
}
```

We can derive it (or manually impmlement) for most types with #[proto_message] macro:

```rust
#[proto_message(file = "protos/gen_proto/goon_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct RizzPing;
```


But that's not all — `#[proto_message]` and `#[proto_rpc]` will also create .proto definitions for non-Rust clients.

You can see more in examples:

- **proto_gen_example** - simple service with streaming (generated .proto saved here: protos/gen_proto)
- **prosto_proto** - showcase of type possibilities (generated .proto saved here: protos/showcase_proto)

You can also test implementations in tests.

The macros support all prost types, imports, skipping with default and custom functions, custom conversions, support for native Rust enums (like `Status` below) and prost enumerations (TestEnum in this example, see more in prosto_proto).

```rust
#[proto_message(file = "protos/showcase_proto/show.proto")]
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

Generated:
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

Complex enums:

```rust
#[proto_message(file = "protos/showcase_proto/show.proto")]
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

Generated Proto:

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

Rust simple enum:

```rust
#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum Status {
    Pending,
    #[default]
    Active,
    Inactive,
    Completed,
}
```

```proto
enum Status {
  PENDING = 0;
  ACTIVE = 1;
  INACTIVE = 2;
  COMPLETED = 3;
}
```

Crate pulled dependencies:

```
01:04:53 ➜ cargo tree
proto_rs v0.1.0
└── prost v0.14.1
    ├── bytes v1.10.1
    └── prost-derive v0.14.1 (proc-macro)
        ├── anyhow v1.0.100
        ├── itertools v0.14.0
        │   └── either v1.15.0
        ├── proc-macro2 v1.0.101
        │   └── unicode-ident v1.0.19
        ├── quote v1.0.41
        │   └── proc-macro2 v1.0.101 (*)
        └── syn v2.0.106
            ├── proc-macro2 v1.0.101 (*)
            ├── quote v1.0.41 (*)
            └── unicode-ident v1.0.19
```