#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::convert::TryInto;

use bytes::Bytes;
use prost::Message as ProstMessage;
use proto_rs::ProtoArchive;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::ProtoShadowEncode;
use proto_rs::RevWriter;
use proto_rs::encoding::DecodeContext;
use proto_rs::proto_message;

#[proto_message(proto_path = "protos/tests/advanced_features.proto")]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Stage {
    #[default]
    Unspecified,
    Alpha,
    Beta,
    Gamma,
}

#[proto_message(proto_path = "protos/tests/advanced_features.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AdvancedNested {
    pub value: i64,
    pub labels: Vec<String>,
}

#[proto_message(proto_path = "protos/tests/advanced_features.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub enum AdvancedOrigin {
    #[proto(tag = 1)]
    Raw(String),
    #[proto(tag = 2)]
    Nested(AdvancedNested),
    #[default]
    #[proto(tag = 3)]
    Missing,
}

#[proto_message(proto_path = "protos/tests/advanced_features.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub enum AdvancedComplexUnion {
    #[default]
    #[proto(tag = 1)]
    Unit,
    #[proto(tag = 2)]
    Named { label: String, count: i32 },
    #[proto(tag = 3)]
    Nested(AdvancedNested),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct AdvancedTimestamp {
    pub seconds: i64,
}

#[proto_message(proto_path = "protos/tests/advanced_features.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AdvancedEdgeCase {
    pub id: u64,
    pub note: Option<String>,
    pub blob: Bytes,
    pub zipped: Vec<i32>,
    pub stage_lookup: HashMap<String, Stage>,
    pub ordered_lookup: BTreeMap<String, Stage>,
    pub origin: AdvancedOrigin,
    pub nested: AdvancedNested,
    pub nested_list: Vec<AdvancedNested>,
    pub stage_history: Vec<Stage>,
    pub attachments: Vec<Bytes>,
    pub flag: bool,
    #[proto(tag = 15)]
    pub explicit_bytes: [u8; 8],
    #[proto(tag = 16, into = "i64", into_fn = "timestamp_to_i64", from_fn = "i64_to_timestamp")]
    pub event_time: AdvancedTimestamp,
    #[proto(tag = 17, skip = "recompute_digest")]
    pub digest: u32,
    #[proto(tag = 18)]
    pub optional_blob: Option<Bytes>,
}

const fn timestamp_to_i64(value: &AdvancedTimestamp) -> i64 {
    value.seconds
}

const fn i64_to_timestamp(value: i64) -> AdvancedTimestamp {
    AdvancedTimestamp { seconds: value }
}

fn compute_digest(value: &AdvancedEdgeCase) -> u32 {
    let mut acc = value.id as u32;

    if let Some(note) = &value.note {
        acc = acc.wrapping_add(note.len() as u32);
    }

    acc = acc.wrapping_mul(31).wrapping_add(value.blob.len() as u32);

    for byte in value.explicit_bytes {
        acc = acc.wrapping_mul(33).wrapping_add(byte as u32);
    }

    for chunk in &value.zipped {
        acc = acc.wrapping_mul(17).wrapping_add(*chunk as u32);
    }

    for stage in &value.stage_history {
        acc = acc.wrapping_add(stage_code(*stage));
    }

    for attachment in &value.attachments {
        acc = acc.wrapping_add(attachment.len() as u32);
    }

    if let Some(optional) = &value.optional_blob {
        acc = acc.wrapping_add(optional.len() as u32);
    }

    match &value.origin {
        AdvancedOrigin::Raw(text) => {
            acc = acc.wrapping_add(1);
            acc = acc.wrapping_add(text.len() as u32);
        }
        AdvancedOrigin::Nested(nested) => {
            acc = acc.wrapping_add(2);
            acc = acc.wrapping_add(nested.value as u32);
        }
        AdvancedOrigin::Missing => acc = acc.wrapping_add(3),
    }

    acc = acc.wrapping_add(value.nested.value as u32);

    for nested in &value.nested_list {
        acc = acc.wrapping_add(nested.value as u32);
    }

    for stage in value.stage_lookup.values() {
        acc = acc.wrapping_add(stage_code(*stage));
    }

    for stage in value.ordered_lookup.values() {
        acc = acc.wrapping_add(stage_code(*stage));
    }

    acc = acc.wrapping_add(value.event_time.seconds as u32);

    acc
}

