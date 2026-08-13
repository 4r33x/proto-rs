use proc_macro2::TokenStream;
use quote::quote;

use crate::proto_rpc::rpc_common::generate_response_proto_type;
use crate::proto_rpc::rpc_common::generate_route_path;
use crate::utils::MethodInfo;

pub fn generate_service_module(trait_name: &syn::Ident, vis: &syn::Visibility, package_name: &str, methods: &[MethodInfo]) -> TokenStream {
    let module_name = syn::Ident::new(
        &format!("{}_service", crate::utils::to_snake_case(&trait_name.to_string())),
        trait_name.span(),
    );
    let router_name = syn::Ident::new(&format!("{trait_name}Router"), trait_name.span());
    let descriptors = methods.iter().map(|method| {
        let route = generate_route_path(package_name, trait_name, &method.name);
        let kind = if method.is_streaming {
            quote! { ::proto_rs::grpc::RpcKind::ServerStreaming }
        } else {
            quote! { ::proto_rs::grpc::RpcKind::Unary }
        };
        quote! {
            ::proto_rs::grpc::MethodDescriptor {
                path: #route,
                kind: #kind,
            }
        }
    });
    let routes = methods.iter().map(|method| generate_route(trait_name, method, package_name));

    quote! {
        #vis mod #module_name {
            use super::*;

            pub const METHODS: &[::proto_rs::grpc::MethodDescriptor] = &[
                #(#descriptors,)*
            ];

            pub struct #router_name<S> {
                inner: ::proto_rs::alloc::sync::Arc<S>,
            }

            impl<S> #router_name<S> {
                pub fn new(service: S) -> Self {
                    Self {
                        inner: ::proto_rs::alloc::sync::Arc::new(service),
                    }
                }

                pub fn from_arc(service: ::proto_rs::alloc::sync::Arc<S>) -> Self {
                    Self { inner: service }
                }

                pub fn into_inner(self) -> ::proto_rs::alloc::sync::Arc<S> {
                    self.inner
                }
            }

            impl<S> ::proto_rs::grpc::GrpcService for #router_name<S>
            where
                S: super::#trait_name + ::core::marker::Send + ::core::marker::Sync + 'static,
            {
                fn methods(&self) -> &'static [::proto_rs::grpc::MethodDescriptor] {
                    METHODS
                }

                async fn call(
                    &self,
                    path: &str,
                    request: ::proto_rs::grpc::Request<::proto_rs::grpc::MessageStream>,
                ) -> ::core::result::Result<
                    ::proto_rs::grpc::Response<::proto_rs::grpc::MessageStream>,
                    ::proto_rs::grpc::Status,
                > {
                    match path {
                        #(#routes,)*
                        _ => Err(::proto_rs::grpc::Status::unimplemented(
                            ::proto_rs::alloc::format!("unknown RPC method: {path}"),
                        )),
                    }
                }
            }
        }
    }
}

fn generate_route(trait_name: &syn::Ident, method: &MethodInfo, package_name: &str) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let route = generate_route_path(package_name, trait_name, method_name);
    let request = if method.request_is_wrapped {
        quote! { request }
    } else {
        quote! { request.into_inner() }
    };
    let await_suffix = if method.is_async {
        quote! { .await }
    } else {
        quote! {}
    };
    let result_suffix = if method.response_is_result {
        quote! { ? }
    } else {
        quote! {}
    };
    let response = if method.is_streaming {
        let item_type = method.stream_item_type.as_ref().expect("stream item type");
        let response_proto = generate_response_proto_type(method.inner_response_type.as_ref().expect("stream response type"));
        let normalized = if method.response_is_response {
            quote! { response }
        } else {
            quote! { ::proto_rs::grpc::Response::new(response) }
        };
        quote! {
            let response = #normalized;
            Ok(::proto_rs::grpc::encode_streaming_response::<#item_type, #response_proto, _>(response))
        }
    } else {
        let response_type = &method.response_return_type;
        let response_proto = generate_response_proto_type(&method.response_type);
        quote! {
            ::proto_rs::grpc::encode_unary_response::<#response_type, #response_proto>(response)
        }
    };

    quote! {
        #route => {
            let request = ::proto_rs::grpc::decode_unary_request::<#request_type>(request).await?;
            let response = <S as super::#trait_name>::#method_name(
                self.inner.as_ref(),
                #request,
            )#await_suffix #result_suffix;
            #response
        }
    }
}
