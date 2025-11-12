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
