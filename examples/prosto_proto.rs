use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

use chrono::DateTime;
use chrono::Utc;
use prosto_derive::proto_dump;
use proto_rs::DecodeError;
use proto_rs::ProtoShadowDecode;
use proto_rs::inject_proto_import;
use proto_rs::proto_message;
use serde::Deserialize;
use serde::Serialize;

inject_proto_import!("protos/test.proto", "google.protobuf.timestamp", "common.types");

//example of custom encode\decode impl
pub struct ValueCanBeFolded {
    a: u64,
    b: u64,
    c: u64,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto", sun = [ValueCanBeFolded])]
pub struct FoldedValue {
    pub a: u64,
    pub b: u64,
}

impl ProtoShadowDecode<ValueCanBeFolded> for FoldedValue {
    fn to_sun(self) -> Result<ValueCanBeFolded, proto_rs::DecodeError> {
        Err(DecodeError::new("TokenBalanceSealed can't be accepted by server"))
    }
}

#[proto_message(transparent)]
#[derive(Debug)]
pub struct TinyLruTransparent<T, const CAP: usize> {
    items: VecDeque<T>, // MRU..LRU
}
#[proto_message(transparent)]
#[derive(Debug)]
pub struct TinyLruVecTransparent<T, const CAP: usize> {
    items: Vec<T>, // MRU..LRU
}

#[proto_message(transparent)]
#[derive(Debug)]
pub struct GenericMapTransparent<K: std::hash::Hash + Eq, V, S: std::hash::BuildHasher + Default, const CAP: usize> {
    kv: HashMap<K, V, S>,
}

#[proto_message(transparent)]
#[derive(Debug)]
pub enum GenericEnum<T, K: std::hash::Hash + Eq, V, S: std::hash::BuildHasher + Default, const CAP: usize> {
    Map(HashMap<K, V, S>),
    Vec { inner: Vec<T> },
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[proto(generic_types = [T = [u8]])]
pub struct TinyLru<T, const CAP: usize> {
    items: VecDeque<T>, // MRU..LRU
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[proto(generic_types = [K = [String], V = [u8]])]
pub struct LruPair<K, V> {
    k: K,
    v: V,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[proto(generic_types = [K = [String], V = [u8]])]
pub struct TinyLruKeyd<K, V, const CAP: usize> {
    items: VecDeque<LruPair<K, V>>, // MRU..LRU
}

#[proto_message]
pub struct ConcreteLru {
    inner: TinyLru<u32, 32>,
}

#[proto_message]
pub struct ConcreteLru2 {
    inner: TinyLru<String, 32>,
}

#[proto_message]
#[derive(Debug)]
pub struct TinyLruVec<T, const CAP: usize> {
    items: Vec<T>, // MRU..LRU
}

#[proto_message]
#[derive(Debug)]
pub struct GenericStruct<K: std::hash::Hash + Eq, V, S, const CAP: usize> {
    k: K,
    v: V,
    s: S,
    vec_k: Vec<K>,
    vec_v: VecDeque<V>,
    kv: HashMap<K, V>,
    //TODO! tuples not yet supported, but I think they should be, especially when new build system with auto import resolving lands
    // kvs: (K, V, S),
    // vec_kv: Vec<(K, V)>,
}

#[proto_message]
#[derive(Debug)]
pub struct GenericMap<K: std::hash::Hash + Eq, V, S: std::hash::BuildHasher + Default, const CAP: usize> {
    kv: HashMap<K, V, S>,
}
impl<K: std::hash::Hash + Eq, V, S: std::hash::BuildHasher + Default, const CAP: usize> GenericMap<K, V, S, CAP> {
    #[allow(clippy::new_without_default)]
    #[allow(clippy::must_use_candidate)]
    pub fn new() -> Self {
        Self {
            kv: HashMap::with_capacity_and_hasher(CAP, S::default()),
        }
    }
}

type CustomMutex<T> = std::sync::Mutex<T>;
type CustomArc<T> = std::sync::Arc<T>;
type CustomBox<T> = Box<T>;
type CustomMap<K, V, S> = HashMap<K, V, S>;
type CustomOption<T> = Option<T>;
type CustomVec<T> = Vec<T>;
type CustomVecDeq<T> = VecDeque<T>;

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
struct ConcreteStdMap {
    map_alias: CustomMap<u32, MEx, std::hash::RandomState>,
    map: HashMap<u16, u8, std::hash::RandomState>,
}

#[proto_message]
struct MEx {
    id: u64,
}

#[proto_message]
struct CustomEx {
    mutex: std::sync::Mutex<MEx>,
    mutex_copy: std::sync::Mutex<u64>,
    mutex_custom: CustomMutex<MEx>,
    mutex_copy_custom: CustomMutex<u64>,
    arc: std::sync::Arc<MEx>,
    arc_copy: std::sync::Arc<u64>,
    arc_custom: CustomArc<MEx>,
    arc_copy_custom: CustomArc<u64>,
    boxed: Box<MEx>,
    box_copy: Box<u64>,
    boxed_custom: CustomBox<MEx>,
    box_copy_custom: CustomBox<u64>,
    custom_map: CustomMap<u32, MEx, std::hash::RandomState>,
    custom_option: CustomOption<MEx>,
    custom_option_copy: CustomOption<u64>,
    custom_vec_bytes: CustomVec<u8>,
    custom_vec_deque_bytes: CustomVecDeq<u8>,
    custom_vec_copy: CustomVec<u64>,
    custom_vec_deque_copy: CustomVecDeq<u64>,
    custom_vec: CustomVec<MEx>,
    custom_vec_deque: CustomVecDeq<MEx>,
}

#[cfg(feature = "parking_lot")]
pub mod parking_lot {
    use proto_rs::proto_message;

