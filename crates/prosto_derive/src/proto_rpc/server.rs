//! Server generation - refactored to use common RPC utilities

use proc_macro2::TokenStream;
use quote::quote;

use crate::proto_rpc::rpc_common::generate_codec_init;
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

    // Generate trait methods and associated types
    let (trait_methods, associated_types) = generate_trait_components(methods);

    // Generate blanket impl
    let (blanket_types, blanket_methods) = generate_blanket_impl_components(methods, trait_name);

    // Generate route handlers
    let route_handlers = methods.iter().map(|m| generate_route_handler(m, package_name, trait_name)).collect::<Vec<_>>();

    let service_name_value = format!("{}.{}", package_name, trait_name);
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

            // Trait definition
            pub trait #trait_name: std::marker::Send + std::marker::Sync + 'static {
                #(#associated_types)*
                #(#trait_methods)*
            }

            // Blanket implementation
            impl<T> #trait_name for T
            where
                T: super::#trait_name + std::marker::Send + std::marker::Sync + 'static,
            {
                #(#blanket_types)*
                #(#blanket_methods)*
            }

            // Server struct
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

            // Service implementation
            impl<T, B> tonic::codegen::Service<http::Request<B>> for #server_struct<T>
            where
                T: #trait_name,
                B: Body + std::marker::Send + 'static,
                B::Error: Into<StdError> + std::marker::Send + 'static,
            {
                type Response = http::Response<tonic::body::Body>;
                type Error = std::convert::Infallible;
                type Future = BoxFuture<Self::Response, Self::Error>;

                fn poll_ready(
                    &mut self,
                    _cx: &mut Context<'_>
                ) -> Poll<std::result::Result<(), Self::Error>> {
                    Poll::Ready(Ok(()))
                }

                fn call(&mut self, req: http::Request<B>) -> Self::Future {
                    match req.uri().path() {
                        #(#route_handlers)*
                        _ => Box::pin(async move {
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
                        }),
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
    let request_proto = quote! { <#request_type as super::HasProto>::Proto };

    if is_streaming_method(method) {
        let stream_name = method.stream_type_name.as_ref().unwrap();
        quote! {
            #[must_use]
            #[allow(elided_named_lifetimes, clippy::type_complexity, clippy::type_repetition_in_bounds)]
            fn #method_name<'life0, 'async_trait>(
                &'life0 self,
                request: tonic::Request<#request_proto>,
            ) -> ::core::pin::Pin<Box<
                dyn ::core::future::Future<
                    Output = std::result::Result<tonic::Response<Self::#stream_name>, tonic::Status>
                > + ::core::marker::Send + 'async_trait
            >>
            where
                'life0: 'async_trait,
                Self: 'async_trait;
        }
    } else {
        let response_type = &method.response_type;
        let response_proto = quote! { <#response_type as super::HasProto>::Proto };
        quote! {
            #[must_use]
            #[allow(elided_named_lifetimes, clippy::type_complexity, clippy::type_repetition_in_bounds)]
            fn #method_name<'life0, 'async_trait>(
                &'life0 self,
                request: tonic::Request<#request_proto>,
            ) -> ::core::pin::Pin<Box<
                dyn ::core::future::Future<
                    Output = std::result::Result<tonic::Response<#response_proto>, tonic::Status>
                > + ::core::marker::Send + 'async_trait
            >>
            where
                'life0: 'async_trait,
                Self: 'async_trait;
        }
    }
}

fn generate_stream_associated_type(method: &MethodInfo) -> TokenStream {
    let stream_name = method.stream_type_name.as_ref().unwrap();
    let inner_type = method.inner_response_type.as_ref().unwrap();
    let response_proto = quote! { <#inner_type as super::HasProto>::Proto };

    quote! {
        type #stream_name: tonic::codegen::tokio_stream::Stream<
            Item = std::result::Result<#response_proto, tonic::Status>
        > + std::marker::Send + 'static;
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
            blanket_types.push(generate_blanket_stream_type(method));
        }
        blanket_methods.push(generate_blanket_method(method, trait_name));
    }

    (blanket_types, blanket_methods)
}

fn generate_blanket_stream_type(method: &MethodInfo) -> TokenStream {
    let stream_name = method.stream_type_name.as_ref().unwrap();
    let inner_type = method.inner_response_type.as_ref().unwrap();
    let response_proto = quote! { <#inner_type as super::HasProto>::Proto };

    quote! {
        type #stream_name = std::pin::Pin<Box<
            dyn tonic::codegen::tokio_stream::Stream<
                Item = std::result::Result<#response_proto, tonic::Status>
            > + std::marker::Send
        >>;
    }
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
    let request_proto = quote! { <#request_type as super::HasProto>::Proto };
    let response_proto = quote! { <#response_type as super::HasProto>::Proto };

    quote! {
        fn #method_name<'life0, 'async_trait>(
            &'life0 self,
            request: tonic::Request<#request_proto>,
        ) -> ::core::pin::Pin<Box<
            dyn ::core::future::Future<
                Output = std::result::Result<tonic::Response<#response_proto>, tonic::Status>
            > + ::core::marker::Send + 'async_trait
        >>
        where
            'life0: 'async_trait,
            Self: 'async_trait
        {
            Box::pin(async move {
                // Convert proto request to native
                let (metadata, extensions, proto_msg) = request.into_parts();
                let native_msg = #request_type::from_proto(proto_msg)
                    .map_err(|e| tonic::Status::invalid_argument(
                        format!("Failed to convert request: {}", e)
                    ))?;

                // Call user trait method
                let native_request = tonic::Request::from_parts(metadata, extensions, native_msg);
                let native_response = <Self as super::#trait_name>::#method_name(
                    self,
                    native_request
                ).await?;

                // Convert native response to proto
                let (metadata, native_body, extensions) = native_response.into_parts();
                let proto_msg = native_body.to_proto();
                Ok(tonic::Response::from_parts(metadata, proto_msg, extensions))
            })
        }
    }
}

fn generate_blanket_streaming_method(method: &MethodInfo, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let stream_name = method.stream_type_name.as_ref().unwrap();
    let request_proto = quote! { <#request_type as super::HasProto>::Proto };

    quote! {
        fn #method_name<'life0, 'async_trait>(
            &'life0 self,
            request: tonic::Request<#request_proto>,
        ) -> ::core::pin::Pin<Box<
            dyn ::core::future::Future<
                Output = std::result::Result<tonic::Response<Self::#stream_name>, tonic::Status>
            > + ::core::marker::Send + 'async_trait
        >>
        where
            'life0: 'async_trait,
            Self: 'async_trait
        {
            Box::pin(async move {
                use tonic::codegen::tokio_stream::StreamExt;

                // Convert proto request to native
                let (metadata, extensions, proto_msg) = request.into_parts();
                let native_msg = #request_type::from_proto(proto_msg)
                    .map_err(|e| tonic::Status::invalid_argument(
                        format!("Failed to convert request: {}", e)
                    ))?;

                // Call user trait method
                let native_request = tonic::Request::from_parts(metadata, extensions, native_msg);
                let native_response = <Self as super::#trait_name>::#method_name(
                    self,
                    native_request
                ).await?;

                // Convert native stream to proto stream
                let (metadata, native_stream, extensions) = native_response.into_parts();
                let proto_stream = native_stream.map(|result| {
                    result.map(|native_item| native_item.to_proto())
                });

                Ok(tonic::Response::from_parts(
                    metadata,
                    Box::pin(proto_stream) as Self::#stream_name,
                    extensions
                ))
            })
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
    let request_proto = quote! { <#request_type as super::HasProto>::Proto };
    let response_proto = quote! { <#response_type as super::HasProto>::Proto };

    let codec_init = generate_codec_init();

    quote! {
        #route_path => {
            #[allow(non_camel_case_types)]
            struct #svc_name<T: #trait_name>(pub Arc<T>);

            impl<T: #trait_name> tonic::server::UnaryService<#request_proto> for #svc_name<T> {
                type Response = #response_proto;
                type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;

                fn call(&mut self, request: tonic::Request<#request_proto>) -> Self::Future {
                    let inner = Arc::clone(&self.0);
                    let fut = async move {
                        <T as #trait_name>::#method_name(&inner, request).await
                    };
                    Box::pin(fut)
                }
            }

            let accept_compression_encodings = self.accept_compression_encodings;
            let send_compression_encodings = self.send_compression_encodings;
            let max_decoding_message_size = self.max_decoding_message_size;
            let max_encoding_message_size = self.max_encoding_message_size;
            let inner = self.inner.clone();

            let fut = async move {
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
            };
            Box::pin(fut)
        }
    }
}

fn generate_streaming_route_handler(method: &MethodInfo, route_path: &str, svc_name: &syn::Ident, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let inner_type = method.inner_response_type.as_ref().unwrap();
    let stream_name = method.stream_type_name.as_ref().unwrap();
    let request_proto = quote! { <#request_type as super::HasProto>::Proto };
    let response_proto = quote! { <#inner_type as super::HasProto>::Proto };

    let codec_init = generate_codec_init();

    quote! {
        #route_path => {
            #[allow(non_camel_case_types)]
            struct #svc_name<T: #trait_name>(pub Arc<T>);

            impl<T: #trait_name> tonic::server::ServerStreamingService<#request_proto> for #svc_name<T> {
                type Response = #response_proto;
                type ResponseStream = T::#stream_name;
                type Future = BoxFuture<tonic::Response<Self::ResponseStream>, tonic::Status>;

                fn call(&mut self, request: tonic::Request<#request_proto>) -> Self::Future {
                    let inner = Arc::clone(&self.0);
                    let fut = async move {
                        <T as #trait_name>::#method_name(&inner, request).await
                    };
                    Box::pin(fut)
                }
            }

            let accept_compression_encodings = self.accept_compression_encodings;
            let send_compression_encodings = self.send_compression_encodings;
            let max_decoding_message_size = self.max_decoding_message_size;
            let max_encoding_message_size = self.max_encoding_message_size;
            let inner = self.inner.clone();

            let fut = async move {
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
            };
            Box::pin(fut)
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
