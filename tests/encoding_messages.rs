#![allow(clippy::must_use_candidate)]
#![allow(clippy::cast_possible_truncation)]
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use proto_rs::proto_message;

#[proto_message(proto_path = "protos/tests/encoding.proto")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum SampleEnum {
    #[default]
    Zero,
    One,
    Two,
    Unspecified,
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
pub struct ZeroCopyContainer {
    pub bytes32: [u8; 32],
    pub smalls: [u16; 32],
    pub nested_items: Vec<NestedMessage>,
    pub boxed: Option<Box<NestedMessage>>,
    pub shared: Option<Arc<NestedMessage>>,
    pub enum_lookup: HashMap<String, SampleEnum>,
}

#[proto_message(proto_path = "protos/tests/encoding.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ZeroCopyMessage {
    pub payload: NestedMessage,
}

#[proto_message(proto_path = "protos/tests/encoding.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SampleEnumList {
    pub values: Vec<SampleEnum>,
}

#[proto_message(proto_path = "protos/tests/encoding.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ZeroCopyEnumMessage {
    pub bag: SampleEnumList,
    pub status: SampleEnum,
    pub timeline: Vec<SampleEnum>,
}

#[proto_message(proto_path = "protos/tests/encoding.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ZeroCopyEnumContainer {
    pub direct: SampleEnum,
    pub raw_direct: SampleEnum,
    pub repeated_direct: Vec<SampleEnum>,
    pub raw_list: SampleEnumList,
    pub nested: ZeroCopyEnumMessage,
    pub nested_message: SampleMessage,
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

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "compat")]
pub struct ZeroCopyContainerProst {
    #[prost(bytes, tag = "1")]
    pub bytes32: Vec<u8>,
    #[prost(uint32, repeated, tag = "2")]
    pub smalls: Vec<u32>,
    #[prost(message, repeated, tag = "3")]
    pub nested_items: Vec<NestedMessageProst>,
    #[prost(message, optional, tag = "4")]
    pub boxed: Option<NestedMessageProst>,
    #[prost(message, optional, tag = "5")]
    pub shared: Option<NestedMessageProst>,
    #[prost(map = "string, enumeration(SampleEnumProst)", tag = "6")]
    pub enum_lookup: HashMap<String, i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum SampleEnumProst {
    #[prost(enumeration = "SampleEnumProst")]
    Zero = 0,
    One = 1,
    Two = 2,
    Unspecified = 3,
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
            SampleEnum::Unspecified => SampleEnumProst::Unspecified,
        }
    }
}

impl From<SampleEnumProst> for SampleEnum {
    fn from(value: SampleEnumProst) -> Self {
        match value {
            SampleEnumProst::Zero => SampleEnum::Zero,
            SampleEnumProst::One => SampleEnum::One,
            SampleEnumProst::Two => SampleEnum::Two,
            SampleEnumProst::Unspecified => SampleEnum::Unspecified,
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

impl From<&ZeroCopyContainer> for ZeroCopyContainerProst {
    fn from(value: &ZeroCopyContainer) -> Self {
        Self {
            bytes32: value.bytes32.as_slice().to_vec(),
            smalls: value.smalls.iter().map(|&entry| u32::from(entry)).collect(),
            nested_items: value.nested_items.iter().map(NestedMessageProst::from).collect(),
            boxed: value.boxed.as_ref().map(|boxed| NestedMessageProst::from(boxed.as_ref())),
            shared: value.shared.as_ref().map(|shared| NestedMessageProst::from(shared.as_ref())),
            enum_lookup: value.enum_lookup.iter().map(|(key, value)| (key.clone(), SampleEnumProst::from(*value) as i32)).collect(),
        }
    }
}

impl From<&ZeroCopyContainerProst> for ZeroCopyContainer {
    fn from(value: &ZeroCopyContainerProst) -> Self {
        let mut bytes32 = [0u8; 32];
        let copy_len = value.bytes32.len().min(32);
        bytes32[..copy_len].copy_from_slice(&value.bytes32[..copy_len]);

        let mut smalls = [0u16; 32];
        for (idx, entry) in value.smalls.iter().copied().enumerate().take(32) {
            smalls[idx] = u16::try_from(entry).expect("value must fit in u16");
        }

        let nested_items = value.nested_items.iter().map(NestedMessage::from).collect();
        let boxed = value.boxed.as_ref().map(|msg| Box::new(NestedMessage::from(msg)));
        let shared = value.shared.as_ref().map(|msg| Arc::new(NestedMessage::from(msg)));
        let enum_lookup = value
            .enum_lookup
            .iter()
            .map(|(key, raw)| {
                let prost = SampleEnumProst::try_from(*raw).expect("invalid enum value");
                (key.clone(), SampleEnum::from(prost))
            })
            .collect();

        Self {
            bytes32,
            smalls,
            nested_items,
            boxed,
            shared,
            enum_lookup,
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

pub fn zero_copy_fixture() -> ZeroCopyContainer {
    let mut container = ZeroCopyContainer::default();
    container.bytes32[..8].copy_from_slice(&[1, 3, 5, 7, 9, 11, 13, 15]);
    for (idx, slot) in container.smalls.iter_mut().enumerate() {
        *slot = ((idx as u16) + 1) * 2;
    }
    container.nested_items.push(NestedMessage { value: 256 });
    container.nested_items.push(NestedMessage { value: -512 });
    container.boxed = Some(Box::new(NestedMessage { value: 1024 }));
    container.shared = Some(Arc::new(NestedMessage { value: -2048 }));
    container.enum_lookup.insert("alpha".into(), SampleEnum::One);
    container.enum_lookup.insert("omega".into(), SampleEnum::Two);
    container
}

pub fn complex_enum_list_fixture() -> SampleEnumList {
    SampleEnumList {
        values: vec![SampleEnum::One, SampleEnum::Two],
    }
}

pub fn nested_complex_enum_list_fixture() -> SampleEnumList {
    SampleEnumList {
        values: vec![SampleEnum::Unspecified, SampleEnum::Two],
    }
}

pub fn zero_copy_enum_fixture() -> ZeroCopyEnumContainer {
    let outer_list = complex_enum_list_fixture();
    let nested_list = nested_complex_enum_list_fixture();

    let nested_message = ZeroCopyEnumMessage {
        bag: nested_list.clone(),
        status: SampleEnum::Two,
        timeline: nested_list.values.clone(),
    };

    ZeroCopyEnumContainer {
        direct: SampleEnum::Two,
        raw_direct: SampleEnum::Two,
        repeated_direct: vec![SampleEnum::One, SampleEnum::Two],
        raw_list: outer_list.clone(),
        nested: nested_message,
        nested_message: sample_message(),
    }
}
