use std::sync::Mutex;
use std::sync::MutexGuard;

use proto_rs::DecodeError;
use proto_rs::ProtoExt;
use proto_rs::ProtoShadow;
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

impl ProtoShadow<IdOwned> for IdShadow {
    type Sun<'a> = &'a IdOwned;
    type OwnedSun = IdOwned;
    type View<'a> = IdRef<'a>;

    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(IdOwned { id: Mutex::new(self.id) })
    }

    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        let guard = value.id.lock().unwrap();
        IdRef { id: *guard, _guard: guard }
    }
}

#[test]
fn encode_decode_reference_sun_top_level() {
    let id = IdOwned { id: Mutex::new(42) };
    let bytes = IdOwned::encode_to_vec(&id);
    let decoded = IdOwned::decode(bytes.as_slice()).expect("decode owned id");

    assert_eq!(decoded, id);
}