fn recompute_digest(value: &AdvancedEdgeCase) -> u32 {
    compute_digest(value)
}

const fn stage_code(stage: Stage) -> u32 {
    match stage {
        Stage::Unspecified => 0,
        Stage::Alpha => 1,
        Stage::Beta => 2,
        Stage::Gamma => 3,
    }
}

impl From<Stage> for tonic_prost_test::advanced::Stage {
    fn from(value: Stage) -> Self {
        match value {
            Stage::Unspecified => tonic_prost_test::advanced::Stage::Unspecified,
            Stage::Alpha => tonic_prost_test::advanced::Stage::Alpha,
            Stage::Beta => tonic_prost_test::advanced::Stage::Beta,
            Stage::Gamma => tonic_prost_test::advanced::Stage::Gamma,
        }
    }
}

impl From<tonic_prost_test::advanced::Stage> for Stage {
    fn from(value: tonic_prost_test::advanced::Stage) -> Self {
        match value {
            tonic_prost_test::advanced::Stage::Unspecified => Stage::Unspecified,
            tonic_prost_test::advanced::Stage::Alpha => Stage::Alpha,
            tonic_prost_test::advanced::Stage::Beta => Stage::Beta,
            tonic_prost_test::advanced::Stage::Gamma => Stage::Gamma,
        }
    }
}

impl From<&AdvancedNested> for tonic_prost_test::advanced::AdvancedNested {
    fn from(value: &AdvancedNested) -> Self {
        Self {
            value: value.value,
            labels: value.labels.clone(),
        }
    }
}

impl From<&tonic_prost_test::advanced::AdvancedNested> for AdvancedNested {
    fn from(value: &tonic_prost_test::advanced::AdvancedNested) -> Self {
        Self {
            value: value.value,
            labels: value.labels.clone(),
        }
    }
}

impl From<&AdvancedOrigin> for tonic_prost_test::advanced::AdvancedOrigin {
    fn from(value: &AdvancedOrigin) -> Self {
        let value = if value.is_default() {
            None
        } else {
            match value {
                AdvancedOrigin::Raw(text) => Some(tonic_prost_test::advanced::advanced_origin::Value::Raw(text.clone())),
                AdvancedOrigin::Nested(nested) => Some(tonic_prost_test::advanced::advanced_origin::Value::Nested(
                    tonic_prost_test::advanced::AdvancedNested::from(nested),
                )),
                AdvancedOrigin::Missing => Some(tonic_prost_test::advanced::advanced_origin::Value::Missing(
                    tonic_prost_test::advanced::AdvancedOriginMissing {},
                )),
            }
        };

        Self { value }
    }
}

impl From<&tonic_prost_test::advanced::AdvancedOrigin> for AdvancedOrigin {
    fn from(value: &tonic_prost_test::advanced::AdvancedOrigin) -> Self {
        match value.value.as_ref() {
            Some(tonic_prost_test::advanced::advanced_origin::Value::Raw(raw)) => AdvancedOrigin::Raw(raw.clone()),
            Some(tonic_prost_test::advanced::advanced_origin::Value::Nested(nested)) => {
                let nested_value = AdvancedNested::from(nested);
                AdvancedOrigin::Nested(nested_value)
            }
            Some(tonic_prost_test::advanced::advanced_origin::Value::Missing(_)) | None => AdvancedOrigin::Missing,
        }
    }
}

impl From<&AdvancedComplexUnion> for tonic_prost_test::advanced::AdvancedComplexUnion {
    fn from(value: &AdvancedComplexUnion) -> Self {
        let value = if value.is_default() {
            None
        } else {
            match value {
                AdvancedComplexUnion::Unit => Some(tonic_prost_test::advanced::advanced_complex_union::Value::Unit(
                    tonic_prost_test::advanced::AdvancedComplexUnionUnit {},
                )),
                AdvancedComplexUnion::Named { label, count } => Some(tonic_prost_test::advanced::advanced_complex_union::Value::Named(
                    tonic_prost_test::advanced::AdvancedComplexUnionNamed {
                        label: label.clone(),
                        count: *count,
                    },
                )),
                AdvancedComplexUnion::Nested(nested) => Some(tonic_prost_test::advanced::advanced_complex_union::Value::Nested(
                    tonic_prost_test::advanced::AdvancedNested::from(nested),
                )),
            }
        };

        Self { value }
    }
}

