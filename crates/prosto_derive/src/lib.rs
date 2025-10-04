use proc_macro::TokenStream;

mod proto_import;
mod proto_message;
mod proto_rpc;

mod utils;
mod write_file;

#[proc_macro_attribute]
pub fn proto_message(attr: TokenStream, item: TokenStream) -> TokenStream {
    proto_message::proto_message_impl(attr, item)
}

#[proc_macro]
pub fn inject_proto_import(input: TokenStream) -> TokenStream {
    proto_import::inject_proto_import_impl(input)
}

/// Attribute macro for generating Tonic gRPC services with automatic Proto/Native conversion
///
/// This macro works in conjunction with #[proto_message] to generate:
/// - A trait definition for your service (using native Rust types)
/// - An internal proto trait (using Proto types from HasProto)
/// - Automatic conversion layer between native and proto types
/// - Complete Tonic server boilerplate
///
/// # Arguments
///
/// - `package` (optional) - The gRPC package name. Defaults to the trait name if not provided.
///
/// # Example
///
/// ```rust
/// use ftl_proto::{proto_message, proto_rpc, HasProto};
/// use tonic::{Request, Response, Status};
///
/// // Without package name (defaults to "TestRpc")
/// #[proto_rpc]
/// pub trait TestRpc {
///     async fn ping(&self, request: Request<Ping>)
///         -> Result<Response<Pong>, Status>;
/// }
///
/// // With custom package name
/// #[proto_rpc(my_package)]
/// pub trait MyService {
///     async fn ping(&self, request: Request<Ping>)
///         -> Result<Response<Pong>, Status>;
/// }
/// ```
///
/// The generated service will have routes like:
/// - Without package: `/TestRpc/Ping`
/// - With package: `/my_package.MyService/Ping`
#[proc_macro_attribute]
pub fn proto_rpc(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr2 = proc_macro2::TokenStream::from(attr);
    let item2 = proc_macro2::TokenStream::from(item);
    let output = proto_rpc::proto_rpc_impl(attr2, item2);
    TokenStream::from(output)
}
