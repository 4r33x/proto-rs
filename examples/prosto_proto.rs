use chrono::DateTime;
use chrono::Utc;
use proto_rs::HasProto;
use proto_rs::inject_proto_import;
use proto_rs::proto_message;
use serde::Deserialize;
use serde::Serialize;

inject_proto_import!("test.proto", "google/protobuf/timestamp", "common/types");

fn get_current_timestamp(_proto: &UserProto) -> DateTime<Utc> {
    Utc::now()
}
fn compute_hash(proto: &UserProto) -> String {
    format!("hash_{}_{}", proto.id, proto.name)
}
fn compute_hash_for_enum(proto: &VeryComplexProtoAttr) -> String {
    format!("hash_{}_{}", proto.status, proto.status)
}
fn compute_hash_for_struct(proto: &AttrProto) -> String {
    format!("hash_{}_{}", proto.status, proto.status)
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum Status {
    Pending,
    #[default]
    Active,
    Inactive,
    Completed,
}

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

#[derive(::prost::Message, Clone, PartialEq)]
pub struct Timestamp {}

#[derive(::prost::Enumeration, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TestEnum {
    Test = 0i32,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
pub struct User {
    pub id: u64,
    pub name: String,
    #[proto(skip = "compute_hash")]
    pub hash: String, // Will call compute_hash(&proto)
    #[proto(skip = "get_current_timestamp")]
    pub created_at: DateTime<Utc>, // Will call get_current_timestamp(&proto)
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
pub struct MyMessage {
    pub id: u64,
    #[proto(import_path = "google.protobuf")]
    #[proto(message)]
    pub timestamp: Timestamp,
    #[proto(message)]
    pub timestamp_vec: Vec<Timestamp>,
    #[proto(message)]
    pub timestamp_opt: Option<Timestamp>,
    pub name: String,
    #[proto(enum)]
    pub test_enum: TestEnum,
    #[proto(enum)]
    pub test_enum_opt: Option<TestEnum>,
    #[proto(enum)]
    pub test_enum_vec: Vec<TestEnum>,
}

// ============================================================================
// Test 1: Simple struct - no shadow needed
// ============================================================================
#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone, Serialize, Deserialize)]
pub struct SimpleMessage {
    pub id: u64,
    pub name: String,
    pub active: bool,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct FileData {
    pub name: String,
    pub content: Vec<u8>, // Should be bytes in proto
    pub size: u64,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct UserList {
    pub ids: Vec<u64>,
    pub names: Vec<String>,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct OptionalFields {
    pub id: u64,
    pub email: Option<String>,
    pub age: Option<u32>,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone, Serialize, Deserialize)]
pub enum QuoteLamports {
    Lamports(u64),
    Wsol(u64),
    Usdc(u64),
    Usdt(u64),
}

// ============================================================================
// Test 7: Struct with complex enum field - needs shadow
// ============================================================================
#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct Order {
    pub id: u64,
    pub amount: u64,
    pub quote: QuoteLamports, // Complex type - needs shadow
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub country: String,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct Person {
    pub id: u64,
    pub name: String,
    pub address: Address, // Complex type - needs shadow
}

fn datetime_to_i64(dt: &DateTime<Utc>) -> i64 {
    dt.timestamp()
}

fn i64_to_datetime(ts: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(ts, 0).unwrap()
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct UserWithMetadata {
    pub id: u64,
    pub name: String,
    #[proto(skip)]
    pub internal_cache: Option<String>,
    #[proto(skip)]
    pub computed_value: u64,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct Transaction {
    pub id: u64,
    #[proto(into = "i64", into_fn = "datetime_to_i64", from_fn = "i64_to_datetime")]
    pub created_at: DateTime<Utc>,
    #[proto(into = "i64", into_fn = "datetime_to_i64", from_fn = "i64_to_datetime")]
    pub updated_at: DateTime<Utc>,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub enum PaymentMethod {
    Cash(u64),
    Card(String),
    Crypto(QuoteLamports),
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct Payment {
    pub id: u64,
    pub amount: u64,
    pub method: PaymentMethod,
    #[proto(into = "i64", into_fn = "datetime_to_i64", from_fn = "i64_to_datetime")]
    pub created_at: DateTime<Utc>,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct Invoice {
    pub id: u64,
    pub customer: Person,
    pub payments: Vec<Payment>,
    #[proto(rust_enum)]
    pub status: Status,
    #[proto(skip)]
    pub internal_notes: String,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct NumericTypes {
    pub uint32_field: u32,
    pub uint64_field: u64,
    pub int32_field: i32,
    pub int64_field: i64,
    pub float_field: f32,
    pub double_field: f64,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct OptionalComplex {
    pub id: u64,
    pub address: Option<Address>,
    pub quote: Option<QuoteLamports>,
    #[proto(rust_enum)]
    pub status: Option<Status>,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct RepeatedComplex {
    pub id: u64,
    pub addresses: Vec<Address>,
    pub orders: Vec<Order>,
    #[proto(rust_enum)]
    pub statuses: Vec<Status>,
    #[proto(rust_enum)]
    pub status_opt: Option<Status>,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct MixedAnnotations {
    pub id: u64,
    #[proto(into = "i64", into_fn = "datetime_to_i64", from_fn = "i64_to_datetime")]
    pub created_at: DateTime<Utc>,
    #[proto(skip)]
    pub cached_result: Option<String>,
    pub address: Address,
    #[proto(skip)]
    pub dirty_flag: bool,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct Empty {}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct SingleField {
    pub value: u64,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct UserWithDefaults {
    pub id: u64,
    pub name: String,
    #[proto(skip)]
    pub created_at: DateTime<Utc>, // Will use Default::default() in From conversion
    #[proto(skip)]
    pub version: u32, // Will use Default::default() (0)
}

fn serialize_json(value: &serde_json::Value) -> String {
    value.to_string()
}

fn deserialize_json(value: String) -> serde_json::Value {
    serde_json::from_str(&value).unwrap_or(serde_json::Value::Null)
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct ComplexConversions {
    pub id: u64,

    // Function-based conversion with type annotation
    #[proto(into = "i64", into_fn = "datetime_to_i64", from_fn = "i64_to_datetime")]
    pub timestamp: DateTime<Utc>,

    // Function-based conversion with type annotation
    #[proto(into = "String", into_fn = "serialize_json", from_fn = "deserialize_json")]
    pub metadata: serde_json::Value,

    // Skip field
    #[proto(skip)]
    pub internal_state: String,

    // Simple enum
    #[proto(rust_enum)]
    pub status: Status,

    // Regular field
    pub name: String,
}

#[proto_message(file = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct FieldNumbering {
    pub field1: u64, // tag = 1
    #[proto(skip)]
    pub skipped: String, // No tag
    pub field2: String, // tag = 2 (not 3!)
    pub field3: u32, // tag = 3
    #[proto(skip)]
    pub skipped2: bool, // No tag
    pub field4: bool, // tag = 4
}

fn main() {}
