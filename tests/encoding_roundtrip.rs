use bytes::{Bytes, BytesMut};
use prost::Message as ProstMessage;
use proto_rs::ProtoExt;
use proto_rs::encoding::varint::encoded_len_varint;
use proto_rs::encoding::{self};
use proto_rs::proto_message;

#[proto_message(proto_path = "protos/tests/encoding.proto")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum SampleEnum {
    #[default]
    Zero,
    One,
    Two,
}

#[proto_message(proto_path = "protos/tests/encoding.proto")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum StatusWithDefaultAttribute {
    Pending,
    #[default]
    Active,
    Inactive,
    Completed,
}

#[proto_message(proto_path = "protos/tests/encoding.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct NestedMessage {
    pub value: i64,
}

#[proto_message(proto_path = "protos/tests/encoding.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SampleMessage {
    pub id: u32,
    pub flag: bool,
    pub name: String,
    pub data: Vec<u8>,
    pub nested: Option<NestedMessage>,
    pub nested_list: Vec<NestedMessage>,
    pub values: Vec<i64>,
    pub mode: SampleEnum,
    pub optional_mode: Option<SampleEnum>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "compat")]
pub struct NestedMessageProst {
    #[prost(int64, tag = "1")]
    pub value: i64,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "compat")]
pub struct SampleMessageProst {
    #[prost(uint32, tag = "1")]
    pub id: u32,
    #[prost(bool, tag = "2")]
    pub flag: bool,
    #[prost(string, tag = "3")]
    pub name: String,
    #[prost(bytes, tag = "4")]
    pub data: Vec<u8>,
    #[prost(message, optional, tag = "5")]
    pub nested: Option<NestedMessageProst>,
    #[prost(message, repeated, tag = "6")]
    pub nested_list: Vec<NestedMessageProst>,
    #[prost(int64, repeated, tag = "7")]
    pub values: Vec<i64>,
    #[prost(enumeration = "SampleEnumProst", tag = "8")]
    pub mode: i32,
    #[prost(enumeration = "SampleEnumProst", optional, tag = "9")]
    pub optional_mode: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum SampleEnumProst {
    #[prost(enumeration = "SampleEnumProst")]
    Zero = 0,
    One = 1,
    Two = 2,
}

impl From<&NestedMessage> for NestedMessageProst {
    fn from(value: &NestedMessage) -> Self {
        Self { value: value.value }
    }
}

impl From<&NestedMessageProst> for NestedMessage {
    fn from(value: &NestedMessageProst) -> Self {
        Self { value: value.value }
    }
}

impl From<SampleEnum> for SampleEnumProst {
    fn from(value: SampleEnum) -> Self {
        match value {
            SampleEnum::Zero => SampleEnumProst::Zero,
            SampleEnum::One => SampleEnumProst::One,
            SampleEnum::Two => SampleEnumProst::Two,
        }
    }
}

impl From<SampleEnumProst> for SampleEnum {
    fn from(value: SampleEnumProst) -> Self {
        match value {
            SampleEnumProst::Zero => SampleEnum::Zero,
            SampleEnumProst::One => SampleEnum::One,
            SampleEnumProst::Two => SampleEnum::Two,
        }
    }
}

impl From<&SampleMessage> for SampleMessageProst {
    fn from(value: &SampleMessage) -> Self {
        Self {
            id: value.id,
            flag: value.flag,
            name: value.name.clone(),
            data: value.data.clone(),
            nested: value.nested.as_ref().map(NestedMessageProst::from),
            nested_list: value.nested_list.iter().map(NestedMessageProst::from).collect(),
            values: value.values.clone(),
            mode: SampleEnumProst::from(value.mode) as i32,
            optional_mode: value.optional_mode.map(|m| SampleEnumProst::from(m) as i32),
        }
    }
}

impl From<&SampleMessageProst> for SampleMessage {
    fn from(value: &SampleMessageProst) -> Self {
        Self {
            id: value.id,
            flag: value.flag,
            name: value.name.clone(),
            data: value.data.clone(),
            nested: value.nested.as_ref().map(NestedMessage::from),
            nested_list: value.nested_list.iter().map(NestedMessage::from).collect(),
            values: value.values.clone(),
            mode: SampleEnum::try_from(value.mode).expect("invalid enum value"),
            optional_mode: value.optional_mode.map(|m| SampleEnum::try_from(m).expect("invalid enum value")),
        }
    }
}

fn sample_message() -> SampleMessage {
    SampleMessage {
        id: 42,
        flag: true,
        name: "proto-rs".into(),
        data: vec![1, 2, 3, 4],
        nested: Some(NestedMessage { value: -7 }),
        nested_list: vec![NestedMessage { value: 11 }, NestedMessage { value: 23 }],
        values: vec![-1, 0, 1, 2],
        mode: SampleEnum::Two,
        optional_mode: Some(SampleEnum::One),
    }
}

fn sample_message_prost() -> SampleMessageProst {
    SampleMessageProst::from(&sample_message())
}

fn assert_decode_roundtrip(bytes: Bytes, proto_expected: &SampleMessage, prost_expected: &SampleMessageProst) {
    let decoded_proto = SampleMessage::decode(bytes.clone()).expect("proto decode failed");
    assert_eq!(decoded_proto, *proto_expected);

    let decoded_prost = SampleMessageProst::decode(bytes).expect("prost decode failed");
    assert_eq!(decoded_prost, *prost_expected);
}

fn encode_proto_message<M: ProtoExt>(value: &M) -> Bytes {
    let mut buf = BytesMut::with_capacity(value.encoded_len());
    value.encode(&mut buf).expect("proto encode failed");
    buf.freeze()
}

fn encode_prost_message<M: ProstMessage>(value: &M) -> Bytes {
    let mut buf = BytesMut::with_capacity(value.encoded_len());
    value.encode(&mut buf).expect("prost encode failed");
    buf.freeze()
}

fn encode_proto_length_delimited<M: ProtoExt>(value: &M) -> Bytes {
    let len = value.encoded_len();
    let mut buf = BytesMut::with_capacity(len + encoded_len_varint(len as u64));
    value.encode_length_delimited(&mut buf).expect("proto length-delimited encode failed");
    buf.freeze()
}

fn encode_prost_length_delimited<M: ProstMessage>(value: &M) -> Bytes {
    let len = value.encoded_len();
    let mut buf = BytesMut::with_capacity(len + encoded_len_varint(len as u64));
    value.encode_length_delimited(&mut buf).expect("prost length-delimited encode failed");
    buf.freeze()
}

#[test]
fn enum_default_attribute_maps_to_zero_discriminant() {
    assert_eq!(StatusWithDefaultAttribute::proto_default(), StatusWithDefaultAttribute::Active);
    assert_eq!(StatusWithDefaultAttribute::Active as i32, 0);
    assert_eq!(StatusWithDefaultAttribute::Pending as i32, 1);
    assert_eq!(StatusWithDefaultAttribute::Inactive as i32, 2);
    assert_eq!(StatusWithDefaultAttribute::Completed as i32, 3);

    let default_bytes = StatusWithDefaultAttribute::Active.encode_to_vec();
    assert!(default_bytes.is_empty(), "default enum variant must encode to empty payload");

    let pending_bytes = StatusWithDefaultAttribute::Pending.encode_to_vec();
    assert!(!pending_bytes.is_empty(), "non-default enum variant must encode field value");
    let decoded = StatusWithDefaultAttribute::decode(Bytes::from(pending_bytes)).expect("decode enum with explicit value");
    assert_eq!(decoded, StatusWithDefaultAttribute::Pending);
}

#[test]
fn proto_and_prost_encodings_are_equivalent() {
    let proto_msg = sample_message();
    let prost_msg = SampleMessageProst::from(&proto_msg);

    let proto_bytes = encode_proto_message(&proto_msg);
    let prost_bytes = encode_prost_message(&prost_msg);

    let prost_decoded_from_proto = SampleMessageProst::decode(proto_bytes.clone()).expect("prost decode from proto bytes failed");
    assert_eq!(prost_decoded_from_proto, prost_msg);

    let proto_decoded_from_prost = SampleMessage::decode(prost_bytes.clone()).expect("proto decode from prost bytes failed");
    assert_eq!(proto_decoded_from_prost, proto_msg);

    let normalized_prost = encode_prost_message(&prost_decoded_from_proto);
    assert_eq!(normalized_prost, prost_bytes, "prost re-encode mismatch");

    let normalized_proto = encode_proto_message(&proto_decoded_from_prost);
    assert_eq!(normalized_proto, proto_bytes, "proto re-encode mismatch");

    assert_eq!(proto_msg.encoded_len(), proto_bytes.len());
}

#[test]
fn cross_decode_round_trips() {
    let proto_msg = sample_message();
    let prost_msg = sample_message_prost();

    let proto_bytes = encode_proto_message(&proto_msg);
    let prost_bytes = encode_prost_message(&prost_msg);

    assert_decode_roundtrip(proto_bytes.clone(), &proto_msg, &prost_msg);
    assert_decode_roundtrip(prost_bytes, &proto_msg, &prost_msg);

    let decoded_proto_from_proto = SampleMessage::decode(proto_bytes.clone()).expect("proto decode failed");
    assert_eq!(decoded_proto_from_proto, proto_msg);
}

#[test]
fn length_delimited_round_trips() {
    let proto_msg = sample_message();
    let prost_msg = SampleMessageProst::from(&proto_msg);

    let proto_bytes = encode_proto_length_delimited(&proto_msg);
    let prost_bytes = encode_prost_length_delimited(&prost_msg);

    let decoded_proto = SampleMessage::decode_length_delimited(proto_bytes.clone()).expect("proto length-delimited decode failed");
    assert_eq!(decoded_proto, proto_msg);

    let decoded_proto_from_prost = SampleMessage::decode_length_delimited(prost_bytes.clone()).expect("proto decode from prost length-delimited failed");
    assert_eq!(decoded_proto_from_prost, proto_msg);

    let decoded_prost = SampleMessageProst::decode_length_delimited(prost_bytes.clone()).expect("prost length-delimited decode failed");
    assert_eq!(decoded_prost, prost_msg);

    let decoded_prost_from_proto = SampleMessageProst::decode_length_delimited(proto_bytes).expect("prost decode from proto length-delimited failed");
    assert_eq!(decoded_prost_from_proto, prost_msg);
}

#[test]
fn decode_handles_non_canonical_field_order() {
    let source = sample_message();
    let mut buf = BytesMut::new();

    encoding::int32::encode(8, &(SampleEnumProst::from(source.mode) as i32), &mut buf);
    encoding::bytes::encode(4, &source.data, &mut buf);
    encoding::int64::encode(7, &source.values[0], &mut buf);
    encoding::message::encode(6, &source.nested_list[0], &mut buf);
    encoding::string::encode(3, &source.name, &mut buf);
    encoding::bool::encode(2, &source.flag, &mut buf);
    encoding::uint32::encode(1, &source.id, &mut buf);
    encoding::message::encode(6, &source.nested_list[1], &mut buf);
    encoding::int64::encode(7, &source.values[1], &mut buf);
    encoding::message::encode(5, source.nested.as_ref().expect("missing nested"), &mut buf);
    encoding::int64::encode(7, &source.values[2], &mut buf);
    encoding::int64::encode(7, &source.values[3], &mut buf);
    if let Some(optional_mode) = source.optional_mode {
        encoding::int32::encode(9, &(SampleEnumProst::from(optional_mode) as i32), &mut buf);
    }

    let bytes = buf.freeze();
    assert_decode_roundtrip(bytes, &source, &SampleMessageProst::from(&source));
}

#[test]
fn decode_prefers_last_value_for_singular_fields() {
    let source = sample_message();
    let mut buf = BytesMut::new();

    // Initial values that should be overwritten.
    encoding::uint32::encode(1, &1u32, &mut buf);
    encoding::bool::encode(2, &false, &mut buf);
    encoding::int32::encode(8, &(SampleEnumProst::from(SampleEnum::One) as i32), &mut buf);
    if source.optional_mode.is_some() {
        encoding::int32::encode(9, &(SampleEnumProst::from(SampleEnum::Zero) as i32), &mut buf);
    }

    // Final values should be preserved after decoding.
    encoding::uint32::encode(1, &source.id, &mut buf);
    encoding::bool::encode(2, &source.flag, &mut buf);
    encoding::int32::encode(8, &(SampleEnumProst::from(source.mode) as i32), &mut buf);
    if let Some(optional_mode) = source.optional_mode {
        encoding::int32::encode(9, &(SampleEnumProst::from(optional_mode) as i32), &mut buf);
    }

    let bytes = buf.freeze();

    let expected = SampleMessage {
        id: source.id,
        flag: source.flag,
        mode: source.mode,
        optional_mode: source.optional_mode,
        ..SampleMessage::default()
    };

    assert_decode_roundtrip(bytes, &expected, &SampleMessageProst::from(&expected));
}

#[test]
fn decode_handles_mixed_packed_repeated_values() {
    let values = sample_message().values;

    let mut buf = BytesMut::new();
    encoding::int64::encode(7, &values[0], &mut buf);
    encoding::int64::encode_packed(7, &values[1..3], &mut buf);
    encoding::int64::encode(7, &values[3], &mut buf);

    let bytes = buf.freeze();

    let mut expected = SampleMessage::default();
    expected.values = values.clone();

    assert_decode_roundtrip(bytes, &expected, &SampleMessageProst::from(&expected));
}

#[test]
fn enum_discriminants_match_proto_requirements() {
    assert_eq!(SampleEnum::Zero as i32, 0);
    assert_eq!(SampleEnum::One as i32, 1);
    assert_eq!(SampleEnum::Two as i32, 2);
}
