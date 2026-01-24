use std::collections::VecDeque;

use bytes::Bytes;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::encoding::DecodeContext;
use proto_rs::proto_message;

#[proto_message]
#[derive(Debug, PartialEq)]
struct Pair<K, V> {
    key: K,
    value: V,
}

#[proto_message]
#[derive(Debug, PartialEq)]
struct Lru<K, V, const CAP: usize> {
    items: VecDeque<Pair<K, V>>,
}

#[test]
fn generic_const_message_roundtrip() {
    let mut items = VecDeque::new();
    items.push_back(Pair { key: 10u32, value: 20u64 });
    items.push_back(Pair { key: 11u32, value: 21u64 });

    let lru = Lru::<u32, u64, 8> { items };
    let encoded = <Lru<u32, u64, 8> as ProtoEncode>::encode_to_vec(&lru);
    let decoded = <Lru<u32, u64, 8> as ProtoDecode>::decode(Bytes::from(encoded), DecodeContext::default()).expect("decode");

    assert_eq!(lru, decoded);
}
