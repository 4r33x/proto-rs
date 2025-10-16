#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_possible_truncation)]

use std::sync::Arc;

use bytes::Bytes;
use bytes::BytesMut;
use prost::Message as ProstMessage;
use proto_rs::ProtoExt;
use proto_rs::SingularField;
use proto_rs::encoding::varint::encoded_len_varint;
use proto_rs::encoding::{self};
use proto_rs::proto_message;

mod encoding_messages;

pub use encoding_messages::CollectionsMessage;
pub use encoding_messages::CollectionsMessageProst;
pub use encoding_messages::NestedMessage;
pub use encoding_messages::NestedMessageProst;
pub use encoding_messages::SampleEnum;
pub use encoding_messages::SampleEnumProst;
pub use encoding_messages::SampleMessage;
pub use encoding_messages::SampleMessageProst;
pub use encoding_messages::StatusWithDefaultAttribute;
pub use encoding_messages::sample_collections_messages as shared_sample_collections_messages;
pub use encoding_messages::sample_message as shared_sample_message;

#[proto_message(proto_path = "protos/tests/mixed_roundtrip.proto")]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ConversionInner {
    #[proto(tag = 1)]
    pub id: u64,
    #[proto(tag = 2)]
    pub label: String,
    #[proto(tag = 3)]
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FakeTime {
    seconds: i64,
}

#[proto_message(proto_path = "protos/tests/mixed_roundtrip.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MixedProto {
    #[proto(tag = 9)]
    pub name: String,
    pub raw: Vec<u8>,
    #[proto(tag = 11)]
    pub bytes_field: Bytes,
    #[proto(tag = 10)]
    pub optional_data: Option<Bytes>,
    #[proto(tag = 20)]
    pub optional_payload: Option<Vec<u8>>,
    #[proto(tag = 7)]
    pub attachments: Vec<Bytes>,
    #[proto(tag = 12, into = "i64", into_fn = "fake_time_to_i64", from_fn = "i64_to_fake_time")]
    pub timestamp: FakeTime,
    #[proto(tag = 4)]
    pub bools: Vec<bool>,
    #[proto(tag = 18)]
    pub byte_array: [u8; 4],
    #[proto(tag = 5)]
    pub optional_inner: Option<ConversionInner>,
    #[proto(tag = 6)]
    pub inner_list: Vec<ConversionInner>,
    #[proto(tag = 8)]
    pub fixed_inner: Vec<ConversionInner>,
    #[proto(tag = 15)]
    pub values: Vec<i32>,
    #[proto(tag = 25, skip)]
    pub cached: Vec<u8>,
    #[proto(tag = 30, skip = "rebuild_checksum")]
    pub checksum: u32,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "encoding")]
pub struct ConversionInnerProst {
    #[prost(uint64, tag = "1")]
    pub id: u64,
    #[prost(string, tag = "2")]
    pub label: String,
    #[prost(bytes = "vec", tag = "3")]
    pub payload: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "encoding")]
pub struct MixedProtoProst {
    #[prost(string, tag = "9")]
    pub name: String,
    #[prost(bytes = "vec", tag = "1")]
    pub raw: Vec<u8>,
    #[prost(bytes = "vec", tag = "11")]
    pub bytes_field: Vec<u8>,
    #[prost(bytes = "vec", optional, tag = "10")]
    pub optional_data: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "20")]
    pub optional_payload: Option<Vec<u8>>,
    #[prost(bytes = "vec", repeated, tag = "7")]
    pub attachments: Vec<Vec<u8>>,
    #[prost(int64, tag = "12")]
    pub timestamp: i64,
    #[prost(bool, repeated, tag = "4")]
    pub bools: Vec<bool>,
    #[prost(bytes = "vec", tag = "18")]
    pub byte_array: Vec<u8>,
    #[prost(message, optional, tag = "5")]
    pub optional_inner: Option<ConversionInnerProst>,
    #[prost(message, repeated, tag = "6")]
    pub inner_list: Vec<ConversionInnerProst>,
    #[prost(message, repeated, tag = "8")]
    pub fixed_inner: Vec<ConversionInnerProst>,
    #[prost(int32, repeated, tag = "15")]
    pub values: Vec<i32>,
}

impl From<&ConversionInner> for ConversionInnerProst {
    fn from(value: &ConversionInner) -> Self {
        Self {
            id: value.id,
            label: value.label.clone(),
            payload: value.payload.clone(),
        }
    }
}

