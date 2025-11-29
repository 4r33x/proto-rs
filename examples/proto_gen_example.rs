#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]
#![allow(clippy::missing_errors_doc)]

#[cfg(feature = "stable")]
use std::pin::Pin;
use std::sync::Arc;

use proto_rs::ToZeroCopyResponse;
use proto_rs::ZeroCopyResponse;
use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Request;
use tonic::Response;
use tonic::Status;

#[proto_message(proto_path = "protos/gen_proto/goon_types.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RizzPing;

#[proto_message(proto_path = "protos/gen_proto/goon_types.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GoonPong;

#[proto_message(proto_path = "protos/gen_proto/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FooResponse;

#[proto_message(proto_path = "protos/gen_proto/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BarSub;

// Define trait with the proto_rpc macro
#[proto_rpc(rpc_package = "sigma_rpc", rpc_server = true, rpc_client = true, proto_path = "protos/gen_proto/sigma_rpc.proto")]
#[proto_imports(rizz_types = ["BarSub", "FooResponse"], goon_types = ["RizzPing", "GoonPong"] )]
pub trait SigmaRpc {
    type RizzUniStream: Stream<Item = Result<ZeroCopyResponse<FooResponse>, Status>> + Send;
    async fn rizz_uni(&self, request: Request<BarSub>) -> Response<Self::RizzUniStream>;
    async fn zero_copy_ping(&self, request: Request<RizzPing>) -> Result<ZeroCopyResponse<GoonPong>, Status>;
    async fn just_ping(&self, request: Request<RizzPing>) -> Result<GoonPong, Status>;
    async fn infallible_just_ping(&self, request: Request<RizzPing>) -> GoonPong;
    async fn infallible_zero_copy_ping(&self, request: Request<RizzPing>) -> ZeroCopyResponse<GoonPong>;
    async fn infallible_ping(&self, request: Request<RizzPing>) -> Response<GoonPong>;
    async fn rizz_ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;
    async fn rizz_ping_arced_resp(&self, request: Request<RizzPing>) -> Result<Response<Arc<GoonPong>>, Status>;
    async fn rizz_ping_boxed_resp(&self, request: Request<RizzPing>) -> Result<Response<Box<GoonPong>>, Status>;
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
    type RizzUniStream = Pin<Box<dyn Stream<Item = Result<ZeroCopyResponse<FooResponse>, Status>> + Send>>;
    #[cfg(not(feature = "stable"))]
    type RizzUniStream = impl Stream<Item = Result<ZeroCopyResponse<FooResponse>, Status>> + Send;

    async fn zero_copy_ping(&self, _request: Request<RizzPing>) -> Result<ZeroCopyResponse<GoonPong>, Status> {
        Ok(GoonPong {}.to_zero_copy())
    }

    async fn just_ping(&self, _request: Request<RizzPing>) -> Result<GoonPong, Status> {
        Ok(GoonPong {})
    }

    async fn infallible_just_ping(&self, _request: Request<RizzPing>) -> GoonPong {
        GoonPong {}
    }

    async fn infallible_zero_copy_ping(&self, _request: Request<RizzPing>) -> ZeroCopyResponse<GoonPong> {
        GoonPong {}.to_zero_copy()
    }

    async fn infallible_ping(&self, _request: Request<RizzPing>) -> Response<GoonPong> {
        Response::new(GoonPong {})
    }

    async fn rizz_ping(&self, _req: Request<RizzPing>) -> Result<Response<GoonPong>, Status> {
        Ok(Response::new(GoonPong {}))
    }
    async fn rizz_ping_arced_resp(&self, _req: Request<RizzPing>) -> Result<Response<Arc<GoonPong>>, Status> {
        Ok(Response::new(Arc::new(GoonPong {})))
    }
    async fn rizz_ping_boxed_resp(&self, _req: Request<RizzPing>) -> Result<Response<Box<GoonPong>>, Status> {
        Ok(Response::new(Box::new(GoonPong {})))
    }

    async fn rizz_uni(&self, _request: Request<BarSub>) -> Response<Self::RizzUniStream> {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        tokio::spawn(async move {
            for _ in 0..5 {
                let response = ZeroCopyResponse::from_message(FooResponse {});
                if tx.send(Ok(response)).await.is_err() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let stream = ReceiverStream::new(rx);

        #[cfg(feature = "stable")]
        let stream: Self::RizzUniStream = Box::pin(stream);

        Response::new(stream)
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
        let res = client.rizz_ping(RizzPing {}).await.unwrap();
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

    #[tokio::test]
    async fn test_zero_copy_client_requests() {
        use proto_rs::ZeroCopyRequest;
        let mut client = SigmaRpcClient::connect("http://127.0.0.1:50051").await.unwrap();

        let borrowed = RizzPing {};
        let zero_copy: ZeroCopyRequest<_> = proto_rs::ToZeroCopyRequest::to_zero_copy(&borrowed);
        client.rizz_ping(zero_copy).await.unwrap();
    }
}
