use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::ItemTrait;

mod client;
pub mod rpc_common;
mod server;
pub mod utils;
pub mod generic_helpers;

use client::generate_client_module;
use server::generate_server_module;
use utils::extract_methods_and_types;
use generic_helpers::generate_type_id_impls;

use crate::emit_proto::generate_service_content;
use crate::parse::UnifiedProtoConfig;
use crate::parse::substitute_generic_types;

pub fn proto_rpc_impl(args: TokenStream, item: TokenStream) -> TokenStream2 {
    let input: ItemTrait = syn::parse(item).expect("Failed to parse trait");
    let trait_name = &input.ident;
    let ty_ident = trait_name.to_string();
    let mut config = UnifiedProtoConfig::from_attributes(args, &ty_ident, &input.attrs, &input);
    let vis = &input.vis;
    let package_name = config.get_rpc_package().to_owned();

    // Extract methods, types, and imports
    let (methods, user_associated_types) = extract_methods_and_types(&input);

    // Check if generic types are specified
    if config.has_generic_types() {
        return handle_generic_rpc(input, config, methods, user_associated_types);
    }

    // Generate .proto file if requested
    let service_content = generate_service_content(trait_name, &methods, &config.type_imports);
    config.register_and_emit_proto(&ty_ident, &service_content);
    let proto = config.imports_mat.clone();

    // Generate user-facing trait
    let user_methods: Vec<_> = methods.iter().map(|m| &m.user_method_signature).collect();

    // Generate client module if requested
    let client_module = if config.rpc_client {
        generate_client_module(trait_name, vis, &package_name, &methods)
    } else {
        quote! {}
    };

    // Generate server module if requested
    let server_module = if config.rpc_server {
        generate_server_module(trait_name, vis, &package_name, &methods)
    } else {
        quote! {}
    };

    quote! {
        #vis trait #trait_name {
            #(#user_associated_types)*
            #(#user_methods)*
        }

        #client_module
        #server_module
        #proto
    }
}

fn handle_generic_rpc(
    input: ItemTrait,
    mut config: UnifiedProtoConfig,
    methods: Vec<crate::utils::MethodInfo>,
    user_associated_types: Vec<TokenStream2>,
) -> TokenStream2 {
    use crate::utils::MethodInfo;
    use generic_helpers::extract_generic_types_from_methods;

    let trait_name = &input.ident;
    let ty_ident = trait_name.to_string();
    let vis = &input.vis;
    let package_name = config.get_rpc_package().to_owned();
    let instantiations = config.compute_generic_instantiations();

    // Extract all generic types used in the service
    let generic_type_names = extract_generic_types_from_methods(&methods);

    // Generate TYPE_ID implementations for all generic types used in the service
    let mut type_id_impls = Vec::new();
    for type_name in &generic_type_names {
        // Create a minimal Generics for the type based on the instantiations
        let generics: syn::Generics = {
            if let Some(first_inst) = instantiations.first() {
                let params: Vec<_> = first_inst
                    .substitutions
                    .keys()
                    .map(|k| {
                        let ident = quote::format_ident!("{}", k);
                        syn::GenericParam::Type(syn::TypeParam {
                            attrs: vec![],
                            ident,
                            colon_token: None,
                            bounds: Default::default(),
                            eq_token: None,
                            default: None,
                        })
                    })
                    .collect();

                syn::Generics {
                    lt_token: Some(Default::default()),
                    params: params.into_iter().collect(),
                    gt_token: Some(Default::default()),
                    where_clause: None,
                }
            } else {
                Default::default()
            }
        };

        let type_id_impl = generate_type_id_impls(type_name, &generics, &instantiations);
        type_id_impls.push(type_id_impl);
    }

    // Generate proto services for all instantiations
    for instantiation in &instantiations {
        let service_name = format!("{}{}", ty_ident, instantiation.name_suffix);

        // Substitute generic types in methods
        let concrete_methods: Vec<MethodInfo> = methods
            .iter()
            .map(|method| {
                let method_name = method.name.clone();
                let concrete_request = substitute_generic_types(&method.request_type, &instantiation.substitutions);
                let concrete_response = substitute_generic_types(&method.response_type, &instantiation.substitutions);
                let concrete_return = substitute_generic_types(&method.response_return_type, &instantiation.substitutions);
                let concrete_inner_response = method
                    .inner_response_type
                    .as_ref()
                    .map(|ty| substitute_generic_types(ty, &instantiation.substitutions));
                let concrete_stream_item = method
                    .stream_item_type
                    .as_ref()
                    .map(|ty| substitute_generic_types(ty, &instantiation.substitutions));

                MethodInfo {
                    name: method_name,
                    request_type: concrete_request,
                    response_type: concrete_response,
                    response_return_type: concrete_return,
                    response_is_result: method.response_is_result,
                    is_streaming: method.is_streaming,
                    stream_type_name: method.stream_type_name.clone(),
                    inner_response_type: concrete_inner_response,
                    stream_item_type: concrete_stream_item,
                    user_method_signature: method.user_method_signature.clone(),
                }
            })
            .collect();

        // Generate proto service content
        let service_content = generate_service_content(&quote::format_ident!("{}", service_name), &concrete_methods, &config.type_imports);
        config.register_and_emit_proto(&service_name, &service_content);
    }

    let proto = config.imports_mat.clone();

    // Keep the user-facing trait generic
    let user_methods: Vec<_> = methods.iter().map(|m| &m.user_method_signature).collect();

    // Generate generic-aware client and server implementations
    let client_module = if config.rpc_client {
        generate_generic_client_module(trait_name, vis, &package_name, &methods, &instantiations)
    } else {
        quote! {}
    };

    let server_module = if config.rpc_server {
        generate_generic_server_module(trait_name, vis, &package_name, &methods, &instantiations)
    } else {
        quote! {}
    };

    // Combine all TYPE_ID implementations
    let all_type_ids = quote! {
        #(#type_id_impls)*
    };

    quote! {
        #vis trait #trait_name {
            #(#user_associated_types)*
            #(#user_methods)*
        }

        #all_type_ids
        #client_module
        #server_module
        #proto
    }
}

