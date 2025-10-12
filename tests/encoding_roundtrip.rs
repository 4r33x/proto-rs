use prost::Message as ProstMessage;
use proto_rs::proto_message;
use proto_rs::ProtoExt;

#[proto_message(proto_path = "protos/tests/encoding.proto")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum SampleEnum {
    #[default]
    Zero,
    One,
    Two,
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
    #[prost(int64, repeated, tag = "7", packed = "false")]
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
            optional_mode: value
                .optional_mode
                .map(|m| SampleEnum::try_from(m).expect("invalid enum value")),
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

fn collect_bytes<M: ProstMessage>(value: &M) -> Vec<u8> {
    let mut buf = Vec::with_capacity(value.encoded_len());
    value.encode(&mut buf).expect("prost encode failed");
    buf
}

#[test]
fn proto_and_prost_wire_formats_match() {
    let proto_msg = sample_message();
    let prost_msg = SampleMessageProst::from(&proto_msg);

    let proto_bytes = proto_msg.encode_to_vec();
    let prost_bytes = collect_bytes(&prost_msg);

    assert_eq!(proto_bytes, prost_bytes, "wire format diverged between proto_rs and prost");
}

#[test]
fn cross_decode_round_trips() {
    let proto_msg = sample_message();
    let prost_msg = sample_message_prost();

    let proto_bytes = proto_msg.encode_to_vec();
    let prost_bytes = collect_bytes(&prost_msg);

    let decoded_proto_from_proto = SampleMessage::decode(proto_bytes.as_slice()).expect("proto decode failed");
    assert_eq!(decoded_proto_from_proto, proto_msg);

    let decoded_proto_from_prost = SampleMessage::decode(prost_bytes.as_slice()).expect("proto decode from prost bytes failed");
    assert_eq!(decoded_proto_from_prost, proto_msg);

    let decoded_prost_from_prost = SampleMessageProst::decode(prost_bytes.as_slice()).expect("prost decode failed");
    assert_eq!(decoded_prost_from_prost, prost_msg);

    let decoded_prost_from_proto = SampleMessageProst::decode(proto_bytes.as_slice()).expect("prost decode from proto bytes failed");
    assert_eq!(decoded_prost_from_proto, prost_msg);
}

#[test]
fn length_delimited_round_trips() {
    let proto_msg = sample_message();
    let prost_msg = SampleMessageProst::from(&proto_msg);

    let proto_bytes = proto_msg.encode_length_delimited_to_vec();
    let prost_bytes = {
        let mut buf = Vec::new();
        prost_msg
            .encode_length_delimited(&mut buf)
            .expect("prost length-delimited encode failed");
        buf
    };

    assert_eq!(proto_bytes, prost_bytes, "length-delimited wire format diverged");

    let decoded_proto = SampleMessage::decode_length_delimited(proto_bytes.as_slice()).expect("proto length-delimited decode failed");
    assert_eq!(decoded_proto, proto_msg);

    let decoded_proto_from_prost =
        SampleMessage::decode_length_delimited(prost_bytes.as_slice()).expect("proto decode from prost length-delimited failed");
    assert_eq!(decoded_proto_from_prost, proto_msg);

    let decoded_prost = SampleMessageProst::decode_length_delimited(prost_bytes.as_slice())
        .expect("prost length-delimited decode failed");
    assert_eq!(decoded_prost, prost_msg);

    let decoded_prost_from_proto = SampleMessageProst::decode_length_delimited(proto_bytes.as_slice())
        .expect("prost decode from proto length-delimited failed");
    assert_eq!(decoded_prost_from_proto, prost_msg);
}

#[test]
fn enum_discriminants_match_proto_requirements() {
    assert_eq!(SampleEnum::Zero as i32, 0);
    assert_eq!(SampleEnum::One as i32, 1);
    assert_eq!(SampleEnum::Two as i32, 2);
}
