use std::sync::Mutex;
use proto_rs::DecodeError;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::ProtoShadowDecode;
use proto_rs::encoding::DecodeContext;
use proto_rs::proto_message;

fn locked_id(value: &IdOwned) -> u64 {
    *value.id.lock().unwrap()
}

#[proto_message(sun = IdOwned)]
#[allow(dead_code)]
struct IdShadow {
    #[proto(tag = 1, getter = "locked_id($)")]
    id: u64,
}

#[derive(Debug)]
struct IdOwned {
    id: Mutex<u64>,
}

impl PartialEq for IdOwned {
    fn eq(&self, other: &Self) -> bool {
        *self.id.lock().unwrap() == *other.id.lock().unwrap()
    }
}

impl ProtoShadowDecode<IdOwned> for IdShadow {
    fn to_sun(self) -> Result<IdOwned, DecodeError> {
        Ok(IdOwned { id: Mutex::new(self.id) })
    }
}

#[test]
fn encode_decode_reference_sun_top_level() {
    let id = IdOwned { id: Mutex::new(42) };
    let bytes = IdOwned::encode_to_vec(&id);
    let decoded = <IdOwned as ProtoDecode>::decode(bytes.as_slice(), DecodeContext::default()).expect("decode owned id");

    assert_eq!(decoded, id);
}