impl From<&ConversionInnerProst> for ConversionInner {
    fn from(value: &ConversionInnerProst) -> Self {
        Self {
            id: value.id,
            label: value.label.clone(),
            payload: value.payload.clone(),
        }
    }
}

impl From<&MixedProto> for MixedProtoProst {
    fn from(value: &MixedProto) -> Self {
        Self {
            name: value.name.clone(),
            raw: value.raw.clone(),
            bytes_field: value.bytes_field.clone().to_vec(),
            optional_data: value.optional_data.as_ref().map(|b| b.clone().to_vec()),
            optional_payload: value.optional_payload.clone(),
            attachments: value.attachments.iter().map(|b| b.clone().to_vec()).collect(),
            timestamp: fake_time_to_i64(&value.timestamp),
            bools: value.bools.clone(),
            byte_array: value.byte_array.to_vec(),
            optional_inner: value.optional_inner.as_ref().map(ConversionInnerProst::from),
            inner_list: value.inner_list.iter().map(ConversionInnerProst::from).collect(),
            fixed_inner: value.fixed_inner.iter().map(ConversionInnerProst::from).collect(),
            values: value.values.clone(),
        }
    }
}

impl From<&MixedProtoProst> for MixedProto {
    fn from(value: &MixedProtoProst) -> Self {
        let mut byte_array = [0u8; 4];
        for (dst, src) in byte_array.iter_mut().zip(value.byte_array.iter().copied()) {
            *dst = src;
        }

        let mut message = Self {
            name: value.name.clone(),
            raw: value.raw.clone(),
            bytes_field: Bytes::from(value.bytes_field.clone()),
            optional_data: value.optional_data.as_ref().map(|b| Bytes::from(b.clone())),
            optional_payload: value.optional_payload.clone(),
            attachments: value.attachments.iter().map(|b| Bytes::from(b.clone())).collect(),
            timestamp: i64_to_fake_time(value.timestamp),
            bools: value.bools.clone(),
            byte_array,
            optional_inner: value.optional_inner.as_ref().map(ConversionInner::from),
            inner_list: value.inner_list.iter().map(ConversionInner::from).collect(),
            fixed_inner: value.fixed_inner.iter().map(ConversionInner::from).collect(),
            values: value.values.clone(),
            cached: Vec::new(),
            checksum: 0,
        };
        message.checksum = compute_checksum(&message);
        message
    }
}

fn fake_time_to_i64(value: &FakeTime) -> i64 {
    value.seconds
}

fn i64_to_fake_time(value: i64) -> FakeTime {
    FakeTime { seconds: value }
}

fn compute_checksum(value: &MixedProto) -> u32 {
    let mut acc = 0u32;
    acc = value.raw.iter().fold(acc, |sum, &b| sum.wrapping_add(b as u32));
    acc = acc.wrapping_add(value.bytes_field.len() as u32);
    if let Some(optional) = &value.optional_data {
        acc = acc.wrapping_add(optional.len() as u32);
    }
    if let Some(optional) = &value.optional_payload {
        acc = acc.wrapping_add(optional.len() as u32);
    }
    acc = acc.wrapping_add(value.attachments.iter().map(|b| b.len() as u32).fold(0, u32::wrapping_add));
    acc = acc.wrapping_add(value.bools.iter().filter(|&&b| b).count() as u32);
    acc = acc.wrapping_add(value.byte_array.iter().map(|&b| b as u32).fold(0, u32::wrapping_add));
    if let Some(inner) = &value.optional_inner {
        acc = acc.wrapping_add(inner.id as u32);
        acc = acc.wrapping_add(inner.label.len() as u32);
        acc = acc.wrapping_add(inner.payload.len() as u32);
    }
    acc = acc.wrapping_add(
        value
            .inner_list
            .iter()
            .map(|inner| inner.id as u32 + inner.label.len() as u32 + inner.payload.len() as u32)
            .fold(0, u32::wrapping_add),
    );
    acc = acc.wrapping_add(
        value
            .fixed_inner
            .iter()
            .map(|inner| inner.id as u32 + inner.label.len() as u32 + inner.payload.len() as u32)
            .fold(0, u32::wrapping_add),
    );
    acc = acc.wrapping_add(value.values.iter().fold(0, |sum, &v| sum.wrapping_add(v as u32)));
    acc = acc.wrapping_add(value.timestamp.seconds as u32);
    acc = acc.wrapping_add(value.name.len() as u32);
    acc
}

fn rebuild_checksum(value: &MixedProto) -> u32 {
    compute_checksum(value)
}

