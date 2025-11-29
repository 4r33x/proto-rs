use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::ItemTrait;
use syn::Type;
use std::collections::HashMap;

mod client;
pub mod rpc_common;
mod server;
pub mod utils; // Add this

use client::generate_client_module;
use server::generate_server_module;
use utils::extract_methods_and_types; // Add this import

use crate::emit_proto::generate_service_content;
use crate::parse::UnifiedProtoConfig;
use crate::utils::MethodInfo;

pub fn proto_rpc_impl(args: TokenStream, item: TokenStream) -> TokenStream2 {
    let input: ItemTrait = syn::parse(item).expect("Failed to parse trait");
    let trait_name = &input.ident;
    let ty_ident = trait_name.to_string();
    let mut config = UnifiedProtoConfig::from_attributes(args, &ty_ident, &input.attrs, &input);
    let vis = &input.vis;
    let package_name = config.get_rpc_package().to_owned();

    // Extract methods, types, and imports
    let (methods, user_associated_types) = extract_methods_and_types(&input);

    // For generic types, we keep the trait methods generic but expand in server/client
    let expanded_methods = if !config.proto_generic_types.is_empty() {
        expand_generic_methods(&methods, &config)
    } else {
        methods.clone()
    };

    // Generate .proto file if requested (use expanded methods for proto)
    let service_content = generate_service_content(trait_name, &expanded_methods, &config.type_imports);
    config.register_and_emit_proto(&ty_ident, &service_content);
    let proto = config.imports_mat.clone();

    // Generate user-facing trait
    let user_trait_def = if !config.proto_generic_types.is_empty() {
        // For generic traits, add type parameter(s) to the trait itself
        let generic_param_names: Vec<_> = config.proto_generic_types.keys().map(|k| {
            syn::Ident::new(k, proc_macro2::Span::call_site())
        }).collect();

        let user_methods: Vec<_> = methods.iter().map(|m| &m.user_method_signature).collect();

        quote! {
            #vis trait #trait_name<#(#generic_param_names),*> {
                #(#user_associated_types)*
                #(#user_methods)*
            }
        }
    } else {
        let user_methods: Vec<_> = methods.iter().map(|m| &m.user_method_signature).collect();
        quote! {
            #vis trait #trait_name {
                #(#user_associated_types)*
                #(#user_methods)*
            }
        }
    };

    // Generate client module if requested (use expanded methods)
    let client_module = if config.rpc_client {
        generate_client_module(trait_name, vis, &package_name, &expanded_methods)
    } else {
        quote! {}
    };

    // Generate server module if requested (use expanded methods, include info about generics)
    let server_module = if config.rpc_server {
        if !config.proto_generic_types.is_empty() {
            // Pass mapping from expanded methods to their generic types
            let combinations = config.generate_generic_combinations();
            generate_server_module_generic(trait_name, vis, &package_name, &methods, &expanded_methods, &combinations)
        } else {
            generate_server_module(trait_name, vis, &package_name, &expanded_methods)
        }
    } else {
        quote! {}
    };

    quote! {
        #user_trait_def

        #client_module
        #server_module
        #proto
    }
}

/// Expand methods that use generic types into concrete implementations
fn expand_generic_methods(methods: &[MethodInfo], config: &UnifiedProtoConfig) -> Vec<MethodInfo> {
    let combinations = config.generate_generic_combinations();
    if combinations.is_empty() {
        return methods.to_vec();
    }

    let mut expanded_methods = Vec::new();

    for method in methods {
        // Check if this method uses any generic types
        if method_uses_generics(method, &config.proto_generic_types) {
            // Generate a concrete method for each combination
            for (substitutions, suffix) in &combinations {
                let concrete_method = substitute_generics(method, substitutions, suffix);
                expanded_methods.push(concrete_method);
            }
        } else {
            // Keep non-generic methods as-is
            expanded_methods.push(method.clone());
        }
    }

    expanded_methods
}

