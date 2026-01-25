#![cfg(feature = "papaya")]

use std::hash::BuildHasherDefault;
use std::hash::Hasher;

use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::encoding::DecodeContext;
use proto_rs::proto_message;

#[proto_message(proto_path = "protos/tests/papaya.proto")]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PapayaCollections {
    #[proto(tag = 1)]
    pub label_by_id: papaya::HashMap<u32, String>,
    #[proto(tag = 2)]
    pub metrics: papaya::HashSet<u64>,
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
    pub label_by_id: papaya::HashMap<u32, String, IdentityBuildHasher>,
    #[proto(tag = 2)]
    pub flags: papaya::HashSet<u32, IdentityBuildHasher>,
}

#[proto_message(proto_path = "protos/tests/papaya.proto")]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PapayaStringSet {
    #[proto(tag = 1)]
    pub tags: papaya::HashSet<String>,
}

#[proto_message(proto_path = "protos/tests/papaya.proto")]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PapayaCustomStringSet {
    #[proto(tag = 1)]
    pub tags: papaya::HashSet<String, IdentityBuildHasher>,
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
        set_guard.insert(0);
        set_guard.insert(7);
        set_guard.insert(11);
    }

    let encoded = PapayaCollections::encode_to_vec(&message);
    let decoded = <PapayaCollections as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).expect("decode papaya collections");

    assert_eq!(decoded, message);

    let guard = decoded.label_by_id.pin();
    assert_eq!(guard.iter().count(), 2);
    assert_eq!(guard.get(&1).map(String::as_str), Some("alpha"));

    let metric_guard = decoded.metrics.pin();
    assert_eq!(metric_guard.iter().count(), 3);
    assert!(metric_guard.contains(&0));
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
    let decoded =
        <PapayaCustomCollections as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).expect("decode papaya custom collections");

    assert_eq!(decoded, message);
}

#[test]
fn papaya_hashset_roundtrip_with_strings() {
    let message = PapayaStringSet::default();

    {
        let guard = message.tags.pin();
        guard.insert("red".to_string());
        guard.insert("green".to_string());
        guard.insert("blue".to_string());
    }

    let encoded = PapayaStringSet::encode_to_vec(&message);
    let decoded = <PapayaStringSet as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).expect("decode papaya string set");

    assert_eq!(decoded, message);

    let guard = decoded.tags.pin();
    assert_eq!(guard.len(), 3);
    assert!(guard.contains("red"));
}

#[test]
fn papaya_hashset_roundtrip_with_custom_hasher_strings() {
    let message = PapayaCustomStringSet::default();

    {
        let guard = message.tags.pin();
        guard.insert("alpha".to_string());
        guard.insert("beta".to_string());
    }

    let encoded = PapayaCustomStringSet::encode_to_vec(&message);
    let decoded =
        <PapayaCustomStringSet as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).expect("decode papaya custom string set");

    assert_eq!(decoded, message);
}
