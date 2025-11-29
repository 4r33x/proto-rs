use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tonic::{Request, Response, Status};

/// Generic request type with proto_generic_types attribute
/// This generates proto messages for all K,V combinations
#[proto_message(
    proto_path = "protos/gen_proto/generic_rpc.proto",
    proto_generic_types = [K = [u64, u32], V = [String, u16]]
)]
pub struct RizzPing<K, V> {
    #[proto(tag = 1)]
    pub key: K,
    #[proto(tag = 2)]
    pub value: V,
}

/// Generic response type with proto_generic_types attribute
/// This generates proto messages for all K,V combinations
#[proto_message(
    proto_path = "protos/gen_proto/generic_rpc.proto",
    proto_generic_types = [K = [u64, u32], V = [String, u16]]
)]
pub struct GoonPong<K, V> {
    #[proto(tag = 1)]
    pub result: K,
    #[proto(tag = 2)]
    pub data: V,
}

// NOTE: Full generic RPC trait support (with proto_generic_types on proto_rpc)
// is a work in progress. The macro generates client/server code that will
// dispatch based on TYPE_ID constants to call the correct gRPC endpoint.
//
// Example usage:
// - Create RizzPing<u64, String> { key: 42, value: "test".to_string() }
// - Check RizzPing::<u64, String>::TYPE_ID == "u64String"
// - Client dispatches to "/sigma_rpc.SigmaRpcu64String/RizzPingGeneric"

fn main() {
    // Demonstrate TYPE_ID constants
    println!("Generic RPC example with TYPE_ID constants:");
    println!("RizzPing<u64, String>::TYPE_ID = {}", RizzPing::<u64, String>::TYPE_ID);
    println!("RizzPing<u64, u16>::TYPE_ID = {}", RizzPing::<u64, u16>::TYPE_ID);
    println!("RizzPing<u32, String>::TYPE_ID = {}", RizzPing::<u32, String>::TYPE_ID);
    println!("RizzPing<u32, u16>::TYPE_ID = {}", RizzPing::<u32, u16>::TYPE_ID);

    println!("\nGoonPong<u64, String>::PROTO_TYPE_NAME = {}", GoonPong::<u64, String>::PROTO_TYPE_NAME);
}