impl From<&tonic_prost_test::advanced::AdvancedComplexUnion> for AdvancedComplexUnion {
    fn from(value: &tonic_prost_test::advanced::AdvancedComplexUnion) -> Self {
        match value.value.as_ref() {
            Some(tonic_prost_test::advanced::advanced_complex_union::Value::Unit(_)) | None => AdvancedComplexUnion::Unit,
            Some(tonic_prost_test::advanced::advanced_complex_union::Value::Named(named)) => AdvancedComplexUnion::Named {
                label: named.label.clone(),
                count: named.count,
            },
            Some(tonic_prost_test::advanced::advanced_complex_union::Value::Nested(nested)) => {
                AdvancedComplexUnion::Nested(AdvancedNested::from(nested))
            }
        }
    }
}

impl From<&AdvancedEdgeCase> for tonic_prost_test::advanced::AdvancedEdgeCase {
    fn from(value: &AdvancedEdgeCase) -> Self {
        let mut stage_lookup = HashMap::new();
        for (key, stage) in &value.stage_lookup {
            stage_lookup.insert(key.clone(), tonic_prost_test::advanced::Stage::from(*stage) as i32);
        }

        let mut ordered_lookup = HashMap::new();
        for (key, stage) in &value.ordered_lookup {
            ordered_lookup.insert(key.clone(), tonic_prost_test::advanced::Stage::from(*stage) as i32);
        }

        Self {
            id: value.id,
            note: value.note.clone(),
            blob: value.blob.clone().to_vec(),
            zipped: value.zipped.clone(),
            stage_lookup,
            ordered_lookup,
            origin: if value.origin.is_default() {
                None
            } else {
                Some(tonic_prost_test::advanced::AdvancedOrigin::from(&value.origin))
            },
            nested: if value.nested == AdvancedNested::default() {
                None
            } else {
                Some(tonic_prost_test::advanced::AdvancedNested::from(&value.nested))
            },
            nested_list: value.nested_list.iter().map(tonic_prost_test::advanced::AdvancedNested::from).collect(),
            stage_history: value.stage_history.iter().map(|stage| tonic_prost_test::advanced::Stage::from(*stage) as i32).collect(),
            attachments: value.attachments.iter().map(|bytes| bytes.clone().to_vec()).collect(),
            flag: value.flag,
            explicit_bytes: value.explicit_bytes.to_vec(),
            event_time: value.event_time.seconds,
            optional_blob: value.optional_blob.as_ref().map(|bytes| bytes.clone().to_vec()),
        }
    }
}

impl From<&tonic_prost_test::advanced::AdvancedEdgeCase> for AdvancedEdgeCase {
    fn from(value: &tonic_prost_test::advanced::AdvancedEdgeCase) -> Self {
        let stage_lookup: HashMap<String, Stage> = value
            .stage_lookup
            .iter()
            .map(|(key, stage)| {
                let stage = Stage::try_from(*stage).expect("invalid stage");
                (key.clone(), stage)
            })
            .collect();

        let ordered_lookup: BTreeMap<String, Stage> = value
            .ordered_lookup
            .iter()
            .map(|(key, stage)| {
                let stage = Stage::try_from(*stage).expect("invalid stage");
                (key.clone(), stage)
            })
            .collect();

        let attachments: Vec<Bytes> = value.attachments.iter().map(|bytes: &Vec<u8>| Bytes::from(bytes.clone())).collect();

        let nested = value.nested.as_ref().map(AdvancedNested::from).unwrap_or_default();

        let mut message = Self {
            id: value.id,
            note: value.note.clone(),
            blob: Bytes::from(value.blob.clone()),
            zipped: value.zipped.clone(),
            stage_lookup,
            ordered_lookup,
            origin: value.origin.as_ref().map_or(AdvancedOrigin::Missing, AdvancedOrigin::from),
            nested,
            nested_list: value.nested_list.iter().map(AdvancedNested::from).collect(),
            stage_history: value.stage_history.iter().map(|stage| Stage::try_from(*stage).expect("invalid stage history")).collect(),
            attachments,
            flag: value.flag,
            explicit_bytes: value
                .explicit_bytes
                .iter()
                .copied()
                .chain(std::iter::repeat(0))
                .take(8)
                .collect::<Vec<_>>()
                .try_into()
                .expect("exactly 8 bytes"),
            event_time: AdvancedTimestamp { seconds: value.event_time },
            digest: 0,
            optional_blob: value.optional_blob.as_ref().map(|bytes: &Vec<u8>| Bytes::from(bytes.clone())),
        };

        message.digest = compute_digest(&message);
        message
    }
}

