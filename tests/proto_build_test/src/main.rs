#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::num::NonZeroI8;
use std::num::NonZeroI16;
use std::num::NonZeroI32;
use std::num::NonZeroI64;
use std::num::NonZeroIsize;
use std::num::NonZeroU8;
use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI8;
use std::sync::atomic::AtomicI16;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;

use chrono::DateTime;
use chrono::Utc;
use proto_rs::proto_message;
use proto_rs::proto_rpc;
use proto_rs::schemas::AttrLevel;
use proto_rs::schemas::ClientAttrTarget;
use proto_rs::schemas::MethodReplace;
use proto_rs::schemas::ProtoIdentifiable;
use proto_rs::schemas::TypeReplace;
use proto_rs::schemas::UserAttr;
use tokio_stream::Stream;
use tonic::Response;
use tonic::Status;

type CustomMutex<T> = std::sync::Mutex<T>;
type CustomArc<T> = std::sync::Arc<T>;
type CustomBox<T> = Box<T>;
type CustomMap<K, V, S> = HashMap<K, V, S>;
type CustomOption<T> = Option<T>;
type CustomVec<T> = Vec<T>;
type CustomVecDeq<T> = VecDeque<T>;

#[proto_message(proto_path = "protos/build_system_test/custom_types.proto")]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MEx {
    pub id: u64,
}

#[proto_message(proto_path = "protos/build_system_test/custom_types.proto")]
#[derive(Debug)]
pub struct CustomEx {
    pub mutex: std::sync::Mutex<MEx>,
    pub mutex_copy: std::sync::Mutex<u64>,
    pub mutex_custom: CustomMutex<MEx>,
    pub mutex_copy_custom: CustomMutex<u64>,
    pub arc: std::sync::Arc<MEx>,
    pub arc_copy: std::sync::Arc<u64>,
    pub arc_custom: CustomArc<MEx>,
    pub arc_copy_custom: CustomArc<u64>,
    pub boxed: Box<MEx>,
    pub box_copy: Box<u64>,
    pub boxed_custom: CustomBox<MEx>,
    pub box_copy_custom: CustomBox<u64>,
    pub custom_map: CustomMap<u32, MEx, std::hash::RandomState>,
    pub custom_option: CustomOption<MEx>,
    pub custom_option_copy: CustomOption<u64>,
    pub custom_vec_bytes: CustomVec<u8>,
    pub custom_vec_deque_bytes: CustomVecDeq<u8>,
    pub custom_vec_copy: CustomVec<u64>,
    pub custom_vec_deque_copy: CustomVecDeq<u64>,
    pub custom_vec: CustomVec<MEx>,
    pub custom_vec_deque: CustomVecDeq<MEx>,
}

#[proto_message(proto_path = "protos/build_system_test/atomic_types.proto")]
#[derive(Debug)]
pub struct AtomicPrimitives {
    pub flag: AtomicBool,
    pub count: AtomicU64,
    pub small: AtomicU8,
    pub smaller: AtomicU16,
    pub signed_small: AtomicI8,
    pub signed_smaller: AtomicI16,
    pub signed: AtomicI32,
    pub sized: AtomicUsize,
    pub signed_sized: AtomicIsize,
}

#[proto_message(proto_path = "protos/build_system_test/atomic_types.proto")]
#[derive(Debug, Clone, PartialEq)]
pub struct NonZeroPrimitives {
    pub nz_u8: NonZeroU8,
    pub nz_u16: NonZeroU16,
    pub nz_u32: NonZeroU32,
    pub nz_u64: NonZeroU64,
    pub nz_usize: NonZeroUsize,
    pub nz_i8: NonZeroI8,
    pub nz_i16: NonZeroI16,
    pub nz_i32: NonZeroI32,
    pub nz_i64: NonZeroI64,
    pub nz_isize: NonZeroIsize,
}

#[proto_message(proto_path = "protos/build_system_test/lru_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct LruPair<K, V> {
    pub key: K,
    pub value: V,
}

#[proto_message(proto_path = "protos/build_system_test/lru_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct Lru<K, V, const CAP: usize> {
    pub items: VecDeque<LruPair<K, V>>, // MRU..LRU
}

#[proto_message(proto_path = "protos/build_system_test/lru_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct WithConcreteLru {
    lru1: Lru<u64, u64, 32>,
    lru2: Lru<u64, u64, 128>,
}

