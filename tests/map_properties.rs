//! Exercise the live trait codec, rather than the retired callback map engine.
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use proptest::prelude::*;
use proto_rs::DecodeContext;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::ProtoExt;

fn check<K, V>(pairs: Vec<(K, V)>)
where
    K: Clone + Debug + Eq + Hash + Ord + ProtoDecode + ProtoEncode + 'static,
    V: Clone + Debug + PartialEq + ProtoDecode + ProtoEncode + ProtoExt + 'static,
{
    let hash: HashMap<_, _> = pairs.iter().cloned().collect();
    let tree: BTreeMap<_, _> = pairs.into_iter().collect();
    let hash_bytes = hash.encode_to_vec();
    let tree_bytes = tree.encode_to_vec();
    assert_eq!(
        HashMap::<K, V>::decode(hash_bytes.as_slice(), DecodeContext::default()).unwrap(),
        hash
    );
    assert_eq!(
        BTreeMap::<K, V>::decode(tree_bytes.as_slice(), DecodeContext::default()).unwrap(),
        tree
    );
    // Different map implementations must accept one another's wire format.
    assert_eq!(
        HashMap::<K, V>::decode(tree_bytes.as_slice(), DecodeContext::default()).unwrap(),
        hash
    );
    assert_eq!(
        BTreeMap::<K, V>::decode(hash_bytes.as_slice(), DecodeContext::default()).unwrap(),
        tree
    );
}

macro_rules! values {
    ($key:ty) => {
        proptest! {
            #[test] fn int32(v in prop::collection::vec((any::<$key>(), any::<i32>()), 0..32)) { check(v); }
            #[test] fn int64(v in prop::collection::vec((any::<$key>(), any::<i64>()), 0..32)) { check(v); }
            #[test] fn uint32(v in prop::collection::vec((any::<$key>(), any::<u32>()), 0..32)) { check(v); }
            #[test] fn uint64(v in prop::collection::vec((any::<$key>(), any::<u64>()), 0..32)) { check(v); }
            #[test] fn boolean(v in prop::collection::vec((any::<$key>(), any::<bool>()), 0..32)) { check(v); }
            #[test] fn string(v in prop::collection::vec((any::<$key>(), any::<String>()), 0..32)) { check(v); }
            #[test] fn bytes(v in prop::collection::vec((any::<$key>(), prop::collection::vec(any::<u8>(), 0..32)), 0..32)) { check(v); }
            #[test] fn float(v in prop::collection::vec((any::<$key>(), -1e20f32..1e20f32), 0..32)) { check(v); }
            #[test] fn double(v in prop::collection::vec((any::<$key>(), -1e100f64..1e100f64), 0..32)) { check(v); }
        }
    };
}
mod int32 {
    use super::*;
    values!(i32);
}
mod int64 {
    use super::*;
    values!(i64);
}
mod uint32 {
    use super::*;
    values!(u32);
}
mod uint64 {
    use super::*;
    values!(u64);
}
mod boolean {
    use super::*;
    values!(bool);
}
mod string {
    use super::*;
    values!(String);
}