    type CustomMutex<T> = parking_lot::Mutex<T>;
    #[proto_message]
    struct MEx;

    #[proto_message]
    struct ExMutex {
        a: CustomMutex<MEx>,
        b: CustomMutex<u64>,
    }
}

#[cfg(feature = "cache_padded")]
pub mod cache_padded {
    use proto_rs::proto_message;

    type CustomCachePadded<T> = crossbeam_utils::CachePadded<T>;

    #[proto_message]
    struct Ex {
        inner: CustomCachePadded<core::sync::atomic::AtomicU64>,
        inner2: CustomCachePadded<super::MEx>,
        inner3: crossbeam_utils::CachePadded<core::sync::atomic::AtomicU64>,
        inner4: crossbeam_utils::CachePadded<super::MEx>,
    }
}

#[cfg(feature = "papaya")]
pub mod papaya_test {
    use proto_rs::proto_message;

    type MapWithHasher<K, V, S> = papaya::HashMap<K, V, S>;
    type SetWithHasher<K, S> = papaya::HashSet<K, S>;

    #[proto_message]
    pub struct ConcretePapayaEx {
        inner: MapWithHasher<u32, u8, std::hash::RandomState>,
        inner2: SetWithHasher<u32, std::hash::RandomState>,
    }

    #[proto_message]
    pub struct GenericPapayaMap<K: std::hash::Hash + Eq, V> {
        inner: papaya::HashMap<K, V>,
    }

