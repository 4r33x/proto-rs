use proto_rs::DecodeError;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::ProtoShadowDecode;
use proto_rs::ProtoShadowEncode;
use proto_rs::encoding::DecodeContext;
use proto_rs::proto_message;

#[derive(Clone, Debug, PartialEq)]
struct OwnedSun {
    id: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct OwnedIr {
    id: u64,
}

#[proto_message(sun = [OwnedSun], sun_ir = OwnedIr)]
struct OwnedProto {
    id: u64,
}

impl ProtoShadowDecode<OwnedSun> for OwnedProto {
    fn to_sun(self) -> Result<OwnedSun, DecodeError> {
        Ok(OwnedSun { id: self.id })
    }
}

impl<'a> ProtoShadowEncode<'a, OwnedSun> for OwnedIr {
    fn from_sun(value: &'a OwnedSun) -> Self {
        OwnedIr { id: value.id }
    }
}

impl<'a> ProtoShadowEncode<'a, OwnedSun> for OwnedProto {
    fn from_sun(value: &'a OwnedSun) -> Self {
        OwnedProto { id: value.id }
    }
}

#[test]
fn encode_decode_owned_sun_ir() {
    let value = OwnedSun { id: 21 };
    let bytes = OwnedSun::encode_to_vec(&value);
    let decoded =
        <OwnedSun as ProtoDecode>::decode(bytes.as_slice(), DecodeContext::default()).expect("decode owned sun");

    assert_eq!(decoded, value);
}
