// These owned wrapper combinations intentionally exercise the former layout-cast bug.
#![allow(clippy::vec_box)]

use std::collections::BTreeSet;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;

use proto_rs::DecodeContext;
use proto_rs::ProtoDecode;
use proto_rs::ProtoDecoder;
use proto_rs::ProtoEncode;
use proto_rs::ProtoExt;
use proto_rs::proto_message;

#[proto_message]
#[derive(Debug, PartialEq)]
struct Numbers {
    values: [u32; 2],
}

#[proto_message]
#[derive(Debug, PartialEq)]
struct Strings {
    values: [String; 2],
}

#[proto_message]
#[derive(Debug, PartialEq)]
struct Nested {
    value: Option<Box<Numbers>>,
}

#[proto_message]
#[derive(Debug, PartialEq)]
struct Pair {
    a: u32,
    b: u32,
}

#[proto_message]
#[derive(Debug, PartialEq)]
enum Choice {
    Empty,
    Pair(Pair),
    Fields { a: u32, b: u32 },
    Array { values: [u32; 2] },
}

#[proto_message]
struct Locked {
    first: Arc<Mutex<u32>>,
    second: Arc<Mutex<u32>>,
}

#[proto_message]
#[derive(Debug, PartialEq)]
struct Sets {
    tree: BTreeSet<u8>,
    hash: HashSet<u8>,
}

#[proto_message]
#[derive(Debug, PartialEq)]
struct WrappedBytes {
    values: Vec<Box<u8>>,
    deque: VecDeque<Option<u8>>,
    array: [Box<u8>; 2],
}

#[proto_message]
#[derive(Debug, PartialEq)]
struct MessageArray {
    values: [Pair; 2],
}

#[proto_message]
#[derive(Debug, PartialEq)]
struct TwoArrays {
    first: [u32; 2],
    second: [u32; 2],
}

fn decode<T: ProtoDecode>(bytes: &[u8]) -> T {
    T::decode(bytes, DecodeContext::default()).unwrap()
}

#[allow(clippy::needless_pass_by_value)]
fn roundtrip<T: ProtoEncode + ProtoExt + ProtoDecode + PartialEq + std::fmt::Debug>(value: T) {
    let wire = value.encode_to_vec();
    let decoded = T::decode(wire.as_slice(), DecodeContext::default())
        .unwrap_or_else(|error| panic!("{}: {wire:?}: {error}", std::any::type_name::<T>()));
    assert_eq!(decoded, value);
}

#[test]
fn arrays_accept_packed_unpacked_and_split_occurrences() {
    for wire in [
        vec![0x0a, 2, 1, 2],
        vec![0x08, 1, 0x08, 2],
        vec![0x0a, 1, 1, 0x0a, 1, 2],
        vec![0x08, 1, 0x18, 99, 0x0a, 1, 2],
    ] {
        assert_eq!(decode::<Numbers>(&wire).values, [1, 2]);
        assert_eq!(
            <Numbers as ProtoDecoder>::decode(wire.as_slice(), DecodeContext::default()).unwrap().values,
            [1, 2]
        );
    }
    assert!(<Numbers as ProtoDecode>::decode(&[0x0a, 1, 1, 0x0a, 2, 2, 3][..], DecodeContext::default()).is_err());
    roundtrip(Strings {
        values: ["a".into(), "b".into()],
    });
    roundtrip([1u32, 2]);
    roundtrip(["a".to_owned(), "b".to_owned()]);
    roundtrip(MessageArray {
        values: [Pair { a: 1, b: 2 }, Pair { a: 3, b: 4 }],
    });
    assert_eq!(
        decode::<TwoArrays>(&[8, 1, 16, 3, 8, 2, 16, 4]),
        TwoArrays {
            first: [1, 2],
            second: [3, 4]
        }
    );
}

#[test]
fn array_cursors_survive_repeated_nested_messages() {
    let nested = decode::<Nested>(&[0x0a, 3, 0x0a, 1, 1, 0x0a, 3, 0x0a, 1, 2]);
    assert_eq!(nested.value.unwrap().values, [1, 2]);
    assert_eq!(
        decode::<Choice>(&[0x22, 3, 0x0a, 1, 1, 0x22, 3, 0x0a, 1, 2]),
        Choice::Array { values: [1, 2] }
    );
}