fn sample_raw_message() -> AdvancedEdgeCase {
    let mut message = AdvancedEdgeCase {
        id: 42,
        note: Some("raw-origin".into()),
        blob: Bytes::from_static(b"primary-blob"),
        zipped: vec![-7, 0, 14, -21],
        stage_lookup: HashMap::from([("alpha".into(), Stage::Alpha), ("beta".into(), Stage::Beta)]),
        ordered_lookup: BTreeMap::from([("first".into(), Stage::Alpha), ("second".into(), Stage::Gamma)]),
        origin: AdvancedOrigin::Raw("ingress".into()),
        nested: AdvancedNested {
            value: 123,
            labels: vec!["root".into(), "branch".into()],
        },
        nested_list: vec![
            AdvancedNested {
                value: -1,
                labels: vec!["leaf".into()],
            },
            AdvancedNested {
                value: 88,
                labels: vec!["leaf".into(), "subleaf".into()],
            },
        ],
        stage_history: vec![Stage::Alpha, Stage::Beta, Stage::Gamma],
        attachments: vec![Bytes::from_static(b"a"), Bytes::from_static(b"bc")],
        flag: true,
        explicit_bytes: [1, 2, 3, 4, 5, 6, 7, 8],
        event_time: AdvancedTimestamp { seconds: 9001 },
        digest: 0,
        optional_blob: None,
    };
    message.digest = compute_digest(&message);
    message
}

fn sample_nested_message() -> AdvancedEdgeCase {
    let mut message = AdvancedEdgeCase {
        id: 1337,
        note: None,
        blob: Bytes::from_static(b"nested-blob"),
        zipped: vec![1, -2, 3, -4],
        stage_lookup: HashMap::from([("gamma".into(), Stage::Gamma)]),
        ordered_lookup: BTreeMap::from([("third".into(), Stage::Beta)]),
        origin: AdvancedOrigin::Nested(AdvancedNested {
            value: -512,
            labels: vec!["inner".into()],
        }),
        nested: AdvancedNested {
            value: 7,
            labels: vec!["root".into()],
        },
        nested_list: vec![AdvancedNested {
            value: 64,
            labels: vec!["child".into(), "desc".into()],
        }],
        stage_history: vec![Stage::Unspecified, Stage::Beta],
        attachments: vec![Bytes::from_static(b"xyz")],
        flag: false,
        explicit_bytes: [8, 7, 6, 5, 4, 3, 2, 1],
        event_time: AdvancedTimestamp { seconds: -2048 },
        digest: 0,
        optional_blob: None,
    };
    message.digest = compute_digest(&message);
    message
}

fn sample_missing_origin_message() -> AdvancedEdgeCase {
    let mut message = AdvancedEdgeCase {
        id: 7,
        note: Some("missing".into()),
        blob: Bytes::from_static(b"empty"),
        zipped: vec![],
        stage_lookup: HashMap::new(),
        ordered_lookup: BTreeMap::new(),
        origin: AdvancedOrigin::Missing,
        nested: AdvancedNested::default(),
        nested_list: Vec::new(),
        stage_history: vec![Stage::Unspecified],
        attachments: vec![],
        flag: false,
        explicit_bytes: [5; 8],
        event_time: AdvancedTimestamp { seconds: 0 },
        digest: 0,
        optional_blob: None,
    };
    message.digest = compute_digest(&message);
    message
}

