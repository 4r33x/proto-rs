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

// Define the user ID type
pub type UserId = u64;

#[derive(Clone, Debug)]
pub struct UserCtx(pub UserId);

impl From<UserCtx> for UserId {
    fn from(value: UserCtx) -> Self {
        value.0
    }
}

// Define trait with the proto_rpc macro using the rpc_client_ctx parameter
#[proto_rpc(
    rpc_package = "interceptor_rpc",
    rpc_server = true,
    rpc_client = true,
    rpc_client_ctx = "UserAdvancedInterceptor<Ctx>",
    proto_path = "protos/gen_proto/interceptor_rpc.proto"
)]
#[proto_imports(goon_types = ["RizzPing", "GoonPong"])]
pub trait InterceptorRpc {
    async fn ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;
}

pub trait UserAdvancedInterceptor<Ctx>: Send + Sync + 'static {
    fn intercept<T>(&self, ctx: Ctx, req: &mut tonic::Request<T>);
}

impl UserAdvancedInterceptor<UserId> for UserCtx {
    fn intercept<T>(&self, ctx: UserId, request: &mut tonic::Request<T>) {
        request.metadata_mut().insert("user-id", ctx.to_string().parse().unwrap());
        println!("Interceptor called with user_id: {ctx}");
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
        let res = client.ping(UserCtx(1), RizzPing {}).await.unwrap();
    }
}