    #[proto_message]
    pub struct GenericPapayaSet<K: std::hash::Hash + Eq> {
        inner: papaya::HashSet<K>,
    }
    #[proto_message]
    pub struct ConcretePapayaSet {
        inner: GenericPapayaSet<u8>,
    }
    #[proto_message]
    pub struct ConcretePapayaSet2 {
        inner: papaya::HashSet<u8, std::hash::RandomState>,
    }
}

#[proto_message]
pub struct ConcreteMap {
    inner: GenericMap<u64, String, std::hash::RandomState, 32>,
}

#[proto_message]
pub struct ConcreteMap2 {
    inner: HashMap<u64, String, std::hash::RandomState>,
}
#[proto_message]
pub struct ConcreteSet {
    inner: HashSet<u64, std::hash::RandomState>,
}

#[proto_message]
pub struct GenericMapInMap<K: std::hash::Hash + Eq, V, S: std::hash::BuildHasher + Default, const CAP: usize> {
    inner: GenericMap<K, V, S, CAP>,
}

pub type ComplexType = proto_rs::alloc::collections::BTreeMap<u64, u64>;
pub type ComplexType2 = std::collections::HashMap<u64, u64, std::hash::RandomState>;

#[proto_message]
#[derive(Debug, PartialEq, Eq)]
pub struct UserIdTreatAs {
    #[proto(treat_as = "proto_rs::alloc::collections::BTreeMap<u64, u64>")]
    pub id: ComplexType,
    #[proto(treat_as = "std::collections::HashMap<u64, u64>")]
    pub id2: ComplexType2,
}

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
    inner3: u16,
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

#[derive(Debug, Hash, PartialEq, Eq)]
#[proto_message]
pub struct Foo {
    #[proto(tag = 1)]
    pub id: u32,
    #[proto(tag = 2)]
    pub meta: u32,
}

#[derive(Debug)]
#[proto_message]
pub struct FooBar {
    #[proto(tag = 1)]
    pub map: std::collections::HashMap<Foo, u32>,
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
pub enum EnumArrayRustEnumAttributeFailTest {
    Fail { timestamp_array: [Status; 8] },
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
pub enum EnumArrayRustEnumAttributeFailTest2 {
    Fail([Status; 8]),
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
    Test0(Status),
    Test1(Vec<Status>),
    Test2 {
        test1: Vec<Status>,
        test2: Option<Status>,
        test3: Status,
    },
    Test3(Option<Status>),

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
    Test0(Status),
    Test1(Vec<Status>),
    Test2 {
        test1: Vec<Status>,
        test2: Option<Status>,
        test3: Status,
    },
    Test3(Option<Status>),
}

#[proto_message(proto_path = "protos/showcase_proto/show.proto")]
#[derive(Clone)]
pub enum VecFailingTestEnum {
    Test2 {
        test1: Vec<Status>,
        test2: Option<Status>,
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
        status: Status,
        status_opt: Option<Status>,
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
    status: Status,
    status_opt: Option<Status>,
    status_vec: Vec<Status>,
    #[proto(skip = "compute_hash_for_struct")]
    hash: String,

    #[proto(into = "i64", into_fn = "datetime_to_i64", from_fn = "i64_to_datetime")]
    pub updated_at: DateTime<Utc>,
}

fn compute_hash(user: &User) -> String {
    format!("{}:{}", user.id, user.name)
}

fn get_current_timestamp(_user: &User) -> DateTime<Utc> {
    Utc::now()
}

fn compute_hash_for_struct(attr: &Attr) -> String {
    let mut parts = attr.id_vec.join("|");
    if let Some(opt) = &attr.id_opt {
        if !parts.is_empty() {
            parts.push('|');
        }
        parts.push_str(opt);
    }

    format!("{}|{:?}|{:?}", parts, attr.status, attr.status_opt)
}

fn compute_hash_for_enum_test(value: &VeryComplexTestSkip) -> String {
    match value {
        VeryComplexTestSkip::Attr { hash_2, .. } => format!("enum-attr:{hash_2}"),
        VeryComplexTestSkip::Tuple(inner) => format!("enum-tuple:{inner}"),
        VeryComplexTestSkip::Tuple2(inner) => format!("enum-tuple2:{inner}"),
    }
}

fn compute_hash_for_enum_test_2(value: &VeryComplexTestSkip) -> String {
    match value {
        VeryComplexTestSkip::Tuple(inner) => format!("tuple-only:{inner}"),
        _ => compute_hash_for_enum_test(value),
    }
}

fn compute_hash_for_enum(value: &VeryComplex) -> String {
    match value {
        VeryComplex::Attr {
            id_vec,
            id_opt,
            status,
            status_opt,
            ..
        } => {
            let mut parts = id_vec.join("|");
            if let Some(opt) = id_opt {
                if !parts.is_empty() {
                    parts.push('|');
                }
                parts.push_str(opt);
            }
            format!("{parts}|{status:?}|{status_opt:?}")
        }
        _ => String::new(),
    }
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
pub struct OrderBook {
    pub inner: BTreeMap<u64, Order>,
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

const fn datetime_to_i64(dt: &DateTime<Utc>) -> i64 {
    dt.timestamp()
}

const fn i64_to_datetime(ts: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(ts, 0).unwrap()
}
fn try_i64_to_datetime(ts: i64) -> Result<DateTime<Utc>, DecodeError> {
    DateTime::from_timestamp(ts, 0).ok_or(DecodeError::new("Bad timestamp"))
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
    #[proto(into = "i64", into_fn = "datetime_to_i64", try_from_fn = "try_i64_to_datetime")]
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

#[allow(clippy::needless_pass_by_value)]
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