#[test]
fn collections_roundtrip() {
    let mut msg = CollectionsMessage::default();
    msg.hash_scores.insert(7, 42);
    msg.hash_scores.insert(1, -5);
    msg.tree_messages.insert("alice".to_string(), NestedMessage { value: 9 });
    msg.tree_messages.insert("bob".to_string(), NestedMessage { value: -11 });
    msg.hash_tags.insert("alpha".to_string());
    msg.hash_tags.insert("beta".to_string());
    msg.tree_ids.extend([3, 1, 8]);

    let bytes = encode_proto_message(&msg);
    let decoded = CollectionsMessage::decode(bytes.clone()).expect("decode collections message");

    assert_eq!(decoded.hash_scores, msg.hash_scores);
    assert_eq!(decoded.tree_messages, msg.tree_messages);
    assert_eq!(decoded.hash_tags, msg.hash_tags);
    assert_eq!(decoded.tree_ids, msg.tree_ids);
}

#[test]
fn collections_matches_prost_for_ordered_structures() {
    let mut msg = CollectionsMessage::default();
    msg.tree_messages.insert("carol".to_string(), NestedMessage { value: 123 });
    msg.tree_messages.insert("dave".to_string(), NestedMessage { value: -7 });
    msg.tree_ids.extend([10, 2, 5]);

    let proto_bytes = encode_proto_message(&msg);
    let decoded_prost = CollectionsMessageProst::decode(proto_bytes.clone()).expect("prost decode");
    assert_eq!(decoded_prost, CollectionsMessageProst::from(&msg));

    let prost_roundtrip = encode_prost_message(&CollectionsMessageProst::from(&msg));
    let decoded_proto = CollectionsMessage::decode(prost_roundtrip.clone()).expect("proto decode");
    assert_eq!(decoded_proto.tree_messages, msg.tree_messages);
    assert_eq!(decoded_proto.tree_ids, msg.tree_ids);
}

fn sample_mixed_proto() -> MixedProto {
    let mut message = MixedProto {
        name: "complex-roundtrip".to_string(),
        raw: vec![1, 2, 3, 4, 5],
        bytes_field: Bytes::from_static(b"proto-bytes"),
        optional_data: Some(Bytes::from_static(b"optional-bytes")),
        optional_payload: Some(vec![9, 8, 7]),
        attachments: vec![Bytes::from_static(b"alpha"), Bytes::from_static(b"beta")],
        timestamp: FakeTime { seconds: 42 },
        bools: vec![true, false, true],
        byte_array: [0xAA, 0xBB, 0xCC, 0xDD],
        optional_inner: Some(ConversionInner {
            id: 7,
            label: "optional".into(),
            payload: vec![1, 1, 2, 3],
        }),
        inner_list: vec![
            ConversionInner {
                id: 1,
                label: "first".into(),
                payload: vec![0, 1],
            },
            ConversionInner {
                id: 2,
                label: "second".into(),
                payload: vec![2, 3, 4],
            },
        ],
        fixed_inner: vec![
            ConversionInner {
                id: 10,
                label: "fixed-0".into(),
                payload: vec![0, 1],
            },
            ConversionInner {
                id: 11,
                label: "fixed-1".into(),
                payload: vec![1, 2],
            },
        ],
        values: vec![-5, 0, 5, 10],
        cached: Vec::new(),
        checksum: 0,
    };
    message.checksum = compute_checksum(&message);
    message
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
    encoding::message::encode::<NestedMessage>(6, &source.nested_list[0], &mut buf);
    encoding::string::encode(3, &source.name, &mut buf);
    encoding::bool::encode(2, &source.flag, &mut buf);
    encoding::uint32::encode(1, &source.id, &mut buf);
    encoding::message::encode::<NestedMessage>(6, &source.nested_list[1], &mut buf);
    encoding::int64::encode(7, &source.values[1], &mut buf);
    encoding::message::encode::<NestedMessage>(5, source.nested.as_ref().expect("missing nested"), &mut buf);
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
    #[allow(clippy::field_reassign_with_default)]
    {
        expected.values = values.clone();
    }

    assert_decode_roundtrip(bytes, &expected, &SampleMessageProst::from(&expected));
}

