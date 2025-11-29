use std::collections::{HashMap, HashSet};

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Fields, Type};

use crate::parse::substitute_generic_types;
use crate::utils::MethodInfo;

/// Substitute generic types in struct/enum fields
pub fn substitute_fields(fields: &Fields, substitutions: &HashMap<String, syn::Type>) -> Fields {
    match fields {
        Fields::Named(named) => {
            let mut new_fields = named.clone();
            for field in new_fields.named.iter_mut() {
                field.ty = substitute_generic_types(&field.ty, substitutions);
            }
            Fields::Named(new_fields)
        }
        Fields::Unnamed(unnamed) => {
            let mut new_fields = unnamed.clone();
            for field in new_fields.unnamed.iter_mut() {
                field.ty = substitute_generic_types(&field.ty, substitutions);
            }
            Fields::Unnamed(new_fields)
        }
        Fields::Unit => Fields::Unit,
    }
}

/// Substitute generic types in enum variants
pub fn substitute_enum_variants(
    data: &syn::DataEnum,
    substitutions: &HashMap<String, syn::Type>,
) -> syn::DataEnum {
    let mut new_data = data.clone();
    for variant in new_data.variants.iter_mut() {
        variant.fields = substitute_fields(&variant.fields, substitutions);
    }
    new_data
}

/// Generate TYPE_ID enum and associated const implementations for generic types
pub fn generate_type_id_impls(
    type_name: &syn::Ident,
    generics: &syn::Generics,
    instantiations: &[crate::parse::GenericTypeInstantiation],
) -> TokenStream2 {
    if instantiations.is_empty() {
        return quote! {};
    }

    // Generate enum for TYPE_ID
    let enum_name = quote::format_ident!("{}TypeId", type_name);
    let enum_variants: Vec<_> = instantiations
        .iter()
        .map(|inst| {
            let variant_name = quote::format_ident!("{}", inst.name_suffix);
            quote! { #variant_name }
        })
        .collect();

    let enum_def = quote! {
        /// Type identifier enum for generic instantiations
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #enum_name {
            #(#enum_variants),*
        }
    };

    // Generate impl blocks for each instantiation
    let impls: Vec<_> = instantiations
        .iter()
        .map(|inst| {
            let variant_name = quote::format_ident!("{}", inst.name_suffix);
            let proto_type_name = format!("{}{}", type_name, inst.name_suffix);

            // Build concrete type arguments
            let concrete_args: Vec<_> = generics
                .params
                .iter()
                .filter_map(|param| {
                    if let syn::GenericParam::Type(type_param) = param {
                        let param_name = type_param.ident.to_string();
                        inst.substitutions.get(&param_name).cloned()
                    } else {
                        None
                    }
                })
                .collect();

            if concrete_args.is_empty() {
                return quote! {};
            }

            quote! {
                impl #type_name<#(#concrete_args),*> {
                    /// Type identifier for this generic instantiation
                    pub const TYPE_ID: #enum_name = #enum_name::#variant_name;

                    /// Proto message name for this generic instantiation
                    pub const PROTO_TYPE_NAME: &'static str = #proto_type_name;
                }
            }
        })
        .collect();

    quote! {
        #enum_def
        #(#impls)*
    }
}

/// Extract the base type name from a Type (without generic parameters)
fn extract_type_ident(ty: &Type) -> Option<syn::Ident> {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                Some(segment.ident.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Check if a type has generic parameters
fn has_generic_params(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                matches!(segment.arguments, syn::PathArguments::AngleBracketed(_))
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Extract unique generic types used in RPC methods
pub fn extract_generic_types_from_methods(methods: &[MethodInfo]) -> HashSet<syn::Ident> {
    let mut generic_types = HashSet::new();

    for method in methods {
        // Check request type
        if has_generic_params(&method.request_type) {
            if let Some(ident) = extract_type_ident(&method.request_type) {
                generic_types.insert(ident);
            }
        }

        // Check response type
        if has_generic_params(&method.response_type) {
            if let Some(ident) = extract_type_ident(&method.response_type) {
                generic_types.insert(ident);
            }
        }

        // Check stream item type if present
        if let Some(stream_ty) = &method.stream_item_type {
            if has_generic_params(stream_ty) {
                if let Some(ident) = extract_type_ident(stream_ty) {
                    generic_types.insert(ident);
                }
            }
        }
    }

    generic_types
}
