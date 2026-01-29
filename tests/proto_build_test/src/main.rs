#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;

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

// Test case for const generics (Issue #1)
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
struct StdMutexHolder {
    pub stdd: Mutex<MEx>,
}

#[proto_message(proto_path = "protos/build_system_test/extra_types.proto")]
#[derive(Debug)]
struct LotMutexHolder {
    pub stdd: parking_lot::Mutex<MEx>,
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
    println!("{:?}", sigma_entries);
    assert_eq!(sigma_entries.len(), 1);
    let sigma_schema = sigma_entries.into_iter().next().unwrap();
    let sigma_ident = sigma_schema.id;
    let rust_ctx = proto_rs::schemas::RustClientCtx::enabled(rust_client_path)
        .with_imports(&[
            "fastnum::UD128",
            "solana_address::Address",
            "solana_keypair::Keypair",
            "solana_signature::Signature",
        ])
        //(mod name, statement) format
        .with_statements(&[("extra_types", "const MY_CONST: usize = 1337")])
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
        ]);
    proto_rs::schemas::write_all("build_protos", &rust_ctx).expect("Failed to write proto files");

    let client_contents = std::fs::read_to_string(rust_client_path).expect("Failed to read rust client output");
    assert!(client_contents.contains("use fastnum::UD128;"));
    assert!(!client_contents.contains("pub struct UD128"));
    assert!(client_contents.contains("pub mod"));
    assert!(client_contents.contains("pub trait"));
    assert!(client_contents.contains("#[allow(dead_code)]"));
    assert!(client_contents.contains("const MY_CONST: usize = 1337;"));
    assert!(client_contents.contains("#[allow(clippy::upper_case_acronyms)]"));
    assert!(client_contents.contains("status: ::core::primitive::u32"));
    assert!(client_contents.contains("request: ::tonic::Request<::core::primitive::u64>"));
    assert!(client_contents.contains("::tonic::Response<::core::primitive::u32>"));

    // Test case #1: Verify const generics don't have malformed output
    // The bug was: "const CAP: const CAP : usize.ty" instead of "const CAP: usize"
    assert!(!client_contents.contains("usize.ty"), "Should not have malformed const generic type (.ty suffix)");
    assert!(!client_contents.contains(": const"), "Should not have malformed const generic (: const pattern)");

    // Test case #2: Verify getter attributes are filtered from client output
    // The getter attribute is source-only and should never appear in generated clients
    assert!(!client_contents.contains("getter ="), "Getter attributes should not appear in generated client");
    assert!(!client_contents.contains("getter="), "Getter attributes should not appear in generated client (no space)");

    for schema in inventory::iter::<ProtoSchema> {
        println!("Collected: {}", schema.id.name);
    }
}