fn generate_generic_client_module(
    trait_name: &syn::Ident,
    vis: &syn::Visibility,
    package_name: &str,
    methods: &[crate::utils::MethodInfo],
    instantiations: &[crate::parse::GenericTypeInstantiation],
) -> TokenStream2 {
    let client_mod_name = quote::format_ident!("{}_client", crate::utils::to_snake_case(&trait_name.to_string()));
    let client_name = quote::format_ident!("{}Client", trait_name);

    // Extract generic parameter names from instantiations
    let generic_param_names: Vec<syn::Ident> = if let Some(first_inst) = instantiations.first() {
        first_inst.substitutions.keys()
            .map(|k| quote::format_ident!("{}", k))
            .collect()
    } else {
        vec![]
    };

    // Generate dispatch methods for each RPC method using const matching
    let dispatch_methods: Vec<_> = methods
        .iter()
        .map(|method| {
            let method_name = &method.name;
            let method_name_snake = quote::format_ident!("{}", crate::utils::to_snake_case(&method_name.to_string()));
            let request_type = &method.request_type;
            let response_type = &method.response_type;

            // Get the request type name for the TypeId enum
            let request_type_name = match request_type {
                syn::Type::Path(type_path) => {
                    if let Some(segment) = type_path.path.segments.last() {
                        &segment.ident
                    } else {
                        return quote! {};
                    }
                }
                _ => return quote! {},
            };

            let type_id_enum = quote::format_ident!("{}TypeId", request_type_name);

            // Generate match arms using TYPE_ID const (now an enum)
            let match_arms: Vec<_> = instantiations
                .iter()
                .map(|inst| {
                    let type_id_variant = quote::format_ident!("{}", inst.name_suffix);
                    let service_name = format!("{}{}", trait_name, inst.name_suffix);
                    let method_path = format!("/{}.{}/{}", package_name, service_name, method_name);

                    quote! {
                        #type_id_enum::#type_id_variant => {
                            let path = http::uri::PathAndQuery::from_static(#method_path);
                            self.inner.unary(request, path, tonic::codec::ProstCodec::default()).await
                        }
                    }
                })
                .collect();

            quote! {
                pub async fn #method_name_snake<#(#generic_param_names),*>(
                    &mut self,
                    request: tonic::Request<#request_type>,
                ) -> Result<tonic::Response<#response_type>, tonic::Status>
                {
                    // Dispatch based on the TYPE_ID of the request type (enum matching)
                    match <#request_type>::TYPE_ID {
                        #(#match_arms)*
                    }
                }
            }
        })
        .collect();

    quote! {
        #vis mod #client_mod_name {
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
            pub struct #client_name<T> {
                inner: tonic::client::Grpc<T>,
            }

            impl #client_name<tonic::transport::Channel> {
                pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
                where
                    D: TryInto<tonic::transport::Endpoint>,
                    D::Error: Into<StdError>,
                {
                    let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
                    Ok(Self::new(conn))
                }
            }

            impl<T> #client_name<T>
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

                pub fn with_interceptor<F>(
                    inner: T,
                    interceptor: F,
                ) -> #client_name<InterceptedService<T, F>>
                where
                    F: tonic::service::Interceptor,
                    T::ResponseBody: Default,
                    T: tonic::codegen::Service<
                        http::Request<tonic::body::Body>,
                        Response = http::Response<
                            <T as tonic::client::GrpcService<tonic::body::Body>>::ResponseBody,
                        >,
                    >,
                    <T as tonic::codegen::Service<http::Request<tonic::body::Body>>>::Error: Into<StdError> + Send + Sync,
                {
                    #client_name::new(InterceptedService::new(inner, interceptor))
                }

                #(#dispatch_methods)*
            }
        }
    }
}

