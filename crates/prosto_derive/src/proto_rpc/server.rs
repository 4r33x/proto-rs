//! Server generation - refactored to use common RPC utilities

use proc_macro2::TokenStream;
use quote::quote;

use crate::proto_rpc::rpc_common::generate_codec_init;
use crate::proto_rpc::rpc_common::generate_native_to_proto_response;
use crate::proto_rpc::rpc_common::generate_proto_to_native_request;
use crate::proto_rpc::rpc_common::generate_request_proto_type;
use crate::proto_rpc::rpc_common::generate_response_proto_type;
use crate::proto_rpc::rpc_common::generate_route_path;
use crate::proto_rpc::rpc_common::generate_service_constructors;
use crate::proto_rpc::rpc_common::generate_service_struct_fields;
use crate::proto_rpc::rpc_common::is_streaming_method;
use crate::proto_rpc::rpc_common::server_module_name;
use crate::proto_rpc::rpc_common::server_struct_name;
use crate::utils::MethodInfo;
use crate::utils::to_pascal_case;

// ============================================================================
// SERVER MODULE GENERATION
// ============================================================================

pub fn generate_server_module(trait_name: &syn::Ident, vis: &syn::Visibility, package_name: &str, methods: &[MethodInfo]) -> TokenStream {
    let server_module = server_module_name(trait_name);
    let server_struct = server_struct_name(trait_name);

    let (trait_methods, associated_types) = generate_trait_components(methods);
    let (blanket_types, blanket_methods) = generate_blanket_impl_components(methods, trait_name);
    let route_handlers = methods.iter().map(|m| generate_route_handler(m, package_name, trait_name)).collect::<Vec<_>>();

    let service_name_value = format!("{package_name}.{trait_name}");
    let compression_methods = generate_server_compression_methods();
    let service_fields = generate_service_struct_fields();
    let service_constructors = generate_service_constructors();

    quote! {
        #vis mod #server_module {
            #![allow(
                unused_variables,
                dead_code,
                missing_docs,
                clippy::wildcard_imports,
                clippy::let_unit_value
            )]
            use tonic::codegen::*;
            use super::*;

            pub trait #trait_name: std::marker::Send + std::marker::Sync + 'static {
                #(#associated_types)*
                #(#trait_methods)*
            }

            impl<T> #trait_name for T
            where
                T: super::#trait_name + std::marker::Send + std::marker::Sync + 'static,
            {
                #(#blanket_types)*
                #(#blanket_methods)*
            }

            #[derive(Debug)]
            pub struct #server_struct<T> {
                #service_fields
            }

            impl<T> #server_struct<T> {
                #service_constructors

                pub fn with_interceptor<F>(
                    inner: T,
                    interceptor: F,
                ) -> InterceptedService<Self, F>
                where
                    F: tonic::service::Interceptor,
                {
                    InterceptedService::new(Self::new(inner), interceptor)
                }

                #compression_methods
            }

            impl<T, B> tonic::codegen::Service<http::Request<B>> for #server_struct<T>
            where
                T: #trait_name,
                B: Body + std::marker::Send + 'static,
                B::Error: Into<StdError> + std::marker::Send + 'static,
            {
                type Response = http::Response<tonic::body::Body>;
                type Error = std::convert::Infallible;
                type Future = impl std::future::Future<Output = std::result::Result<Self::Response, Self::Error>> + std::marker::Send;

                fn poll_ready(
                    &mut self,
                    _cx: &mut Context<'_>
                ) -> Poll<std::result::Result<(), Self::Error>> {
                    Poll::Ready(Ok(()))
                }

                fn call(&mut self, req: http::Request<B>) -> Self::Future {
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    async move {
                        match req.uri().path() {
                            #(#route_handlers)*
                            _ =>  {
                                let mut response = http::Response::new(tonic::body::Body::default());
                                let headers = response.headers_mut();
                                headers.insert(
                                    tonic::Status::GRPC_STATUS,
                                    (tonic::Code::Unimplemented as i32).into(),
                                );
                                headers.insert(
                                    http::header::CONTENT_TYPE,
                                    tonic::metadata::GRPC_CONTENT_TYPE,
                                );
                                Ok(response)
                            },
                        }
                    }
                }
            }

            impl<T> Clone for #server_struct<T> {
                fn clone(&self) -> Self {
                    Self {
                        inner: self.inner.clone(),
                        accept_compression_encodings: self.accept_compression_encodings,
                        send_compression_encodings: self.send_compression_encodings,
                        max_decoding_message_size: self.max_decoding_message_size,
                        max_encoding_message_size: self.max_encoding_message_size,
                    }
                }
            }

            pub const SERVICE_NAME: &str = #service_name_value;

            impl<T> tonic::server::NamedService for #server_struct<T> {
                const NAME: &'static str = SERVICE_NAME;
            }
        }
    }
}

