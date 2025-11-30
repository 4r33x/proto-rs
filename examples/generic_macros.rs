#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]
#![cfg(feature = "generic-examples")]

use std::collections::HashMap;

use proto_rs::ProtoExt;
use proto_rs::ProtoShadow;
use proto_rs::ProtoWire;
use proto_rs::Shadow;
use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tonic::Request;
use tonic::Response;
use tonic::Status;

#[proto_message]
pub struct MapWrapperStruct<K: ProtoWire + Eq + std::hash::Hash + Clone + Send + Sync + 'static, V: ProtoWire + Clone + Send + Sync + 'static>(pub HashMap<K, V>);

#[proto_rpc(rpc_package = "generic_example_rpc", rpc_server = true, rpc_client = true, proto_path = "target/examples/generic_example_rpc.proto")]
#[proto_generic_types = [K = [u64, u32], V = [String, u16]]]
pub trait GenericExampleService<K: ProtoWire + Eq + std::hash::Hash + Clone + Send + Sync + 'static, V: ProtoWire + Clone + Send + Sync + 'static> {
    async fn echo_map(&self, request: Request<MapWrapperStruct<K, V>>) -> Result<Response<MapWrapperStruct<K, V>>, Status>;
}

struct ExampleService;

#[tonic::async_trait]
impl GenericExampleService<u64, String> for ExampleService {
    async fn echo_map(&self, request: Request<MapWrapperStruct<u64, String>>) -> Result<Response<MapWrapperStruct<u64, String>>, Status> {
        Ok(Response::new(request.into_inner()))
    }
}

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut map = HashMap::new();
    map.insert(7, "lucky".to_string());

    let encoded = encode_proto_message(&MapWrapperStruct::<u64, String>(map.clone()));
    let decoded = MapWrapperStruct::<u64, String>::decode(encoded)?;
    println!("roundtrip: {:?}", decoded.0);

    let svc = ExampleService;
    let echoed = svc.echo_map(Request::new(MapWrapperStruct(map.clone()))).await?.into_inner();
    println!("echoed => {:?}", echoed.0);

    Ok(())
}
