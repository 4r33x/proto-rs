#![allow(clippy::missing_errors_doc)]
use std::pin::Pin;

use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Request;
use tonic::Response;
use tonic::Status;

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum ServiceStatus {
    Pending,
    #[default]
    Active,
    Inactive,
    Completed,
}

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Id {
    pub id: u64,
}

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct RizzPing {
    id: Id,
    status: ServiceStatus,
}

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct GoonPong {
    id: Id,
    status: ServiceStatus,
}

#[proto_message(proto_path = "protos/gen_complex_proto/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct FooResponse;

#[proto_message(proto_path = "protos/gen_complex_proto/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct BarSub;

// Define trait with the proto_rpc macro
#[proto_rpc(rpc_package = "sigma_rpc", rpc_server = true, rpc_client = true, proto_path = "protos/gen_complex_proto/sigma_rpc.proto")]
#[proto_imports(rizz_types = ["BarSub", "FooResponse"], goon_types = ["RizzPing", "GoonPong", "ServiceStatus", "Id"] )]
pub trait SigmaRpc {
    type RizzUniStream: Stream<Item = Result<FooResponse, Status>> + Send;
    async fn rizz_ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;

    async fn rizz_uni(&self, request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status>;
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
    type RizzUniStream = Pin<Box<dyn Stream<Item = Result<FooResponse, Status>> + Send>>;
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
        let boxed_stream: Self::RizzUniStream = Box::pin(stream);

        Ok(Response::new(boxed_stream))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_server().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio_stream::StreamExt;

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
    async fn test_proto_client_stream_impl() {
        let mut client = SigmaRpcClient::connect("http://127.0.0.1:50051").await.unwrap();
        let mut res = client.rizz_uni(BarSub {}).await.unwrap().into_inner();
        while let Some(v) = res.next().await {
            println!("{:?}", v.unwrap())
        }
    }
}
