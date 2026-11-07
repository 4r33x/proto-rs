#![allow(dead_code)]
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::hash::DefaultHasher;

use proto_rs::DecodeContext;
use proto_rs::DecodeError;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::proto_message;

#[proto_message]
struct SkipFirst(#[proto(skip)] u32, u64);
#[proto_message]
struct SkipMiddle(u64, #[proto(skip)] u32, u64);
#[proto_message]
struct SkipLast(u64, #[proto(skip)] u32);

#[test]
fn skipped_tuple_fields_use_compact_shadow_positions() {
    assert_eq!(SkipFirst(99, 7).encode_to_vec(), [8, 7]);
    assert_eq!(SkipMiddle(7, 99, 8).encode_to_vec(), [8, 7, 16, 8]);
    assert_eq!(SkipLast(7, 99).encode_to_vec(), [8, 7]);
    let restored = SkipMiddle::decode(&[8, 7, 16, 8][..], DecodeContext::default()).unwrap();
    assert_eq!((restored.0, restored.1, restored.2), (7, 0, 8));
}

#[test]
fn root_map_preserves_custom_hasher_type() {
    type Map = HashMap<u32, u32, BuildHasherDefault<DefaultHasher>>;
    let mut original = Map::default();
    original.insert(7, 9);
    let bytes = original.encode_to_vec();
    assert_eq!(Map::decode(bytes.as_slice(), DecodeContext::default()).unwrap(), original);
}

fn validate_pair(value: &Pair) -> Result<(), DecodeError> {
    if value.a == 0 || value.b == 0 {
        Err(DecodeError::new("both fields required"))
    } else {
        Ok(())
    }
}

#[proto_message]
#[proto(validator = validate_pair)]
#[derive(Debug)]
struct Pair {
    a: u32,
    b: u32,
}
#[proto_message]
struct Direct {
    pair: Pair,
}
#[proto_message]
struct Boxed {
    pair: Option<Box<Pair>>,
}
#[proto_message]
struct Repeated {
    pairs: Vec<Pair>,
}

const SPLIT: &[u8] = &[10, 2, 8, 1, 10, 2, 16, 2];

#[proto_message]
enum Choice {
    Empty,
    Pair(Pair),
    Fields { pair: Pair },
}

#[proto_message(transparent)]
struct Transparent(Pair);
#[proto_message]
struct Wrapped {
    pair: Transparent,
}

#[test]
fn validation_finalizes_through_oneofs_transparent_and_map_values() {
    let value = Choice::decode(&[18, 2, 8, 1, 18, 2, 16, 2][..], DecodeContext::default()).unwrap();
    assert!(matches!(value, Choice::Pair(Pair { a: 1, b: 2 })));
    let value = Choice::decode(&[26, 4, 10, 2, 8, 1, 26, 4, 10, 2, 16, 2][..], DecodeContext::default()).unwrap();
    assert!(matches!(value, Choice::Fields { pair: Pair { a: 1, b: 2 } }));
    let value = Wrapped::decode(SPLIT, DecodeContext::default()).unwrap();
    assert_eq!((value.pair.0.a, value.pair.0.b), (1, 2));
    let value = HashMap::<u32, Pair>::decode(&[10, 10, 8, 7, 18, 2, 8, 1, 18, 2, 16, 2][..], DecodeContext::default()).unwrap();
    assert_eq!((value[&7].a, value[&7].b), (1, 2));
}

thread_local! { static CONVERSIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
fn clone_data(value: &String) -> String {
    CONVERSIONS.set(CONVERSIONS.get() + 1);
    value.clone()
}
#[proto_message]
struct Converted {
    #[proto(into = "String", into_fn = "clone_data")]
    data: String,
}

#[test]
fn size_hints_do_not_run_field_conversions() {
    let values = vec![Converted { data: "payload".into() }];
    CONVERSIONS.set(0);
    std::hint::black_box(values[0].size_hint::<0>());
    assert_eq!(CONVERSIONS.get(), 0);
    std::hint::black_box(values.to_encoded_snapshot());
    assert_eq!(CONVERSIONS.get(), 1);
}

#[test]
fn nested_validation_runs_after_all_occurrences_merge() {
    let direct = Direct::decode(SPLIT, DecodeContext::default()).unwrap();
    assert_eq!((direct.pair.a, direct.pair.b), (1, 2));
    let boxed = Boxed::decode(SPLIT, DecodeContext::default()).unwrap().pair.unwrap();
    assert_eq!((boxed.a, boxed.b), (1, 2));
    assert!(Direct::decode(&[10, 0][..], DecodeContext::default()).is_err());
    assert!(Direct::decode(&[][..], DecodeContext::default()).is_ok());
    // Repeated occurrences are separate values, not fragments of one value.
    assert!(Repeated::decode(SPLIT, DecodeContext::default()).is_err());
}

#[cfg(feature = "arc_swap")]
#[test]
fn arc_swap_preserves_fragments_and_validates_after_merge() {
    #[proto_message]
    struct Swapped {
        pair: arc_swap::ArcSwap<Pair>,
    }
    #[proto_message]
    struct Optional {
        pair: arc_swap::ArcSwapOption<Pair>,
    }
    let value = Swapped::decode(SPLIT, DecodeContext::default()).unwrap();
    assert_eq!((value.pair.load().a, value.pair.load().b), (1, 2));
    let value = Optional::decode(SPLIT, DecodeContext::default()).unwrap();
    let pair = value.pair.load_full().unwrap();
    assert_eq!((pair.a, pair.b), (1, 2));
}

#[cfg(feature = "arc_swap")]
#[test]
fn arc_swap_forwards_array_cursors_and_preserves_shared_values_on_error() {
    use proto_rs::ProtoDecoder;
    #[proto_message]
    struct Array {
        values: [u32; 2],
    }
    #[proto_message]
    struct WrappedArray {
        values: arc_swap::ArcSwap<Array>,
    }
    let decoded = <WrappedArray as ProtoDecode>::decode(&[10, 2, 8, 1, 10, 2, 8, 2][..], DecodeContext::default()).unwrap();
    assert_eq!(decoded.values.load().values, [1, 2]);
    let mut shared = std::sync::Arc::new(Pair { a: 7, b: 9 });
    let observer = shared.clone();
    assert!(
        shared
            .merge(
                proto_rs::encoding::WireType::LengthDelimited,
                &mut &[2, 8, 1][..],
                DecodeContext::default()
            )
            .is_err()
    );
    assert_eq!((observer.a, shared.b), (7, 9));
    let mut swapped = arc_swap::ArcSwap::from(shared);
    assert!(
        swapped
            .merge(
                proto_rs::encoding::WireType::LengthDelimited,
                &mut &[2, 8, 1][..],
                DecodeContext::default()
            )
            .is_err()
    );
    assert_eq!((swapped.load().a, swapped.load().b), (7, 9));
}

#[cfg(feature = "fastnum")]
#[test]
fn malformed_decimals_return_errors_without_panicking() {
    for scale in [i32::MIN, i32::MAX, i16::MIN as i32 - 1, i16::MAX as i32 + 1] {
        let mut bytes = vec![8, 1];
        proto_rs::encoding::int32::encode_tagged(3, scale, &mut bytes);
        assert!(fastnum::D64::decode(bytes.as_slice(), DecodeContext::default()).is_err());
        assert!(fastnum::D128::decode(bytes.as_slice(), DecodeContext::default()).is_err());
        assert!(fastnum::UD128::decode(bytes.as_slice(), DecodeContext::default()).is_err());
    }
    assert!(fastnum::D64::decode(&[16, 1][..], DecodeContext::default()).is_err());
}
