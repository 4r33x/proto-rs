use std::collections::VecDeque;

use bytes::Bytes;
use prost::Message as ProstMessage;
use proto_rs::ProtoExt;
use proto_rs::proto_message;

#[proto_message(proto_path = "protos/tests/vecdeque.proto")]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Nested {
    #[proto(tag = 1)]
    pub id: u32,
    #[proto(tag = 2)]
    pub name: String,
}

#[proto_message(proto_path = "protos/tests/vecdeque.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct VecDequeMessage {
    #[proto(tag = 1)]
    pub numbers: VecDeque<i32>,
    #[proto(tag = 2)]
    pub data: VecDeque<u8>,
    #[proto(tag = 3)]
    pub children: VecDeque<Nested>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "vecdeque")]
pub struct NestedProst {
    #[prost(uint32, tag = "1")]
    pub id: u32,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "vecdeque")]
pub struct VecDequeMessageProst {
    #[prost(int32, repeated, tag = "1")]
    pub numbers: ::prost::alloc::vec::Vec<i32>,
    #[prost(bytes = "vec", tag = "2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, repeated, tag = "3")]
    pub children: ::prost::alloc::vec::Vec<NestedProst>,
}

#[test]
fn vecdeque_roundtrip_primitives_and_messages() {
    let children = VecDeque::from([Nested { id: 7, name: "alpha".into() }, Nested { id: 8, name: "beta".into() }]);

    let message = VecDequeMessage {
        numbers: VecDeque::from([1, 2, 3, 4]),
        data: VecDeque::from(Vec::from(b"hello world".as_slice())),
        children,
    };

    let encoded = VecDequeMessage::encode_to_vec(&message);
    let decoded = VecDequeMessage::decode(Bytes::from(encoded.clone())).expect("decode VecDequeMessage");
    assert_eq!(message, decoded);

    let prost_message = VecDequeMessageProst {
        numbers: message.numbers.iter().copied().collect(),
        data: message.data.iter().copied().collect(),
        children: message
            .children
            .iter()
            .map(|child| NestedProst {
                id: child.id,
                name: child.name.clone(),
            })
            .collect(),
    };

    let prost_encoded = prost_message.encode_to_vec();
    assert_eq!(encoded, prost_encoded);
}

#[test]
fn vecdeque_bytes_encode_equivalence() {
    let payload = VecDeque::from(vec![0_u8, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let message = VecDequeMessage {
        numbers: VecDeque::new(),
        data: payload.clone(),
        children: VecDeque::new(),
    };

    let encoded = VecDequeMessage::encode_to_vec(&message);
    let decoded = VecDequeMessage::decode(Bytes::from(encoded.clone())).expect("decode VecDequeMessage bytes");
    assert_eq!(message, decoded);

    let prost_message = VecDequeMessageProst {
        numbers: Vec::new(),
        data: payload.into(),
        children: Vec::new(),
    };

    let prost_encoded = prost_message.encode_to_vec();
    assert_eq!(encoded, prost_encoded);
}