#[proto_message(proto_path = "protos/build_system_test/lru_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct WithComplexOption {
    pub inner: Option<Arc<WithConcreteLru>>,
}

// Test case for getter attribute filtering (Issue #2)
// Using getter attribute on a field - this is typically used with sun_ir types
// but here we just test that the attribute is filtered from client output
#[proto_message(proto_path = "protos/build_system_test/getter_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct GetterTestStruct {
    // The getter here just returns the same field - a simple case to test attribute filtering
    #[proto(tag = 1, getter = "$.id")]
    pub id: u64,
    #[proto(tag = 2)]
    pub name: String,
}

#[proto_message(proto_path = "protos/build_system_test/goon_types.proto")]
#[derive(Debug, Default, Clone, PartialEq, Copy)]
pub enum ServiceStatus {
    Pending,
    #[default]
    Active,
    Inactive,
    Completed,
}

#[proto_message(proto_path = "protos/build_system_test/goon_types.proto")]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Id {
    pub id: u64,
}

#[proto_message(proto_path = "protos/build_system_test/goon_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct RizzPing {
    id: Id,
    status: ServiceStatus,
}

#[proto_message(proto_path = "protos/build_system_test/goon_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct GoonPong {
    id: Id,
    status: ServiceStatus,
    // Test case for generic type args preservation (DateTime<Utc> should not become just DateTime)
    expire_at: Option<DateTime<Utc>>,
    expire_at2: DateTime<Utc>,
}

#[proto_message(proto_path = "protos/build_system_test/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct FooResponse;

#[proto_message(proto_path = "protos/build_system_test/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct BarSub;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MilliSeconds(pub i64);

impl From<MilliSeconds> for i64 {
    fn from(value: MilliSeconds) -> Self {
        value.0
    }
}

impl From<i64> for MilliSeconds {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

#[proto_message(transparent)]
#[derive(Clone, Debug, PartialEq)]
pub struct TransparentId(pub Id);

#[proto_message(proto_path = "protos/build_system_test/extra_types.proto")]
//unnecessary for build system, this imports would be auto-resolved
// #[proto(generic_types = [T = [BuildRequest, BuildResponse, GoonPong]])]
#[derive(Clone, Debug, PartialEq)]
pub struct Envelope<T> {
    #[proto(tag = 1)]
    pub payload: T,
    #[proto(tag = 2)]
    pub trace_id: String,
}

#[proto_message(proto_path = "protos/build_system_test/extra_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct BuildConfig {
    #[proto(tag = 1, into = "i64")]
    pub timeout: MilliSeconds,
    #[proto(tag = 2, skip)]
    pub cache_hint: String,
    #[proto(tag = 3)]
    pub owner: TransparentId,
}

#[proto_message(proto_path = "protos/build_system_test/extra_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct BuildRequest {
    #[proto(tag = 1)]
    pub config: BuildConfig,
    #[proto(tag = 2)]
    pub ping: RizzPing,
    #[proto(tag = 3)]
    pub owner: TransparentId,
}

#[proto_message(proto_path = "protos/build_system_test/extra_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct BuildResponse {
    #[proto(tag = 1)]
    pub status: ServiceStatus,
    #[proto(tag = 2)]
    pub envelope: Envelope<GoonPong>,
}

#[proto_message(proto_path = "protos/build_system_test/extra_types.proto")]
#[derive(Debug)]
pub struct StdMutexHolder {
    pub stdd: Mutex<MEx>,
}

#[proto_message(proto_path = "protos/build_system_test/extra_types.proto")]
#[derive(Debug)]
pub struct LotMutexHolder {
    pub stdd: parking_lot::Mutex<MEx>,
}
#[proto_message(proto_path = "protos/build_system_test/extra_types.proto")]
#[derive(Debug)]
struct Order {
    id: u32,
}

#[proto_message(proto_path = "protos/build_system_test/extra_types.proto")]
#[derive(Debug)]
struct OrderBytes {
    id: Vec<u8>,
}

#[proto_message(proto_path = "protos/build_system_test/extra_types.proto")]
#[derive(Debug)]
struct Orders {
    orders: Vec<Order>,
}

