use proto_rs::ProtoExt;
use proto_rs::ProtoWire;
use proto_rs::encoding::DecodeContext;
use proto_rs::encoding::WireType;
use proto_rs::proto_message;

#[proto_message]
#[derive(Debug, PartialEq, Eq)]
pub struct UserIdTuple(#[proto(trasparent)] u64);

#[proto_message]
#[derive(Debug, PartialEq, Eq)]
pub struct UserIdNamed {
    #[proto(trasparent)]
    pub id: u64,
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

#[proto_message]
#[derive(Debug, PartialEq, Eq)]
pub struct MessageWrapper(#[proto(trasparent)] InnerMessage);

#[test]
fn transparent_tuple_roundtrip() {
    let original = UserIdTuple(123);
    let mut buf = Vec::new();
    <UserIdTuple as ProtoWire>::encode_raw_unchecked(&original, &mut buf);
    assert_eq!(buf, vec![123]);

    let mut decoded = UserIdTuple::proto_default();
    <UserIdTuple as ProtoWire>::decode_into(WireType::Varint, &mut decoded, &mut &buf[..], DecodeContext::default()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn transparent_named_roundtrip() {
    let original = UserIdNamed { id: 77 };
    let mut buf = Vec::new();
    <UserIdNamed as ProtoWire>::encode_raw_unchecked(&original, &mut buf);
    assert_eq!(buf, vec![77]);

    let mut decoded = UserIdNamed::proto_default();
    <UserIdNamed as ProtoWire>::decode_into(WireType::Varint, &mut decoded, &mut &buf[..], DecodeContext::default()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn transparent_in_holder_encodes_inner_once() {
    let holder = Holder {
        tuple: UserIdTuple(5),
        named: UserIdNamed { id: 9 },
    };

    let mut buf = Vec::new();
    <Holder as ProtoWire>::encode_raw_unchecked(&holder, &mut buf);

    // Expected encoding:
    // field 1 (tuple): key 0x08 followed by value 0x05
    // field 2 (named): key 0x10 followed by value 0x09
    assert_eq!(buf, vec![0x08, 0x05, 0x10, 0x09]);
}

#[test]
fn transparent_message_roundtrip_top_level() {
    let original = MessageWrapper(InnerMessage { value: 42 });
    let buf = <MessageWrapper as ProtoExt>::encode_to_vec(&original);
    let decoded = <MessageWrapper as ProtoExt>::decode(&buf[..]).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn transparent_message_decode_length_delimited_body() {
    let body = [0x02, 0x08, 0x2A];
    let decoded = <MessageWrapper as ProtoExt>::decode_length_delimited(&body[..], DecodeContext::default()).unwrap();
    assert_eq!(decoded, MessageWrapper(InnerMessage { value: 42 }));
}
