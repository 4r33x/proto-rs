//! type_info.rs
//! Lightweight type analysis used by the codegen. 100% `syn` v2 compatible.

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::GenericArgument;
use syn::PathArguments;
use syn::Type;
use syn::TypeArray;
use syn::TypePath;
use syn::parse_quote;

/// Parsed metadata about a field's Rust type.
#[derive(Clone)]
pub struct ParsedFieldType {
    /// The concrete Rust type seen in the AST (e.g. `Vec<u8>` or `MyType`).
    pub rust_type: Type,
    /// Protobuf scalar kind used for prost attributes ("uint32", "bytes", ...).
    pub proto_type: String,
    /// Prost attribute fragment (e.g. `quote! { uint32 }`).
    pub prost_type: TokenStream,
    /// Whether the field is wrapped in `Option<T>`.
    pub is_option: bool,
    /// Whether the field behaves like `repeated` (Vec/array of non-bytes).
    pub is_repeated: bool,
    /// Whether the field should be treated as a protobuf message (length-delimited payload).
    pub is_message_like: bool,
    /// Whether the field encodes as a numeric scalar (eligible for packed encoding).
    pub is_numeric_scalar: bool,
    /// The protobuf-compatible Rust type used on the wire (after widening conversions).
    pub proto_rust_type: Type,
    /// The logical element type (inner `T` for `Option<T>`/`Vec<T>`/`[T;N]`).
    pub elem_type: Type,
}

impl ParsedFieldType {
    fn new(rust_type: Type, proto_type: &str, prost_type: TokenStream, is_message_like: bool, is_numeric_scalar: bool, proto_rust_type: Type, elem_type: Type) -> Self {
        Self {
            rust_type,
            proto_type: proto_type.to_string(),
            prost_type,
            is_option: false,
            is_repeated: false,
            is_message_like,
            is_numeric_scalar,
            proto_rust_type,
            elem_type,
        }
    }
}

/// Entry point used throughout the codegen to analyse a Rust `Type`.
pub fn parse_field_type(ty: &Type) -> ParsedFieldType {
    match ty {
        Type::Array(array) => parse_array_type(array),
        Type::Path(path) => parse_path_type(path, ty),
        _ => parse_custom_type(ty),
    }
}

/// True if the type is `[u8; N]`.
pub fn is_bytes_array(ty: &Type) -> bool {
    matches!(ty, Type::Array(array) if matches!(&*array.elem, Type::Path(inner) if last_ident(inner).map(|id| id == "u8").unwrap_or(false)))
}

/// True if the type is `Vec<u8>` or `Bytes`.
pub fn is_bytes_vec(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => {
            if let Some(id) = last_ident(path) {
                if id == "Bytes" {
                    return true;
                }
                if id == "Vec"
                    && let Some(inner) = single_generic(path)
                {
                    return matches!(inner, Type::Path(inner_path) if last_ident(inner_path).map(|i| i == "u8").unwrap_or(false));
                }
            }
            false
        }
        _ => false,
    }
}