fn sample_optional_blob_message() -> AdvancedEdgeCase {
    let mut message = AdvancedEdgeCase {
        id: 4096,
        note: Some("optional".into()),
        blob: Bytes::from_static(b"optional-blob"),
        zipped: vec![99, -42, 17],
        stage_lookup: HashMap::from([("delta".into(), Stage::Gamma)]),
        ordered_lookup: BTreeMap::from([("ordered".into(), Stage::Alpha)]),
        origin: AdvancedOrigin::Raw("side-channel".into()),
        nested: AdvancedNested {
            value: -256,
            labels: vec!["optional".into(), "blob".into()],
        },
        nested_list: vec![AdvancedNested {
            value: 32,
            labels: vec!["optional".into()],
        }],
        stage_history: vec![Stage::Gamma],
        attachments: vec![Bytes::from_static(b"payload")],
        flag: true,
        explicit_bytes: [9, 8, 7, 6, 5, 4, 3, 2],
        event_time: AdvancedTimestamp { seconds: 31415 },
        digest: 0,
        optional_blob: Some(Bytes::from_static(b"aux-bytes")),
    };

    message.digest = compute_digest(&message);
    message
}

#[allow(clippy::needless_pass_by_value)]
fn assert_roundtrip(message: AdvancedEdgeCase) {
    assert_eq!(message.digest, compute_digest(&message));

    let proto_bytes = AdvancedEdgeCase::encode_to_vec(&message);
    let decoded_proto =
        <AdvancedEdgeCase as ProtoDecode>::decode(proto_bytes.as_slice(), DecodeContext::default()).expect("decode proto edge case");
    assert_eq!(decoded_proto, message);

    let prost_message = tonic_prost_test::advanced::AdvancedEdgeCase::from(&message);
    let decoded_prost = tonic_prost_test::advanced::AdvancedEdgeCase::decode(proto_bytes.as_slice()).expect("decode prost edge case");
    assert_eq!(decoded_prost, prost_message);

    let prost_bytes = prost_message.encode_to_vec();
    let decoded_from_prost =
        <AdvancedEdgeCase as ProtoDecode>::decode(prost_bytes.as_slice(), DecodeContext::default()).expect("decode proto from prost bytes");
    assert_eq!(decoded_from_prost, message);
}

#[allow(clippy::needless_pass_by_value)]
fn assert_union_roundtrip(value: AdvancedComplexUnion) {
    let proto_bytes = AdvancedComplexUnion::encode_to_vec(&value);
    eprintln!("DEBUG: proto_bytes = {proto_bytes:?}");
    eprintln!("DEBUG: value = {value:?}");
    let decoded_proto =
        <AdvancedComplexUnion as ProtoDecode>::decode(proto_bytes.as_slice(), DecodeContext::default()).expect("decode proto union");
    assert_eq!(decoded_proto, value);

    let prost_value = tonic_prost_test::advanced::AdvancedComplexUnion::from(&value);
    let decoded_prost = tonic_prost_test::advanced::AdvancedComplexUnion::decode(proto_bytes.as_slice()).expect("decode prost union");
    assert_eq!(decoded_prost, prost_value);

    let prost_bytes = prost_value.encode_to_vec();
    let decoded_from_prost = <AdvancedComplexUnion as ProtoDecode>::decode(prost_bytes.as_slice(), DecodeContext::default())
        .expect("decode proto from prost union bytes");
    assert_eq!(decoded_from_prost, value);
}

#[test]
fn advanced_roundtrip_len_match_min() {
    let m = AdvancedNested {
        value: 1,
        labels: vec!["str".to_owned()],
    };
    let p: tonic_prost_test::advanced::AdvancedNested = (&m).into();
    let pl = p.encoded_len();
    let shadow = <<AdvancedNested as ProtoEncode>::Shadow<'_> as ProtoShadowEncode<'_, AdvancedNested>>::from_sun(&m);
    let mut writer = proto_rs::RevVec::with_capacity(64);
    <<AdvancedNested as ProtoEncode>::Shadow<'_> as ProtoArchive>::archive::<0>(&shadow, &mut writer);
    let rl = writer.len();
    let e = <AdvancedNested as ProtoEncode>::encode_to_vec(&m);
    let el = e.len();
    println!("Prost: {pl} Proto: {rl} Encoded: {el}");
    assert_eq!(pl, rl);
    assert_eq!(el, rl);
}

