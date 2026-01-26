use std::sync::Arc;

pub use api_client::ApiClient as NextBLockClient;
use tokio_stream::Stream;
use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::metadata::MetadataValue;

use crate::proto_message;
use crate::proto_rpc;

#[proto_message(proto_path = "protos/next_block.proto")]
pub struct TipFloorRequest;

#[proto_message(proto_path = "protos/next_block.proto")]
pub struct TipFloorStreamRequest {
    #[proto(tag = 1)]
    pub update_frequency: String,
}

#[proto_message(proto_path = "protos/next_block.proto")]
#[derive(Debug)]
pub struct TipStats {
    #[proto(tag = 1)]
    pub time: String,
    #[proto(tag = 2)]
    pub landed_tips_25th_percentile: f64,
    #[proto(tag = 3)]
    pub landed_tips_50th_percentile: f64,
    #[proto(tag = 4)]
    pub landed_tips_75th_percentile: f64,
    #[proto(tag = 5)]
    pub landed_tips_95th_percentile: f64,
    #[proto(tag = 6)]
    pub landed_tips_99th_percentile: f64,
    #[proto(tag = 7)]
    pub ema_landed_tips_50th_percentile: f64,
}

#[proto_message(proto_path = "protos/next_block.proto")]
#[derive(Debug)]
pub struct TipFloorResponse {
    #[proto(tag = 1)]
    pub stats: Vec<TipStats>,
}

#[proto_message(proto_path = "protos/next_block.proto")]
pub struct PingRequest;

#[proto_message(proto_path = "protos/next_block.proto")]
#[derive(Debug)]
pub struct PongResponse;

#[proto_message(proto_path = "protos/next_block.proto")]
pub struct PostSubmitRequest {
    pub transaction: TransactionMessage,
    pub skip_pre_flight: bool,
    pub front_running_protection: Option<bool>,
    #[proto(tag = 8)]
    pub experimental_front_running_protection: Option<bool>,
    #[proto(tag = 9)]
    pub snipe_transaction: Option<bool>,
    #[proto(tag = 10)]
    pub disable_retries: Option<bool>,
    #[proto(tag = 11)]
    pub revert_on_fail: Option<bool>,
}

#[proto_message(proto_path = "protos/next_block.proto")]
pub struct PostSubmitRequestEntry {
    pub transaction: TransactionMessage,
}

#[proto_message(proto_path = "protos/next_block.proto")]
pub struct PostSubmitBatchRequest {
    pub entries: ::proto_rs::alloc::vec::Vec<PostSubmitRequestEntry>,
}

#[proto_message(proto_path = "protos/next_block.proto")]
pub struct PostSubmitResponse {
    pub signature: String,
}

#[proto_message(proto_path = "protos/next_block.proto")]
pub struct TransactionMessage {
    pub content: String,
    pub is_cleanup: bool,
}

#[proto_message(proto_path = "protos/next_block.proto")]
pub struct TransactionMessageV2 {
    pub content: String,
}

#[proto_rpc(rpc_package = "api", rpc_server = false, rpc_client = true, proto_path = "protos/next_block.proto")]
pub trait Api {
    type StreamTipFloorStream: Stream<Item = Result<TipFloorResponse, Status>>;

    async fn post_submit_v2(&self, request: Request<PostSubmitRequest>) -> Result<Response<PostSubmitResponse>, Status>;

    async fn post_submit_batch_v2(&self, request: Request<PostSubmitBatchRequest>) -> Result<Response<PostSubmitResponse>, Status>;

    async fn ping(&self, request: Request<PingRequest>) -> Result<Response<PongResponse>, Status>;

    async fn get_tip_floor(&self, request: Request<TipFloorRequest>) -> Result<Response<TipFloorResponse>, Status>;

    async fn stream_tip_floor(&self, request: Request<TipFloorStreamRequest>) -> Result<Response<Self::StreamTipFloorStream>, Status>;
}

#[derive(Clone)]
pub struct NextBlockAuthInterceptor {
    api_key: Arc<MetadataValue<tonic::metadata::Ascii>>,
}

impl NextBlockAuthInterceptor {
    pub fn new(api_key: String) -> Result<Self, tonic::metadata::errors::InvalidMetadataValue> {
        let api_key = Arc::new(MetadataValue::try_from(api_key)?);
        Ok(Self { api_key })
    }
}

impl tonic::service::Interceptor for NextBlockAuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request.metadata_mut().insert("authorization", (*self.api_key).clone());
        Ok(request)
    }
}

#[cfg(test)]
mod tests {

    use tonic::transport::Channel;

    use super::NextBLockClient;
    use super::NextBlockAuthInterceptor;
    use super::PingRequest;

    #[tokio::test]
    async fn test_ping() {
        let c = Channel::from_shared("http://frankfurt.nextblock.io".to_owned()).unwrap().connect().await.unwrap();
        let i = NextBlockAuthInterceptor::new("key".to_owned()).unwrap();
        let mut client = NextBLockClient::with_interceptor(c, i);
        let res = client.ping(PingRequest {}).await.expect_err("No auth key");
        println!("{res:?}");
    }
}
