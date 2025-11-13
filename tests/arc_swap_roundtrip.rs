#![cfg(feature = "arc_swap")]

use std::sync::Arc;

use arc_swap::ArcSwap;
use arc_swap::ArcSwapOption;
use proto_rs::ProtoExt;
use proto_rs::proto_message;

#[proto_message(proto_path = "protos/tests/arc_swap.proto")]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SwapInner {
    #[proto(tag = 1)]
    pub label: String,
    #[proto(tag = 2)]
    pub count: u32,
}

#[proto_message(proto_path = "protos/tests/arc_swap.proto")]
#[derive(Debug)]
pub struct SwapHolder {
    #[proto(tag = 1)]
    pub primary: ArcSwap<SwapInner>,
}

impl Default for SwapHolder {
    fn default() -> Self {
        Self {
            primary: ArcSwap::from_pointee(SwapInner::default()),
        }
    }
}

#[proto_message(proto_path = "protos/tests/arc_swap.proto")]
#[derive(Debug)]
pub struct OptionalSwapHolder {
    #[proto(tag = 1)]
    pub maybe: ArcSwapOption<SwapInner>,
}

impl Default for OptionalSwapHolder {
    fn default() -> Self {
        Self { maybe: ArcSwapOption::new(None) }
    }
}

#[proto_message(proto_path = "protos/tests/arc_swap.proto")]
#[derive(Debug)]
pub struct ArcSwapContainerHolder {
    #[proto(tag = 1)]
    pub swap_bytes: ArcSwap<Vec<u8>>,
    #[proto(tag = 2)]
    pub swap_u64s: ArcSwap<Vec<u64>>,
    #[proto(tag = 3)]
    pub swap_array_u64: ArcSwap<[u64; 32]>,
    #[proto(tag = 4)]
    pub swap_array_bytes: ArcSwap<[u8; 32]>,
}

impl Default for ArcSwapContainerHolder {
    fn default() -> Self {
        Self {
            swap_bytes: ArcSwap::from_pointee(Vec::new()),
            swap_u64s: ArcSwap::from_pointee(Vec::new()),
            swap_array_u64: ArcSwap::from_pointee([0_u64; 32]),
            swap_array_bytes: ArcSwap::from_pointee([0_u8; 32]),
        }
    }
}

#[proto_message(proto_path = "protos/tests/arc_swap.proto")]
#[derive(Debug)]
pub struct ArcSwapOptionBytesHolder {
    #[proto(tag = 1)]
    pub maybe_swap_bytes: ArcSwapOption<Vec<u8>>,
}

impl Default for ArcSwapOptionBytesHolder {
    fn default() -> Self {
        Self {
            maybe_swap_bytes: ArcSwapOption::new(None),
        }
    }
}

#[test]
fn arc_swap_roundtrip_preserves_inner_value() {
    let holder = SwapHolder {
        primary: ArcSwap::from_pointee(SwapInner { label: "alpha".into(), count: 7 }),
    };

    let encoded = <SwapHolder as ProtoExt>::encode_to_vec(&holder);
    let decoded = <SwapHolder as ProtoExt>::decode(&encoded[..]).expect("decode arc swap");

    let guard = decoded.primary.load();
    assert_eq!(guard.label, "alpha");
    assert_eq!(guard.count, 7);
}

#[test]
fn arc_swap_option_roundtrip_handles_present_value() {
    let holder = OptionalSwapHolder {
        maybe: ArcSwapOption::new(Some(Arc::new(SwapInner { label: "beta".into(), count: 13 }))),
    };

    let encoded = <OptionalSwapHolder as ProtoExt>::encode_to_vec(&holder);
    let decoded = <OptionalSwapHolder as ProtoExt>::decode(&encoded[..]).expect("decode arc swap option");

    let guard = decoded.maybe.load();
    let inner = guard.as_ref().expect("expected inner value");
    assert_eq!(inner.label, "beta");
    assert_eq!(inner.count, 13);
}

#[test]
fn arc_swap_option_roundtrip_handles_absent_value() {
    let holder = OptionalSwapHolder::default();

    let encoded = <OptionalSwapHolder as ProtoExt>::encode_to_vec(&holder);
    let decoded = <OptionalSwapHolder as ProtoExt>::decode(&encoded[..]).expect("decode default arc swap option");

    let guard = decoded.maybe.load();
    assert!(guard.as_ref().is_none());
}

#[test]
fn arc_swap_vec_u8_roundtrip_encodes_as_bytes() {
    let holder = ArcSwapContainerHolder {
        swap_bytes: ArcSwap::from_pointee(vec![5, 8, 13, 21]),
        ..ArcSwapContainerHolder::default()
    };

    let encoded = <ArcSwapContainerHolder as ProtoExt>::encode_to_vec(&holder);
    let decoded = <ArcSwapContainerHolder as ProtoExt>::decode(&encoded[..]).expect("decode arc swap container holder bytes");

    let bytes_guard = decoded.swap_bytes.load();
    assert_eq!(bytes_guard.as_slice(), &[5, 8, 13, 21]);
}

#[test]
fn arc_swap_option_bytes_roundtrip_handles_presence_and_absence() {
    let holder = ArcSwapOptionBytesHolder {
        maybe_swap_bytes: ArcSwapOption::new(Some(Arc::new(vec![1, 2, 3, 4]))),
    };

    let encoded = <ArcSwapOptionBytesHolder as ProtoExt>::encode_to_vec(&holder);
    let decoded = <ArcSwapOptionBytesHolder as ProtoExt>::decode(&encoded[..]).expect("decode arc swap option bytes");

    let present_guard = decoded.maybe_swap_bytes.load();
    let bytes = present_guard.as_ref().expect("expected bytes");
    assert_eq!(bytes.as_slice(), &[1, 2, 3, 4]);

    let default_holder = ArcSwapOptionBytesHolder::default();
    let encoded_default = <ArcSwapOptionBytesHolder as ProtoExt>::encode_to_vec(&default_holder);
    let decoded_default = <ArcSwapOptionBytesHolder as ProtoExt>::decode(&encoded_default[..]).expect("decode default arc swap option bytes");
    assert!(decoded_default.maybe_swap_bytes.load().as_ref().is_none());
}
