use std::sync::Mutex;
use std::sync::MutexGuard;

use proto_rs::DecodeError;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::ProtoShadowDecode;
use proto_rs::ProtoShadowEncode;
use proto_rs::encoding::DecodeContext;
use proto_rs::proto_message;

#[proto_message(sun = IdOwned)]
#[allow(dead_code)]
struct IdShadow {
    #[proto(tag = 1)]
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

struct IdRef<'a> {
    _guard: MutexGuard<'a, u64>,
    id: u64,
}

impl ProtoShadowDecode<IdOwned> for IdShadow {
    fn to_sun(self) -> Result<IdOwned, DecodeError> {
        Ok(IdOwned { id: Mutex::new(self.id) })
    }
}

impl<'a> ProtoShadowEncode<'a, IdOwned> for IdShadow {
    fn from_sun(value: &'a IdOwned) -> Self {
        let guard = value.id.lock().unwrap();
        let view = IdRef { id: *guard, _guard: guard };
        Self { id: view.id }
    }
}

impl ProtoEncode for IdOwned {
    type Shadow<'a> = IdShadow;
}

impl ProtoDecode for IdOwned {
    type ShadowDecoded = IdShadow;
}

#[test]
fn encode_decode_reference_sun_top_level() {
    let id = IdOwned { id: Mutex::new(42) };
    let bytes = id.encode_to_vec();
    let decoded = <IdOwned as ProtoDecode>::decode(bytes.as_slice(), DecodeContext::default()).expect("decode owned id");

    assert_eq!(decoded, id);
}
