use bytes::Bytes;
use prost::Message;
use proto_rs::proto_message;
use proto_rs::HasProto;

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ExternalMessage {
    #[prost(uint32, tag = 1)]
    pub value: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ::prost::Enumeration)]
#[repr(i32)]
pub enum ExternalStatus {
    Idle = 0,
    Busy = 1,
}

#[proto_message(proto_path = "protos/tests/roundtrip.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct Nested {
    #[proto(tag = 7)]
    pub manual: i64,
    pub optional_name: Option<String>,
    pub fixed_bytes: [u8; 4],
    pub values: Vec<u32>,
    #[proto(message)]
    pub external: Option<ExternalMessage>,
    pub blob: Bytes,
}

#[proto_message(proto_path = "protos/tests/roundtrip.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct RoundTrip {
    #[proto(tag = 5)]
    pub manual_id: u32,
    pub automatic: bool,
    #[proto(into = "String", into_fn = "bytes_to_hex", from_fn = "hex_to_bytes")]
    pub hash: [u8; 4],
    #[proto(skip)]
    pub skipped: i32,
    #[proto(skip = "compute_magic")]
    pub computed: u32,
    pub nested: Nested,
    pub nested_list: Vec<Nested>,
    pub data: Vec<u32>,
    pub payload: Bytes,
    #[proto(enum)]
    pub ext_status: ExternalStatus,
    #[proto(enum)]
    pub opt_status: Option<ExternalStatus>,
    #[proto(enum)]
    pub many_status: Vec<ExternalStatus>,
    pub raw_bytes: Vec<u8>,
}

#[proto_message(proto_path = "protos/tests/roundtrip.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct PackedVec {
    pub values: Vec<u32>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PackedVecReference {
    #[prost(uint32, repeated, packed = "true", tag = 1)]
    pub values: Vec<u32>,
}

#[proto_message(proto_path = "protos/tests/roundtrip.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct ManualTag {
    #[proto(tag = 9)]
    pub custom: u32,
    pub auto: u32,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ManualTagReference {
    #[prost(uint32, tag = 9)]
    pub custom: u32,
    #[prost(uint32, tag = 1)]
    pub auto: u32,
}

fn compute_magic(_proto: &RoundTripProto) -> u32 {
    999
}

fn bytes_to_hex(value: &[u8; 4]) -> String {
    value.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_to_bytes(value: String) -> [u8; 4] {
    let mut bytes = [0u8; 4];
    for (i, chunk) in value.as_bytes().chunks(2).enumerate().take(4) {
        bytes[i] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap();
    }
    bytes
}

#[test]
fn roundtrip_struct_conversions() {
    let nested = Nested {
        manual: 42,
        optional_name: Some("nested".to_string()),
        fixed_bytes: [1, 2, 3, 4],
        values: vec![3, 4, 5],
        external: Some(ExternalMessage { value: 77 }),
        blob: Bytes::from_static(b"blob-data"),
    };

    let original = RoundTrip {
        manual_id: 123,
        automatic: true,
        hash: [0xAA, 0xBB, 0xCC, 0xDD],
        skipped: 0,
        computed: 999,
        nested: nested.clone(),
        nested_list: vec![nested.clone(), nested],
        data: vec![10, 20, 30],
        payload: Bytes::from_static(b"payload"),
        ext_status: ExternalStatus::Busy,
        opt_status: Some(ExternalStatus::Idle),
        many_status: vec![ExternalStatus::Idle, ExternalStatus::Busy],
        raw_bytes: vec![9, 8, 7],
    };

    let proto = original.to_proto();
    let restored = RoundTrip::from_proto(proto).expect("roundtrip conversion failed");

    assert_eq!(restored, original);
}

#[test]
fn packed_repeated_fields_are_compatible() {
    let packed = PackedVec {
        values: vec![1, 2, 3, 4],
    };

    let bytes = packed.to_proto().encode_to_vec();
    let decoded = PackedVecReference::decode(bytes.as_slice()).expect("decode packed reference");
    assert_eq!(decoded.values, packed.values);

    let reference = PackedVecReference {
        values: vec![10, 20, 30],
    };
    let reference_bytes = reference.encode_to_vec();
    let proto = PackedVecProto::decode(reference_bytes.as_slice()).expect("decode into proto");
    let restored = PackedVec::from_proto(proto).expect("from_proto for packed vec");
    assert_eq!(restored.values, reference.values);
}

#[test]
fn custom_tag_matches_reference() {
    let message = ManualTag {
        custom: 55,
        auto: 88,
    };

    let bytes = message.to_proto().encode_to_vec();
    let decoded = ManualTagReference::decode(bytes.as_slice()).expect("decode manual tag reference");
    assert_eq!(decoded.custom, 55);
    assert_eq!(decoded.auto, 88);

    let reference = ManualTagReference {
        custom: 101,
        auto: 202,
    };
    let reference_bytes = reference.encode_to_vec();
    let proto = ManualTagProto::decode(reference_bytes.as_slice()).expect("decode into macro proto");
    let restored = ManualTag::from_proto(proto).expect("from_proto for manual tag");
    assert_eq!(restored.custom, 101);
    assert_eq!(restored.auto, 202);
}
