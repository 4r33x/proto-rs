use std::pin::Pin;

use prost_rs_test::goon_types::GoonPong;
use prost_rs_test::goon_types::RizzPing;
use prost_rs_test::rizz_types::BarSub;
use prost_rs_test::rizz_types::FooResponse;
use prost_rs_test::sigma_rpc::sigma_rpc_server::SigmaRpc;
use prost_rs_test::sigma_rpc::sigma_rpc_server::SigmaRpcServer;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Request;
use tonic::Response;
use tonic::Status;

// A dummy server impl
struct S;

#[tonic::async_trait]
impl SigmaRpc for S {
    type RizzPingResponse = tonic::Response<GoonPong>;
    type RizzUniStream = Pin<Box<dyn Stream<Item = Result<FooResponse, Status>> + Send>>;
    async fn rizz_ping(&self, _req: Request<RizzPing>) -> Result<Response<GoonPong>, Status> {
        Ok(Response::new(GoonPong {}))
    }
    async fn rizz_uni(&self, _request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(128);

        // Spawn a task to send some test events
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
pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    use tonic::transport::Server;

    let addr = "127.0.0.1:50051".parse()?;
    let service = S;

    println!("TestRpc server listening on {}", addr);

    Server::builder().add_service(SigmaRpcServer::new(service)).serve(addr).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_server().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use prost_rs_test::sigma_rpc::sigma_rpc_client::SigmaRpcClient;
    use tokio_stream::StreamExt;

    use super::*;

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
}