#[test]
fn oneof_merges_same_variant_and_clears_switched_variants() {
    assert_eq!(
        decode::<Choice>(&[0x12, 2, 8, 1, 0x12, 2, 16, 2]),
        Choice::Pair(Pair { a: 1, b: 2 })
    );
    assert_eq!(decode::<Choice>(&[0x1a, 2, 8, 1, 0x1a, 2, 16, 2]), Choice::Fields { a: 1, b: 2 });
    assert_eq!(
        decode::<Choice>(&[0x12, 2, 8, 1, 0x0a, 0, 0x12, 2, 16, 2]),
        Choice::Pair(Pair { a: 0, b: 2 })
    );
    assert_eq!(
        decode::<Choice>(&[0x22, 3, 0x0a, 1, 1, 0x0a, 0, 0x22, 3, 0x0a, 1, 2]),
        Choice::Array { values: [2, 0] }
    );
}

#[test]
fn aliased_mutex_fields_do_not_deadlock() {
    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let shared = Arc::new(Mutex::new(150));
        let value = Locked {
            first: shared.clone(),
            second: shared,
        };
        tx.send(value.encode_to_vec()).unwrap();
    });
    let wire = rx.recv_timeout(Duration::from_secs(3)).expect("encoding deadlocked");
    worker.join().unwrap();
    assert_eq!(wire, [8, 150, 1, 16, 150, 1]);
}

#[test]
fn byte_sets_use_varints_consistently() {
    let tree: BTreeSet<u8> = [0, 127, 128, 255].into_iter().collect();
    let hash: HashSet<u8> = tree.iter().copied().collect();
    roundtrip(Sets {
        tree: tree.clone(),
        hash: hash.clone(),
    });
    assert_eq!(tree.encode_to_vec(), [10, 6, 0, 127, 128, 1, 255, 1]);
    roundtrip(tree);
    roundtrip(hash);
}

#[test]
fn byte_wrappers_are_encoded_as_elements_without_layout_casts() {
    roundtrip(WrappedBytes {
        values: vec![Box::new(128), Box::new(255)],
        deque: [Some(128), Some(255)].into_iter().collect(),
        array: [Box::new(128), Box::new(255)],
    });
    roundtrip(vec![Box::new(128u8), Box::new(255)]);
    roundtrip(VecDeque::from([Box::new(128u8), Box::new(255)]));
    roundtrip(vec![0u8, 128, 255]);
    roundtrip([0u8, 128, 255]);
}

#[test]
fn top_level_values_have_consistent_framing() {
    assert_eq!(150u32.encode_to_vec(), [8, 150, 1]);
    roundtrip(150u32);
    roundtrip(0u32);
    roundtrip(-1i64);
    roundtrip(255u8);
    roundtrip(true);
    roundtrip(1.25f64);
    roundtrip("hello".to_owned());
    roundtrip(Some(0u32));
    roundtrip(vec![1u32, 150]);
    roundtrip(VecDeque::from([1u32, 150]));
}

#[test]
fn varint_decoding_does_not_reserve_one_element_per_wire_byte() {
    let wire = vec![-1i64; 1000].encode_to_vec();
    let values: Vec<i64> = decode(&wire);
    assert_eq!(values, vec![-1; 1000]);
    assert!(values.capacity() < 2000, "capacity {}", values.capacity());
    let values: VecDeque<i64> = decode(&wire);
    assert_eq!(values.len(), 1000);
    assert!(values.capacity() < 2000, "capacity {}", values.capacity());
}

#[test]
fn atomic_bytes_keep_repeated_scalar_encoding() {
    use std::sync::atomic::AtomicU8;
    use std::sync::atomic::Ordering;
    let values = vec![AtomicU8::new(128), AtomicU8::new(255)];
    let wire = values.encode_to_vec();
    assert_eq!(wire, [10, 4, 128, 1, 255, 1]);
    let decoded: Vec<AtomicU8> = decode(&wire);
    assert_eq!(decoded.iter().map(|v| v.load(Ordering::Relaxed)).collect::<Vec<_>>(), [128, 255]);
}

#[cfg(feature = "parking_lot")]
#[test]
fn aliased_parking_lot_mutex_fields_do_not_deadlock() {
    #[proto_message]
    struct LockedParking {
        first: Arc<parking_lot::Mutex<u32>>,
        second: Arc<parking_lot::Mutex<u32>>,
    }
    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let shared = Arc::new(parking_lot::Mutex::new(150));
        tx.send(
            LockedParking {
                first: shared.clone(),
                second: shared,
            }
            .encode_to_vec(),
        )
        .unwrap();
    });
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(3)).expect("encoding deadlocked"),
        [8, 150, 1, 16, 150, 1]
    );
    worker.join().unwrap();
}

#[cfg(feature = "papaya")]
#[test]
fn papaya_byte_sets_use_varints() {
    let values = papaya::HashSet::new();
    values.pin().insert(128u8);
    values.pin().insert(255u8);
    let decoded: papaya::HashSet<u8> = decode(&values.encode_to_vec());
    assert_eq!(decoded.len(), 2);
    assert!(decoded.pin().contains(&128));
    assert!(decoded.pin().contains(&255));
}