#[test]
fn mixed_proto_cross_roundtrip_with_prost() {
    let proto_msg = sample_mixed_proto();
    let prost_msg = MixedProtoProst::from(&proto_msg);
    let proto_from_prost = MixedProto::from(&prost_msg);
    assert_eq!(proto_from_prost, proto_msg);

    let proto_bytes = encode_proto_message(&proto_msg);
    let decoded_proto = MixedProto::decode(proto_bytes.clone()).expect("mixed proto decode failed");
    assert_eq!(decoded_proto, proto_msg);
    let decoded_prost = MixedProtoProst::decode(proto_bytes.clone()).expect("mixed prost decode from proto bytes failed");
    assert_eq!(decoded_prost, prost_msg);
    let reconverted_prost = MixedProtoProst::from(&decoded_proto);
    assert_eq!(reconverted_prost, prost_msg);

    let prost_bytes = encode_prost_message(&prost_msg);
    let decoded_proto_from_prost = MixedProto::decode(prost_bytes.clone()).expect("mixed proto decode from prost bytes failed");
    assert_eq!(decoded_proto_from_prost, proto_msg);
    let reconverted_prost_from_proto = MixedProtoProst::from(&decoded_proto_from_prost);
    assert_eq!(reconverted_prost_from_proto, prost_msg);
    let decoded_prost_from_prost = MixedProtoProst::decode(prost_bytes.clone()).expect("mixed prost decode failed");
    assert_eq!(decoded_prost_from_prost, prost_msg);
    let reconverted_proto = MixedProto::from(&decoded_prost_from_prost);
    assert_eq!(reconverted_proto, proto_msg);
}

#[test]
fn mixed_proto_skip_and_rebuild_behaviour() {
    let mut proto_msg = sample_mixed_proto();
    proto_msg.cached = vec![0xAA, 0xBB];
    proto_msg.checksum = 0;

    let bytes = encode_proto_message(&proto_msg);
    let decoded = MixedProto::decode(bytes).expect("mixed proto decode failed");

    assert!(decoded.cached.is_empty(), "skipped field should remain at default");
    assert_eq!(decoded.checksum, compute_checksum(&decoded), "checksum must be recomputed after decode");
}

#[test]
fn enum_discriminants_match_proto_requirements() {
    assert_eq!(SampleEnum::Zero as i32, 0);
    assert_eq!(SampleEnum::One as i32, 1);
    assert_eq!(SampleEnum::Two as i32, 2);
}

#[test]
fn merge_option_box_reuses_allocation() {
    let mut buf = BytesMut::new();
    encoding::message::encode::<NestedMessage>(1, &NestedMessage { value: 123 }, &mut buf);
    let mut bytes = buf.freeze();

    let (tag, wire_type) = encoding::decode_key(&mut bytes).expect("decode key");
    assert_eq!(tag, 1);
    assert_eq!(wire_type, encoding::WireType::LengthDelimited);

    let mut target = Some(Box::new(NestedMessage { value: 0 }));
    let ptr_before = target.as_ref().map(|b| &raw const **b).unwrap();

    <Box<NestedMessage> as SingularField>::merge_option_field(wire_type, &mut target, &mut bytes, encoding::DecodeContext::default()).expect("merge succeeded");

    let ptr_after = target.as_ref().map(|b| &raw const **b).unwrap();
    assert_eq!(ptr_before, ptr_after, "Box allocation should be reused");
    assert_eq!(target.as_ref().unwrap().value, 123);
}

#[test]
fn merge_option_arc_reuses_allocation() {
    let mut buf = BytesMut::new();
    encoding::message::encode::<NestedMessage>(1, &NestedMessage { value: 456 }, &mut buf);
    let mut bytes = buf.freeze();

    let (tag, wire_type) = encoding::decode_key(&mut bytes).expect("decode key");
    assert_eq!(tag, 1);
    assert_eq!(wire_type, encoding::WireType::LengthDelimited);

    let mut target = Some(Arc::new(NestedMessage { value: 0 }));
    let ptr_before = Arc::as_ptr(target.as_ref().unwrap());

    <Arc<NestedMessage> as SingularField>::merge_option_field(wire_type, &mut target, &mut bytes, encoding::DecodeContext::default()).expect("merge succeeded");

    let ptr_after = Arc::as_ptr(target.as_ref().unwrap());
    assert_eq!(ptr_before, ptr_after, "Arc allocation should be reused when unique");
    assert_eq!(target.as_ref().unwrap().value, 456);
}

#[test]
fn heap_wrappers_support_array_protoext() {
    fn assert_proto_ext<T: ProtoExt>() {}

    assert_proto_ext::<Box<[u8; 4]>>();
    assert_proto_ext::<Box<[NestedMessage; 2]>>();
    assert_proto_ext::<Arc<[u8; 4]>>();
    assert_proto_ext::<Arc<[NestedMessage; 2]>>();
}