// ============================================================================
// TRAIT COMPONENTS
// ============================================================================

fn generate_trait_components(methods: &[MethodInfo]) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut trait_methods = Vec::new();
    let mut associated_types = Vec::new();

    for method in methods {
        trait_methods.push(generate_trait_method(method));

        if is_streaming_method(method) {
            associated_types.push(generate_stream_associated_type(method));
        }
    }

    (trait_methods, associated_types)
}

fn generate_trait_method(method: &MethodInfo) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let request_proto = generate_request_proto_type(request_type);

    if is_streaming_method(method) {
        let stream_name = method.stream_type_name.as_ref().unwrap();
        quote! {
            #[must_use]
            fn #method_name(
                &self,
                request: tonic::Request<#request_proto>,
            ) -> impl std::future::Future<
                Output = std::result::Result<tonic::Response<Self::#stream_name>, tonic::Status>
            > + std::marker::Send + '_
            where
                Self: std::marker::Send + std::marker::Sync;
        }
    } else {
        let response_type = &method.response_type;
        let response_proto = generate_response_proto_type(response_type);
        quote! {
            #[must_use]
            fn #method_name(
                &self,
                request: tonic::Request<#request_proto>,
            ) -> impl std::future::Future<
                Output = std::result::Result<tonic::Response<#response_proto>, tonic::Status>
            > + std::marker::Send + '_
            where
                Self: std::marker::Send + std::marker::Sync;
        }
    }
}

fn generate_stream_associated_type(method: &MethodInfo) -> TokenStream {
    let stream_name = method.stream_type_name.as_ref().unwrap();
    let inner_type = method.inner_response_type.as_ref().unwrap();
    let response_proto = generate_response_proto_type(inner_type);

    quote! {
        type #stream_name: tonic::codegen::tokio_stream::Stream<Item = std::result::Result<#response_proto, tonic::Status>> + std::marker::Send + 'static;
    }
}

// ============================================================================
// BLANKET IMPL COMPONENTS
// ============================================================================

fn generate_blanket_impl_components(methods: &[MethodInfo], trait_name: &syn::Ident) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut blanket_types = Vec::new();
    let mut blanket_methods = Vec::new();

    for method in methods {
        if is_streaming_method(method) {
            blanket_types.push(generate_blanket_stream_type(method, trait_name));
        }
        blanket_methods.push(generate_blanket_method(method, trait_name));
    }

    (blanket_types, blanket_methods)
}

fn generate_blanket_stream_type(method: &MethodInfo, trait_name: &syn::Ident) -> TokenStream {
    let stream_name = method.stream_type_name.as_ref().unwrap();
    quote! { type #stream_name = <Self as super::#trait_name>::#stream_name; }
}

fn generate_blanket_method(method: &MethodInfo, trait_name: &syn::Ident) -> TokenStream {
    if is_streaming_method(method) {
        generate_blanket_streaming_method(method, trait_name)
    } else {
        generate_blanket_unary_method(method, trait_name)
    }
}

fn generate_blanket_unary_method(method: &MethodInfo, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let response_type = &method.response_type;
    let request_proto = generate_request_proto_type(request_type);
    let response_proto = generate_response_proto_type(response_type);

    let request_conversion = generate_proto_to_native_request(request_type);
    let response_conversion = generate_native_to_proto_response();

    quote! {
        fn #method_name(
            &self,
            request: tonic::Request<#request_proto>,
        ) -> impl std::future::Future<
            Output = std::result::Result<tonic::Response<#response_proto>, tonic::Status>
        > + std::marker::Send + '_ {
            async move {
                #request_conversion

                let native_response = <Self as super::#trait_name>::#method_name(
                    self,
                    native_request
                ).await?;

                #response_conversion
            }
        }
    }
}

