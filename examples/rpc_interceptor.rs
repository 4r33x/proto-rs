#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]
#![allow(clippy::missing_errors_doc)]

use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tonic::Request;
use tonic::Response;
use tonic::Status;

#[proto_message(proto_path = "protos/gen_proto/goon_types.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct RizzPing;

#[proto_message(proto_path = "protos/gen_proto/goon_types.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GoonPong;

// Define trait with the proto_rpc macro using the rpc_client_ctx parameter
#[proto_rpc(
    rpc_package = "interceptor_rpc",
    rpc_server = true,
    rpc_client = true,
    rpc_client_ctx = "UserAdvancedInterceptor",
    proto_path = "protos/gen_proto/interceptor_rpc.proto"
)]
#[proto_imports(goon_types = ["RizzPing", "GoonPong"])]
pub trait InterceptorRpc {
    async fn ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;
}

// Define the user ID type
pub type UserId = u64;

#[derive(Clone, Debug)]
pub struct UserCtx(pub UserId);

impl From<UserCtx> for UserId {
    fn from(value: UserCtx) -> Self {
        value.0
    }
}

pub trait UserAdvancedInterceptor: Send + Sync + 'static + Sized {
    type Payload: From<Self>;
    fn intercept<T>(&self, req: &mut tonic::Request<T>);
}

impl UserAdvancedInterceptor for UserCtx {
    type Payload = u64;
    fn intercept<T>(&self, request: &mut tonic::Request<T>) {
        request.metadata_mut().insert("user-id", self.0.to_string().parse().unwrap());
        println!("Interceptor called with user_id: {}", self.0);
    }
}

// A dummy server impl
struct S;

impl InterceptorRpc for S {
    async fn ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status> {
        // Server can access the user_id from metadata
        if let Some(user_id) = request.metadata().get("user-id") {
            println!("Server received user_id: {user_id:?}");
        }
        Ok(Response::new(GoonPong {}))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("RPC Interceptor example");
    println!("This example demonstrates the rpc_client_ctx feature");
    println!("which allows users to inject context into every client method call.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interceptor_rpc_client::InterceptorRpcClient;

    #[tokio::test]
    async fn test_interceptor_syntax() {
        // This test just ensures the macro expands correctly
        println!("Interceptor example compiles successfully!");
        let mut client = InterceptorRpcClient::connect("http://127.0.0.1:50051").await.unwrap();
        let res = client.ping(0u64, RizzPing {}).await.unwrap();
    }
}
