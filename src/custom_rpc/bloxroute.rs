use std::sync::Arc;

pub use api_client::ApiClient as BloxrouteClient;
use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::metadata::MetadataValue;

use crate::custom_types::well_known::Timestamp;
use crate::proto_message;
use crate::proto_rpc;

#[proto_message(proto_path = "protos/bloxroute.proto")]
pub struct GetServerTimeRequest;

#[proto_message(proto_path = "protos/bloxroute.proto")]
#[derive(Debug)]
pub struct GetServerTimeResponse {
    pub timestamp: String,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
pub struct PostSubmitRequest {
    pub transaction: TransactionMessage,
    pub skip_pre_flight: bool,
    pub front_running_protection: Option<bool>,
    pub tip: Option<u64>,
    #[proto(tag = 6)]
    pub use_staked_rpcs: Option<bool>,
    #[proto(tag = 7)]
    pub fast_best_effort: Option<bool>,
    #[proto(tag = 8)]
    pub allow_back_run: Option<bool>,
    #[proto(tag = 9)]
    pub revenue_address: Option<String>,
    #[proto(tag = 10)]
    pub sniping: Option<bool>,
    #[proto(tag = 11, import_path = "google.protobuf")]
    pub timestamp: Option<Timestamp>,
    #[proto(tag = 12)]
    pub submit_protection: Option<SubmitProtection>,
    #[proto(tag = 13)]
    pub revert_protection: Option<bool>,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
pub struct PostSubmitPaladinRequest {
    pub transaction: TransactionMessageV2,
    pub revert_protection: Option<bool>,
    #[proto(import_path = "google.protobuf")]
    pub timestamp: Option<Timestamp>,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
pub struct PostSubmitRequestEntry {
    pub transaction: TransactionMessage,
    pub skip_pre_flight: bool,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
#[allow(non_camel_case_types)]
pub enum SubmitStrategy {
    P_UKNOWN = 0,
    P_SUBMIT_ALL = 1,
    P_ABORT_ON_FIRST_ERROR = 2,
    P_WAIT_FOR_CONFIRMATION = 3,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
#[allow(non_camel_case_types)]
pub enum SubmitProtection {
    SP_LOW = 0,
    SP_MEDIUM = 1,
    SP_HIGH = 2,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
pub struct PostSubmitBatchRequest {
    pub entries: Vec<PostSubmitRequestEntry>,
    pub submit_strategy: SubmitStrategy,
    pub use_bundle: Option<bool>,
    pub front_running_protection: Option<bool>,
    #[proto(import_path = "google.protobuf")]
    pub timestamp: Option<Timestamp>,
    #[proto(tag = 6)]
    pub submit_protection: Option<SubmitProtection>,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
#[derive(Debug)]
pub struct PostSubmitBatchResponseEntry {
    pub signature: String,

    pub error: String,

    pub submitted: bool,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
#[derive(Debug)]
pub struct PostSubmitBatchResponse {
    pub transactions: Vec<PostSubmitBatchResponseEntry>,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
pub struct PostSubmitSnipeRequest {
    pub entries: Vec<PostSubmitRequestEntry>,
    pub use_staked_rpcs: Option<bool>,
    #[proto(import_path = "google.protobuf")]
    pub timestamp: Option<Timestamp>,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
#[derive(Debug)]
pub struct PostSubmitSnipeResponse {
    pub transactions: ::proto_rs::alloc::vec::Vec<PostSubmitBatchResponseEntry>,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
#[derive(Debug)]
pub struct PostSubmitResponse {
    pub signature: String,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
pub struct GetRateLimitRequest;

#[proto_message(proto_path = "protos/bloxroute.proto")]
#[derive(Debug)]
pub struct GetRateLimitResponse {
    pub account_id: String,
    pub tier: String,
    pub interval: String,
    pub interval_num: u64,
    pub limit: u64,
    pub count: u64,
    pub reset: u64,
    pub stream_infos: Vec<StreamInfo>,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
#[derive(Debug)]
pub struct StreamInfo {
    pub stream_name: String,
    pub subscription_id: String,
    pub start_time: i64,
    pub credit_used: i64,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
pub struct TransactionMessage {
    pub content: String,
    pub is_cleanup: bool,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
pub struct TransactionMessageV2 {
    pub content: String,
}

#[proto_message(proto_path = "protos/bloxroute.proto")]
pub struct GetTransactionTraceRequest;

#[proto_message(proto_path = "protos/bloxroute.proto")]
#[derive(Debug)]
pub struct GetTransactionTraceResponse;

#[proto_rpc(rpc_package = "api", rpc_server = false, rpc_client = true, proto_path = "protos/bloxroute.proto")]
pub trait Api {
    async fn get_server_time(&self, request: Request<GetServerTimeRequest>) -> Result<Response<GetServerTimeResponse>, Status>;

    async fn post_submit_v2(&self, request: Request<PostSubmitRequest>) -> Result<Response<PostSubmitResponse>, Status>;

    async fn post_submit_batch_v2(&self, request: Request<PostSubmitBatchRequest>) -> Result<Response<PostSubmitBatchResponse>, Status>;

    async fn post_submit_snipe_v2(&self, request: Request<PostSubmitSnipeRequest>) -> Result<Response<PostSubmitSnipeResponse>, Status>;

    async fn get_rate_limit(&self, request: Request<GetRateLimitRequest>) -> Result<Response<GetRateLimitResponse>, Status>;

    async fn get_transaction_trace(
        &self,
        request: Request<GetTransactionTraceRequest>,
    ) -> Result<Response<GetTransactionTraceResponse>, Status>;

    async fn post_submit_paladin_v2(&self, request: Request<PostSubmitPaladinRequest>) -> Result<Response<PostSubmitResponse>, Status>;
}

#[derive(Clone)]
pub struct BloxrouteAuthInterceptor {
    api_key: Arc<MetadataValue<tonic::metadata::Ascii>>,
}

impl BloxrouteAuthInterceptor {
    pub fn new(api_key: String) -> Result<Self, tonic::metadata::errors::InvalidMetadataValue> {
        let api_key = Arc::new(MetadataValue::try_from(api_key)?);
        Ok(Self { api_key })
    }
}

impl tonic::service::Interceptor for BloxrouteAuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request.metadata_mut().insert("authorization", (*self.api_key).clone());
        Ok(request)
    }
}
