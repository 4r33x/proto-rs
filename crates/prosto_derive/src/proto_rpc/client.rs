//! Client generation - refactored to use common RPC utilities

use proc_macro2::TokenStream;
use quote::quote;

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

pub fn generate_client_module(trait_name: &syn::Ident, vis: &syn::Visibility, package_name: &str, methods: &[MethodInfo], trait_generics: &syn::Generics) -> TokenStream {
    let client_module = client_module_name(trait_name);
    let client_struct = client_struct_name(trait_name);
    let (_, ty_generics, where_clause) = trait_generics.split_for_impl();
    let where_clause_tokens = where_clause.map(|wc| quote! { #wc }).unwrap_or_default();
    let generic_params: Vec<_> = trait_generics.params.iter().collect();
    let type_params: Vec<_> = trait_generics.type_params().map(|p| &p.ident).collect();
    let mut struct_generics_items: Vec<TokenStream> = vec![quote! { T }];
    struct_generics_items.extend(generic_params.iter().map(|param| quote! { #param }));
    let struct_generics = quote! { <#(#struct_generics_items),*> };

    let mut struct_type_args_items: Vec<TokenStream> = vec![quote! { T }];
    struct_type_args_items.extend(type_params.iter().map(|param| quote! { #param }));
    let struct_type_args = quote! { <#(#struct_type_args_items),*> };

    let mut channel_type_args_items: Vec<TokenStream> = vec![quote! { tonic::transport::Channel }];
    channel_type_args_items.extend(type_params.iter().map(|param| quote! { #param }));
    let channel_type_args = quote! { <#(#channel_type_args_items),*> };

    let client_methods = methods.iter().map(|m| generate_client_method(m, package_name, trait_name)).collect::<Vec<_>>();

    let compression_methods = generate_client_compression_methods();
    let mut with_interceptor_items: Vec<TokenStream> = vec![quote! { InterceptedService<T, F> }];
    with_interceptor_items.extend(type_params.iter().map(|param| quote! { #param }));
    let with_interceptor_args = quote! { <#(#with_interceptor_items),*> };
    let with_interceptor = generate_client_with_interceptor(&client_struct, with_interceptor_args);

    let phantom_field = if type_params.is_empty() {
        quote! {}
    } else {
        quote! { _marker: ::core::marker::PhantomData<(#(#type_params),*)>, }
    };
    let phantom_init = if type_params.is_empty() {
        quote! {}
    } else {
        quote! { _marker: ::core::marker::PhantomData, }
    };

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
            pub struct #client_struct #struct_generics {
                inner: tonic::client::Grpc<T>,
                #phantom_field
            }

            impl #ty_generics #client_struct #channel_type_args #where_clause_tokens {
                pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
                where
                    D: TryInto<tonic::transport::Endpoint>,
                    D::Error: Into<StdError>,
                {
                    let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
                    Ok(Self::new(conn))
                }
            }

            impl #struct_generics #client_struct #struct_type_args
            where
                T: tonic::client::GrpcService<tonic::body::Body>,
                T::Error: Into<StdError>,
                T::ResponseBody: Body<Data = ::proto_rs::bytes::Bytes> + ::core::marker::Send + 'static,
                <T::ResponseBody as Body>::Error: Into<StdError> + ::core::marker::Send,
                #where_clause_tokens
            {
                pub fn new(inner: T) -> Self {
                    let inner = tonic::client::Grpc::new(inner);
                    Self { inner, #phantom_init }
                }

                pub fn with_origin(inner: T, origin: http::Uri) -> Self {
                    let inner = tonic::client::Grpc::with_origin(inner, origin);
                    Self { inner, #phantom_init }
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
    let request_conversion = generate_native_to_proto_request_unary(request_type);
    let response_conversion = generate_proto_to_native_response(response_type);

    quote! {
        pub async fn #method_name<R>(
            &mut self,
            request: R,
        ) -> ::core::result::Result<tonic::Response<#response_type>, tonic::Status>
        where
            R: ::proto_rs::ProtoRequest<#request_type>,
            ::proto_rs::ProtoEncoder<R::Encode, R::Mode>: ::proto_rs::EncoderExt<R::Encode, R::Mode>,
        {
            #request_conversion
            #ready_check
            let mut request = request.into_request();
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

fn generate_streaming_client_method(method: &MethodInfo, package_name: &str, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let inner_response_type = method.inner_response_type.as_ref().unwrap();
    let route_path = generate_route_path(package_name, trait_name, method_name);

    let ready_check = generate_ready_check();
    let request_conversion = generate_native_to_proto_request_streaming(request_type);
    let stream_conversion = generate_stream_conversion(inner_response_type);

    quote! {
        pub async fn #method_name<R>(
            &mut self,
            request: R,
        ) -> ::core::result::Result<tonic::Response<impl tonic::codegen::tokio_stream::Stream<Item = ::core::result::Result<#inner_response_type, tonic::Status>>>, tonic::Status>
        where
            R: ::proto_rs::ProtoRequest<#request_type>,
            ::proto_rs::ProtoEncoder<R::Encode, R::Mode>: ::proto_rs::EncoderExt<R::Encode, R::Mode>,
        {
            #request_conversion
            #ready_check
            let request = request.into_request();
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

        let module = generate_client_module(&trait_name, &vis, "test_package", &methods);

        let module_str = module.to_string();
        assert!(module_str.contains("test_service_client"));
        assert!(module_str.contains("TestServiceClient"));
    }
}
