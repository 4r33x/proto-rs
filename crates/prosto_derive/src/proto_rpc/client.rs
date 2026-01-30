//! Client generation - refactored to use common RPC utilities

use proc_macro2::TokenStream;
use quote::quote;

use crate::parse::InterceptorConfig;
use crate::proto_rpc::rpc_common::client_module_name;
use crate::proto_rpc::rpc_common::client_struct_name;
use crate::proto_rpc::rpc_common::generate_client_with_interceptor;
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

pub fn generate_client_module(
    trait_name: &syn::Ident,
    vis: &syn::Visibility,
    package_name: &str,
    methods: &[MethodInfo],
    interceptor_config: Option<&InterceptorConfig>,
) -> TokenStream {
    let client_module = client_module_name(trait_name);
    let client_struct = client_struct_name(trait_name);

    let client_methods =
        methods.iter().map(|m| generate_client_method(m, package_name, trait_name, interceptor_config)).collect::<Vec<_>>();

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
                T::ResponseBody: Body<Data = ::proto_rs::bytes::Bytes> + ::core::marker::Send + 'static,
                <T::ResponseBody as Body>::Error: Into<StdError> + ::core::marker::Send,
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

fn generate_client_method(
    method: &MethodInfo,
    package_name: &str,
    trait_name: &syn::Ident,
    interceptor_config: Option<&InterceptorConfig>,
) -> TokenStream {
    if is_streaming_method(method) {
        generate_streaming_client_method(method, package_name, trait_name, interceptor_config)
    } else {
        generate_unary_client_method(method, package_name, trait_name, interceptor_config)
    }
}

fn generate_unary_client_method(
    method: &MethodInfo,
    package_name: &str,
    trait_name: &syn::Ident,
    interceptor_config: Option<&InterceptorConfig>,
) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let response_type = &method.response_type;
    let route_path = generate_route_path(package_name, trait_name, method_name);

    let ready_check = generate_ready_check();
    let request_conversion = generate_native_to_proto_request_unary(request_type);
    let response_conversion = generate_proto_to_native_response(response_type);

    // Generate ctx parameter and interceptor call if configured
    let (ctx_param, interceptor_call, interceptor_generics, interceptor_bounds) = if let Some(config) = interceptor_config {
        let trait_ident = &config.trait_ident;

        let ctx_param = quote! { ctx: I, };
        let interceptor_call = quote! {
            let ctx_value: ProtoInter = ::core::convert::Into::into(ctx);
            ctx_value.intercept(&mut request);
        };
        let interceptor_generics = quote! { , I, ProtoInter };
        let interceptor_bounds = quote! { I: ::core::convert::Into<ProtoInter>, ProtoInter: #trait_ident };
        (ctx_param, interceptor_call, interceptor_generics, interceptor_bounds)
    } else {
        (quote! {}, quote! {}, quote! {}, quote! {})
    };

    quote! {
        pub async fn #method_name<R #interceptor_generics>(
            &mut self,
            #ctx_param
            request: R,
        ) -> ::core::result::Result<tonic::Response<#response_type>, tonic::Status>
        where
            R: ::proto_rs::ProtoRequest<#request_type>,
            ::proto_rs::ProtoEncoder<R::Encode, R::Mode>: ::proto_rs::EncoderExt<R::Encode, R::Mode>,
            #interceptor_bounds
        {
            #request_conversion
            #ready_check
            let mut request = request.into_request();
            #interceptor_call
            request.extensions_mut().insert(
                tonic::codegen::GrpcMethod::new(#package_name, stringify!(#method_name))
            );

            let codec = ::proto_rs::ProtoCodec::<R::Encode, #response_type, R::Mode>::default();
            let path = http::uri::PathAndQuery::from_static(#route_path);
            let response = self.inner.unary(request, path, codec).await?;

            #response_conversion
        }
    }
}

fn generate_streaming_client_method(
    method: &MethodInfo,
    package_name: &str,
    trait_name: &syn::Ident,
    interceptor_config: Option<&InterceptorConfig>,
) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let inner_response_type = method.inner_response_type.as_ref().unwrap();
    let route_path = generate_route_path(package_name, trait_name, method_name);

    let ready_check = generate_ready_check();
    let request_conversion = generate_native_to_proto_request_streaming(request_type);
    let stream_conversion = generate_stream_conversion(inner_response_type);

    // Generate ctx parameter and interceptor call if configured
    let (ctx_param, interceptor_call, interceptor_generics, interceptor_bounds) = if let Some(config) = interceptor_config {
        let trait_ident = &config.trait_ident;

        let ctx_param = quote! { ctx: I, };
        let interceptor_call = quote! {
            let ctx_value: ProtoInter = ::core::convert::Into::into(ctx);
            ctx_value.intercept(&mut request);
        };
        let interceptor_generics = quote! { , I, ProtoInter };
        let interceptor_bounds = quote! { I: ::core::convert::Into<ProtoInter>, ProtoInter: #trait_ident };
        (ctx_param, interceptor_call, interceptor_generics, interceptor_bounds)
    } else {
        (quote! {}, quote! {}, quote! {}, quote! {})
    };

    quote! {
        pub async fn #method_name<R #interceptor_generics>(
            &mut self,
            #ctx_param
            request: R,
        ) -> ::core::result::Result<tonic::Response<impl tonic::codegen::tokio_stream::Stream<Item = ::core::result::Result<#inner_response_type, tonic::Status>>>, tonic::Status>
        where
            R: ::proto_rs::ProtoRequest<#request_type>,
            ::proto_rs::ProtoEncoder<R::Encode, R::Mode>: ::proto_rs::EncoderExt<R::Encode, R::Mode>,
            #interceptor_bounds
        {
            #request_conversion
            #ready_check
            let mut request = request.into_request();
            #interceptor_call
            let codec = ::proto_rs::ProtoCodec::<R::Encode, #inner_response_type, R::Mode>::default();
            let path = http::uri::PathAndQuery::from_static(#route_path);
            let response = self.inner.server_streaming(request, path, codec).await?;

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

        let module = generate_client_module(&trait_name, &vis, "test_package", &methods, None);

        let module_str = module.to_string();
        assert!(module_str.contains("test_service_client"));
        assert!(module_str.contains("TestServiceClient"));
    }
}
