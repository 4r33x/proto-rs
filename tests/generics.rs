#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]
#![cfg(feature = "generic-examples")]

use proto_rs::ProtoExt;
use proto_rs::ProtoShadow;
use proto_rs::ProtoWire;
use proto_rs::Shadow;
use proto_rs::proto_message;
use proto_rs::proto_rpc;

mod generic_service {
    use std::collections::HashMap;

    use tonic::Request;
    use tonic::Response;
    use tonic::Status;

    use super::*;

    #[proto_message]
    pub struct MapWrapperStruct<
        K: ProtoWire<EncodeInput<'static> = &'static K> + ProtoWire + Eq + std::hash::Hash + Clone + Send + Sync + 'static,
        V: ProtoWire<EncodeInput<'static> = &'static V> + ProtoWire + Clone + Send + Sync + 'static,
    >(pub HashMap<K, V>);

    #[proto_rpc(rpc_package = "generic_rpc", rpc_server = true, rpc_client = true, proto_path = "target/tests/generic_rpc.proto")]
    #[proto_generic_types = [K = [u64, u32], V = [String, u16]]]
    pub trait GenericMapService<
        K: ProtoWire<EncodeInput<'static> = &'static K> + ProtoWire + Eq + std::hash::Hash + Clone + Send + Sync + 'static,
        V: ProtoWire<EncodeInput<'static> = &'static V> + ProtoWire + Clone + Send + Sync + 'static,
    >
    {
        async fn echo_map(&self, request: Request<MapWrapperStruct<K, V>>) -> Result<Response<MapWrapperStruct<K, V>>, Status>;
    }
}

use generic_service::MapWrapperStruct;
use generic_service::ProtoGenericK;
use generic_service::ProtoGenericV;
use generic_service::SealedK;
use generic_service::SealedV;

fn encode_proto_message<M>(value: &M) -> bytes::Bytes
where
    for<'a> M: ProtoExt + ProtoWire<EncodeInput<'a> = &'a M>,
    for<'a> Shadow<'a, M>: ProtoShadow<M, Sun<'a> = &'a M, View<'a> = &'a M>,
{
    let len = <M as ProtoWire>::encoded_len(value);
    let mut buf = bytes::BytesMut::with_capacity(len);
    <M as ProtoExt>::encode(value, &mut buf).expect("proto encode failed");
    buf.freeze()
}

#[test]
fn generic_proto_message_roundtrip() {
    let mut map: std::collections::HashMap<u64, String> = std::collections::HashMap::new();
    map.insert(7, "lucky".to_string());
    map.insert(13, "spooky".to_string());

    let wrapper = MapWrapperStruct(map.clone());
    let bytes = encode_proto_message(&wrapper);
    let decoded = MapWrapperStruct::<u64, String>::decode(bytes).expect("decode failed");

    assert_eq!(decoded.0, map);
}

#[test]
fn generic_proto_rpc_generates_sealed_traits_and_proto() {
    assert!(matches!(<u64 as SealedK>::ENUM, ProtoGenericK::u64));
    assert!(matches!(<u32 as SealedK>::ENUM, ProtoGenericK::u32));
    assert!(matches!(<String as SealedV>::ENUM, ProtoGenericV::String));
    assert!(matches!(<u16 as SealedV>::ENUM, ProtoGenericV::u16));

    assert_eq!(<u64 as SealedK>::ROUTE_PREFIX, "/generic_rpc.GenericMapService");
    assert_eq!(<String as SealedV>::ROUTE_PREFIX, "/generic_rpc.GenericMapService");

    let proto_path = std::path::Path::new("target/tests/generic_rpc.proto");
    assert!(proto_path.exists(), "proto output should be generated in target");
    let contents = std::fs::read_to_string(proto_path).expect("proto file should be readable");
    assert!(contents.contains("service GenericMapService"));
    assert!(contents.contains("echo_map"));
}
