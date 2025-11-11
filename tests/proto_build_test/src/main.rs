#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use proto_rs::ZeroCopy;
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

// Define trait with the proto_rpc macro
#[proto_rpc(rpc_package = "sigma_rpc", rpc_server = true, rpc_client = true, proto_path = "protos/gen_complex_proto/sigma_rpc_simple.proto")]
#[proto_imports(rizz_types = ["BarSub", "FooResponse"], goon_types = ["RizzPing", "GoonPong", "ServiceStatus", "Id"] )]
pub trait SigmaRpc {
    type RizzUniStream: Stream<Item = Result<ZeroCopy<FooResponse>, Status>> + Send;
    async fn rizz_ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;

    async fn rizz_uni(&self, request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status>;
}

use proto_rs::schemas::ProtoSchema;
fn main() {
    proto_rs::schemas::write_all("build_protos").expect("Failed to write proto files");

    for schema in inventory::iter::<ProtoSchema> {
        println!("Collected: {}", schema.name);
    }
}
