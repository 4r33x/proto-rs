//! Centralized utilities for proto macro code generation

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
pub mod type_info;

pub use string_helpers::*;
pub use type_info::*;

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
    pub skip_deser_fn: Option<String>,
    pub is_rust_enum: bool,
    pub import_path: Option<String>,
    pub is_message: bool,
    pub is_proto_enum: bool,
    pub tag: Option<usize>,
}

pub fn parse_field_config(field: &Field) -> FieldConfig {
    let mut config = FieldConfig::default();

    for attr in &field.attrs {
        if !attr.path().is_ident("proto") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            let ident = meta.path.get_ident().map(|i| i.to_string());

            match ident.as_deref() {
                Some("skip") => parse_skip_attribute(&meta, &mut config),
                Some("rust_enum") => config.is_rust_enum = true,
                Some("enum") => config.is_proto_enum = true,
                Some("message") => config.is_message = true,
                Some("tag") => config.tag = parse_tag_value(&meta),
                Some("into") => config.into_type = parse_string_value(&meta),
                Some("from") => config.from_type = parse_string_value(&meta),
                Some("into_fn") => config.into_fn = parse_string_value(&meta),
                Some("from_fn") => config.from_fn = parse_string_value(&meta),
                Some("import_path") => config.import_path = parse_string_value(&meta),
                _ => {}
            }
            Ok(())
        });
    }

    config
}

fn parse_skip_attribute(meta: &syn::meta::ParseNestedMeta, config: &mut FieldConfig) {
    config.skip = true;
    if meta.input.peek(syn::Token![=])
        && let Some(fn_name) = parse_string_value(meta)
    {
        config.skip_deser_fn = Some(fn_name);
    }
}

fn parse_tag_value(meta: &syn::meta::ParseNestedMeta) -> Option<usize> {
    let value = meta
        .value()
        .expect("proto(tag = ...) requires an integer literal value");
    let lit: Lit = value
        .parse()
        .expect("proto(tag = ...) expects an integer literal");

    if let Lit::Int(lit_int) = lit {
        Some(
            lit_int
                .base10_parse::<usize>()
                .expect("proto(tag = ...) must be a positive integer"),
        )
    } else {
        panic!("proto(tag = ...) must use an integer literal");
    }
}

fn parse_string_value(meta: &syn::meta::ParseNestedMeta) -> Option<String> {
    meta.value()
        .ok()
        .and_then(|v| v.parse::<Lit>().ok())
        .and_then(|lit| if let Lit::Str(s) = lit { Some(s.value()) } else { None })
}

// ============================================================================
// PARSED FIELD TYPE
// ============================================================================

#[derive(Clone)]
pub struct ParsedFieldType {
    pub rust_type: Type,
    pub proto_type: String,
    pub prost_type: TokenStream,
    pub is_option: bool,
    pub is_repeated: bool,
    pub is_message_like: bool,
    pub proto_rust_type: Type,
}

impl ParsedFieldType {
    pub fn primitive(rust_type: Type, proto_type: &str, prost_type: TokenStream) -> Self {
        Self {
            rust_type: rust_type.clone(),
            proto_type: proto_type.to_string(),
            prost_type,
            is_option: false,
            is_repeated: false,
            is_message_like: false,
            proto_rust_type: rust_type,
        }
    }
}

