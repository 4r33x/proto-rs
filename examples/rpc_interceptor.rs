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

// Define the interceptor function that will be called on every client request
fn user_advanced_interceptor<T>(ctx: UserId, request: &mut tonic::Request<T>) {
    // Add the user ID to the request metadata
    request.metadata_mut().insert(
        "user-id",
        ctx.to_string().parse().unwrap(),
    );
    println!("Interceptor called with user_id: {}", ctx);
}

// Define trait with the proto_rpc macro using the rpc_client_ctx parameter
#[proto_rpc(
    rpc_package = "interceptor_rpc",
    rpc_server = true,
    rpc_client = true,
    rpc_client_ctx = "user_advanced_interceptor<UserId>",
    proto_path = "protos/gen_proto/interceptor_rpc.proto"
)]
#[proto_imports(goon_types = ["RizzPing", "GoonPong"])]
pub trait InterceptorRpc {
    async fn ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status>;
}

// A dummy server impl
struct S;

impl InterceptorRpc for S {
    async fn ping(&self, request: Request<RizzPing>) -> Result<Response<GoonPong>, Status> {
        // Server can access the user_id from metadata
        if let Some(user_id) = request.metadata().get("user-id") {
            println!("Server received user_id: {:?}", user_id);
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

    #[test]
    fn test_interceptor_syntax() {
        // This test just ensures the macro expands correctly
        println!("Interceptor example compiles successfully!");
    }
}
