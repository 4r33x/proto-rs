#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]
#![allow(clippy::missing_errors_doc)]

#[cfg(feature = "stable")]
use std::pin::Pin;

use proto_rs::ToZeroCopyResponse;
use proto_rs::ZeroCopy;
use proto_rs::ZeroCopyResponse;
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
#[derive(Clone, Debug, PartialEq)]
pub struct RizzPing {
    id: Id,
    status: ServiceStatus,
}

#[proto_message(proto_path = "protos/gen_complex_proto/goon_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct GoonPong {
    id: Id,
    status: ZeroCopy<ServiceStatus>,
}

#[proto_message(proto_path = "protos/gen_complex_proto/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct FooResponse;

#[proto_message(proto_path = "protos/gen_complex_proto/rizz_types.proto")]
#[derive(Clone, Debug, PartialEq)]
pub struct BarSub;

// Define trait with the proto_rpc macro
#[proto_rpc(rpc_package = "sigma_rpc", rpc_server = true, rpc_client = true, proto_path = "protos/gen_complex_proto/sigma_rpc_complex.proto")]
#[proto_imports(rizz_types = ["BarSub", "FooResponse"], goon_types = ["RizzPing", "GoonPong", "ServiceStatus", "Id"] )]
pub trait SigmaRpc {
    type RizzUniStream: Stream<Item = Result<ZeroCopyResponse<FooResponse>, Status>> + Send;
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
    #[cfg(feature = "stable")]
    type RizzUniStream = Pin<Box<dyn Stream<Item = Result<ZeroCopyResponse<FooResponse>, Status>> + Send>>;
    #[cfg(not(feature = "stable"))]
    type RizzUniStream = impl Stream<Item = Result<ZeroCopyResponse<FooResponse>, Status>> + Send;

    async fn rizz_ping(&self, _req: Request<RizzPing>) -> Result<Response<GoonPong>, Status> {
        Ok(Response::new(GoonPong {
            id: Id { id: 10 },
            status: ZeroCopy::from(&ServiceStatus::Completed),
        }))
    }

    async fn rizz_uni(&self, _request: Request<BarSub>) -> Result<Response<Self::RizzUniStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        tokio::spawn(async move {
            for _ in 0..5 {
                if tx.send(Ok(FooResponse {}.to_zero_copy())).await.is_err() {
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_server().await?;

    Ok(())
}

// ============================================================================
// GENERIC TYPES EXAMPLE
// ============================================================================
// This section demonstrates the proto_generic_types feature

use std::collections::HashMap;

/// Generic struct that generates proto messages for all K,V combinations
/// Generates:
/// - MapWrapperU64String
/// - MapWrapperU64U16
/// - MapWrapperU32String
/// - MapWrapperU32U16
#[proto_message(
    proto_path = "protos/gen_complex_proto/generic_types.proto",
    proto_generic_types = [K = [u64, u32], V = [String, u16]]
)]
#[derive(Clone, Debug)]
pub struct MapWrapper<K, V> {
    #[proto(tag = 1)]
    pub data: HashMap<K, V>,

    #[proto(tag = 2)]
    pub count: u32,
}

/// Generic enum example
/// Generates:
/// - GenericResultU64
/// - GenericResultString
#[proto_message(
    proto_path = "protos/gen_complex_proto/generic_types.proto",
    proto_generic_types = [T = [u64, String]]
)]
#[derive(Clone, Debug)]
pub enum GenericResult<T> {
    #[proto(tag = 1)]
    Success { value: T },

    #[proto(tag = 2)]
    Error { message: String },
}

// ============================================================================
// GENERIC RPC EXAMPLE
// ============================================================================
// This section demonstrates the proto_generic_types feature for message types
// that would be used in RPC calls

/// Generic RPC request type
/// Generates proto messages for all K,V combinations with associated TYPE_ID constants
#[proto_message(
    proto_path = "protos/gen_complex_proto/generic_rpc.proto",
    proto_generic_types = [K = [u64, String], V = [u32, u16]]
)]
#[derive(Clone, Debug)]
pub struct GenericRequest<K, V> {
    #[proto(tag = 1)]
    pub key: K,

    #[proto(tag = 2)]
    pub value: V,
}

/// Generic RPC response type
/// Generates proto messages for all K,V combinations with associated TYPE_ID constants
#[proto_message(
    proto_path = "protos/gen_complex_proto/generic_rpc.proto",
    proto_generic_types = [K = [u64, String], V = [u32, u16]]
)]
#[derive(Clone, Debug)]
pub struct GenericResponse<K, V> {
    #[proto(tag = 1)]
    pub result: K,

    #[proto(tag = 2)]
    pub metadata: V,
}

