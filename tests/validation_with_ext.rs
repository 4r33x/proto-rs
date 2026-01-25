#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;

use proto_rs::DecodeError;
use proto_rs::ProtoDecode;
use proto_rs::ProtoShadowDecode;
use proto_rs::ProtoShadowEncode;
use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tonic::Extensions;
use tonic::Request;
use tonic::Response;
use tonic::Status;

#[derive(Clone, Debug)]
struct ValidationFlag(u8);

fn validate_pong_with_ext(pong: &mut Pong, ext: &Extensions) -> Result<(), DecodeError> {
    if pong.id == 0 {
        return Err(DecodeError::new("id must be non-zero"));
    }
    if let Some(flag) = ext.get::<ValidationFlag>()
        && flag.0 == 1
    {
        return Err(DecodeError::new("blocked by extension flag"));
    }
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
const fn validate_pong_shadow_with_ext(_pong: &mut PongWithShadow, _ext: &Extensions) -> Result<(), DecodeError> {
    Ok(())
}

#[proto_message(proto_path = "protos/tests/validation_with_ext.proto")]
#[proto(validator_with_ext = validate_pong_with_ext)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pong {
    pub id: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PongWithShadow {
    pub id: u32,
}

#[proto_message(proto_path = "protos/tests/validation_with_ext.proto", sun = [PongWithShadow])]
#[proto(validator_with_ext = validate_pong_shadow_with_ext)]
pub struct PongShadowProto {
    pub id: u32,
}

impl ProtoShadowDecode<PongWithShadow> for PongShadowProto {
    fn to_sun(self) -> Result<PongWithShadow, proto_rs::DecodeError> {
        Ok(PongWithShadow { id: self.id })
    }
}

impl<'a> ProtoShadowEncode<'a, PongWithShadow> for PongShadowProto {
    fn from_sun(value: &'a PongWithShadow) -> Self {
        Self { id: value.id }
    }
}

#[proto_message(proto_path = "protos/tests/validation_with_ext.proto")]
struct WithPongs {
    pongs: Vec<PongWithShadow>,
    pongs2: HashMap<u32, PongWithShadow>,
    pong3: Arc<PongWithShadow>,
    pong4: [PongWithShadow; 2],
    pong5: VecDeque<PongWithShadow>,
    pong6: Box<PongWithShadow>,
    pong_set: HashSet<PongWithShadow>,
}

#[cfg(feature = "cache_padded")]
#[proto_message(proto_path = "protos/tests/validation_with_ext.proto")]
struct WithPongPapaya {
    #[cfg(feature = "papaya")]
    pong8: papaya::HashMap<u32, PongWithShadow>,

    #[cfg(feature = "papaya")]
    pong9: papaya::HashSet<PongWithShadow>,
}

#[cfg(feature = "cache_padded")]
#[proto_message(proto_path = "protos/tests/validation_with_ext.proto")]

struct WithPongCached {
    pong7: crossbeam_utils::CachePadded<PongWithShadow>,
}

#[proto_rpc(
    rpc_package = "validation_with_ext",
    rpc_server = true,
    proto_path = "protos/tests/validation_with_ext.proto"
)]
pub trait ValidationWithExt {
    async fn check(&self, request: Request<Pong>) -> Result<Response<Pong>, Status>;
}

#[derive(Default)]
struct ValidationWithExtService;

impl ValidationWithExt for ValidationWithExtService {
    async fn check(&self, request: Request<Pong>) -> Result<Response<Pong>, Status> {
        Ok(Response::new(request.into_inner()))
    }
}

#[cfg(feature = "tonic")]
#[test]
fn validates_with_ext_flag_is_enabled() {
    const _: () = {
        assert!(<Pong as ProtoDecode>::VALIDATE_WITH_EXT);
    };
}

#[cfg(feature = "tonic")]
#[tokio::test]
async fn server_validation_with_ext_rejects_flagged_request() {
    let service = ValidationWithExtService {};
    let mut request = Request::new(Pong { id: 42 });
    request.extensions_mut().insert(ValidationFlag(1));

    let result = <ValidationWithExtService as validation_with_ext_server::ValidationWithExt>::check(&service, request).await;

    let status = result.expect_err("expected extension validator to reject request");
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("blocked by extension flag"));
}

#[cfg(feature = "tonic")]
#[tokio::test]
async fn server_validation_with_ext_accepts_clean_request() {
    let service = ValidationWithExtService {};
    let request = Request::new(Pong { id: 7 });

    let response = <ValidationWithExtService as validation_with_ext_server::ValidationWithExt>::check(&service, request)
        .await
        .expect("request should succeed");

    assert_eq!(response.into_inner(), Pong { id: 7 });
}
