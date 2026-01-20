use std::collections::BTreeMap;
use std::collections::HashMap;

use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::encoding::DecodeContext;
use proto_rs::proto_message;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[proto_message]
pub struct Foo {
    #[proto(tag = 1)]
    pub id: u32,
    #[proto(tag = 2)]
    pub meta: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[proto_message]
pub struct MapPrimitives {
    #[proto(tag = 1)]
    pub hash: HashMap<Foo, u32>,
    #[proto(tag = 2)]
    pub tree: BTreeMap<Foo, u32>,
}

#[test]
fn map_with_primitive_values_roundtrips() {
    let mut message = MapPrimitives::default();
    message.hash.insert(Foo { id: 7, meta: 11 }, 42);
    message.tree.insert(Foo { id: 1, meta: 2 }, 3);

    let encoded = message.encode_to_vec();
    let decoded = <MapPrimitives as ProtoDecode>::decode(&encoded[..], DecodeContext::default())
        .expect("decode map with primitives");

    assert_eq!(decoded, message);
}

#[cfg(feature = "papaya")]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[proto_message]
pub struct PapayaPrimitives {
    #[proto(tag = 1)]
    pub map: papaya::HashMap<Foo, u32>,
}

#[cfg(feature = "papaya")]
#[test]
fn papaya_map_with_primitive_values_roundtrips() {
    let message = PapayaPrimitives::default();

    {
        let guard = message.map.pin();
        guard.insert(Foo { id: 5, meta: 6 }, 99);
    }

    let encoded = message.encode_to_vec();
    let decoded = <PapayaPrimitives as ProtoDecode>::decode(&encoded[..], DecodeContext::default())
        .expect("decode papaya map with primitives");

    assert_eq!(decoded, message);
}
