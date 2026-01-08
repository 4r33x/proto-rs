#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use proto_rs::ZeroCopyResponse;
use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tokio_stream::Stream;
use tonic::Response;
use tonic::Status;

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[derive(Debug, Default, Clone, PartialEq, Copy)]
pub enum ServiceStatus {
    Pending,
    #[default]
    Active,
    Inactive,
    Completed,
}

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Id {
    pub id: u64,
}

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct RizzPing {
    id: Id,
    status: ServiceStatus,
}

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct GoonPong {
    id: Id,
    status: ServiceStatus,
}

#[proto_message(proto_path = "protos/gen_complex_proto/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct FooResponse;

#[proto_message(proto_path = "protos/gen_complex_proto/rizz_types.proto")]
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

#[proto_message(proto_path = "protos/gen_complex_proto/extra_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct Envelope<T> {
    #[proto(tag = 1)]
    pub payload: T,
    #[proto(tag = 2)]
    pub trace_id: String,
}

#[proto_message(proto_path = "protos/gen_complex_proto/extra_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct BuildConfig {
    #[proto(tag = 1, into = "i64")]
    pub timeout: MilliSeconds,
    #[proto(tag = 2, skip)]
    pub cache_hint: String,
    #[proto(tag = 3)]
    pub owner: TransparentId,
}

#[proto_message(proto_path = "protos/gen_complex_proto/extra_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct BuildRequest {
    #[proto(tag = 1)]
    pub config: BuildConfig,
    #[proto(tag = 2)]
    pub ping: RizzPing,
    #[proto(tag = 3)]
    pub owner: TransparentId,
}

#[proto_message(proto_path = "protos/gen_complex_proto/extra_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct BuildResponse {
    #[proto(tag = 1)]
    pub status: ServiceStatus,
    #[proto(tag = 2)]
    pub envelope: Envelope<GoonPong>,
}

// Define trait with the proto_rpc macro
#[proto_rpc(rpc_package = "sigma_rpc", rpc_server = true, rpc_client = true, proto_path = "protos/gen_complex_proto/sigma_rpc_simple.proto")]
#[proto_imports(
    rizz_types = ["BarSub", "FooResponse"],
    goon_types = ["RizzPing", "GoonPong", "ServiceStatus", "Id"],
    extra_types = ["Envelope", "BuildConfig", "BuildRequest", "BuildResponse"]
)]
pub trait SigmaRpc {
    type RizzUniStream: Stream<Item = Result<ZeroCopyResponse<FooResponse>, Status>> + Send;
    async fn rizz_ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;

    async fn rizz_uni(&self, request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status>;

    async fn build(&self, request: Request<Envelope<BuildRequest>>) -> Result<Response<Envelope<BuildResponse>>, Status>;

    async fn owner_lookup(&self, request: Request<TransparentId>) -> Result<Response<BuildResponse>, Status>;
}

use proto_rs::schemas::ProtoSchema;
fn main() {
    proto_rs::schemas::write_all("build_protos").expect("Failed to write proto files");

    for schema in inventory::iter::<ProtoSchema> {
        println!("Collected: {}", schema.id.name);
    }
}
