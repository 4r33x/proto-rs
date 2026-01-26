#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]
#![allow(clippy::missing_errors_doc)]

#[cfg(feature = "stable")]
use std::pin::Pin;

use proto_rs::DecodeError;
use proto_rs::ProtoEncode;
use proto_rs::ZeroCopy;
use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Extensions;
use tonic::Request;
use tonic::Response;
use tonic::Status;

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum ServiceStatus {
    #[default]
    Active,
    Pending,
    Inactive,
    Completed,
}

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Id {
    pub id: u64,
}

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[proto(generic_types = [T = [u64, u32]])]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct IdGeneric<T> {
    pub id: T,
}

#[proto_message(transparent)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct IdGenericTransparent<T> {
    pub id: T,
}

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct RizzPing {
    id: Id,
    status: ServiceStatus,
}

fn validate_id(id: &mut Id) -> Result<(), DecodeError> {
    if id.id == 1 {
        return Err(DecodeError::new("Bad field id"));
    }
    Ok(())
}
fn validate_pong(id: &mut GoonPong) -> Result<(), DecodeError> {
    if id.id.id == 1 {
        return Err(DecodeError::new("Bad top id"));
    }
    Ok(())
}

fn validate_pong_with_ext(id: &mut GoonPong, _ext: &Extensions) -> Result<(), DecodeError> {
    if id.id.id == 1 {
        return Err(DecodeError::new("Bad top id"));
    }
    Ok(())
}

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[proto(validator = validate_pong)]
#[proto(validator_with_ext = validate_pong_with_ext)]
#[derive(Clone, Debug, PartialEq)]
pub struct GoonPong {
    #[proto(validator = validate_id)]
    id: Id,
    status: ServiceStatus,
}

const _: () = {
    assert!(<GoonPong as proto_rs::ProtoDecode>::VALIDATE_WITH_EXT);
};

#[proto_message(proto_path = "protos/gen_complex_proto/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct FooResponse;

#[proto_message(proto_path = "protos/gen_complex_proto/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct BarSub;

// Define trait with the proto_rpc macro
#[proto_rpc(
    rpc_package = "sigma_rpc",
    rpc_server = true,
    rpc_client = true,
    proto_path = "protos/gen_complex_proto/sigma_rpc_complex.proto"
)]
#[proto_imports(rizz_types = ["BarSub", "FooResponse"], goon_types = ["RizzPing", "GoonPong", "ServiceStatus", "Id"] )]
pub trait SigmaRpc {
    type RizzUniStream: Stream<Item = Result<FooResponse, Status>> + Send;
    type RizzUniStream2: Stream<Item = Result<FooResponse, Status>> + Send;
    type GenericUniStream: Stream<Item = Result<ZeroCopy<IdGeneric<u64>>, Status>> + Send;
    async fn rizz_ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;
    async fn goon_pong(&self, request: Request<GoonPong>) -> Result<Response<ZeroCopy<RizzPing>>, Status>;
    async fn rizz_uni(&self, request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status>;
    async fn rizz_uni2(&self, request: Request<BarSub>) -> Result<Response<Self::RizzUniStream2>, Status>;
    async fn generic_uni(&self, request: Request<BarSub>) -> Result<Response<Self::GenericUniStream>, Status>;
    async fn rizz_uni_other(&self, request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status>;
    async fn with_generic(&self, request: Request<IdGeneric<u64>>) -> Result<Response<IdGeneric<u32>>, Status>;
    async fn with_generic_transparent(
        &self,
        request: Request<IdGenericTransparent<u64>>,
    ) -> Result<Response<IdGenericTransparent<u32>>, Status>;
}

// A dummy server impl
struct S;

pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    use tonic::transport::Server;

    use crate::sigma_rpc_server::SigmaRpcServer;

    let addr = "127.0.0.1:50051".parse()?;
    let service = S;

    println!("TestRpc server listening on {addr}");

    Server::builder().add_service(SigmaRpcServer::new(service)).serve(addr).await?;

    Ok(())
}

impl SigmaRpc for S {
    #[cfg(feature = "stable")]
    type RizzUniStream = Pin<Box<dyn Stream<Item = Result<FooResponse, Status>> + Send>>;
    #[cfg(not(feature = "stable"))]
    type RizzUniStream = impl Stream<Item = Result<FooResponse, Status>> + Send;

    #[cfg(feature = "stable")]
    type RizzUniStream2 = Pin<Box<dyn Stream<Item = Result<FooResponse, Status>> + Send>>;
    #[cfg(not(feature = "stable"))]
    type RizzUniStream2 = impl Stream<Item = Result<FooResponse, Status>> + Send;

    #[cfg(feature = "stable")]
    type GenericUniStream = Pin<Box<dyn Stream<Item = Result<ZeroCopy<IdGeneric<u64>>, Status>> + Send>>;

