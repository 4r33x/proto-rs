use proc_macro2::TokenStream;
use quote::quote;

use super::utils::MethodInfo;
use crate::utils::to_pascal_case;
use crate::utils::to_snake_case;

pub fn generate_server_module(trait_name: &syn::Ident, vis: &syn::Visibility, package_name: &str, methods: &[MethodInfo]) -> TokenStream {
    let server_module = syn::Ident::new(&format!("{}_server", to_snake_case(&trait_name.to_string())), trait_name.span());
    let server_struct = syn::Ident::new(&format!("{}Server", trait_name), trait_name.span());

    let server_trait_methods: Vec<_> = methods.iter().map(generate_server_trait_method).collect();

    let server_associated_types: Vec<_> = methods
        .iter()
        .filter_map(|m| {
            if m.is_streaming {
                let stream_name = m.stream_type_name.as_ref().unwrap();
                let inner_type = m.inner_response_type.as_ref().unwrap();
                let response_proto = quote! { <#inner_type as super::HasProto>::Proto };
                Some(quote! {
                    type #stream_name: tonic::codegen::tokio_stream::Stream<
                        Item = std::result::Result<#response_proto, tonic::Status>
                    > + std::marker::Send + 'static;
                })
            } else {
                None
            }
        })
        .collect();

    let blanket_impl_types: Vec<_> = methods
        .iter()
        .filter_map(|m| {
            if m.is_streaming {
                let stream_name = m.stream_type_name.as_ref().unwrap();
                let inner_type = m.inner_response_type.as_ref().unwrap();
                let response_proto = quote! { <#inner_type as super::HasProto>::Proto };
                Some(quote! {
                    type #stream_name = std::pin::Pin<Box<
                        dyn tonic::codegen::tokio_stream::Stream<
                            Item = std::result::Result<#response_proto, tonic::Status>
                        > + std::marker::Send
                    >>;
                })
            } else {
                None
            }
        })
        .collect();

    let blanket_impl_methods: Vec<_> = methods.iter().map(|m| generate_blanket_impl_method(m, trait_name)).collect();

    let route_handlers: Vec<_> = methods.iter().map(|m| generate_route_handler(m, package_name, trait_name)).collect();

    let service_name_value = format!("{}.{}", package_name, trait_name);

    quote! {
        #vis mod #server_module {
            #![allow(unused_variables, dead_code, missing_docs, clippy::wildcard_imports, clippy::let_unit_value)]
            use tonic::codegen::*;
            use super::*;

            pub trait #trait_name: std::marker::Send + std::marker::Sync + 'static {
                #(#server_associated_types)*
                #(#server_trait_methods)*
            }

            impl<T> #trait_name for T
            where
                T: super::#trait_name + std::marker::Send + std::marker::Sync + 'static,
            {
                #(#blanket_impl_types)*
                #(#blanket_impl_methods)*
            }

            #[derive(Debug)]
            pub struct #server_struct<T> {
                inner: Arc<T>,
                accept_compression_encodings: EnabledCompressionEncodings,
                send_compression_encodings: EnabledCompressionEncodings,
                max_decoding_message_size: Option<usize>,
                max_encoding_message_size: Option<usize>,
            }

            impl<T> #server_struct<T> {
                pub fn new(inner: T) -> Self {
                    Self::from_arc(Arc::new(inner))
                }

                pub fn from_arc(inner: Arc<T>) -> Self {
                    Self {
                        inner,
                        accept_compression_encodings: Default::default(),
                        send_compression_encodings: Default::default(),
                        max_decoding_message_size: None,
                        max_encoding_message_size: None,
                    }
                }

                pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
                where
                    F: tonic::service::Interceptor,
                {
                    InterceptedService::new(Self::new(inner), interceptor)
                }

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

            impl<T, B> tonic::codegen::Service<http::Request<B>> for #server_struct<T>
            where
                T: #trait_name,
                B: Body + std::marker::Send + 'static,
                B::Error: Into<StdError> + std::marker::Send + 'static,
            {
                type Response = http::Response<tonic::body::Body>;
                type Error = std::convert::Infallible;
                type Future = BoxFuture<Self::Response, Self::Error>;

                fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
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
                    let inner = self.inner.clone();
                    Self {
                        inner,
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

fn generate_server_trait_method(method: &MethodInfo) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let request_proto = quote! { <#request_type as super::HasProto>::Proto };

    if method.is_streaming {
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

fn generate_blanket_impl_method(method: &MethodInfo, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let request_proto = quote! { <#request_type as super::HasProto>::Proto };

    if method.is_streaming {
        let stream_name = method.stream_type_name.as_ref().unwrap();

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

                    let (metadata, extensions, proto_msg) = request.into_parts();
                    let native_msg = #request_type::from_proto(proto_msg)
                        .map_err(|e| tonic::Status::invalid_argument(
                            format!("Failed to convert request: {}", e)
                        ))?;

                    let native_request = tonic::Request::from_parts(metadata, extensions, native_msg);
                    let native_response = <Self as super::#trait_name>::#method_name(
                        self,
                        native_request
                    ).await?;

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
    } else {
        let response_type = &method.response_type;
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
                    let (metadata, extensions, proto_msg) = request.into_parts();
                    let native_msg = #request_type::from_proto(proto_msg)
                        .map_err(|e| tonic::Status::invalid_argument(
                            format!("Failed to convert request: {}", e)
                        ))?;

                    let native_request = tonic::Request::from_parts(metadata, extensions, native_msg);
                    let native_response = <Self as super::#trait_name>::#method_name(
                        self,
                        native_request
                    ).await?;

                    let (metadata, native_body, extensions) = native_response.into_parts();
                    let proto_msg = native_body.to_proto();
                    Ok(tonic::Response::from_parts(metadata, proto_msg, extensions))
                })
            }
        }
    }
}

fn generate_route_handler(method: &MethodInfo, package_name: &str, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let route_path = format!("/{}.{}/{}", package_name, trait_name, to_pascal_case(&method_name.to_string()));
    let svc_name = syn::Ident::new(&format!("{}Svc", to_pascal_case(&method_name.to_string())), method_name.span());

    if method.is_streaming {
        let request_type = &method.request_type;
        let request_proto = quote! { <#request_type as super::HasProto>::Proto };
        let inner_type = method.inner_response_type.as_ref().unwrap();
        let response_proto = quote! { <#inner_type as super::HasProto>::Proto };
        let stream_name = method.stream_type_name.as_ref().unwrap();

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
                    let codec = tonic_prost::ProstCodec::default();
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
    } else {
        let request_type = &method.request_type;
        let request_proto = quote! { <#request_type as super::HasProto>::Proto };
        let response_type = &method.response_type;
        let response_proto = quote! { <#response_type as super::HasProto>::Proto };

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
                    let codec = tonic_prost::ProstCodec::default();
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
}
