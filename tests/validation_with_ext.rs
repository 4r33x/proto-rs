#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use proto_rs::DecodeError;
use proto_rs::ProtoExt;
use proto_rs::ProtoShadow;
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

fn validate_pong_shadow_with_ext(pong: &mut PongShadow, ext: &Extensions) -> Result<(), DecodeError> {
    // This validator runs on the shadow type, not the sun type
    if pong.id == 999 {
        return Err(DecodeError::new("shadow id cannot be 999"));
    }
    if let Some(flag) = ext.get::<ValidationFlag>()
        && flag.0 == 2
    {
        return Err(DecodeError::new("shadow blocked by extension flag 2"));
    }
    Ok(())
}

#[proto_message(proto_path = "protos/tests/validation_with_ext.proto")]
#[proto(validator_with_ext = validate_pong_with_ext)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pong {
    pub id: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PongWithShadow {
    pub id: u32,
}

#[proto_message(proto_path = "protos/tests/validation_with_ext.proto", sun = [PongWithShadow])]
#[proto(validator_with_ext = validate_pong_shadow_with_ext)]
pub struct PongShadow {
    pub id: u32,
}

impl ProtoShadow<PongWithShadow> for PongShadow {
    type Sun<'a> = &'a PongWithShadow;
    type OwnedSun = PongWithShadow;
    type View<'a> = Self;

    fn to_sun(self) -> Result<Self::OwnedSun, proto_rs::DecodeError> {
        Ok(PongWithShadow { id: self.id })
    }

    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        Self { id: value.id }
    }
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

// Tests for shadow type validation
#[cfg(feature = "tonic")]
#[test]
fn shadow_validates_with_ext_flag_is_enabled() {
    // Verify that PongWithShadow (the sun type) has VALIDATE_WITH_EXT enabled
    const _: () = {
        assert!(<PongWithShadow as ProtoExt>::VALIDATE_WITH_EXT);
    };
}

#[cfg(feature = "tonic")]
#[test]
fn shadow_validator_receives_shadow_type() {
    // This test verifies that the validator for sun types receives the shadow type
    // The validate_pong_shadow_with_ext function takes &mut PongShadow, not &mut PongWithShadow
    // If this compiles and runs, it means the validator is correctly receiving the shadow type
    let mut sun = PongWithShadow { id: 42 };
    let ext = Extensions::new();

    // Call validate_with_ext on the sun type
    let result = <PongWithShadow as ProtoExt>::validate_with_ext(&mut sun, &ext);
    assert!(result.is_ok());
}

#[cfg(feature = "tonic")]
#[test]
fn shadow_validator_rejects_invalid_shadow_id() {
    // Test that the shadow validator correctly rejects id = 999
    let mut sun = PongWithShadow { id: 999 };
    let ext = Extensions::new();

    let result = <PongWithShadow as ProtoExt>::validate_with_ext(&mut sun, &ext);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("shadow id cannot be 999"));
}

#[cfg(feature = "tonic")]
#[test]
fn shadow_validator_checks_extensions() {
    let mut sun = PongWithShadow { id: 42 };
    let mut ext = Extensions::new();
    ext.insert(ValidationFlag(2));

    let result = <PongWithShadow as ProtoExt>::validate_with_ext(&mut sun, &ext);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("shadow blocked by extension flag 2"));
}