pub fn parse_field_type(ty: &Type) -> ParsedFieldType {
    match ty {
        Type::Array(type_array) => parse_array_type(type_array),
        Type::Path(type_path) => parse_path_type(type_path, ty),
        _ => panic!("Unsupported type: {:?}", quote!(#ty)),
    }
}

fn parse_array_type(type_array: &syn::TypeArray) -> ParsedFieldType {
    use syn::parse_quote;

    let elem_ty = &*type_array.elem;

    // [u8; N] -> bytes
    if is_u8_type(elem_ty) {
        return ParsedFieldType::primitive(parse_quote! { [u8] }, "bytes", quote! { bytes });
    }

    // Other arrays -> repeated
    let inner_parsed = parse_field_type(elem_ty);
    ParsedFieldType {
        rust_type: Type::Array(type_array.clone()),
        proto_type: inner_parsed.proto_type.clone(),
        prost_type: inner_parsed.prost_type.clone(),
        is_option: false,
        is_repeated: true,
        is_message_like: inner_parsed.is_message_like,
        proto_rust_type: parse_quote! { Vec<#elem_ty> },
    }
}

fn parse_path_type(type_path: &TypePath, ty: &Type) -> ParsedFieldType {
    let segment = type_path.path.segments.last().unwrap();
    let type_name = segment.ident.to_string();

    match type_name.as_str() {
        "Option" => parse_option_type(segment, ty),
        "Vec" => parse_vec_type(segment),
        _ => parse_primitive_or_custom(ty, &type_name),
    }
}

fn parse_option_type(segment: &syn::PathSegment, ty: &Type) -> ParsedFieldType {
    if let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner_ty)) = args.args.first()
    {
        let mut parsed = parse_field_type(inner_ty);
        parsed.is_option = true;
        parsed.proto_rust_type = ty.clone();
        return parsed;
    }
    panic!("Invalid Option type");
}

fn parse_vec_type(segment: &syn::PathSegment) -> ParsedFieldType {
    use syn::parse_quote;

    if let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner_ty)) = args.args.first()
    {
        // Vec<u8> -> bytes
        if is_u8_type(inner_ty) {
            return ParsedFieldType::primitive(parse_quote! { Vec<u8> }, "bytes", quote! { bytes });
        }

        let mut inner_parsed = parse_field_type(inner_ty);
        inner_parsed.is_repeated = true;
        return inner_parsed;
    }
    panic!("Invalid Vec type");
}

fn parse_primitive_or_custom(ty: &Type, type_name: &str) -> ParsedFieldType {
    match type_name {
        "u8" | "u16" | "u32" => ParsedFieldType::primitive(ty.clone(), "uint32", quote! { uint32 }),
        "u64" => ParsedFieldType::primitive(ty.clone(), "uint64", quote! { uint64 }),
        "u128" => ParsedFieldType::primitive(ty.clone(), "bytes", quote! { bytes }),
        "i8" | "i16" | "i32" => ParsedFieldType::primitive(ty.clone(), "int32", quote! { int32 }),
        "i64" => ParsedFieldType::primitive(ty.clone(), "int64", quote! { int64 }),
        "i128" => ParsedFieldType::primitive(ty.clone(), "bytes", quote! { bytes }),
        "f32" => ParsedFieldType::primitive(ty.clone(), "float", quote! { float }),
        "f64" => ParsedFieldType::primitive(ty.clone(), "double", quote! { double }),
        "String" => ParsedFieldType::primitive(ty.clone(), "string", quote! { string }),
        "Bytes" => ParsedFieldType::primitive(ty.clone(), "bytes", quote! { bytes }),
        "bool" => ParsedFieldType::primitive(ty.clone(), "bool", quote! { bool }),
        custom => parse_custom_type(ty, custom),
    }
}

fn parse_custom_type(ty: &Type, type_name: &str) -> ParsedFieldType {
    let proto_rust_type = if type_name.ends_with("Proto") {
        ty.clone()
    } else {
        syn::parse_str::<Type>(&format!("{}Proto", type_name)).unwrap()
    };

    ParsedFieldType {
        rust_type: ty.clone(),
        proto_type: "message".to_string(),
        prost_type: quote! { message },
        is_option: false,
        is_repeated: false,
        is_message_like: true,
        proto_rust_type,
    }
}

fn is_u8_type(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        path.path.segments.last().map(|s| s.ident == "u8").unwrap_or(false)
    } else {
        false
    }
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

// ============================================================================
// TAG ALLOCATION UTILITIES
// ============================================================================

use std::collections::HashSet;

#[derive(Default)]
pub struct TagAllocator {
    used: HashSet<usize>,
    next: usize,
}

impl TagAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Assign a protobuf tag number for a field.
    ///
    /// Custom tags are validated to be unique and greater than zero.
    /// Automatically assigned tags skip numbers that have been reserved
    /// by explicit assignments.
    pub fn assign(&mut self, requested: Option<usize>, context: &str) -> usize {
        if let Some(tag) = requested {
            if tag == 0 {
                panic!("proto(tag = 0) is invalid for field `{}`", context);
            }
            if !self.used.insert(tag) {
                panic!("duplicate proto tag {} detected for `{}`", tag, context);
            }
            return tag;
        }

        loop {
            let candidate = if self.next == 0 { 1 } else { self.next };
            self.next = candidate + 1;

            if self.used.insert(candidate) {
                return candidate;
            }
        }
    }
}
