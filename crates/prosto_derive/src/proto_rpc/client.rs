//! Client generation - refactored to use common RPC utilities

use proc_macro2::TokenStream;
use quote::quote;

use crate::parse::InterceptorConfig;
use crate::proto_rpc::rpc_common::client_module_name;
use crate::proto_rpc::rpc_common::client_struct_name;
use crate::proto_rpc::rpc_common::generate_client_with_interceptor;
use crate::proto_rpc::rpc_common::generate_route_path;
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
    let with_interceptor = generate_client_with_interceptor(&client_struct, interceptor_config.is_some());
    let (
        client_struct_generics,
        client_struct_fields,
        client_struct_init,
        client_impl_generics,
        client_connect_impl_generics,
        client_connect_type_args,
    ) = if interceptor_config.is_some() {
        (
            quote! { <T, Ctx> },
            quote! { inner: tonic::client::Grpc<T>, max_encode_preallocation: usize, _ctx: ::core::marker::PhantomData<Ctx> },
            quote! { Self { inner, max_encode_preallocation: ::proto_rs::DEFAULT_MAX_ENCODE_PREALLOCATION, _ctx: ::core::marker::PhantomData } },
            quote! { <T, Ctx> },
            quote! { <Ctx> },
            quote! { <tonic::transport::Channel, Ctx> },
        )
    } else {
        (
            quote! { <T> },
            quote! { inner: tonic::client::Grpc<T>, max_encode_preallocation: usize },
            quote! { Self { inner, max_encode_preallocation: ::proto_rs::DEFAULT_MAX_ENCODE_PREALLOCATION } },
            quote! { <T> },
            quote! {},
            quote! { <tonic::transport::Channel> },
        )
    };

    let connect_impl = if cfg!(feature = "tonic-transport") {
        let auto_args = if interceptor_config.is_some() {
            quote! { <::proto_rs::grpc::AutoChannel, Ctx> }
        } else {
            quote! { <::proto_rs::grpc::AutoChannel> }
        };
        quote! {
            impl #client_connect_impl_generics #client_struct #auto_args {
                /// Select owned Linux sends when requested, with automatic
                /// ordinary-write/TLS fallback and one warning per channel.
                pub async fn connect_auto(
                    endpoint: tonic::transport::Endpoint,
                    options: ::proto_rs::grpc::ChannelOptions,
                ) -> Result<Self, tonic::codegen::StdError> {
                    Ok(Self::new(::proto_rs::grpc::AutoChannel::connect(endpoint, options).await?))
                }
            }
            impl #client_connect_impl_generics #client_struct #client_connect_type_args {
                pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
                where
                    D: TryInto<tonic::transport::Endpoint>,
                    D::Error: Into<StdError>,
                {
                    let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
                    Ok(Self::new(conn))
                }
            }
        }
    } else {
        TokenStream::new()
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
            use ::proto_rs::tonic_crate as tonic;
            use tonic::codegen::*;
            use super::*;

            #[derive(Debug, Clone)]
            pub struct #client_struct #client_struct_generics {
                #client_struct_fields,
            }

            #connect_impl

            impl #client_impl_generics #client_struct #client_struct_generics
            where
                T: tonic::client::GrpcService<tonic::body::Body> + Send,
                T::Future: Send,
                T::Error: Into<StdError>,
                T::ResponseBody: Body<Data = ::proto_rs::bytes::Bytes> + ::core::marker::Send + 'static,
                <T::ResponseBody as Body>::Error: Into<StdError> + ::core::marker::Send,
            {
                pub fn new(inner: T) -> Self {
                    let inner = tonic::client::Grpc::new(inner);
                    #client_struct_init
                }

                pub fn with_origin(inner: T, origin: http::Uri) -> Self {
                    let inner = tonic::client::Grpc::with_origin(inner, origin);
                    #client_struct_init
                }

                #with_interceptor

                #compression_methods

                #(#client_methods)*
            }
        }
    }
}

