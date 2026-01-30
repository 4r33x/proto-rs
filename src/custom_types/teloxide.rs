use prosto_derive::proto_message;
use teloxide_core::types::UserId;

use crate::DecodeError;
use crate::ProtoShadowDecode;
use crate::ProtoShadowEncode;

#[proto_message(proto_path = "protos/teloxide.proto", sun = UserId)]
#[derive(Clone, Copy)]
pub struct UserIdProto(pub u64);

impl ProtoShadowDecode<UserId> for UserIdProto {
    #[inline(always)]
    fn to_sun(self) -> Result<UserId, DecodeError> {
        Ok(UserId(self.0))
    }
}

impl<'a> ProtoShadowEncode<'a, UserId> for UserIdProto {
    #[inline(always)]
    fn from_sun(value: &'a UserId) -> Self {
        Self(value.0)
    }
}