// NOTE: Full generic RPC trait support (with proto_generic_types on proto_rpc)
// is a work in progress. The generated client/server code will dispatch based on
// TYPE_ID constants to call the correct gRPC endpoint for each type combination.
//
// Example of how it would work:
// - User creates GenericRequest<u64, u32> { key: 42, value: 100 }
// - Client checks GenericRequest::<u64, u32>::TYPE_ID == "U64U32"
// - Client dispatches to route "/package.ServiceU64U32/Method"
// - Server receives request and dispatches back to generic trait implementation

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

    #[test]
    fn test_generic_types() {
        // Test MapWrapper with different type combinations
        // Note: The generic struct/enum definitions are preserved
        // and proto messages are generated for all concrete type combinations
        let mut map1: MapWrapper<u64, String> = MapWrapper {
            data: HashMap::new(),
            count: 0,
        };
        map1.data.insert(1, "hello".to_string());
        map1.count = 1;
        assert_eq!(map1.count, 1);
        assert!(map1.data.contains_key(&1));

        let mut map2: MapWrapper<u32, u16> = MapWrapper {
            data: HashMap::new(),
            count: 0,
        };
        map2.data.insert(1u32, 42u16);
        map2.count = 1;
        assert_eq!(map2.count, 1);
        assert_eq!(map2.data.get(&1u32), Some(&42u16));

        // Test GenericResult
        let success: GenericResult<u64> = GenericResult::Success { value: 42 };
        match success {
            GenericResult::Success { value } => assert_eq!(value, 42),
            _ => panic!("Expected Success"),
        }

        let error: GenericResult<String> = GenericResult::Error {
            message: "test error".to_string(),
        };
        match error {
            GenericResult::Error { message } => assert_eq!(message, "test error"),
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_generic_type_ids() {
        // Test that TYPE_ID constants are generated for each generic instantiation
        // This demonstrates the associated const approach used for RPC dispatching

        // MapWrapper TYPE_IDs
        assert_eq!(MapWrapper::<u64, String>::TYPE_ID, "u64String");
        assert_eq!(MapWrapper::<u64, u16>::TYPE_ID, "u64u16");
        assert_eq!(MapWrapper::<u32, String>::TYPE_ID, "u32String");
        assert_eq!(MapWrapper::<u32, u16>::TYPE_ID, "u32u16");

        // PROTO_TYPE_NAME constants
        assert_eq!(MapWrapper::<u64, String>::PROTO_TYPE_NAME, "MapWrapperu64String");
        assert_eq!(MapWrapper::<u32, u16>::PROTO_TYPE_NAME, "MapWrapperu32u16");

        // GenericResult TYPE_IDs
        assert_eq!(GenericResult::<u64>::TYPE_ID, "u64");
        assert_eq!(GenericResult::<String>::TYPE_ID, "String");

        // GenericRequest TYPE_IDs (for RPC)
        assert_eq!(GenericRequest::<u64, u32>::TYPE_ID, "u64u32");
        assert_eq!(GenericRequest::<u64, u16>::TYPE_ID, "u64u16");
        assert_eq!(GenericRequest::<String, u32>::TYPE_ID, "Stringu32");
        assert_eq!(GenericRequest::<String, u16>::TYPE_ID, "Stringu16");

        // GenericResponse TYPE_IDs
        assert_eq!(GenericResponse::<u64, u32>::TYPE_ID, "u64u32");
        assert_eq!(GenericResponse::<String, u16>::TYPE_ID, "Stringu16");

        println!("All TYPE_ID constants are correctly generated!");
    }

    #[test]
    fn test_generic_rpc_usage() {
        // This test demonstrates how users can work directly with generic types
        // No wrapper enums needed - just use the generic types directly

        // Create a request with concrete types
        let _request1: GenericRequest<u64, u32> = GenericRequest {
            key: 42,
            value: 100,
        };

        // The TYPE_ID constant allows the RPC layer to dispatch correctly
        assert_eq!(GenericRequest::<u64, u32>::TYPE_ID, "u64u32");

        // Create another request with different concrete types
        let _request2: GenericRequest<String, u16> = GenericRequest {
            key: "hello".to_string(),
            value: 256,
        };

        assert_eq!(GenericRequest::<String, u16>::TYPE_ID, "Stringu16");

        // In the actual RPC client/server code, the dispatch would look like:
        // match request_type::TYPE_ID {
        //     "u64u32" => call("/generic_rpc.GenericRpcu64u32/GenericProcess"),
        //     "Stringu16" => call("/generic_rpc.GenericRpcStringu16/GenericProcess"),
        //     ...
        // }

        println!("request1 TYPE_ID: {}", GenericRequest::<u64, u32>::TYPE_ID);
        println!("request2 TYPE_ID: {}", GenericRequest::<String, u16>::TYPE_ID);
    }
}