fn parse_array_type(array: &TypeArray) -> ParsedFieldType {
    let elem_ty = (*array.elem).clone();
    let rust_ty = Type::Array(array.clone());

    if is_bytes_array(&rust_ty) {
        return ParsedFieldType::new(rust_ty.clone(), "bytes", quote! { bytes }, false, false, rust_ty, elem_ty);
    }

    let inner = parse_field_type(&elem_ty);
    let proto_type = inner.proto_type.clone();
    let prost_type = inner.prost_type.clone();
    let is_message_like = inner.is_message_like;
    let is_numeric_scalar = inner.is_numeric_scalar;
    let inner_proto = inner.proto_rust_type.clone();
    let elem = inner.elem_type.clone();

    ParsedFieldType {
        rust_type: rust_ty,
        proto_type,
        prost_type,
        is_option: false,
        is_repeated: true,
        is_message_like,
        is_numeric_scalar,
        proto_rust_type: parse_quote! { ::std::vec::Vec<#inner_proto> },
        elem_type: elem,
    }
}

fn parse_path_type(path: &TypePath, ty: &Type) -> ParsedFieldType {
    if let Some(id) = last_ident(path) {
        match id.to_string().as_str() {
            "Option" => return parse_option_type(path, ty),
            "Vec" => return parse_vec_type(path, ty),
            _ => {}
        }
    }
    parse_primitive_or_custom(ty)
}

fn parse_option_type(path: &TypePath, ty: &Type) -> ParsedFieldType {
    let Some(inner_ty) = single_generic(path) else {
        panic!("Option must have a single generic argument");
    };
    let mut inner = parse_field_type(inner_ty);
    inner.is_option = true;
    inner.rust_type = ty.clone();
    inner.elem_type = (*inner_ty).clone();
    inner
}

fn parse_vec_type(path: &TypePath, ty: &Type) -> ParsedFieldType {
    let Some(inner_ty) = single_generic(path) else {
        panic!("Vec must have a single generic argument");
    };

    if matches!(inner_ty, Type::Path(p) if last_ident(p).map(|id| id == "u8").unwrap_or(false)) {
        return ParsedFieldType::new(ty.clone(), "bytes", quote! { bytes }, false, false, parse_quote! { ::std::vec::Vec<u8> }, (*inner_ty).clone());
    }

    let inner = parse_field_type(inner_ty);
    ParsedFieldType {
        rust_type: ty.clone(),
        proto_type: inner.proto_type.clone(),
        prost_type: inner.prost_type.clone(),
        is_option: false,
        is_repeated: true,
        is_message_like: inner.is_message_like,
        is_numeric_scalar: inner.is_numeric_scalar,
        proto_rust_type: inner.proto_rust_type.clone(),
        elem_type: inner.elem_type.clone(),
    }
}

fn parse_primitive_or_custom(ty: &Type) -> ParsedFieldType {
    match ty {
        Type::Path(path) => {
            if let Some(id) = last_ident(path) {
                return match id.to_string().as_str() {
                    "u8" => numeric_scalar(ty.clone(), parse_quote! { u32 }, "uint32"),
                    "u16" => numeric_scalar(ty.clone(), parse_quote! { u32 }, "uint32"),
                    "u32" => numeric_scalar(ty.clone(), parse_quote! { u32 }, "uint32"),
                    "u64" => numeric_scalar(ty.clone(), parse_quote! { u64 }, "uint64"),
                    "usize" => numeric_scalar(ty.clone(), parse_quote! { u64 }, "uint64"),
                    "i8" => numeric_scalar(ty.clone(), parse_quote! { i32 }, "int32"),
                    "i16" => numeric_scalar(ty.clone(), parse_quote! { i32 }, "int32"),
                    "i32" => numeric_scalar(ty.clone(), parse_quote! { i32 }, "int32"),
                    "i64" => numeric_scalar(ty.clone(), parse_quote! { i64 }, "int64"),
                    "isize" => numeric_scalar(ty.clone(), parse_quote! { i64 }, "int64"),
                    "f32" => ParsedFieldType::new(ty.clone(), "float", quote! { float }, false, true, parse_quote! { f32 }, ty.clone()),
                    "f64" => ParsedFieldType::new(ty.clone(), "double", quote! { double }, false, true, parse_quote! { f64 }, ty.clone()),
                    "bool" => numeric_scalar(ty.clone(), parse_quote! { bool }, "bool"),
                    "String" => ParsedFieldType::new(ty.clone(), "string", quote! { string }, false, false, parse_quote! { ::std::string::String }, ty.clone()),
                    "Bytes" => ParsedFieldType::new(ty.clone(), "bytes", quote! { bytes }, false, false, parse_quote! { ::bytes::Bytes }, ty.clone()),
                    _ => parse_custom_type(ty),
                };
            }
            parse_custom_type(ty)
        }
        _ => parse_custom_type(ty),
    }
}

fn numeric_scalar(rust: Type, proto: Type, name: &str) -> ParsedFieldType {
    let ident = syn::Ident::new(name, Span::call_site());
    ParsedFieldType::new(rust.clone(), name, quote! { #ident }, false, true, proto, rust)
}

fn parse_array_proto_suffix(ty: &Type) -> Type {
    match ty {
        Type::Array(array) => {
            let inner = parse_array_proto_suffix(&array.elem);
            parse_quote! { [#inner] }
        }
        _ => with_proto_suffix(ty),
    }
}

fn parse_custom_type(ty: &Type) -> ParsedFieldType {
    let proto_ty = parse_array_proto_suffix(ty);
    ParsedFieldType::new(ty.clone(), "message", quote! { message }, true, false, proto_ty, ty.clone())
}

fn last_ident(path: &TypePath) -> Option<&syn::Ident> {
    path.path.segments.last().map(|s| &s.ident)
}

fn single_generic(path: &TypePath) -> Option<&Type> {
    path.path
        .segments
        .last()
        .and_then(|seg| match &seg.arguments {
            PathArguments::AngleBracketed(args) => args.args.first(),
            _ => None,
        })
        .and_then(|arg| match arg {
            GenericArgument::Type(t) => Some(t),
            _ => None,
        })
}

fn with_proto_suffix(ty: &Type) -> Type {
    match ty {
        Type::Path(path) => {
            let mut cloned = path.clone();
            if let Some(seg) = cloned.path.segments.last_mut() {
                let ident = seg.ident.to_string();
                if !ident.ends_with("Proto") {
                    let new_ident = syn::Ident::new(&(ident + "Proto"), Span::call_site());
                    seg.ident = new_ident;
                }
            }
            Type::Path(cloned)
        }
        _ => ty.clone(),
    }
}
