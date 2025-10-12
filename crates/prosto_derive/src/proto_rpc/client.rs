//! Client generation - refactored to use common RPC utilities

use proc_macro2::TokenStream;
use quote::quote;

use crate::proto_rpc::rpc_common::client_module_name;
use crate::proto_rpc::rpc_common::client_struct_name;
use crate::proto_rpc::rpc_common::generate_client_with_interceptor;
use crate::proto_rpc::rpc_common::generate_codec_init;
use crate::proto_rpc::rpc_common::generate_native_to_proto_request_streaming;
use crate::proto_rpc::rpc_common::generate_native_to_proto_request_unary;
use crate::proto_rpc::rpc_common::generate_proto_to_native_response;
use crate::proto_rpc::rpc_common::generate_ready_check;
use crate::proto_rpc::rpc_common::generate_route_path;
use crate::proto_rpc::rpc_common::generate_stream_conversion;
use crate::proto_rpc::rpc_common::is_streaming_method;
use crate::utils::MethodInfo;

// ============================================================================
// CLIENT MODULE GENERATION
// ============================================================================

pub fn generate_client_module(trait_name: &syn::Ident, vis: &syn::Visibility, package_name: &str, methods: &[MethodInfo]) -> TokenStream {
    let client_module = client_module_name(trait_name);
    let client_struct = client_struct_name(trait_name);

    let client_methods = methods.iter().map(|m| generate_client_method(m, package_name, trait_name)).collect::<Vec<_>>();

    let compression_methods = generate_client_compression_methods();
    let with_interceptor = generate_client_with_interceptor(&client_struct);

    quote! {
        #vis mod #client_module {
            #![allow(
                unused_variables,
                dead_code,
                missing_docs,
                clippy::wildcard_imports,
                clippy::let_unit_value
            )]
            use tonic::codegen::*;
            use super::*;

            #[derive(Debug, Clone)]
            pub struct #client_struct<T> {
                inner: tonic::client::Grpc<T>,
            }

            impl #client_struct<tonic::transport::Channel> {
                pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
                where
                    D: TryInto<tonic::transport::Endpoint>,
                    D::Error: Into<StdError>,
                {
                    let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
                    Ok(Self::new(conn))
                }
            }

            impl<T> #client_struct<T>
            where
                T: tonic::client::GrpcService<tonic::body::Body>,
                T::Error: Into<StdError>,
                T::ResponseBody: Body<Data = ::proto_rs::bytes::Bytes> + std::marker::Send + 'static,
                <T::ResponseBody as Body>::Error: Into<StdError> + std::marker::Send,
            {
                pub fn new(inner: T) -> Self {
                    let inner = tonic::client::Grpc::new(inner);
                    Self { inner }
                }

                pub fn with_origin(inner: T, origin: http::Uri) -> Self {
                    let inner = tonic::client::Grpc::with_origin(inner, origin);
                    Self { inner }
                }

                #with_interceptor

                #compression_methods

                #(#client_methods)*
            }
        }
    }
}

// ============================================================================
// CLIENT METHOD GENERATION
// ============================================================================

fn generate_client_method(method: &MethodInfo, package_name: &str, trait_name: &syn::Ident) -> TokenStream {
    if is_streaming_method(method) {
        generate_streaming_client_method(method, package_name, trait_name)
    } else {
        generate_unary_client_method(method, package_name, trait_name)
    }
}

fn generate_unary_client_method(method: &MethodInfo, package_name: &str, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let response_type = &method.response_type;
    let route_path = generate_route_path(package_name, trait_name, method_name);

    let ready_check = generate_ready_check();
    let codec_init = generate_codec_init();
    let request_conversion = generate_native_to_proto_request_unary();
    let response_conversion = generate_proto_to_native_response(response_type);

    quote! {
        pub async fn #method_name(
            &mut self,
            request: impl tonic::IntoRequest<#request_type>,
        ) -> std::result::Result<tonic::Response<#response_type>, tonic::Status> {
            #ready_check
            #codec_init
            let path = http::uri::PathAndQuery::from_static(#route_path);

            #request_conversion

            proto_req.extensions_mut().insert(
                tonic::codegen::GrpcMethod::new(#package_name, stringify!(#method_name))
            );

            let response = self.inner.unary(proto_req, path, codec).await?;

            #response_conversion
        }
    }
}

fn generate_streaming_client_method(method: &MethodInfo, package_name: &str, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let inner_response_type = method.inner_response_type.as_ref().unwrap();
    let route_path = generate_route_path(package_name, trait_name, method_name);

    let ready_check = generate_ready_check();
    let codec_init = generate_codec_init();
    let request_conversion = generate_native_to_proto_request_streaming();
    let stream_conversion = generate_stream_conversion(inner_response_type);

    quote! {
        pub async fn #method_name(
            &mut self,
            request: impl tonic::IntoRequest<#request_type>,
        ) -> std::result::Result<tonic::Response<impl tonic::codegen::tokio_stream::Stream<Item = Result<#inner_response_type, tonic::Status>>>, tonic::Status> {
            #ready_check
            #codec_init
            let path = http::uri::PathAndQuery::from_static(#route_path);

            #request_conversion

            let response = self.inner.server_streaming(proto_req, path, codec).await?;

            #stream_conversion
        }
    }
}

// ============================================================================
// CLIENT COMPRESSION METHODS
// ============================================================================
pub fn generate_client_compression_methods() -> TokenStream {
    quote! {
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }

        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }

        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }

        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_client_module_generation() {
        let trait_name: syn::Ident = parse_quote! { TestService };
        let vis: syn::Visibility = parse_quote! { pub };
        let methods = vec![];

        let module = generate_client_module(&trait_name, &vis, "test_package", &methods);

        let module_str = module.to_string();
        assert!(module_str.contains("test_service_client"));
        assert!(module_str.contains("TestServiceClient"));
    }
}
