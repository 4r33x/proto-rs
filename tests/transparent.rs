use proto_rs::ProtoDecode;
use proto_rs::ProtoDecoder;
use proto_rs::ProtoEncode;
use proto_rs::encoding::DecodeContext;
use proto_rs::encoding::WireType;
use proto_rs::proto_message;

#[proto_message(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct UserIdTuple(u64);

#[proto_message(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct UserIdNamed {
    pub id: u64,
}

#[proto_message]
#[derive(Debug, PartialEq, Eq)]
pub struct UserWithId {
    #[proto(rename = "optional uint32")]
    pub id1: UserIdNamed,
    #[proto(rename = "uint64")]
    pub id2: UserIdNamed,
    #[proto(rename = u8)]
    pub id3: UserIdNamed,
    #[proto(rename = Vec<u8>)]
    pub id4: UserIdNamed,
}

pub type ComplexType = proto_rs::alloc::collections::BTreeMap<u64, u64>;
pub type ComplexType2 = std::collections::HashMap<u64, u64, std::hash::RandomState>;

#[proto_message]
#[derive(Debug, PartialEq, Eq)]
pub struct UserIdTreatAs {
    #[proto(treat_as = "proto_rs::alloc::collections::BTreeMap<u64, u64>")]
    pub id: ComplexType,
    #[proto(treat_as = "std::collections::HashMap<u64, u64>")]
    pub id2: ComplexType2,
}

#[proto_message]
#[derive(Debug, PartialEq, Eq)]
pub struct Holder {
    #[proto(tag = 1)]
    pub tuple: UserIdTuple,
    #[proto(tag = 2)]
    pub named: UserIdNamed,
}

#[proto_message]
#[derive(Debug, PartialEq, Eq)]
pub struct InnerMessage {
    #[proto(tag = 1)]
    pub value: u32,
}

#[proto_message(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub struct MessageWrapper(InnerMessage);

#[test]
fn transparent_tuple_roundtrip() {
    let original = UserIdTuple(123);
    let buf = original.encode_to_vec();
    assert_eq!(buf, vec![123]);

    let mut decoded = UserIdTuple::proto_default();
    <UserIdTuple as ProtoDecoder>::merge(&mut decoded, WireType::Varint, &mut &buf[..], DecodeContext::default()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn transparent_named_roundtrip() {
    let original = UserIdNamed { id: 77 };
    let buf = original.encode_to_vec();
    assert_eq!(buf, vec![77]);

    let mut decoded = UserIdNamed::proto_default();
    <UserIdNamed as ProtoDecoder>::merge(&mut decoded, WireType::Varint, &mut &buf[..], DecodeContext::default()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn transparent_in_holder_encodes_inner_once() {
    let holder = Holder {
        tuple: UserIdTuple(5),
        named: UserIdNamed { id: 9 },
    };

    let buf = holder.encode_to_vec();

    // Expected encoding:
    // field 1 (tuple): key 0x08 followed by value 0x05
    // field 2 (named): key 0x10 followed by value 0x09
    assert_eq!(buf, vec![0x08, 0x05, 0x10, 0x09]);
}

#[test]
fn transparent_message_roundtrip_top_level() {
    let original = MessageWrapper(InnerMessage { value: 42 });
    let buf = original.encode_to_vec();
    let decoded = <MessageWrapper as ProtoDecode>::decode(&buf[..], DecodeContext::default()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn transparent_message_decode_length_delimited_body() {
    let body = [0x02, 0x08, 0x2A];
    let mut decoded = MessageWrapper::proto_default();
    <MessageWrapper as ProtoDecoder>::merge(&mut decoded, WireType::LengthDelimited, &mut &body[..], DecodeContext::default()).unwrap();
    assert_eq!(decoded, MessageWrapper(InnerMessage { value: 42 }));
}
