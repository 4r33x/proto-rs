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
    let _package_name = config.get_rpc_package().to_owned();
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

    // TODO: Generate client and server modules that handle generic type dispatching
    // For now, we just generate the trait and proto definitions
    // Users can implement the trait manually for each concrete type combination

    quote! {
        #vis trait #trait_name {
            #(#user_associated_types)*
            #(#user_methods)*
        }

        #proto

        // TODO: Add generic-aware client and server implementations
        // This would require runtime type dispatching or compile-time specialization
    }
}