/// Check if a method uses any of the generic type parameters
fn method_uses_generics(method: &MethodInfo, generic_types: &HashMap<String, Vec<Type>>) -> bool {
    let request_type = &method.request_type;
    let response_type = &method.response_type;
    let request_str = quote::quote!(#request_type).to_string();
    let response_str = quote::quote!(#response_type).to_string();

    for generic_name in generic_types.keys() {
        if request_str.contains(generic_name) || response_str.contains(generic_name) {
            return true;
        }
    }

    false
}

/// Substitute generic parameters with concrete types in a method
fn substitute_generics(
    method: &MethodInfo,
    substitutions: &[(String, Type)],
    suffix: &str,
) -> MethodInfo {
    let new_name = syn::Ident::new(
        &format!("{}_{}", method.name, suffix.to_lowercase()),
        method.name.span(),
    );

    let new_request_type = substitute_type(&method.request_type, substitutions);
    let new_response_type = substitute_type(&method.response_type, substitutions);
    let new_response_return_type = substitute_type(&method.response_return_type, substitutions);
    let new_inner_response_type = method.inner_response_type.as_ref().map(|ty| substitute_type(ty, substitutions));
    let new_stream_item_type = method.stream_item_type.as_ref().map(|ty| substitute_type(ty, substitutions));

    // Reuse the method signature generation logic
    let new_user_method_signature = generate_method_signature(
        &new_name,
        &new_request_type,
        &new_response_return_type,
        method.response_is_result,
        method.is_streaming,
        method.stream_type_name.as_ref(),
    );

    MethodInfo {
        name: new_name,
        request_type: new_request_type,
        response_type: new_response_type,
        response_return_type: new_response_return_type,
        response_is_result: method.response_is_result,
        is_streaming: method.is_streaming,
        stream_type_name: method.stream_type_name.clone(),
        inner_response_type: new_inner_response_type,
        stream_item_type: new_stream_item_type,
        user_method_signature: new_user_method_signature,
    }
}

/// Generate a method signature (reused for both generic and non-generic methods)
fn generate_method_signature(
    method_name: &syn::Ident,
    request_type: &Type,
    response_return_type: &Type,
    response_is_result: bool,
    is_streaming: bool,
    stream_type_name: Option<&syn::Ident>,
) -> TokenStream2 {
    use utils::method_future_return_type;

    let future_output = if is_streaming {
        let stream_name = stream_type_name.expect("streaming method must define stream name");
        if response_is_result {
            quote! { ::core::result::Result<tonic::Response<Self::#stream_name>, tonic::Status> }
        } else {
            quote! { tonic::Response<Self::#stream_name> }
        }
    } else if response_is_result {
        quote! { ::core::result::Result<#response_return_type, tonic::Status> }
    } else {
        quote! { #response_return_type }
    };

    let future_type = method_future_return_type(future_output);

    quote! {
        fn #method_name(
            &self,
            request: tonic::Request<#request_type>,
        ) -> #future_type
        where
            Self: ::core::marker::Send + ::core::marker::Sync;
    }
}

/// Substitute generic parameters in a type with concrete types
fn substitute_type(ty: &Type, substitutions: &[(String, Type)]) -> Type {
    use syn::visit_mut::VisitMut;

    let mut ty = ty.clone();

    struct GenericSubstitutor<'a> {
        substitutions: &'a [(String, Type)],
    }

    impl<'a> VisitMut for GenericSubstitutor<'a> {
        fn visit_type_mut(&mut self, ty: &mut Type) {
            if let Type::Path(type_path) = ty {
                // Check if this is a single-segment path matching a generic parameter
                if type_path.qself.is_none() && type_path.path.segments.len() == 1 {
                    let segment = &type_path.path.segments[0];
                    let ident_str = segment.ident.to_string();

                    for (param_name, concrete_type) in self.substitutions {
                        if &ident_str == param_name {
                            *ty = concrete_type.clone();
                            return;
                        }
                    }
                }
            }

            // Continue visiting child nodes
            syn::visit_mut::visit_type_mut(self, ty);
        }
    }

    let mut substitutor = GenericSubstitutor { substitutions };
    substitutor.visit_type_mut(&mut ty);

    ty
}

/// Generate server module for generic traits
fn generate_server_module_generic(
    trait_name: &syn::Ident,
    vis: &syn::Visibility,
    package_name: &str,
    original_methods: &[MethodInfo],
    expanded_methods: &[MethodInfo],
    combinations: &[(Vec<(String, Type)>, String)],
) -> TokenStream2 {
    // For each expanded method, we need to know:
    // 1. Which original method it came from
    // 2. Which generic type substitution it uses

    // Map expanded methods back to their original + substitution
    let methods_per_original = expanded_methods.len() / original_methods.len();
    let method_mapping: Vec<_> = expanded_methods.iter().enumerate().map(|(idx, expanded)| {
        let original_idx = idx / methods_per_original;
        let combo_idx = idx % methods_per_original;
        (expanded, &original_methods[original_idx], &combinations[combo_idx])
    }).collect();

    // Generate server module with modified blanket impl
    server::generate_server_module_with_generic_mapping(
        trait_name,
        vis,
        package_name,
        expanded_methods,
        &method_mapping,
    )
}
