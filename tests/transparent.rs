use bytes::Buf;
use proto_rs::ProtoDecode;
use proto_rs::ProtoDecoder;
use proto_rs::ProtoDefault;
use proto_rs::ProtoEncode;
use proto_rs::RevWriter;
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
    let shadow = <<UserIdTuple as ProtoEncode>::Shadow<'_> as proto_rs::ProtoShadowEncode<'_, UserIdTuple>>::from_sun(&original);
    let mut writer = proto_rs::RevVec::with_capacity(8);
    <<UserIdTuple as ProtoEncode>::Shadow<'_> as proto_rs::ProtoArchive>::archive::<0>(&shadow, &mut writer);
    let buf = writer.finish_tight();
    assert_eq!(buf.as_slice(), vec![123]);

    let mut decoded = <UserIdTuple as ProtoDefault>::proto_default();
    <UserIdTuple as ProtoDecoder>::merge(&mut decoded, WireType::Varint, &mut buf.as_slice(), DecodeContext::default()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn transparent_named_roundtrip() {
    let original = UserIdNamed { id: 77 };
    let shadow = <<UserIdNamed as ProtoEncode>::Shadow<'_> as proto_rs::ProtoShadowEncode<'_, UserIdNamed>>::from_sun(&original);
    let mut writer = proto_rs::RevVec::with_capacity(8);
    <<UserIdNamed as ProtoEncode>::Shadow<'_> as proto_rs::ProtoArchive>::archive::<0>(&shadow, &mut writer);
    let buf = writer.finish_tight();
    assert_eq!(buf.as_slice(), vec![77]);

    let mut decoded = <UserIdNamed as ProtoDefault>::proto_default();
    <UserIdNamed as ProtoDecoder>::merge(&mut decoded, WireType::Varint, &mut buf.as_slice(), DecodeContext::default()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn transparent_in_holder_encodes_inner_once() {
    let holder = Holder {
        tuple: UserIdTuple(5),
        named: UserIdNamed { id: 9 },
    };

    let shadow = <<Holder as ProtoEncode>::Shadow<'_> as proto_rs::ProtoShadowEncode<'_, Holder>>::from_sun(&holder);
    let mut writer = proto_rs::RevVec::with_capacity(16);
    <<Holder as ProtoEncode>::Shadow<'_> as proto_rs::ProtoArchive>::archive::<0>(&shadow, &mut writer);
    let buf = writer.finish_tight();

    // Expected encoding:
    // field 1 (tuple): key 0x08 followed by value 0x05
    // field 2 (named): key 0x10 followed by value 0x09
    assert_eq!(buf.as_slice(), vec![0x08, 0x05, 0x10, 0x09]);
}

#[test]
fn transparent_message_roundtrip_top_level() {
    let original = MessageWrapper(InnerMessage { value: 42 });
    let buf = <MessageWrapper as ProtoEncode>::encode_to_vec(&original);
    let decoded = <MessageWrapper as ProtoDecode>::decode(&buf[..], DecodeContext::default()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn transparent_message_decode_length_delimited_body() {
    let body = [0x02, 0x08, 0x2A];
    let mut buf = &body[..];
    let len = proto_rs::decode_length_delimiter(&mut buf).expect("decode length");
    let slice = Buf::take(&mut buf, len);
    let decoded = <MessageWrapper as ProtoDecode>::decode(slice, DecodeContext::default()).unwrap();
    assert_eq!(decoded, MessageWrapper(InnerMessage { value: 42 }));
}

// Test for generic transparent struct - this is the regression test for the fix
// where merge_field was incorrectly checking tag == 1 instead of forwarding
// to the inner type's merge_field
#[proto_message(transparent)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct IdGenericTransparent<T> {
    pub id: T,
}

#[test]
fn transparent_generic_roundtrip() {
    let original: IdGenericTransparent<u64> = IdGenericTransparent { id: 12345 };
    let buf = <IdGenericTransparent<u64> as ProtoEncode>::encode_to_vec(&original);
    // For a transparent wrapper around u64, the encoding should just be the varint
    // Check that we get the correct varint encoding of 12345
    println!("Encoded buffer: {:?}, len: {}", buf, buf.len());

    // For a primitive type like u64, the transparent wrapper should encode
    // just the raw varint. Let's decode using the merge method which handles
    // primitives directly
    let mut decoded = <IdGenericTransparent<u64> as ProtoDefault>::proto_default();
    <IdGenericTransparent<u64> as ProtoDecoder>::merge(&mut decoded, WireType::Varint, &mut &buf[..], DecodeContext::default()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn transparent_generic_with_message_roundtrip() {
    let original: IdGenericTransparent<InnerMessage> = IdGenericTransparent {
        id: InnerMessage { value: 999 },
    };
    let buf = <IdGenericTransparent<InnerMessage> as ProtoEncode>::encode_to_vec(&original);

    let decoded = <IdGenericTransparent<InnerMessage> as ProtoDecode>::decode(&buf[..], DecodeContext::default()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn transparent_generic_merge_field_forwards_correctly() {
    // This test verifies that merge_field correctly forwards to the inner type
    // Previously, the code was checking tag == 1 which was wrong
    let original: IdGenericTransparent<InnerMessage> = IdGenericTransparent {
        id: InnerMessage { value: 42 },
    };
    let buf = <IdGenericTransparent<InnerMessage> as ProtoEncode>::encode_to_vec(&original);

    // Decode using merge_field explicitly
    let mut decoded = <IdGenericTransparent<InnerMessage> as ProtoDefault>::proto_default();
    let mut buf_slice = &buf[..];
    while buf_slice.has_remaining() {
        let (tag, wire_type) = proto_rs::encoding::decode_key(&mut buf_slice).unwrap();
        <IdGenericTransparent<InnerMessage> as ProtoDecoder>::merge_field(
            &mut decoded,
            tag,
            wire_type,
            &mut buf_slice,
            DecodeContext::default(),
        )
        .unwrap();
    }
    assert_eq!(decoded, original);
}