pub fn generate_transport_client_module(
    trait_name: &syn::Ident,
    vis: &syn::Visibility,
    package_name: &str,
    methods: &[MethodInfo],
) -> TokenStream {
    let module_name = syn::Ident::new(
        &format!("{}_transport_client", crate::utils::to_snake_case(&trait_name.to_string())),
        trait_name.span(),
    );
    let client_name = syn::Ident::new(&format!("{trait_name}TransportClient"), trait_name.span());
    let methods = methods.iter().map(|method| {
        let method_name = &method.name;
        let request = &method.request_type;
        let route = generate_route_path(package_name, trait_name, method_name);
        if method.is_streaming {
            let response = method.inner_response_type.as_ref().expect("stream response type");
            quote! {
                pub async fn #method_name(
                    &mut self,
                    request: ::proto_rs::grpc::Request<#request>,
                ) -> ::core::result::Result<
                    ::proto_rs::grpc::Response<T::ResponseStream<#response>>,
                    T::Error,
                > {
                    self.inner.server_streaming(#route, request).await
                }
            }
        } else {
            let response = &method.response_type;
            quote! {
                pub async fn #method_name(
                    &mut self,
                    request: ::proto_rs::grpc::Request<#request>,
                ) -> ::core::result::Result<::proto_rs::grpc::Response<#response>, T::Error> {
                    self.inner.unary(#route, request).await
                }
            }
        }
    });

    quote! {
        #vis mod #module_name {
            use super::*;
            use ::proto_rs::grpc::GrpcTransport as _;

            pub struct #client_name<T> {
                inner: T,
            }

            impl<T> #client_name<T> {
                pub const fn new(inner: T) -> Self {
                    Self { inner }
                }

                pub fn into_inner(self) -> T {
                    self.inner
                }
            }

            impl<T: ::proto_rs::grpc::GrpcTransport> #client_name<T> {
                #(#methods)*
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
    let method_name = &method.name;
    let request_type = &method.request_type;
    let (response_type, future, call) = if method.is_streaming {
        (
            method.inner_response_type.as_ref().expect("stream response type"),
            quote! { Streaming },
            quote! { prepared_streaming },
        )
    } else {
        (&method.response_type, quote! { Unary }, quote! { prepared_unary })
    };
    let route_path = generate_route_path(package_name, trait_name, method_name);

    // Generate ctx parameter and interceptor call if configured
    let (ctx_param, interceptor_call, interceptor_generics, interceptor_bounds) = if let Some(config) = interceptor_config {
        let trait_ident = &config.trait_ident;

        let ctx_param = quote! { ctx: I, };
        let interceptor_call = quote! {
            let ctx_payload: Ctx::Payload = ::core::convert::Into::into(ctx);
            Ctx::intercept(ctx_payload, &mut request)?;
        };
        let interceptor_generics = quote! { , I };
        let interceptor_bounds = quote! {
            I: ::core::convert::Into<Ctx::Payload>,
            Ctx: #trait_ident
        };
        (ctx_param, interceptor_call, interceptor_generics, interceptor_bounds)
    } else {
        (quote! {}, quote! {}, quote! {}, quote! {})
    };

    quote! {
        pub fn #method_name<R #interceptor_generics>(
            &mut self,
            #ctx_param
            request: R,
        ) -> <tonic::client::Grpc<T> as ::proto_rs::PreparedRpc<#response_type>>::#future<'_>
        where
            R: ::proto_rs::PrepareRequest<#request_type>,
            #interceptor_bounds
        {
            let prepared = (|| {
            let mut request = request.prepare_request(self.max_encode_preallocation)?;
            #interceptor_call
            request.extensions_mut().insert(
                tonic::codegen::GrpcMethod::new(#package_name, stringify!(#method_name))
            );

            Ok(request)
            })();
            let path = http::uri::PathAndQuery::from_static(#route_path);
            ::proto_rs::PreparedRpc::<#response_type>::#call(&mut self.inner, prepared, path)
        }
    }
}

// ============================================================================
// CLIENT COMPRESSION METHODS
// ============================================================================
pub fn generate_client_compression_methods() -> TokenStream {
    quote! {
        /// Limit speculative output reservation (default 1 MiB, minimum 64 bytes).
        /// This is not a message size or total memory limit.
        #[must_use]
        pub fn with_max_encode_preallocation(mut self, limit: usize) -> Self {
            self.max_encode_preallocation = limit;
            self
        }

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
        assert_eq!(module_str.contains("pub async fn connect"), cfg!(feature = "tonic-transport"));
    }
}
