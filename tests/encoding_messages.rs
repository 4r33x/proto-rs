#![allow(clippy::must_use_candidate)]
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;

use proto_rs::ProtoExt;
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

#[proto_message(proto_path = "protos/tests/encoding.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CollectionsMessage {
    pub hash_scores: HashMap<u32, i64>,
    pub tree_messages: BTreeMap<String, NestedMessage>,
    pub hash_tags: HashSet<String>,
    pub tree_ids: BTreeSet<i32>,
}

#[proto_message(proto_path = "protos/tests/encoding.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub enum ScalarCounter {
    #[default]
    Empty,
    #[proto(tag = 2)]
    Count { count: u32 },
}

#[test]
fn scalar_counter_roundtrip() {
    let original = ScalarCounter::Count { count: 42 };
    let encoded = original.encode_to_vec();
    let decoded = ScalarCounter::decode(encoded.as_slice()).expect("decode scalar counter");
    assert_eq!(decoded, original);
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

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "compat")]
pub struct CollectionsMessageProst {
    #[prost(map = "uint32, int64", tag = "1")]
    pub hash_scores: HashMap<u32, i64>,
    #[prost(map = "string, message", tag = "2")]
    pub tree_messages: HashMap<String, NestedMessageProst>,
    #[prost(string, repeated, tag = "3")]
    pub hash_tags: Vec<String>,
    #[prost(int32, repeated, tag = "4")]
    pub tree_ids: Vec<i32>,
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

impl From<&CollectionsMessage> for CollectionsMessageProst {
    fn from(value: &CollectionsMessage) -> Self {
        Self {
            hash_scores: value.hash_scores.clone(),
            tree_messages: value.tree_messages.iter().map(|(key, msg)| (key.clone(), NestedMessageProst::from(msg))).collect(),
            hash_tags: value.hash_tags.iter().cloned().collect(),
            tree_ids: value.tree_ids.iter().copied().collect(),
        }
    }
}

impl From<&CollectionsMessageProst> for CollectionsMessage {
    fn from(value: &CollectionsMessageProst) -> Self {
        Self {
            hash_scores: value.hash_scores.clone(),
            tree_messages: value.tree_messages.iter().map(|(key, msg)| (key.clone(), NestedMessage::from(msg))).collect::<BTreeMap<_, _>>(),
            hash_tags: value.hash_tags.iter().cloned().collect(),
            tree_ids: value.tree_ids.iter().copied().collect(),
        }
    }
}

pub fn sample_message() -> SampleMessage {
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

pub fn sample_collections_messages() -> Vec<CollectionsMessage> {
    let mut first = CollectionsMessage::default();
    first.hash_scores.insert(7, 42);
    first.hash_scores.insert(1, -5);
    first.tree_messages.insert("alice".to_string(), NestedMessage { value: 9 });
    first.tree_messages.insert("bob".to_string(), NestedMessage { value: -11 });
    first.hash_tags.insert("alpha".to_string());
    first.hash_tags.insert("beta".to_string());
    first.tree_ids.extend([3, 1, 8]);

    let mut second = CollectionsMessage::default();
    second.hash_scores.insert(10, 100);
    second.tree_messages.insert("carol".to_string(), NestedMessage { value: 123 });
    second.hash_tags.insert("gamma".to_string());
    second.hash_tags.insert("delta".to_string());
    second.tree_ids.extend([2, 4, 6]);

    vec![first, second]
}