// Define trait with the proto_rpc macro
#[proto_rpc(
    rpc_package = "sigma_rpc",
    rpc_server = true,
    rpc_client = true,
    proto_path = "protos/build_system_test/sigma_rpc_simple.proto"
)]
//unnecessary for build system, this imports would be auto-resolved
// #[proto_imports(
//     rizz_types = ["BarSub", "FooResponse"],
//     goon_types = ["RizzPing", "GoonPong", "ServiceStatus", "Id"],
//     extra_types = ["EnvelopeBuildRequest", "EnvelopeBuildResponse", "BuildConfig", "BuildRequest", "BuildResponse"]
// )]
pub trait SigmaRpc {
    type RizzUniStream: Stream<Item = Result<FooResponse, Status>> + Send;
    async fn rizz_ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;

    async fn rizz_uni(&self, request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status>;

    async fn rizz_uni2(&self, request: BarSub) -> Self::RizzUniStream;

    async fn build(&self, request: Request<Envelope<BuildRequest>>) -> Result<Response<Envelope<BuildResponse>>, Status>;

    async fn build2(&self, request: Envelope<BuildRequest>) -> Envelope<BuildResponse>;

    async fn owner_lookup(&self, request: Request<TransparentId>) -> Result<Response<BuildResponse>, Status>;

    async fn custom_ex_echo(&self, request: Request<CustomEx>) -> Result<Response<CustomEx>, Status>;

    async fn mutex_echo(&self, request: Request<StdMutexHolder>) -> Result<Response<StdMutexHolder>, Status>;

    async fn parking_log_mutex_echo(&self, request: Request<LotMutexHolder>) -> Result<Response<LotMutexHolder>, Status>;

    async fn arc_echo(&self, request: Request<Arc<MEx>>) -> Result<Response<Arc<MEx>>, Status>;

    async fn box_echo(&self, request: Request<Box<MEx>>) -> Result<Response<Box<MEx>>, Status>;

    async fn option_echo(&self, request: Request<Option<MEx>>) -> Result<Response<Option<MEx>>, Status>;

    async fn vec_echo(&self, request: Request<Vec<MEx>>) -> Result<Response<Vec<MEx>>, Status>;

    async fn vec_deque_echo(&self, request: Request<VecDeque<MEx>>) -> Result<Response<VecDeque<MEx>>, Status>;

    async fn hash_map_echo(&self, request: Request<HashMap<u32, MEx>>) -> Result<Response<HashMap<u32, MEx>>, Status>;

    async fn btree_map_echo(&self, request: Request<BTreeMap<u32, MEx>>) -> Result<Response<BTreeMap<u32, MEx>>, Status>;

    async fn hash_set_echo(&self, request: Request<HashSet<MEx>>) -> Result<Response<HashSet<MEx>>, Status>;

    async fn btree_set_echo(&self, request: Request<BTreeSet<MEx>>) -> Result<Response<BTreeSet<MEx>>, Status>;

    async fn papaya_hash_map_echo(
        &self,
        request: Request<papaya::HashMap<u32, MEx>>,
    ) -> Result<Response<papaya::HashMap<u32, MEx>>, Status>;

    async fn papaya_hash_set_echo(&self, request: Request<papaya::HashSet<MEx>>) -> Result<Response<papaya::HashSet<MEx>>, Status>;

    async fn mex_echo(&self, request: CustomEx) -> MEx;

    async fn test_decimals(&self, request: Request<fastnum::UD128>) -> Result<Response<fastnum::D64>, Status>;
}

use proto_rs::schemas::ProtoSchema;
fn main() {
    let rust_client_path = "src/client.rs";
    let sigma_entries: HashSet<ProtoSchema> = inventory::iter::<ProtoSchema>()
        .filter(|schema| schema.id.name == "SigmaRpc" && schema.id.proto_type != proto_rs::schemas::ProtoType::None)
        .cloned()
        .collect();
    //println!("{:?}", sigma_entries);
    assert_eq!(sigma_entries.len(), 1);
    let sigma_schema = sigma_entries.into_iter().next().unwrap();
    let sigma_ident = sigma_schema.id;
    let rust_ctx = proto_rs::schemas::RustClientCtx::enabled(rust_client_path)
        .with_imports(&[
            "fastnum::UD128",
            "solana_address::Address",
            "solana_keypair::Keypair",
            "solana_signature::Signature",
            "chrono::DateTime",
            "chrono::TimeDelta",
            "chrono::Utc",
        ])
        .type_attribute("goon_types".to_string(), "#[derive(Clone, Debug)]".to_string())
        .type_attribute("goon_types".to_string(), "#[derive(Clone, PartialEq)]".to_string())
        .type_attribute("custom_types".to_string(), "#[derive(Clone, Debug)]".to_string())
        .type_attribute("custom_types".to_string(), "#[derive(Clone, PartialEq)]".to_string())
        //(mod name, statement) format
        .with_statements(&[("extra_types", "const MY_CONST: usize = 1337")])
        .type_attribute("extra_types".to_string(), "#[derive(Clone, Debug)]".to_string())
        .type_attribute("extra_types".to_string(), "#[derive(Clone, PartialEq)]".to_string())
        .remove_type_attribute(
            ClientAttrTarget::Ident(BuildRequest::PROTO_IDENT),
            UserAttr {
                level: AttrLevel::Top,
                attr: "#[derive(Clone)]".to_string(),
            },
        )
        .add_client_attrs(
            ClientAttrTarget::Module("extra_types"),
            UserAttr {
                level: AttrLevel::Top,
                attr: "#[allow(clippy::upper_case_acronyms)]".to_string(),
            },
        )
        .add_client_attrs(
            ClientAttrTarget::Ident(BuildRequest::PROTO_IDENT),
            UserAttr {
                level: AttrLevel::Top,
                attr: "#[allow(dead_code)]".to_string(),
            },
        )
        .add_client_attrs(
            ClientAttrTarget::Ident(BuildResponse::PROTO_IDENT),
            UserAttr {
                level: AttrLevel::Field {
                    field_name: "status".to_string(),
                    id: ServiceStatus::PROTO_IDENT,
                    variant: None,
                },
                attr: "#[allow(dead_code)]".to_string(),
            },
        )
        .add_client_attrs(
            ClientAttrTarget::Ident(sigma_ident),
            UserAttr {
                level: AttrLevel::Method {
                    method_name: "Build".to_string(),
                },
                attr: "#[allow(dead_code)]".to_string(),
            },
        )
        .add_client_attrs(
            ClientAttrTarget::Ident(sigma_ident),
            UserAttr {
                level: AttrLevel::Method {
                    method_name: "Build".to_string(),
                },
                attr: "#[allow(dead_code)]".to_string(),
            },
        )
        .add_client_attrs(
            ClientAttrTarget::Ident(sigma_ident),
            UserAttr {
                level: AttrLevel::Top,
                attr: "#[allow(dead_code)]".to_string(),
            },
        )
        .replace_type(&[
            TypeReplace::Type {
                id: BuildResponse::PROTO_IDENT,
                variant: None,
                field: "status".to_string(),
                type_name: "::core::primitive::u32".to_string(),
            },
            TypeReplace::Trait {
                id: sigma_ident,
                method: "OwnerLookup".to_string(),
                kind: MethodReplace::Argument("::core::primitive::u64".to_string()),
                type_name: "::core::primitive::u64".to_string(),
            },
            TypeReplace::Trait {
                id: sigma_ident,
                method: "Build".to_string(),
                kind: MethodReplace::Return("::core::primitive::u32".to_string()),
                type_name: "::core::primitive::u32".to_string(),
            },
        ])
        .split_module("atomic_types", "src/client_atomic_types.rs");
    proto_rs::schemas::write_all("build_protos", &rust_ctx).expect("Failed to write proto files");

    for schema in inventory::iter::<ProtoSchema> {
        println!("Collected: {}", schema.id.name);
    }

    // Verify proto output for bytes collection fields (Vec<u8>, CustomVec<u8>, etc.)
    let custom_proto =
        std::fs::read_to_string("build_protos/protos/build_system_test/custom_types.proto").expect("Failed to read custom_types.proto");
    assert!(
        custom_proto.contains("bytes custom_vec_bytes"),
        "CustomVec<u8> field should be proto 'bytes', not a wrapper message. Got:\n{custom_proto}"
    );
    assert!(
        custom_proto.contains("bytes custom_vec_deque_bytes"),
        "CustomVecDeq<u8> field should be proto 'bytes', not a wrapper message. Got:\n{custom_proto}"
    );
    assert!(
        !custom_proto.contains("VecU32"),
        "There should be no VecU32 wrapper message for bytes fields. Got:\n{custom_proto}"
    );
    assert!(
        !custom_proto.contains("VecDequeU32"),
        "There should be no VecDequeU32 wrapper message for bytes fields. Got:\n{custom_proto}"
    );

    let client_contents = std::fs::read_to_string(rust_client_path).expect("Failed to read rust client output");
    assert!(client_contents.contains("use fastnum::UD128;"));
    assert!(!client_contents.contains("pub struct UD128"));
    assert!(client_contents.contains("pub mod"));
    assert!(client_contents.contains("pub trait"));
    assert!(client_contents.contains("#[allow(dead_code)]"));
    assert!(client_contents.contains("const MY_CONST: usize = 1337;"));
    assert!(client_contents.contains("#[allow(clippy::upper_case_acronyms)]"));
    assert!(
        client_contents.contains("#[derive(Clone, Debug, PartialEq)]"),
        "Module type attributes should merge derive entries"
    );
    assert!(
        client_contents.contains("#[derive(Debug, PartialEq)]\n    #[allow(dead_code)]\n    #[proto_message]\n    pub struct BuildRequest"),
        "Type-level attribute removals should drop derive entries for specific types"
    );
    assert!(
        !client_contents.contains("#[derive(Clone, Debug, PartialEq)]\npub struct BuildRequest"),
        "Type-level attribute removals should not keep removed derive traits"
    );
    assert!(
        !client_contents.contains("#[derive(Clone, Debug, Copy)]"),
        "Unexpected Copy derives should not be emitted"
    );
    assert!(client_contents.contains("status: ::core::primitive::u32"));
    assert!(client_contents.contains("request: ::tonic::Request<::core::primitive::u64>"));
    assert!(client_contents.contains("::tonic::Response<::core::primitive::u32>"));

    // Test case #1: Verify const generics don't have malformed output
    // The bug was: "const CAP: const CAP : usize.ty" instead of "const CAP: usize"
    assert!(
        !client_contents.contains("usize.ty"),
        "Should not have malformed const generic type (.ty suffix)"
    );
    assert!(
        !client_contents.contains(": const"),
        "Should not have malformed const generic (: const pattern)"
    );

    // Test case #2: Verify getter attributes are filtered from client output
    // The getter attribute is source-only and should never appear in generated clients
    assert!(
        !client_contents.contains("getter ="),
        "Getter attributes should not appear in generated client"
    );
    assert!(
        !client_contents.contains("getter="),
        "Getter attributes should not appear in generated client (no space)"
    );

    // Test case #3: Verify simple enum variants are in PascalCase (not SCREAMING_CASE)
    // The enum should have variants like "Active", "Pending" not "ACTIVE", "PENDING"
    assert!(client_contents.contains("Active,"), "Enum variant should be PascalCase: Active");
    assert!(client_contents.contains("Pending,"), "Enum variant should be PascalCase: Pending");
    assert!(client_contents.contains("Inactive,"), "Enum variant should be PascalCase: Inactive");
    assert!(
        client_contents.contains("Completed,"),
        "Enum variant should be PascalCase: Completed"
    );
    assert!(!client_contents.contains("ACTIVE"), "Enum variant should not be SCREAMING_CASE");
    assert!(!client_contents.contains("PENDING"), "Enum variant should not be SCREAMING_CASE");

    assert!(
        client_contents.contains("DateTime<Utc>"),
        "Generic type args should be preserved: DateTime<Utc>"
    );
    assert!(
        client_contents.contains("use chrono::DateTime;"),
        "chrono::DateTime should be imported"
    );
    assert!(client_contents.contains("use chrono::Utc;"), "chrono::Utc should be imported");

    assert!(
        !client_contents.contains("MEx<MEx>"),
        "Custom wrapper types should not produce erroneous generics like MEx<MEx>"
    );
    assert!(
        !client_contents.contains("u64<u64>"),
        "Custom wrapper types with copy types should not produce erroneous generics like u64<u64>"
    );
    assert!(
        !client_contents.contains("u32<u32>"),
        "Custom wrapper types with copy types should not produce erroneous generics like u32<u32>"
    );
    assert!(
        !client_contents.contains("u8<u8>"),
        "Custom wrapper types with copy types should not produce erroneous generics like u8<u8>"
    );

    // Verify the CustomEx struct fields are correctly generated
    // These should be simple types without erroneous generic duplication
    assert!(
        client_contents.contains("pub mutex: MEx,"),
        "mutex field should be MEx, not MEx<MEx>"
    );
    assert!(
        client_contents.contains("pub mutex_copy: u64,"),
        "mutex_copy field should be u64, not u64<u64>"
    );
    assert!(
        client_contents.contains("pub mutex_custom: MEx,"),
        "mutex_custom field should be MEx, not MEx<MEx>"
    );
    assert!(
        client_contents.contains("pub mutex_copy_custom: u64,"),
        "mutex_copy_custom field should be u64, not u64<u64>"
    );
    assert!(client_contents.contains("pub arc: MEx,"), "arc field should be MEx, not MEx<MEx>");
    assert!(
        client_contents.contains("pub arc_copy: u64,"),
        "arc_copy field should be u64, not u64<u64>"
    );
    assert!(
        client_contents.contains("pub arc_custom: MEx,"),
        "arc_custom field should be MEx, not MEx<MEx>"
    );
    assert!(
        client_contents.contains("pub arc_copy_custom: u64,"),
        "arc_copy_custom field should be u64, not u64<u64>"
    );
    assert!(
        client_contents.contains("pub boxed: MEx,"),
        "boxed field should be MEx, not MEx<MEx>"
    );
    assert!(
        client_contents.contains("pub box_copy: u64,"),
        "box_copy field should be u64, not u64<u64>"
    );
    assert!(
        client_contents.contains("pub boxed_custom: MEx,"),
        "boxed_custom field should be MEx, not MEx<MEx>"
    );
    assert!(
        client_contents.contains("pub box_copy_custom: u64,"),
        "box_copy_custom field should be u64, not u64<u64>"
    );
    assert!(
        client_contents.contains("pub custom_option: ::core::option::Option<MEx>,"),
        "custom_option field should be Option<MEx>, not Option<MEx<MEx>>"
    );
    assert!(
        client_contents.contains("pub custom_option_copy: ::core::option::Option<u64>,"),
        "custom_option_copy field should be Option<u64>, not Option<u64<u64>>"
    );
    assert!(
        client_contents.contains("pub custom_vec: ::proto_rs::alloc::vec::Vec<MEx>,"),
        "custom_vec field should be Vec<MEx>, not Vec<MEx<MEx>>"
    );
    assert!(
        client_contents.contains("pub custom_vec_deque: ::proto_rs::alloc::vec::Vec<MEx>,"),
        "custom_vec_deque field should be Vec<MEx>, not Vec<MEx<MEx>>"
    );
    assert!(
        client_contents.contains("pub custom_vec_copy: ::proto_rs::alloc::vec::Vec<u64>,"),
        "custom_vec_copy field should be Vec<u64>, not Vec<u64<u64>>"
    );
    assert!(
        client_contents.contains("pub custom_vec_deque_copy: ::proto_rs::alloc::vec::Vec<u64>,"),
        "custom_vec_deque_copy field should be Vec<u64>, not Vec<u64<u64>>"
    );
    assert!(
        client_contents.contains("pub custom_vec_bytes: ::proto_rs::alloc::vec::Vec<u8>,"),
        "custom_vec_bytes field should be Vec<u8>, not Vec<u32<u32>>"
    );
    assert!(
        client_contents.contains("pub custom_vec_deque_bytes: ::proto_rs::alloc::vec::Vec<u8>,"),
        "custom_vec_deque_bytes field should be Vec<u8>, not Vec<u32<u32>>"
    );

    // Verify atomic_types module was split into its own file
    assert!(
        !client_contents.contains("pub mod atomic_types"),
        "atomic_types should not be in main client (split module)"
    );
    let split_contents =
        std::fs::read_to_string("src/client_atomic_types.rs").expect("Failed to read split module output");
    assert!(
        split_contents.contains("pub mod atomic_types"),
        "atomic_types should be in split file"
    );
    assert!(
        split_contents.contains("pub struct AtomicPrimitives"),
        "AtomicPrimitives should be in split file"
    );
    assert!(split_contents.contains("pub flag: bool,"), "AtomicBool should render as bool");
    assert!(split_contents.contains("pub count: u64,"), "AtomicU64 should render as u64");
    assert!(split_contents.contains("pub small: u8,"), "AtomicU8 should render as u8");
    assert!(split_contents.contains("pub smaller: u16,"), "AtomicU16 should render as u16");
    assert!(split_contents.contains("pub signed_small: i8,"), "AtomicI8 should render as i8");
    assert!(
        split_contents.contains("pub signed_smaller: i16,"),
        "AtomicI16 should render as i16"
    );
    assert!(split_contents.contains("pub signed: i32,"), "AtomicI32 should render as i32");
    assert!(split_contents.contains("pub sized: u64,"), "AtomicUsize should render as u64");
    assert!(
        split_contents.contains("pub signed_sized: i64,"),
        "AtomicIsize should render as i64"
    );
    assert!(
        !split_contents.contains("AtomicU"),
        "Atomic primitives should not appear in rust client output"
    );

    // Verify NonZero types render as NonZero, not ordinary primitives
    assert!(
        split_contents.contains("pub struct NonZeroPrimitives"),
        "NonZeroPrimitives should be in split file"
    );
    assert!(
        split_contents.contains("pub nz_u8: ::core::num::NonZeroU8,"),
        "NonZeroU8 should render as ::core::num::NonZeroU8"
    );
    assert!(
        split_contents.contains("pub nz_u16: ::core::num::NonZeroU16,"),
        "NonZeroU16 should render as ::core::num::NonZeroU16"
    );
    assert!(
        split_contents.contains("pub nz_u32: ::core::num::NonZeroU32,"),
        "NonZeroU32 should render as ::core::num::NonZeroU32"
    );
    assert!(
        split_contents.contains("pub nz_u64: ::core::num::NonZeroU64,"),
        "NonZeroU64 should render as ::core::num::NonZeroU64"
    );
    assert!(
        split_contents.contains("pub nz_usize: ::core::num::NonZeroU64,"),
        "NonZeroUsize should render as ::core::num::NonZeroU64"
    );
    assert!(
        split_contents.contains("pub nz_i8: ::core::num::NonZeroI8,"),
        "NonZeroI8 should render as ::core::num::NonZeroI8"
    );
    assert!(
        split_contents.contains("pub nz_i16: ::core::num::NonZeroI16,"),
        "NonZeroI16 should render as ::core::num::NonZeroI16"
    );
    assert!(
        split_contents.contains("pub nz_i32: ::core::num::NonZeroI32,"),
        "NonZeroI32 should render as ::core::num::NonZeroI32"
    );
    assert!(
        split_contents.contains("pub nz_i64: ::core::num::NonZeroI64,"),
        "NonZeroI64 should render as ::core::num::NonZeroI64"
    );
    assert!(
        split_contents.contains("pub nz_isize: ::core::num::NonZeroI64,"),
        "NonZeroIsize should render as ::core::num::NonZeroI64"
    );

    // Verify macro imports are tracked correctly (no #[allow(unused_imports)])
    assert!(
        !client_contents.contains("#[allow(unused_imports)]"),
        "Should not have #[allow(unused_imports)] - macro imports should be precise"
    );
    assert!(
        !split_contents.contains("#[allow(unused_imports)]"),
        "Split file should not have #[allow(unused_imports)]"
    );
    assert!(
        split_contents.contains("use proto_rs::proto_message;"),
        "Split file with only structs should import only proto_message"
    );
    assert!(
        !split_contents.contains("proto_rpc"),
        "Split file with only structs should not import proto_rpc"
    );
    // sigma_rpc module has only a service trait, so it should import only proto_rpc
    assert!(
        client_contents.contains("use proto_rs::proto_rpc;\n"),
        "Service-only module should import only proto_rpc"
    );

    // ===== Test only_these_modules mode =====
    let only_modules_ctx = proto_rs::schemas::RustClientCtx::only_these_modules(&[
            ("goon_types", "src/only_goon_types.rs"),
            ("atomic_types", "src/only_atomic_types.rs"),
        ])
        .with_imports(&[
            "fastnum::UD128",
            "chrono::DateTime",
            "chrono::TimeDelta",
            "chrono::Utc",
        ]);
    proto_rs::schemas::write_all("build_protos_only", &only_modules_ctx).expect("Failed to write only_these_modules proto files");

    // goon_types should be in its own file
    let only_goon = std::fs::read_to_string("src/only_goon_types.rs").expect("Failed to read only_goon_types.rs");
    assert!(
        only_goon.contains("pub mod goon_types"),
        "only_these_modules: goon_types should be in its split file"
    );
    assert!(
        only_goon.contains("pub struct RizzPing"),
        "only_these_modules: RizzPing should be in goon_types file"
    );
    assert!(
        only_goon.contains("pub struct GoonPong"),
        "only_these_modules: GoonPong should be in goon_types file"
    );

    // atomic_types should be in its own file
    let only_atomic = std::fs::read_to_string("src/only_atomic_types.rs").expect("Failed to read only_atomic_types.rs");
    assert!(
        only_atomic.contains("pub mod atomic_types"),
        "only_these_modules: atomic_types should be in its split file"
    );
    assert!(
        only_atomic.contains("pub struct AtomicPrimitives"),
        "only_these_modules: AtomicPrimitives should be in atomic_types file"
    );

    // ===== Test only_these_modules with split_module =====
    let split_combo_ctx = proto_rs::schemas::RustClientCtx::only_these_modules(&[
            ("goon_types", "src/combo_goon.rs"),
            ("atomic_types", "src/combo_atomic.rs"),
        ])
        .with_imports(&[
            "fastnum::UD128",
            "chrono::DateTime",
            "chrono::TimeDelta",
            "chrono::Utc",
        ])
        // split_module overrides the path for atomic_types
        .split_module("atomic_types", "src/combo_atomic_split.rs");
    proto_rs::schemas::write_all("build_protos_combo", &split_combo_ctx).expect("Failed to write combo proto files");

    // atomic_types should use the split_module path (overrides only_these_modules path)
    let combo_atomic = std::fs::read_to_string("src/combo_atomic_split.rs").expect("Failed to read combo_atomic_split.rs");
    assert!(
        combo_atomic.contains("pub mod atomic_types"),
        "split_module should override only_these_modules path for atomic_types"
    );
    assert!(
        combo_atomic.contains("pub struct AtomicPrimitives"),
        "AtomicPrimitives should be in split_module file"
    );

    // goon_types should still be at its only_these_modules path
    let combo_goon = std::fs::read_to_string("src/combo_goon.rs").expect("Failed to read combo_goon.rs");
    assert!(
        combo_goon.contains("pub mod goon_types"),
        "goon_types should be at its only_these_modules path"
    );

    // ===== Test write_only_these =====
    let wot_ctx = proto_rs::schemas::RustClientCtx::disabled();
    let wot_count = proto_rs::schemas::write_only_these(
        &[
            ("protos/build_system_test/goon_types.proto", "build_protos_wot/goon_types.proto"),
            ("protos/build_system_test/atomic_types.proto", "build_protos_wot/atomic_types.proto"),
        ],
        &wot_ctx,
    ).expect("Failed to write_only_these");
    assert_eq!(wot_count, 2, "write_only_these should write exactly 2 proto files");

    // goon_types.proto should exist and contain expected content
    let wot_goon = std::fs::read_to_string("build_protos_wot/goon_types.proto").expect("Failed to read wot goon_types.proto");
    assert!(wot_goon.contains("RizzPing"), "write_only_these: goon_types.proto should contain RizzPing");
    assert!(wot_goon.contains("GoonPong"), "write_only_these: goon_types.proto should contain GoonPong");

    // atomic_types.proto should exist and contain expected content
    let wot_atomic = std::fs::read_to_string("build_protos_wot/atomic_types.proto").expect("Failed to read wot atomic_types.proto");
    assert!(wot_atomic.contains("AtomicPrimitives"), "write_only_these: atomic_types.proto should contain AtomicPrimitives");

    // Other proto files should NOT exist
    assert!(
        !std::path::Path::new("build_protos_wot/custom_types.proto").exists(),
        "write_only_these: custom_types.proto should not be written"
    );
    assert!(
        !std::path::Path::new("build_protos_wot/extra_types.proto").exists(),
        "write_only_these: extra_types.proto should not be written"
    );

    // ===== Test only_these_modules with multiple modules to same file =====
    let same_file_ctx = proto_rs::schemas::RustClientCtx::only_these_modules(&[
            ("goon_types", "src/multi_mod_same_file.rs"),
            ("atomic_types", "src/multi_mod_same_file.rs"),
        ])
        .with_imports(&[
            "fastnum::UD128",
            "chrono::DateTime",
            "chrono::TimeDelta",
            "chrono::Utc",
        ]);
    proto_rs::schemas::write_all("build_protos_same_file", &same_file_ctx).expect("Failed to write same-file proto files");

    let same_file = std::fs::read_to_string("src/multi_mod_same_file.rs").expect("Failed to read multi_mod_same_file.rs");
    assert!(
        same_file.contains("pub mod goon_types"),
        "same file: should contain goon_types module"
    );
    assert!(
        same_file.contains("pub mod atomic_types"),
        "same file: should contain atomic_types module"
    );
    assert!(
        same_file.contains("pub struct RizzPing"),
        "same file: should contain RizzPing from goon_types"
    );
    assert!(
        same_file.contains("pub struct AtomicPrimitives"),
        "same file: should contain AtomicPrimitives from atomic_types"
    );
}
