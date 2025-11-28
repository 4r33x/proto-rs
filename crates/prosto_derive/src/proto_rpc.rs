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

    // Collect all unique generic types used in methods
    let mut generic_message_types = std::collections::HashSet::new();
    for method in &methods {
        extract_generic_types(&method.request_type, &mut generic_message_types);
        extract_generic_types(&method.response_type, &mut generic_message_types);
    }

    // Generate enum wrappers for each generic message type
    let enum_wrappers = generate_enum_wrappers(&generic_message_types, &instantiations);

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

        #enum_wrappers
        #client_module
        #server_module
        #proto
    }
}

fn extract_generic_types(ty: &syn::Type, types: &mut std::collections::HashSet<String>) {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            // Check if this type has generic parameters
            if let syn::PathArguments::AngleBracketed(_) = &segment.arguments {
                types.insert(segment.ident.to_string());
            }
        }
    }
}

fn generate_enum_wrappers(
    message_types: &std::collections::HashSet<String>,
    instantiations: &[crate::parse::GenericTypeInstantiation],
) -> TokenStream2 {
    let mut wrappers = Vec::new();

    for msg_type in message_types {
        let enum_name = quote::format_ident!("{}Variant", msg_type);

        let variants: Vec<_> = instantiations
            .iter()
            .map(|inst| {
                let variant_name = quote::format_ident!("{}", inst.name_suffix);
                let concrete_ty_name = quote::format_ident!("{}{}", msg_type, inst.name_suffix);

                quote! {
                    #variant_name(#concrete_ty_name)
                }
            })
            .collect();

        wrappers.push(quote! {
            #[derive(Clone)]
            pub enum #enum_name {
                #(#variants),*
            }
        });
    }

    quote! { #(#wrappers)* }
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

    // Generate dispatch methods for each RPC method
    let dispatch_methods: Vec<_> = methods
        .iter()
        .map(|method| {
            let method_name = &method.name;
            let method_name_snake = crate::utils::to_snake_case(&method_name.to_string());

            // Generate match arms for each instantiation
            let match_arms: Vec<_> = instantiations
                .iter()
                .map(|inst| {
                    let variant_name = quote::format_ident!("{}", inst.name_suffix);
                    let service_path = format!("/{}{}/{}", package_name, trait_name, inst.name_suffix);
                    let method_path = format!("{}/{}", service_path, method_name);

                    quote! {
                        RequestVariant::#variant_name(req) => {
                            let path = tonic::codec::CompressionEncoding::new(#method_path);
                            // Call the concrete proto method
                            // This is a simplified version - would need full tonic client implementation
                            unimplemented!("Generic RPC dispatch for {}", #method_path)
                        }
                    }
                })
                .collect();

            quote! {
                pub async fn #method_name_snake<T>(&mut self, request: RequestVariant) -> Result<ResponseVariant, tonic::Status> {
                    match request {
                        #(#match_arms)*
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
    let server_trait_name = quote::format_ident!("{}Server", trait_name);

    // Generate route matching for each method/instantiation combination
    let route_matches: Vec<_> = methods
        .iter()
        .flat_map(|method| {
            let method_name = &method.name;

            instantiations.iter().map(move |inst| {
                let variant_suffix = &inst.name_suffix;
                let route_path = format!("/{}{}/{}{}", package_name, trait_name, variant_suffix, method_name);

                quote! {
                    #route_path => {
                        // Dispatch to concrete implementation
                        // This would call the trait implementation with concrete types
                        unimplemented!("Server dispatch for {}", #route_path)
                    }
                }
            })
        })
        .collect();

    quote! {
        #vis mod #server_mod_name {
            use super::*;

            pub trait #server_trait_name: Send + Sync + 'static {
                // Generated methods would go here
            }

            pub fn serve<T: #server_trait_name>(service: T) -> impl tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<tonic::body::BoxBody>,
                Error = std::convert::Infallible,
            > {
                // Route dispatcher
                let router = move |req: http::Request<tonic::body::BoxBody>| {
                    match req.uri().path() {
                        #(#route_matches,)*
                        _ => {
                            // Return 404 for unknown routes
                            unimplemented!("404 handler")
                        }
                    }
                };

                // Return tonic service
                unimplemented!("Server implementation")
            }
        }
    }
}