fn generate_generic_server_module(
    trait_name: &syn::Ident,
    vis: &syn::Visibility,
    package_name: &str,
    methods: &[crate::utils::MethodInfo],
    instantiations: &[crate::parse::GenericTypeInstantiation],
) -> TokenStream2 {
    let server_mod_name = quote::format_ident!("{}_server", crate::utils::to_snake_case(&trait_name.to_string()));
    let server_struct_name = quote::format_ident!("{}Server", trait_name);

    // Generate trait methods with generic parameters
    let trait_methods: Vec<_> = methods
        .iter()
        .map(|method| {
            &method.user_method_signature
        })
        .collect();

    // Generate route handlers for each method/instantiation combination
    let route_handlers: Vec<_> = methods
        .iter()
        .flat_map(|method| {
            let method_name = &method.name;
            let svc_suffix = crate::utils::to_pascal_case(&method_name.to_string());

            instantiations.iter().map(move |inst| {
                let type_id = &inst.name_suffix;
                let service_name = format!("{}{}", trait_name, type_id);
                let route_path = format!("/{}.{}/{}", package_name, service_name, method_name);
                let svc_name = quote::format_ident!("{}{}Svc", svc_suffix, type_id);

                // Substitute generic types to get concrete request/response types
                let concrete_request = crate::parse::substitute_generic_types(&method.request_type, &inst.substitutions);
                let concrete_response = crate::parse::substitute_generic_types(&method.response_type, &inst.substitutions);

                // Generate the route handler
                quote! {
                    #route_path => {
                        #[allow(non_camel_case_types)]
                        struct #svc_name<T: #trait_name>(pub ::std::sync::Arc<T>);

                        impl<T: #trait_name> tonic::server::UnaryService<#concrete_request> for #svc_name<T> {
                            type Response = #concrete_response;
                            type Future = ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<tonic::Response<Self::Response>, tonic::Status>> + Send + 'static>>;

                            fn call(&mut self, request: tonic::Request<#concrete_request>) -> Self::Future {
                                let inner = ::std::sync::Arc::clone(&self.0);
                                Box::pin(async move {
                                    <T as #trait_name>::#method_name(&*inner, request).await
                                })
                            }
                        }

                        let method = #svc_name(inner);
                        let codec = tonic::codec::ProstCodec::default();
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
            })
        })
        .collect();

    let service_name_value = format!("{}.{}", package_name, trait_name);

    quote! {
        #vis mod #server_mod_name {
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
                #(#trait_methods)*
            }

            impl<T> #trait_name for T
            where
                T: super::#trait_name + ::core::marker::Send + ::core::marker::Sync + 'static,
            {
                #(#trait_methods)*
            }

            #[derive(Debug)]
            pub struct #server_struct_name<T> {
                inner: ::proto_rs::alloc::sync::Arc<T>,
                accept_compression_encodings: EnabledCompressionEncodings,
                send_compression_encodings: EnabledCompressionEncodings,
                max_decoding_message_size: Option<usize>,
                max_encoding_message_size: Option<usize>,
            }

            impl<T> #server_struct_name<T> {
                pub fn new(inner: T) -> Self {
                    Self::from_arc(::proto_rs::alloc::sync::Arc::new(inner))
                }

                pub fn from_arc(inner: ::proto_rs::alloc::sync::Arc<T>) -> Self {
                    Self {
                        inner,
                        accept_compression_encodings: Default::default(),
                        send_compression_encodings: Default::default(),
                        max_decoding_message_size: None,
                        max_encoding_message_size: None,
                    }
                }

                pub fn with_interceptor<F>(
                    inner: T,
                    interceptor: F,
                ) -> InterceptedService<Self, F>
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

            impl<T, B> tonic::codegen::Service<http::Request<B>> for #server_struct_name<T>
            where
                T: #trait_name,
                B: Body + ::core::marker::Send + 'static,
                B::Error: Into<StdError> + ::core::marker::Send + 'static,
            {
                type Response = http::Response<tonic::body::Body>;
                type Error = ::core::convert::Infallible;
                type Future = ::std::pin::Pin<Box<dyn ::std::future::Future<Output = ::core::result::Result<Self::Response, Self::Error>> + Send + 'static>>;

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

                    Box::pin(async move {
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
                    })
                }
            }

            impl<T> Clone for #server_struct_name<T> {
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

            impl<T> tonic::server::NamedService for #server_struct_name<T> {
                const NAME: &'static str = SERVICE_NAME;
            }
        }
    }
}
