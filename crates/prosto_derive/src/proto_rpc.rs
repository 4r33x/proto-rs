use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::ItemTrait;

mod client;
pub mod rpc_common;
mod server;
pub mod utils; // Add this

use client::generate_client_module;
use server::generate_server_module;
use utils::extract_methods_and_types; // Add this import

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

    let trait_name = &input.ident;
    let ty_ident = trait_name.to_string();
    let vis = &input.vis;
    let package_name = config.get_rpc_package().to_owned();
    let instantiations = config.compute_generic_instantiations();

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

            // Generate match arms using TYPE_ID const
            let match_arms: Vec<_> = instantiations
                .iter()
                .map(|inst| {
                    let type_id = &inst.name_suffix;
                    let service_name = format!("{}{}", trait_name, type_id);
                    let method_path = format!("/{}.{}/{}", package_name, service_name, method_name);

                    quote! {
                        #type_id => {
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
                    // Dispatch based on the TYPE_ID of the request type
                    match <#request_type>::TYPE_ID {
                        #(#match_arms)*
                        _ => Err(tonic::Status::unimplemented("Unsupported type combination")),
                    }
                }
            }
        })
        .collect();

    quote! {
        #vis mod #client_mod_name {
            use super::*;

            pub struct #client_name<T> {
                inner: tonic::client::Grpc<T>,
            }

            impl<T> #client_name<T>
            where
                T: tonic::client::GrpcService<tonic::body::BoxBody>,
                T::Error: Into<tonic::codegen::StdError>,
                T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
                <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
            {
                pub fn new(inner: T) -> Self {
                    let inner = tonic::client::Grpc::new(inner);
                    Self { inner }
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
    let _server_trait_name = trait_name;

    // Extract generic parameter names
    let generic_param_names: Vec<syn::Ident> = if let Some(first_inst) = instantiations.first() {
        first_inst.substitutions.keys()
            .map(|k| quote::format_ident!("{}", k))
            .collect()
    } else {
        vec![]
    };

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
            let _method_name_snake = quote::format_ident!("{}", crate::utils::to_snake_case(&method_name.to_string()));

            instantiations.iter().map(move |inst| {
                let type_id = &inst.name_suffix;
                let service_name = format!("{}{}", trait_name, type_id);
                let route_path = format!("/{}.{}/{}", package_name, service_name, method_name);

                // Substitute generic types to get concrete request/response types
                let concrete_request = crate::parse::substitute_generic_types(&method.request_type, &inst.substitutions);
                let concrete_response = crate::parse::substitute_generic_types(&method.response_type, &inst.substitutions);

                quote! {
                    #route_path => {
                        // This is a placeholder for the full server implementation
                        // In a complete implementation, this would:
                        // 1. Deserialize request as concrete type #concrete_request
                        // 2. Call service.#method_name_snake(request)
                        // 3. Serialize response as concrete type #concrete_response
                        unimplemented!("Server handler for {} with types {} -> {}",
                            #route_path,
                            stringify!(#concrete_request),
                            stringify!(#concrete_response)
                        )
                    }
                }
            })
        })
        .collect();

    let _route_handlers = route_handlers; // Suppress unused variable warning
    let _trait_methods = trait_methods;
    let _generic_param_names = generic_param_names;

    quote! {
        #vis mod #server_mod_name {
            use super::*;

            // NOTE: Server implementation for generic RPCs is currently a work in progress.
            // The trait below defines the interface that server implementations must follow.
            // Each method is generic over the type parameters, and the server will dispatch
            // to the correct handler based on the incoming request path and TYPE_ID.
            //
            // Expected routes for each instantiation will be generated based on the
            // proto_generic_types attribute, creating separate service endpoints for
            // each type combination.
        }
    }
}