fn generate_blanket_streaming_method(method: &MethodInfo, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let stream_name = method.stream_type_name.as_ref().unwrap();
    let request_proto = generate_request_proto_type(request_type);

    let request_conversion = generate_proto_to_native_request(request_type);

    quote! {
        fn #method_name(
            &self,
            request: tonic::Request<#request_proto>,
        ) -> impl std::future::Future<
            Output = std::result::Result<tonic::Response<Self::#stream_name>, tonic::Status>
        > + std::marker::Send + '_ {
            async move {
                #request_conversion

                let native_response = <Self as super::#trait_name>::#method_name(
                    self,
                    native_request
                ).await?;

                Ok(native_response)
            }
        }
    }
}

// ============================================================================
// ROUTE HANDLER GENERATION
// ============================================================================

fn generate_route_handler(method: &MethodInfo, package_name: &str, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let route_path = generate_route_path(package_name, trait_name, method_name);
    let svc_name = syn::Ident::new(&format!("{}Svc", to_pascal_case(&method_name.to_string())), method_name.span());

    if is_streaming_method(method) {
        generate_streaming_route_handler(method, &route_path, &svc_name, trait_name)
    } else {
        generate_unary_route_handler(method, &route_path, &svc_name, trait_name)
    }
}

fn generate_unary_route_handler(method: &MethodInfo, route_path: &str, svc_name: &syn::Ident, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let response_type = &method.response_type;
    let request_proto = generate_request_proto_type(request_type);
    let response_proto = generate_response_proto_type(response_type);

    let encode_type = quote! { #response_proto };
    let decode_type = quote! { #request_proto };
    let codec_init = generate_codec_init(encode_type, decode_type, None);

    quote! {
        #route_path => {
            #[allow(non_camel_case_types)]
            struct #svc_name<T: #trait_name>(pub Arc<T>);

            impl<T: #trait_name> tonic::server::UnaryService<#request_proto> for #svc_name<T> {
                type Response = #response_proto;
                type Future = impl std::future::Future<
                        Output = std::result::Result<tonic::Response<Self::Response>, tonic::Status>
                    > + std::marker::Send + 'static;

                fn call(&mut self, request: tonic::Request<#request_proto>) -> Self::Future {
                    let inner = Arc::clone(&self.0);
                    async move {
                        <T as #trait_name>::#method_name(&inner, request).await
                    }
                }
            }

            let method = #svc_name(inner);
            #codec_init
            let mut grpc = tonic::server::Grpc::new(codec)
                .apply_compression_config(
                    accept_compression_encodings,
                    send_compression_encodings,
                )
                .apply_max_message_size_config(
                    max_decoding_message_size,
                    max_encoding_message_size,
                );
            let res = grpc.unary(method, req).await;
            Ok(res)
        }
    }
}

fn generate_streaming_route_handler(method: &MethodInfo, route_path: &str, svc_name: &syn::Ident, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let inner_type = method.inner_response_type.as_ref().unwrap();
    let stream_name = method.stream_type_name.as_ref().unwrap();
    let request_proto = generate_request_proto_type(request_type);
    let response_proto = generate_response_proto_type(inner_type);

    let encode_type = quote! { #response_proto };
    let decode_type = quote! { #request_proto };
    let codec_init = generate_codec_init(encode_type, decode_type, None);

    quote! {
        #route_path => {
            #[allow(non_camel_case_types)]
            struct #svc_name<T: #trait_name>(pub Arc<T>);

            impl<T: #trait_name> tonic::server::ServerStreamingService<#request_proto> for #svc_name<T> {
                type Response = #response_proto;
                type ResponseStream = T::#stream_name;
                type Future = impl std::future::Future<
                        Output = std::result::Result<tonic::Response<Self::ResponseStream>, tonic::Status>
                    > + std::marker::Send + 'static;

                fn call(&mut self, request: tonic::Request<#request_proto>) -> Self::Future {
                    let inner = Arc::clone(&self.0);
                    async move {
                        <T as #trait_name>::#method_name(&inner, request).await
                    }
                }
            }




            let method = #svc_name(inner);
            #codec_init
            let mut grpc = tonic::server::Grpc::new(codec)
                .apply_compression_config(
                    accept_compression_encodings,
                    send_compression_encodings,
                )
                .apply_max_message_size_config(
                    max_decoding_message_size,
                    max_encoding_message_size,
                );
            let res = grpc.server_streaming(method, req).await;
            Ok(res)
        }
    }
}

// ============================================================================
// SERVER COMPRESSION METHODS
// ============================================================================
pub fn generate_server_compression_methods() -> TokenStream {
    quote! {
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }

        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }

        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }

        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
}
