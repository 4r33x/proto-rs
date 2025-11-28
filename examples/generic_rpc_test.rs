use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tonic::{Request, Response, Status};

#[proto_message]
pub struct RizzPing<K, V> {
    #[proto(tag = 1)]
    pub key: K,
    #[proto(tag = 2)]
    pub value: V,
}

#[proto_message]
pub struct GoonPong<K, V> {
    #[proto(tag = 1)]
    pub result: K,
    #[proto(tag = 2)]
    pub data: V,
}

#[proto_rpc(
    rpc_package = "sigma_rpc",
    rpc_server = true,
    rpc_client = true,
    proto_generic_types = [K = [u64, u32], V = [String, u16]]
)]
pub trait SigmaRpc {
    async fn rizz_ping_generic(&self, request: Request<RizzPing<K, V>>) -> Result<Response<GoonPong<K, V>>, Status>;
}

fn main() {
    println!("Generic RPC example");
}
