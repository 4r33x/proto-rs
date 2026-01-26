use std::sync::Arc;

pub use server_client::ServerClient as BlockRazorClient;
use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::metadata::MetadataValue;

use crate::proto_message;
use crate::proto_rpc;

#[proto_message(proto_path = "protos/block_razor_rpc.proto")]
pub struct SendRequest {
    pub transaction: String,
    pub mode: String,
    pub safe_window: Option<i32>,
    pub revert_protection: bool,
}

#[proto_message(proto_path = "protos/block_razor_rpc.proto")]
pub struct SendResponse {
    pub signature: String,
}

#[proto_message(proto_path = "protos/block_razor_rpc.proto")]
pub struct HealthRequest;

#[proto_message(proto_path = "protos/block_razor_rpc.proto")]
#[derive(Debug)]
pub struct HealthResponse {
    pub status: String,
}

#[proto_rpc(
    rpc_package = "serverpb",
    rpc_server = false,
    rpc_client = true,
    proto_path = "protos/block_razor_rpc.proto"
)]
pub trait Server {
    async fn send_transaction(&self, request: Request<SendRequest>) -> Result<Response<SendResponse>, Status>;

    async fn get_health(&self, request: Request<HealthRequest>) -> Result<Response<HealthResponse>, Status>;
}

#[derive(Clone)]
pub struct BlockRazorAuthInterceptor {
    api_key: Arc<MetadataValue<tonic::metadata::Ascii>>,
}

impl BlockRazorAuthInterceptor {
    pub fn new(api_key: String) -> Result<Self, tonic::metadata::errors::InvalidMetadataValue> {
        let api_key = Arc::new(MetadataValue::try_from(api_key)?);
        Ok(Self { api_key })
    }
}

impl tonic::service::Interceptor for BlockRazorAuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request.metadata_mut().insert("apikey", (*self.api_key).clone());
        Ok(request)
    }
}

#[cfg(test)]
mod tests {

    use tonic::transport::Channel;

    use super::BlockRazorAuthInterceptor;
    use super::BlockRazorClient;
    use super::HealthRequest;

    #[tokio::test]
    async fn test_ping() {
        let c = Channel::from_shared("http://frankfurt.solana-grpc.blockrazor.xyz:80".to_owned()).unwrap().connect().await.unwrap();
        let i = BlockRazorAuthInterceptor::new("key".to_owned()).unwrap();
        let mut client = BlockRazorClient::with_interceptor(c, i);
        let res = client.get_health(HealthRequest {}).await.expect_err("No auth key");
        println!("{res:?}");
    }
}
