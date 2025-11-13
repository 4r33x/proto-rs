#![cfg(feature = "papaya")]

use std::hash::BuildHasherDefault;
use std::hash::Hasher;

use papaya::HashMap;
use papaya::HashSet;
use proto_rs::ProtoExt;
use proto_rs::proto_message;

#[proto_message(proto_path = "protos/tests/papaya.proto")]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PapayaCollections {
    #[proto(tag = 1)]
    pub label_by_id: HashMap<u32, String>,
    #[proto(tag = 2)]
    pub metrics: HashSet<u64>,
}

#[derive(Default)]
pub struct IdentityHasher(u64);

impl Hasher for IdentityHasher {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = self.0.wrapping_mul(0x100_0000_01b3).wrapping_add(u64::from(*byte));
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

type IdentityBuildHasher = BuildHasherDefault<IdentityHasher>;

#[proto_message(proto_path = "protos/tests/papaya.proto")]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PapayaCustomCollections {
    #[proto(tag = 1)]
    pub label_by_id: HashMap<u32, String, IdentityBuildHasher>,
    #[proto(tag = 2)]
    pub flags: HashSet<u32, IdentityBuildHasher>,
}

#[test]
fn papaya_hash_collections_roundtrip() {
    let message = PapayaCollections::default();

    {
        let map_guard = message.label_by_id.pin();
        map_guard.insert(1, "alpha".to_string());
        map_guard.insert(2, "beta".to_string());
    }

    {
        let set_guard = message.metrics.pin();
        set_guard.insert(7);
        set_guard.insert(11);
    }

    let encoded = PapayaCollections::encode_to_vec(&message);
    let decoded = PapayaCollections::decode(&encoded[..]).expect("decode papaya collections");

    assert_eq!(decoded, message);

    let guard = decoded.label_by_id.pin();
    assert_eq!(guard.iter().count(), 2);
    assert_eq!(guard.get(&1).map(String::as_str), Some("alpha"));
}

#[test]
fn papaya_hash_collections_support_custom_hashers() {
    let message = PapayaCustomCollections::default();

    {
        let map_guard = message.label_by_id.pin();
        map_guard.insert(3, "three".to_string());
        map_guard.insert(5, "five".to_string());
    }

    {
        let set_guard = message.flags.pin();
        set_guard.insert(13);
        set_guard.insert(17);
    }

    let encoded = PapayaCustomCollections::encode_to_vec(&message);
    let decoded = PapayaCustomCollections::decode(&encoded[..]).expect("decode papaya custom collections");

    assert_eq!(decoded, message);
}
