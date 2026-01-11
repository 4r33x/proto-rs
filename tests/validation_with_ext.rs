#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use proto_rs::DecodeError;
use proto_rs::ProtoExt;
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

#[proto_message(proto_path = "protos/tests/validation_with_ext.proto")]
#[proto(validator_with_ext = validate_pong_with_ext)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pong {
    pub id: u32,
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
        assert!(<Pong as ProtoExt>::VALIDATE_WITH_EXT);
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
