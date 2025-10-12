use chrono::DateTime;
use chrono::Utc;
use prosto_derive::proto_dump;
use proto_rs::inject_proto_import;
use proto_rs::proto_message;
use serde::Deserialize;
use serde::Serialize;

inject_proto_import!("protos/test.proto", "google.protobuf.timestamp", "common.types");

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq, Default)]
pub struct StructU16 {
    inner: u16,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub struct StructU8 {
    inner: u8,
}
#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub struct StructU816 {
    inner: u8,
    inner2: u64,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub struct TupleStructTest(u64, u8);

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub struct TupleStruct2Test([u8; 32], [u16; 32]);

#[proto_dump(proto_path = "protos/proto_dump.proto")]
#[derive(Clone, PartialEq)]
pub struct ArrayTest {
    pub amount: [u8; 32],
}
#[proto_dump(proto_path = "protos/proto_dump.proto")]
#[derive(Clone, PartialEq)]
pub struct ArrayTest2([u8; 32]);

#[proto_dump(proto_path = "protos/proto_dump.proto")]
#[derive(Clone, PartialEq)]
pub enum ArrayTest3 {
    Test,
    Test1([u8; 32]),
    Test2 { test: [u8; 32] },
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub struct ArrayTestMessageU16 {
    pub amount: [u16; 32],
}

#[proto_dump(proto_path = "protos/proto_dump.proto")]
#[derive(Clone, PartialEq)]
pub struct ArrayTestU16 {
    pub amount: [u16; 32],
}

#[proto_dump(proto_path = "protos/proto_dump.proto")]
#[derive(Clone, PartialEq)]
pub struct ArrayTest2U16([u16; 32]);

#[proto_dump(proto_path = "protos/proto_dump.proto")]
#[derive(Clone, PartialEq)]
pub enum ArrayTest3U64 {
    Test,
    Test1([u16; 32]),
    Test2 { test: [u16; 32] },
}

#[proto_dump(proto_path = "protos/proto_dump.proto")]
#[derive(Clone, PartialEq)]
pub struct ArrayTest4U64([u64; 32]);

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub struct ArrayTestMessageCustom {
    pub amount: [ArrayTestMessageU16; 32],
}
#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub struct VecTestMessageCustom {
    pub amount: Vec<ArrayTestMessageU16>,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub struct VecTestMessageU64 {
    pub amount: Vec<u64>,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub enum ArrayTest3Custom {
    Test,
    Test1([ArrayTestMessageU16; 32]),
    Test2 { test: [ArrayTestMessageU16; 32] },
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub struct StructFailU16 {
    pub amount_2: u16,
    pub amount: Option<u16>,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub struct VecTestMessageU16 {
    pub amount: Vec<u16>,
    pub amount_opt: Option<u16>,
    pub amount_plain: u16,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, Copy)]
pub enum Status {
    Pending,
    #[default]
    Active,
    Inactive,
    Completed,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
pub enum EnumArrayRustEnumAttributeFailTest {
    Fail {
        #[proto(rust_enum)]
        timestamp_array: [Status; 8],
    },
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
pub enum EnumArrayRustEnumAttributeFailTest2 {
    Fail(#[proto(rust_enum)] [Status; 8]),
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub enum VecTestEnumU16 {
    Test,
    Test1(Vec<u16>),
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub enum FailingOptionCustomTupleVariant {
    Test,
    Test1(Option<User>),
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub enum VecTestEnumU64 {
    Test,
    Test1(Vec<u64>),
    Test2 { test: Vec<u64> },
    Test3(Option<User>),
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub enum VecTestEnumProst {
    Test10 { test3: User },
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub enum VecTestEnumCustom {
    Test,
    Test0(#[proto(rust_enum)] Status),
    Test1(#[proto(rust_enum)] Vec<Status>),
    Test2 {
        #[proto(rust_enum)]
        test1: Vec<Status>,
        #[proto(rust_enum)]
        test2: Option<Status>,
        #[proto(rust_enum)]
        test3: Status,
    },
    Test3(#[proto(rust_enum)] Option<Status>),

    Test7(User),
    Test8(Option<User>),
    Test9(Vec<User>),
    Test10 {
        test: Vec<User>,
        test1: Option<User>,
        test3: User,
    },
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub enum VecTestEnumCustom2 {
    Test0(#[proto(rust_enum)] Status),
    Test1(#[proto(rust_enum)] Vec<Status>),
    Test2 {
        #[proto(rust_enum)]
        test1: Vec<Status>,
        #[proto(rust_enum)]
        test2: Option<Status>,
        #[proto(rust_enum)]
        test3: Status,
    },
    Test3(#[proto(rust_enum)] Option<Status>),
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub enum VecFailingTestEnum {
    Test2 {
        #[proto(rust_enum)]
        test1: Vec<Status>,
        #[proto(rust_enum)]
        test2: Option<Status>,
        #[proto(rust_enum)]
        test3: Status,
    },
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Debug, PartialEq, Default, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    #[proto(skip = "compute_hash")]
    pub hash: String, // Will call compute_hash(&proto)
    #[proto(skip = "get_current_timestamp")]
    pub created_at: DateTime<Utc>, // Will call get_current_timestamp(&proto)
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub enum VecTestEnumU8 {
    Test,
    Test1(Vec<u8>),
    Test2 { test: Vec<u8> },
}

#[proto_dump(proto_path = "protos/proto_dump.proto")]
#[derive(Clone, PartialEq)]
pub struct ArrayTestMessageCustomDump {
    pub amount: [ArrayTestMessageU16; 32],
}

#[proto_dump(proto_path = "protos/proto_dump.proto")]
#[derive(Clone, PartialEq)]
pub enum ArrayTest3CustomDump {
    Test,
    Test1([ArrayTestMessageU16; 32]),
    Test2 { test: [ArrayTestMessageU16; 32] },
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
pub enum VeryComplexTestSkip {
    Attr {
        #[proto(skip = "compute_hash_for_enum_test")]
        hash: String,
        #[proto(skip)]
        hash_2: String,
    },
    Tuple(#[proto(skip = "compute_hash_for_enum_test_2")] String),
    Tuple2(#[proto(skip)] String),
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
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
    },
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
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

    #[proto(into = "i64", into_fn = "datetime_to_i64", from_fn = "i64_to_datetime")]
    pub updated_at: DateTime<Utc>,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, Serialize, Deserialize)]
pub struct SimpleMessage {
    pub id: u64,
    pub name: String,
    pub active: bool,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct FileData {
    pub name: String,
    pub content: Vec<u8>, // Should be bytes in proto
    pub size: u64,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct UserList {
    pub ids: Vec<u64>,
    pub names: Vec<String>,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct OptionalFields {
    pub id: u64,
    pub email: Option<String>,
    pub age: Option<u32>,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, Serialize, Deserialize)]
pub enum QuoteLamports {
    Lamports(u64),
    Wsol(u64),
    Usdc(u64),
    Usdt(u64),
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct Order {
    pub id: u64,
    pub amount: u64,
    pub quote: QuoteLamports, // Complex type - needs shadow
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone, PartialEq)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub country: String,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
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

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct UserWithMetadata {
    pub id: u64,
    pub name: String,
    #[proto(skip)]
    pub internal_cache: Option<String>,
    #[proto(skip)]
    pub computed_value: u64,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct Transaction {
    pub id: u64,
    #[proto(into = "i64", into_fn = "datetime_to_i64", from_fn = "i64_to_datetime")]
    pub created_at: DateTime<Utc>,
    #[proto(into = "i64", into_fn = "datetime_to_i64", from_fn = "i64_to_datetime")]
    pub updated_at: DateTime<Utc>,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub enum PaymentMethod {
    Cash(u64),
    Card(String),
    Crypto(QuoteLamports),
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct Payment {
    pub id: u64,
    pub amount: u64,
    pub method: PaymentMethod,
    #[proto(into = "i64", into_fn = "datetime_to_i64", from_fn = "i64_to_datetime")]
    pub created_at: DateTime<Utc>,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
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

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct NumericTypes {
    pub uint32_field: u32,
    pub uint64_field: u64,
    pub int32_field: i32,
    pub int64_field: i64,
    pub float_field: f32,
    pub double_field: f64,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct OptionalComplex {
    pub id: u64,
    pub address: Option<Address>,
    pub quote: Option<QuoteLamports>,
    pub status: Option<Status>,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct RepeatedComplex {
    pub id: u64,
    pub addresses: Vec<Address>,
    pub orders: Vec<Order>,
    pub statuses: Vec<Status>,
    pub status_opt: Option<Status>,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
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

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct Empty {}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub struct SingleField {
    pub value: u64,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
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

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
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

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
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
