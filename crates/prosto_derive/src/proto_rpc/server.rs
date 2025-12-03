//! Server generation - refactored to use common RPC utilities

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::proto_rpc::rpc_common::generate_codec_init;
use crate::proto_rpc::rpc_common::generate_proto_to_native_request;
use crate::proto_rpc::rpc_common::generate_request_proto_type;
use crate::proto_rpc::rpc_common::generate_response_proto_type;
use crate::proto_rpc::rpc_common::generate_route_path;
use crate::proto_rpc::rpc_common::generate_service_constructors;
use crate::proto_rpc::rpc_common::generate_service_struct_fields;
use crate::proto_rpc::rpc_common::is_streaming_method;
use crate::proto_rpc::rpc_common::server_module_name;
use crate::proto_rpc::rpc_common::server_struct_name;
use crate::proto_rpc::utils::associated_future_type;
use crate::proto_rpc::utils::method_future_return_type;
use crate::proto_rpc::utils::wrap_async_block;
use crate::utils::MethodInfo;
use crate::utils::to_pascal_case;

fn is_response_wrapper(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Path(type_path)
            if type_path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "Response")
    )
}

fn response_to_proto_response(response_return_type: &Type, response_binding: &TokenStream, response_proto: &TokenStream) -> TokenStream {
    let normalized = if is_response_wrapper(response_return_type) {
        quote! { #response_binding }
    } else {
        quote! { #response_binding.into() }
    };

    quote! {
        <#response_return_type as ::proto_rs::ProtoResponse<#response_proto>>::into_response(
            #normalized
        )
    }
}

fn wrap_call_future(is_async: bool, body: TokenStream) -> TokenStream {
    if is_async {
        wrap_async_block(quote! { async move { #body } }, true)
    } else {
        if cfg!(feature = "stable") {
            wrap_async_block(quote! { async move { #body } }, true)
        } else {
            wrap_async_block(quote! {  {::core::future::ready( #body )}  }, false)
        }
    }
}

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
    let service_future_type = associated_future_type(quote! { ::core::result::Result<Self::Response, Self::Error> }, false);
    let call_future_body = wrap_async_block(
        quote! {
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
        },
        true,
    );

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

            pub trait #trait_name: ::core::marker::Send + ::core::marker::Sync + 'static {
                #(#associated_types)*
                #(#trait_methods)*
            }

            impl<T> #trait_name for T
            where
                T: super::#trait_name + ::core::marker::Send + ::core::marker::Sync + 'static,
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
                B: Body + ::core::marker::Send + 'static,
                B::Error: Into<StdError> + ::core::marker::Send + 'static,
            {
                type Response = http::Response<tonic::body::Body>;
                type Error = ::core::convert::Infallible;
                type Future = #service_future_type;

                fn poll_ready(
                    &mut self,
                    _cx: &mut Context<'_>
                ) -> Poll<::core::result::Result<(), Self::Error>> {
                    Poll::Ready(Ok(()))
                }

                fn call(&mut self, req: http::Request<B>) -> Self::Future {
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    #call_future_body
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
        let return_type = if method.response_is_result {
            quote! { ::core::result::Result<tonic::Response<Self::#stream_name>, tonic::Status> }
        } else {
            quote! { tonic::Response<Self::#stream_name> }
        };
        let method_return = if method.is_async { method_future_return_type(return_type) } else { return_type };
        quote! {
            #[must_use]
            fn #method_name(
                &self,
                request: tonic::Request<#request_proto>,
            ) -> #method_return
            where
                Self: ::core::marker::Send + ::core::marker::Sync;
        }
    } else {
        let response_type = &method.response_type;
        let response_return_type = &method.response_return_type;
        let response_proto = generate_response_proto_type(response_type);
        let return_type = quote! {
            ::core::result::Result<
                tonic::Response<
                    <#response_return_type as ::proto_rs::ProtoResponse<#response_proto>>::Encode
                >,
                tonic::Status
            >
        };
        let method_return = if method.is_async { method_future_return_type(return_type) } else { return_type };
        quote! {
            #[must_use]
            fn #method_name(
                &self,
                request: tonic::Request<#request_proto>,
            ) -> #method_return
            where
                Self: ::core::marker::Send + ::core::marker::Sync;
        }
    }
}

fn generate_stream_associated_type(method: &MethodInfo) -> TokenStream {
    let stream_name = method.stream_type_name.as_ref().unwrap();
    let item_type = method.stream_item_type.as_ref().unwrap();

    quote! {
        type #stream_name: tonic::codegen::tokio_stream::Stream<Item = ::core::result::Result<#item_type, tonic::Status>> + ::core::marker::Send + 'static;
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
    let response_return_type = &method.response_return_type;
    let request_proto = generate_request_proto_type(request_type);
    let response_proto = generate_response_proto_type(response_type);

    let request_conversion = generate_proto_to_native_request(request_type);
    let response_conversion = response_to_proto_response(response_return_type, &quote! { native_response }, &response_proto);

    if method.is_async {
        let await_suffix = if method.response_is_result {
            quote! { .await? }
        } else {
            quote! { .await }
        };
        let return_type = method_future_return_type(quote! {
            ::core::result::Result<
                tonic::Response<
                    <#response_return_type as ::proto_rs::ProtoResponse<#response_proto>>::Encode
                >,
                tonic::Status
            >
        });

        quote! {
            fn #method_name(
                &self,
                request: tonic::Request<#request_proto>,
            ) -> #return_type {
                async move {
                    #request_conversion

                    let native_response = <Self as super::#trait_name>::#method_name(
                        self,
                        native_request
                    )#await_suffix;

                    let response = #response_conversion;

                    Ok(response)
                }
            }
        }
    } else {
        let return_type = quote! {
            ::core::result::Result<
                tonic::Response<
                    <#response_return_type as ::proto_rs::ProtoResponse<#response_proto>>::Encode
                >,
                tonic::Status
            >
        };
        let question = if method.response_is_result {
            quote! { ? }
        } else {
            quote! {}
        };

        quote! {
            fn #method_name(
                &self,
                request: tonic::Request<#request_proto>,
            ) -> #return_type {
                #request_conversion

                let native_response = <Self as super::#trait_name>::#method_name(
                    self,
                    native_request
                )#question;

                let response = #response_conversion;

                Ok(response)
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

    if method.response_is_result {
        let result_type = quote! { ::core::result::Result<tonic::Response<Self::#stream_name>, tonic::Status> };
        if method.is_async {
            let return_type = method_future_return_type(result_type.clone());
            quote! {
                fn #method_name(
                    &self,
                    request: tonic::Request<#request_proto>,
                ) -> #return_type {
                    async move {
                        #request_conversion

                        <Self as super::#trait_name>::#method_name(
                            self,
                            native_request
                        ).await
                    }
                }
            }
        } else {
            quote! {
                fn #method_name(
                    &self,
                    request: tonic::Request<#request_proto>,
                ) -> #result_type {
                    #request_conversion

                    <Self as super::#trait_name>::#method_name(
                        self,
                        native_request
                    )
                }
            }
        }
    } else {
        let ok_type = quote! { tonic::Response<Self::#stream_name> };
        if method.is_async {
            let return_type = method_future_return_type(ok_type.clone());
            quote! {
                fn #method_name(
                    &self,
                    request: tonic::Request<#request_proto>,
                ) -> #return_type {
                    async move {
                        #request_conversion

                        <Self as super::#trait_name>::#method_name(
                            self,
                            native_request
                        ).await
                    }
                }
            }
        } else {
            quote! {
                fn #method_name(
                    &self,
                    request: tonic::Request<#request_proto>,
                ) -> #ok_type {
                    #request_conversion

                    <Self as super::#trait_name>::#method_name(
                        self,
                        native_request
                    )
                }
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
    let response_return_type = &method.response_return_type;
    let request_proto = generate_request_proto_type(request_type);
    let response_proto = generate_response_proto_type(response_type);

    let encode_type = quote! {
        <#response_return_type as ::proto_rs::ProtoResponse<#response_proto>>::Encode
    };
    let mode_type = quote! {
        <#response_return_type as ::proto_rs::ProtoResponse<#response_proto>>::Mode
    };
    let decode_type = quote! { #request_proto };
    let codec_init = generate_codec_init(encode_type.clone(), decode_type, Some(mode_type));
    let await_suffix = if method.is_async {
        quote! { .await }
    } else {
        quote! {}
    };
    let future_type = associated_future_type(quote! { ::core::result::Result<tonic::Response<Self::Response>, tonic::Status> }, true);
    let call_future = wrap_call_future(
        method.is_async,
        quote! {
            <T as #trait_name>::#method_name(&inner, request)#await_suffix
        },
    );

    quote! {
        #route_path => {
            #[allow(non_camel_case_types)]
            struct #svc_name<T: #trait_name>(pub Arc<T>);

            impl<T: #trait_name> tonic::server::UnaryService<#request_proto> for #svc_name<T> {
                type Response = <#response_return_type as ::proto_rs::ProtoResponse<#response_proto>>::Encode;
                type Future = #future_type;

                fn call(&mut self, request: tonic::Request<#request_proto>) -> Self::Future {
                    let inner = Arc::clone(&self.0);
                    #call_future
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
    let item_type = method.stream_item_type.as_ref().unwrap();
    let stream_name = method.stream_type_name.as_ref().unwrap();
    let request_proto = generate_request_proto_type(request_type);
    let response_proto = generate_response_proto_type(inner_type);

    let encode_type = quote! { <#item_type as ::proto_rs::ProtoResponse<#response_proto>>::Encode };
    let decode_type = quote! { #request_proto };
    let mode_type = quote! { <#item_type as ::proto_rs::ProtoResponse<#response_proto>>::Mode };
    let codec_init = generate_codec_init(encode_type, decode_type, Some(mode_type));
    let await_suffix = if method.is_async {
        quote! { .await }
    } else {
        quote! {}
    };
    let await_question_suffix = if method.is_async {
        quote! { .await? }
    } else {
        quote! { ? }
    };

    let (future_type, call_future) = if method.response_is_result {
        let future_type = associated_future_type(quote! { ::core::result::Result<tonic::Response<Self::ResponseStream>, tonic::Status> }, true);
        let body = quote! {
            let response = <T as #trait_name>::#method_name(&inner, request)#await_question_suffix;
            let mapped = response.map(|stream| {
                ::tonic::codegen::tokio_stream::StreamExt::map(
                    stream,
                    ::proto_rs::map_proto_stream_result::<#item_type, #response_proto>
                        as fn(
                            ::core::result::Result<#item_type, tonic::Status>,
                        ) -> ::core::result::Result<
                            <#item_type as ::proto_rs::ProtoResponse<#response_proto>>::Encode,
                            tonic::Status
                        >,
                )
            });
            Ok(mapped)
        };
        (future_type, wrap_call_future(method.is_async, body))
    } else {
        let future_type = associated_future_type(quote! { ::core::result::Result<tonic::Response<Self::ResponseStream>, tonic::Status> }, true);
        let body = quote! {
            let response = <T as #trait_name>::#method_name(&inner, request)#await_suffix;
            let mapped = response.map(|stream| {
                ::tonic::codegen::tokio_stream::StreamExt::map(
                    stream,
                    ::proto_rs::map_proto_stream_result::<#item_type, #response_proto>
                        as fn(
                            ::core::result::Result<#item_type, tonic::Status>,
                        ) -> ::core::result::Result<
                            <#item_type as ::proto_rs::ProtoResponse<#response_proto>>::Encode,
                            tonic::Status
                        >,
                )
            });
            Ok(mapped)
        };
        (future_type, wrap_call_future(method.is_async, body))
    };

    quote! {
        #route_path => {
            #[allow(non_camel_case_types)]
            struct #svc_name<T: #trait_name>(pub Arc<T>);

            impl<T: #trait_name> tonic::server::ServerStreamingService<#request_proto> for #svc_name<T> {
                type Response = <#item_type as ::proto_rs::ProtoResponse<#response_proto>>::Encode;
                type ResponseStream = ::tonic::codegen::tokio_stream::adapters::Map<
                    T::#stream_name,
                    fn(
                        ::core::result::Result<#item_type, tonic::Status>
                    ) -> ::core::result::Result<
                        <#item_type as ::proto_rs::ProtoResponse<#response_proto>>::Encode,
                        tonic::Status
                    >,
                >;
                type Future = #future_type;

                fn call(&mut self, request: tonic::Request<#request_proto>) -> Self::Future {
                    let inner = Arc::clone(&self.0);
                    #call_future
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
