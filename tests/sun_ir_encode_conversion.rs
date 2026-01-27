use proto_rs::DecodeError;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::ProtoShadowDecode;
use proto_rs::ProtoShadowEncode;
use proto_rs::encoding::DecodeContext;
use proto_rs::proto_message;

#[derive(Clone, Debug, Default, PartialEq)]
struct Fancy(u64);

fn u64_to_fancy(value: u64) -> Fancy {
    Fancy(value - 5)
}

impl From<&Fancy> for u64 {
    fn from(value: &Fancy) -> Self {
        value.0 + 5
    }
}

impl From<Fancy> for u64 {
    fn from(value: Fancy) -> Self {
        value.0 + 5
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Sun {
    value: Fancy,
}

struct SunIr<'a> {
    value: &'a Fancy,
}

#[proto_message(sun = [Sun], sun_ir = SunIr<'a>)]
struct SunProto {
    #[proto(getter = "(*$.value).clone()", into = "u64", from_fn = "u64_to_fancy")]
    value: Fancy,
}

impl ProtoShadowDecode<Sun> for SunProto {
    fn to_sun(self) -> Result<Sun, DecodeError> {
        Ok(Sun { value: self.value })
    }
}

impl<'a> ProtoShadowEncode<'a, Sun> for SunIr<'a> {
    fn from_sun(value: &'a Sun) -> Self {
        SunIr { value: &value.value }
    }
}

#[test]
fn encode_decode_sun_ir_with_into_conversion() {
    let value = Sun { value: Fancy(10) };
    let bytes = Sun::encode_to_vec(&value);
    let decoded = <Sun as ProtoDecode>::decode(bytes.as_slice(), DecodeContext::default()).expect("decode sun with into");

    assert_eq!(decoded, value);
}