    #[cfg(not(feature = "stable"))]
    type GenericUniStream = impl Stream<Item = Result<ZeroCopy<IdGeneric<u64>>, Status>> + Send;

    async fn rizz_ping(&self, _req: Request<RizzPing>) -> Result<Response<GoonPong>, Status> {
        Ok(Response::new(GoonPong {
            id: Id { id: 10 },
            status: ServiceStatus::Completed,
        }))
    }

    async fn rizz_uni(&self, _request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        tokio::spawn(async move {
            for _ in 0..5 {
                if tx.send(Ok(FooResponse {})).await.is_err() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let stream = ReceiverStream::new(rx);
        #[cfg(feature = "stable")]
        let stream: Self::RizzUniStream = Box::pin(stream);

        Ok(Response::new(stream))
    }
    async fn rizz_uni2(&self, _request: Request<BarSub>) -> Result<Response<Self::RizzUniStream2>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        tokio::spawn(async move {
            for _ in 0..5 {
                if tx.send(Ok(FooResponse {})).await.is_err() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let stream = ReceiverStream::new(rx);
        #[cfg(feature = "stable")]
        let stream: Self::RizzUniStream2 = Box::pin(stream);

        Ok(Response::new(stream))
    }

    async fn generic_uni(&self, _request: Request<BarSub>) -> Result<Response<Self::GenericUniStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        tokio::spawn(async move {
            for _ in 0..5 {
                if tx.send(Ok(IdGeneric { id: 0 }.to_zero_copy())).await.is_err() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let stream = ReceiverStream::new(rx);
        #[cfg(feature = "stable")]
        let stream: Self::GenericUniStream = Box::pin(stream);

        Ok(Response::new(stream))
    }
    async fn rizz_uni_other(&self, _request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        tokio::spawn(async move {
            for _ in 0..5 {
                if tx.send(Ok(FooResponse {})).await.is_err() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let stream = ReceiverStream::new(rx);
        #[cfg(feature = "stable")]
        let stream: Self::RizzUniStream = Box::pin(stream);

        Ok(Response::new(stream))
    }

    async fn goon_pong(&self, _request: tonic::Request<GoonPong>) -> Result<Response<ZeroCopy<RizzPing>>, tonic::Status> {
        Ok(Response::new(
            RizzPing {
                id: Id { id: 1 },
                status: ServiceStatus::Active,
            }
            .to_zero_copy(),
        ))
    }

    async fn with_generic(&self, _request: tonic::Request<IdGeneric<u64>>) -> Result<Response<IdGeneric<u32>>, tonic::Status> {
        Ok(IdGeneric { id: 1u32 }.into())
    }
    async fn with_generic_transparent(
        &self,
        _request: tonic::Request<IdGenericTransparent<u64>>,
    ) -> Result<Response<IdGenericTransparent<u32>>, tonic::Status> {
        Ok(IdGenericTransparent { id: 1u32 }.into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_server().await?;

    Ok(())
}

#[cfg(test)]
mod tests {

    use proto_rs::ProtoDecode;
    use tokio_stream::StreamExt;
    use tonic::IntoRequest;

    use super::*;
    use crate::sigma_rpc_client::SigmaRpcClient;

    #[tokio::test]
    async fn test_proto_client_unary_impl() {
        let mut client = SigmaRpcClient::connect("http://127.0.0.1:50051").await.unwrap();
        let res = client
            .rizz_ping(RizzPing {
                id: Id { id: 5 },
                status: ServiceStatus::Pending,
            })
            .await
            .unwrap();
        println!("{:?}", res)
    }
    #[tokio::test]
    async fn test_proto_client_unary_generic_impl() {
        let mut client = SigmaRpcClient::connect("http://127.0.0.1:50051").await.unwrap();
        let res = client.with_generic(IdGeneric { id: 5 }).await.unwrap();
        println!("{:?}", res)
    }
    #[tokio::test]
    async fn test_proto_client_unary_generic_transparent_impl() {
        let mut client = SigmaRpcClient::connect("http://127.0.0.1:50051").await.unwrap();
        let res = client.with_generic_transparent(IdGenericTransparent { id: 5 }).await.unwrap();
        println!("{:?}", res)
    }

    #[tokio::test]
    #[should_panic]
    async fn test_proto_client_unary_impl_bad_input() {
        let mut client = SigmaRpcClient::connect("http://127.0.0.1:50051").await.unwrap();
        let res = client
            .goon_pong(GoonPong {
                id: Id { id: 1 },
                status: ServiceStatus::Pending,
            })
            .await
            .unwrap();
        println!("{:?}", res)
    }

    #[tokio::test]
    async fn test_proto_client_stream_impl() {
        let mut client = SigmaRpcClient::connect("http://127.0.0.1:50051").await.unwrap();
        let mut res = client.rizz_uni(BarSub {}).await.unwrap().into_inner();
        while let Some(v) = res.next().await {
            println!("{:?}", v.unwrap())
        }
    }
}
