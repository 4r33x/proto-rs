use std::collections::HashMap;

pub use auth_service_client::AuthServiceClient as JitoAuthClient;
pub use searcher_service_client::SearcherServiceClient as JitoSearcherClient;
use tokio_stream::Stream;
use tonic::Response;
use tonic::Status;

use crate::custom_types::well_known::Timestamp;
use crate::proto_message;
use crate::proto_rpc;

#[proto_message(proto_path = "protos/jito.proto")]
pub struct SendBundleRequest {
    pub bundle: Bundle,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct SendBundleResponse {
    pub uuid: String,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct Bundle {
    #[proto(tag = 2)]
    pub header: Header,
    #[proto(tag = 3)]
    pub packets: Vec<Packet>,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct BundleUuid {
    pub bundle: Bundle,
    pub uuid: String,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct Packet {
    pub data: Vec<u8>,
    pub meta: Meta,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct Meta {
    pub size: u64,
    pub addr: String,
    pub port: u32,
    pub flags: PacketFlags,
    pub sender_stake: u64,
}

#[allow(clippy::struct_excessive_bools)]
#[proto_message(proto_path = "protos/jito.proto")]
pub struct PacketFlags {
    pub discard: bool,
    pub forwarded: bool,
    pub repair: bool,
    pub simple_vote_tx: bool,
    pub tracer_packet: bool,
    pub from_staked_node: bool,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct Header {
    #[proto(import_path = "google.protobuf")]
    pub ts: Timestamp,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct Heartbeat {
    pub count: u64,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct Socket {
    pub ip: String,
    pub port: i64,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub enum Role {
    Relayer = 0,
    Searcher = 1,
    Validator = 2,
    ShredstreamSubscriber = 3,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct GenerateAuthChallengeRequest {
    pub role: Role,
    pub pubkey: Vec<u8>,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct GenerateAuthChallengeResponse {
    pub challenge: String,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct GenerateAuthTokensRequest {
    pub challenge: String,
    pub client_pubkey: Vec<u8>,
    pub signed_challenge: Vec<u8>,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct Token {
    pub value: String,
    #[proto(import_path = "google.protobuf")]
    pub expires_at_utc: Timestamp,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct GenerateAuthTokensResponse {
    pub access_token: Token,
    pub refresh_token: Token,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct RefreshAccessTokenRequest {
    pub refresh_token: String,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct RefreshAccessTokenResponse {
    pub access_token: Token,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct SlotList {
    pub slots: Vec<u64>,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct ConnectedLeadersResponse {
    pub connected_validators: HashMap<String, SlotList>,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct NextScheduledLeaderRequest {
    pub regions: Vec<String>,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct NextScheduledLeaderResponse {
    pub current_slot: u64,
    pub next_leader_slot: u64,
    pub next_leader_identity: String,
    pub next_leader_region: String,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct ConnectedLeadersRequest;

#[proto_message(proto_path = "protos/jito.proto")]
pub struct ConnectedLeadersRegionedRequest {
    pub regions: Vec<String>,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct ConnectedLeadersRegionedResponse {
    pub connected_validators: HashMap<String, ConnectedLeadersResponse>,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct GetTipAccountsRequest;

#[proto_message(proto_path = "protos/jito.proto")]
pub struct GetTipAccountsResponse {
    pub accounts: Vec<String>,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct SubscribeBundleResultsRequest;

#[proto_message(proto_path = "protos/jito.proto")]
pub struct GetRegionsRequest;

#[proto_message(proto_path = "protos/jito.proto")]
pub struct GetRegionsResponse {
    pub current_region: String,
    pub available_regions: Vec<String>,
}

#[proto_message(proto_path = "protos/jito.proto")]
pub struct BundleResult;

#[proto_rpc(rpc_package = "jito", rpc_server = false, rpc_client = true, proto_path = "protos/jito.proto")]
pub trait AuthService {
    async fn generate_auth_challenge(
        &self,
        request: Request<GenerateAuthChallengeRequest>,
    ) -> Result<Response<GenerateAuthChallengeResponse>, Status>;

    async fn generate_auth_tokens(
        &self,
        request: Request<GenerateAuthTokensRequest>,
    ) -> Result<Response<GenerateAuthTokensResponse>, Status>;

    async fn refresh_access_token(
        &self,
        request: Request<RefreshAccessTokenRequest>,
    ) -> Result<Response<RefreshAccessTokenResponse>, Status>;
}

#[proto_rpc(rpc_package = "jito", rpc_server = false, rpc_client = true, proto_path = "protos/jito.proto")]
pub trait SearcherService {
    type SubscribeBundleResultsStream: Stream<Item = Result<BundleResult, Status>>;

    async fn subscribe_bundle_results(
        &self,
        request: Request<SubscribeBundleResultsRequest>,
    ) -> Result<Response<Self::SubscribeBundleResultsStream>, Status>;

    async fn send_bundle(&self, request: Request<SendBundleRequest>) -> Result<Response<SendBundleResponse>, Status>;

    async fn get_next_scheduled_leader(
        &self,
        request: Request<NextScheduledLeaderRequest>,
    ) -> Result<Response<NextScheduledLeaderResponse>, Status>;

    async fn get_connected_leaders(&self, request: Request<ConnectedLeadersRequest>) -> Result<Response<ConnectedLeadersResponse>, Status>;

    async fn get_connected_leaders_regioned(
        &self,
        request: Request<ConnectedLeadersRegionedRequest>,
    ) -> Result<Response<ConnectedLeadersRegionedResponse>, Status>;

    async fn get_tip_accounts(&self, request: Request<GetTipAccountsRequest>) -> Result<Response<GetTipAccountsResponse>, Status>;

    async fn get_regions(&self, request: Request<GetRegionsRequest>) -> Result<Response<GetRegionsResponse>, Status>;
}
