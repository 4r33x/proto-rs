//! Centralized utilities for proto macro code generation

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Field;
use syn::GenericArgument;
use syn::Lit;
use syn::PathArguments;
use syn::Type;
use syn::TypePath;

// Re-export modular utilities
pub mod array_handling;
pub mod enum_handling;
pub mod field_handling;
pub mod string_helpers;
pub mod type_conversion;
pub mod type_info;

pub use string_helpers::*;
pub use type_conversion::{get_proto_rust_type, needs_into_conversion, needs_try_into_conversion};
pub use type_info::{ParsedFieldType, is_bytes_array, is_bytes_vec, parse_field_type};

// ============================================================================
// FIELD CONFIGURATION
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct FieldConfig {
    pub into_type: Option<String>,
    pub from_type: Option<String>,
    pub into_fn: Option<String>,
    pub from_fn: Option<String>,
    pub skip: bool,
    pub skip_deser_fn: Option<String>, // run after full decode
    pub is_rust_enum: bool,            // treat T as Rust enum -> i32 on wire
    pub is_message: bool,              // force message semantics
    pub is_proto_enum: bool,           // prost-like enum (i32 backing)
    pub import_path: Option<String>,
    pub custom_tag: Option<usize>,
}

pub fn parse_field_config(field: &Field) -> FieldConfig {
    let mut cfg = FieldConfig::default();

    for attr in &field.attrs {
        if !attr.path().is_ident("proto") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            let key = meta.path.get_ident().map(|i| i.to_string());

            match key.as_deref() {
                Some("skip") => {
                    cfg.skip = true;
                    // allow #[proto(skip = "fn_name")]
                    if meta.input.peek(syn::Token![=]) {
                        if let Some(fun) = parse_string_value(&meta) {
                            cfg.skip_deser_fn = Some(fun);
                        }
                    }
                }
                Some("rust_enum") => cfg.is_rust_enum = true,
                Some("enum") => cfg.is_proto_enum = true,
                Some("message") => cfg.is_message = true,
                Some("into") => cfg.into_type = parse_string_value(&meta),
                Some("from") => cfg.from_type = parse_string_value(&meta),
                Some("into_fn") => cfg.into_fn = parse_string_value(&meta),
                Some("from_fn") => cfg.from_fn = parse_string_value(&meta),
                Some("import_path") => cfg.import_path = parse_string_value(&meta),
                Some("tag") => cfg.custom_tag = parse_usize_value(&meta),
                _ => {}
            }
            Ok(())
        });
    }

    cfg
}

fn parse_string_value(meta: &syn::meta::ParseNestedMeta) -> Option<String> {
    meta.value()
        .ok()
        .and_then(|v| v.parse::<Lit>().ok())
        .and_then(|lit| if let syn::Lit::Str(s) = lit { Some(s.value()) } else { None })
}

fn parse_usize_value(meta: &syn::meta::ParseNestedMeta) -> Option<usize> {
    meta.value().ok().and_then(|v| v.parse::<Lit>().ok()).and_then(|lit| match lit {
        syn::Lit::Int(i) => i.base10_parse::<usize>().ok(),
        syn::Lit::Str(s) => s.value().parse::<usize>().ok(),
        _ => None,
    })
}

// ============================================================================
// TYPE HELPERS
// ============================================================================

fn last_path_segment(ty: &Type) -> Option<&syn::PathSegment> {
    match ty {
        Type::Path(path) => path.path.segments.last(),
        _ => None,
    }
}

fn clone_last_ident(path: &TypePath) -> syn::Ident {
    path.path.segments.last().map(|seg| seg.ident.clone()).unwrap_or_else(|| syn::Ident::new("_", Span::call_site()))
}

pub fn rust_type_path_ident(ty: &Type) -> syn::Ident {
    match ty {
        Type::Path(path) => clone_last_ident(path),
        Type::Array(arr) => rust_type_path_ident(&arr.elem),
        Type::Reference(r) => rust_type_path_ident(&r.elem),
        _ => syn::Ident::new("_", Span::call_site()),
    }
}

pub fn is_option_type(ty: &Type) -> bool {
    matches!(last_path_segment(ty), Some(seg) if seg.ident == "Option")
}

pub fn extract_option_inner_type(ty: &Type) -> Option<Type> {
    if let Type::Path(path) = ty {
        if let Some(seg) = path.path.segments.last() {
            if seg.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner.clone());
                    }
                }
            }
        }
    }
    None
}

pub fn vec_inner_type(ty: &Type) -> Option<Type> {
    if let Type::Path(path) = ty {
        if let Some(seg) = path.path.segments.last() {
            if seg.ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner.clone());
                    }
                }
            }
        }
    }
    None
}

pub fn is_complex_type(ty: &Type) -> bool {
    parse_field_type(ty).is_message_like
}
// ============================================================================
// ERROR HELPERS
// ============================================================================

/// Generate error conversion code
pub fn generate_field_error(field_name: &syn::Ident, error_name: &syn::Ident) -> TokenStream {
    quote! {
        .map_err(|e| #error_name::FieldConversion {
            field: stringify!(#field_name).to_string(),
            source: Box::new(e),
        })?
    }
}

/// Generate missing field error
pub fn generate_missing_field_error(field_name: &syn::Ident, error_name: &syn::Ident) -> TokenStream {
    quote! {
        .ok_or_else(|| #error_name::MissingField {
            field: stringify!(#field_name).to_string()
        })?
    }
}

// ============================================================================
// METHOD INFO
// ============================================================================

pub struct MethodInfo {
    pub name: syn::Ident,
    pub _attrs: Vec<syn::Attribute>,
    pub request_type: Box<Type>,
    pub response_type: Box<Type>,
    pub is_streaming: bool,
    pub stream_type_name: Option<syn::Ident>,
    pub inner_response_type: Option<Type>,
    pub user_method_signature: TokenStream,
}