#[test]
fn advanced_roundtrip_len_match() {
    let m = sample_raw_message();
    let shadow = <<AdvancedEdgeCase as ProtoEncode>::Shadow<'_> as ProtoShadowEncode<'_, AdvancedEdgeCase>>::from_sun(&m);
    let mut writer = proto_rs::RevVec::with_capacity(64);
    <<AdvancedEdgeCase as ProtoEncode>::Shadow<'_> as ProtoArchive>::archive::<0>(&shadow, &mut writer);
    let len = writer.len();
    let e = <AdvancedEdgeCase as ProtoEncode>::encode_to_vec(&m);
    assert_eq!(e.len(), len);

    let missing = sample_missing_origin_message();
    let missing_shadow = <<AdvancedEdgeCase as ProtoEncode>::Shadow<'_> as ProtoShadowEncode<'_, AdvancedEdgeCase>>::from_sun(&missing);
    let mut missing_writer = proto_rs::RevVec::with_capacity(64);
    <<AdvancedEdgeCase as ProtoEncode>::Shadow<'_> as ProtoArchive>::archive::<0>(&missing_shadow, &mut missing_writer);
    let missing_len = missing_writer.len();
    let missing_bytes = <AdvancedEdgeCase as ProtoEncode>::encode_to_vec(&missing);
    assert_eq!(missing_bytes.len(), missing_len);
}

#[test]
fn advanced_roundtrip_handles_raw_origin() {
    assert_roundtrip(sample_raw_message());
}

#[test]
fn advanced_roundtrip_handles_nested_origin() {
    assert_roundtrip(sample_nested_message());
}

#[test]
fn advanced_roundtrip_handles_missing_origin() {
    assert_roundtrip(sample_missing_origin_message());
}

#[test]
fn advanced_optional_fields_match_between_impls() {
    let mut message = sample_raw_message();
    message.optional_blob = None;
    message.note = None;
    message.digest = compute_digest(&message);

    let prost_message = tonic_prost_test::advanced::AdvancedEdgeCase::from(&message);
    assert!(prost_message.optional_blob.is_none());
    assert!(prost_message.note.is_none());

    let encoded = AdvancedEdgeCase::encode_to_vec(&message);
    let decoded_prost = tonic_prost_test::advanced::AdvancedEdgeCase::decode(encoded.as_slice()).expect("decode prost optional");
    assert_eq!(decoded_prost, prost_message);
}

#[test]
fn advanced_optional_blob_roundtrip_preserves_digest() {
    let message = sample_optional_blob_message();
    let expected_blob = message.optional_blob.clone().expect("fixture provides optional blob");

    let prost_message = tonic_prost_test::advanced::AdvancedEdgeCase::from(&message);
    assert_eq!(prost_message.optional_blob.as_deref(), Some(expected_blob.as_ref()));

    assert_roundtrip(message);
}

#[test]
fn advanced_complex_enum_roundtrip_variants() {
    assert_union_roundtrip(AdvancedComplexUnion::Named {
        label: "alpha".into(),
        count: 7,
    });

    assert_union_roundtrip(AdvancedComplexUnion::Nested(AdvancedNested {
        value: 21,
        labels: vec!["named".into(), "nested".into()],
    }));

    assert_union_roundtrip(AdvancedComplexUnion::Unit);
}

#[test]
fn advanced_complex_enum_preserves_default_payloads() {
    assert_union_roundtrip(AdvancedComplexUnion::Nested(AdvancedNested::default()));

    assert_union_roundtrip(AdvancedComplexUnion::Named {
        label: String::new(),
        count: 0,
    });
}

#[test]
fn advanced_complex_enum_default_encodes_as_absent_value() {
    let default_union = AdvancedComplexUnion::default();

    let prost_value = tonic_prost_test::advanced::AdvancedComplexUnion::from(&default_union);
    assert!(prost_value.value.is_none());

    let encoded = AdvancedComplexUnion::encode_to_vec(&default_union);
    let decoded_prost = tonic_prost_test::advanced::AdvancedComplexUnion::decode(encoded.as_slice()).expect("decode prost default union");
    assert!(decoded_prost.value.is_none());

    let decoded_proto =
        <AdvancedComplexUnion as ProtoDecode>::decode(encoded.as_slice(), DecodeContext::default()).expect("decode proto default union");
    assert_eq!(decoded_proto, default_union);
}
