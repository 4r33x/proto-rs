use proto_rs::DecodeContext;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::proto_message;

#[proto_message]
#[derive(Clone, Debug, Default, PartialEq)]
struct MixedTags {
    automatic: u32,
    #[proto(tag = 1)]
    explicit: u32,
    #[proto(skip)]
    skipped: u32,
    trailing: u32,
}

#[proto_message]
#[derive(Clone, Debug, PartialEq)]
enum TaggedChoice {
    #[proto(tag = 4)]
    First,
    Second(u32),
}

#[test]
fn runtime_uses_the_same_resolved_field_tags_as_schema_generation() {
    let value = MixedTags {
        automatic: 2,
        explicit: 1,
        skipped: 99,
        trailing: 3,
    };
    let encoded = value.encode_to_vec();

    assert_eq!(encoded, [0x10, 0x02, 0x08, 0x01, 0x18, 0x03]);
    assert_eq!(
        MixedTags::decode(&encoded[..], DecodeContext::default()).unwrap(),
        MixedTags { skipped: 0, ..value }
    );
}

#[test]
fn complex_enum_custom_and_automatic_tags_roundtrip() {
    for value in [TaggedChoice::First, TaggedChoice::Second(9)] {
        let encoded = value.encode_to_vec();
        assert_eq!(TaggedChoice::decode(&encoded[..], DecodeContext::default()).unwrap(), value);
    }
}
