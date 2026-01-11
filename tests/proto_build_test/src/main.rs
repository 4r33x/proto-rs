#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use std::collections::HashSet;

use proto_rs::ZeroCopyResponse;
use proto_rs::proto_message;
use proto_rs::proto_rpc;
use proto_rs::schemas::AttrLevel;
use proto_rs::schemas::ClientAttrTarget;
use proto_rs::schemas::ProtoIdentifiable;
use proto_rs::schemas::UserAttr;
use tokio_stream::Stream;
use tonic::Response;
use tonic::Status;

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
#[proto(generic_types = [T = [BuildRequest, BuildResponse, GoonPong]])]
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

// Define trait with the proto_rpc macro
#[proto_rpc(
    rpc_package = "sigma_rpc",
    rpc_server = true,
    rpc_client = true,
    proto_path = "protos/build_system_test/sigma_rpc_simple.proto"
)]
//unnecessary for build system, this imports would be auto-resolved
#[proto_imports(
    rizz_types = ["BarSub", "FooResponse"],
    goon_types = ["RizzPing", "GoonPong", "ServiceStatus", "Id"],
    extra_types = ["EnvelopeBuildRequest", "EnvelopeBuildResponse", "BuildConfig", "BuildRequest", "BuildResponse"]
)]
pub trait SigmaRpc {
    type RizzUniStream: Stream<Item = Result<ZeroCopyResponse<FooResponse>, Status>> + Send;
    async fn rizz_ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;

    async fn rizz_uni(&self, request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status>;

    // async fn rizz_uni2(&self, request: BarSub) -> Self::RizzUniStream;

    async fn build(&self, request: Request<Envelope<BuildRequest>>) -> Result<Response<Envelope<BuildResponse>>, Status>;

    // async fn build2(&self, request: Envelope<BuildRequest>) -> Envelope<BuildResponse>;

    async fn owner_lookup(&self, request: Request<TransparentId>) -> Result<Response<BuildResponse>, Status>;

    async fn test_decimals(&self, request: Request<fastnum::UD128>) -> Result<Response<fastnum::D64>, Status>;
}

use proto_rs::schemas::ProtoSchema;
fn main() {
    let rust_client_path = "src/client.rs";
    let sigma_entries: HashSet<ProtoSchema> = inventory::iter::<ProtoSchema>()
        .filter(|schema| schema.id.name == "SigmaRpc" && schema.id.proto_type != "Import")
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
                    r#type: ServiceStatus::PROTO_IDENT,
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
        );
    proto_rs::schemas::write_all("build_protos", &rust_ctx).expect("Failed to write proto files");

    let client_contents = std::fs::read_to_string(rust_client_path).expect("Failed to read rust client output");
    assert!(client_contents.contains("use fastnum::UD128;"));
    assert!(!client_contents.contains("pub struct UD128"));
    assert!(client_contents.contains("pub mod"));
    assert!(client_contents.contains("pub trait"));
    assert!(client_contents.contains("#[allow(dead_code)]"));
    assert!(client_contents.contains("const MY_CONST: usize = 1337;"));
    assert!(client_contents.contains("#[allow(clippy::upper_case_acronyms)]"));

    for schema in inventory::iter::<ProtoSchema> {
        println!("Collected: {}", schema.id.name);
    }
}
