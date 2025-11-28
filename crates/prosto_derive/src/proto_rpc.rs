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
    let (mut methods, user_associated_types) = extract_methods_and_types(&input);

    // Expand methods with generic types if proto_generic_types is specified
    if !config.proto_generic_types.is_empty() {
        methods = expand_generic_methods(&methods, &config);
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

    // Regenerate the user method signature with the new types
    let new_user_method_signature = if method.is_streaming {
        let stream_name = method.stream_type_name.as_ref().expect("streaming method must have stream name");
        if method.response_is_result {
            quote! {
                fn #new_name(
                    &self,
                    request: tonic::Request<#new_request_type>,
                ) -> impl ::core::future::Future<Output = ::core::result::Result<tonic::Response<Self::#stream_name>, tonic::Status>> + ::core::marker::Send + '_
                where
                    Self: ::core::marker::Send + ::core::marker::Sync;
            }
        } else {
            quote! {
                fn #new_name(
                    &self,
                    request: tonic::Request<#new_request_type>,
                ) -> impl ::core::future::Future<Output = tonic::Response<Self::#stream_name>> + ::core::marker::Send + '_
                where
                    Self: ::core::marker::Send + ::core::marker::Sync;
            }
        }
    } else if method.response_is_result {
        quote! {
            fn #new_name(
                &self,
                request: tonic::Request<#new_request_type>,
            ) -> impl ::core::future::Future<Output = ::core::result::Result<#new_response_return_type, tonic::Status>> + ::core::marker::Send + '_
            where
                Self: ::core::marker::Send + ::core::marker::Sync;
        }
    } else {
        quote! {
            fn #new_name(
                &self,
                request: tonic::Request<#new_request_type>,
            ) -> impl ::core::future::Future<Output = #new_response_return_type> + ::core::marker::Send + '_
            where
                Self: ::core::marker::Send + ::core::marker::Sync;
        }
    };

    let new_inner_response_type = method.inner_response_type.as_ref().map(|ty| substitute_type(ty, substitutions));
    let new_stream_item_type = method.stream_item_type.as_ref().map(|ty| substitute_type(ty, substitutions));

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
