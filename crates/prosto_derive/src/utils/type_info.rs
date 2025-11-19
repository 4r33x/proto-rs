//! `type_info.rs`
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

use super::rust_type_path_ident;
use super::string_helpers::strip_proto_suffix;

/// Parsed metadata about a field's Rust type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapKind {
    HashMap,
    BTreeMap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetKind {
    HashSet,
    BTreeSet,
}

#[derive(Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ParsedFieldType {
    /// The concrete Rust type seen in the AST (e.g. `Vec<u8>` or `MyType`).
    pub rust_type: Type,
    /// Protobuf scalar kind used for prost attributes ("uint32", "bytes", ...).
    pub proto_type: String,
    /// Prost attribute fragment (e.g. `quote! { uint32 }`).
    pub prost_type: TokenStream,
    /// Whether the field is wrapped in `Option<T>`.
    pub is_option: bool,
    /// Whether the field should be treated as a protobuf message (length-delimited payload).
    pub is_message_like: bool,
    /// Whether the field encodes as a numeric scalar (eligible for packed encoding).
    pub is_numeric_scalar: bool,
    /// The protobuf-compatible Rust type used on the wire (after widening conversions).
    pub proto_rust_type: Type,
    /// The logical element type (inner `T` for `Option<T>`/`Vec<T>`/`[T;N]`).
    pub elem_type: Type,
    /// Whether this type should be encoded as a Rust enum (i32 on the wire).
    pub is_rust_enum: bool,
    /// Whether this type represents a map.
    pub map_kind: Option<MapKind>,
}

impl ParsedFieldType {
    #[allow(clippy::too_many_arguments)]
    fn new(rust_type: Type, proto_type: &str, prost_type: TokenStream, is_message_like: bool, is_numeric_scalar: bool, proto_rust_type: Type, elem_type: Type, is_rust_enum: bool) -> Self {
        Self {
            rust_type,
            proto_type: proto_type.to_string(),
            prost_type,
            is_option: false,
            is_message_like,
            is_numeric_scalar,
            proto_rust_type,
            elem_type,
            is_rust_enum,
            map_kind: None,
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

pub fn is_atomic_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if last_ident(path).is_some_and(|id| matches!(id.to_string().as_str(),
        "AtomicBool" |
        "AtomicU8" | "AtomicU16" | "AtomicU32" | "AtomicU64" | "AtomicUsize" |
        "AtomicI8" | "AtomicI16" | "AtomicI32" | "AtomicI64" | "AtomicIsize"
    )))
}

/// True if the type is `[u8; N]`.
pub fn is_bytes_array(ty: &Type) -> bool {
    match ty {
        Type::Array(array) => matches!(&*array.elem, Type::Path(inner) if last_ident(inner).is_some_and(|id| id == "u8")),
        Type::Path(path) => last_ident(path).is_some_and(|id| id == "FixedBytes"),
        _ => false,
    }
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
                    return matches!(inner, Type::Path(inner_path) if last_ident(inner_path).is_some_and(|i| i == "u8"));
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
        return ParsedFieldType::new(rust_ty.clone(), "bytes", quote! { bytes }, false, false, rust_ty, elem_ty, false);
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

        is_message_like,
        is_numeric_scalar,
        proto_rust_type: parse_quote! { ::proto_rs::alloc::vec::Vec<#inner_proto> },
        elem_type: elem,
        is_rust_enum: inner.is_rust_enum,
        map_kind: None,
    }
}

fn parse_path_type(path: &TypePath, ty: &Type) -> ParsedFieldType {
    if let Some(id) = last_ident(path) {
        match id.to_string().as_str() {
            "Option" => return parse_option_type(path, ty),
            "ArcSwapOption" => return parse_arc_swap_option_type(path, ty),
            "Vec" => return parse_vec_type(path, ty),
            "HashMap" => return parse_map_type(path, ty, MapKind::HashMap),
            "BTreeMap" => return parse_map_type(path, ty, MapKind::BTreeMap),
            "HashSet" | "BTreeSet" => return parse_set_type(path, ty),
            "ArcSwap" | "Box" | "Arc" | "CachePadded" => return parse_box_like_type(path, ty),
            "ZeroCopy" => return parse_zero_copy_type(path, ty),
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

fn parse_arc_swap_option_type(path: &TypePath, ty: &Type) -> ParsedFieldType {
    let Some(inner_ty) = single_generic(path) else {
        panic!("ArcSwapOption must have a single generic argument");
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

    if matches!(inner_ty, Type::Path(p) if last_ident(p).is_some_and(|id| id == "u8")) {
        return ParsedFieldType::new(
            ty.clone(),
            "bytes",
            quote! { bytes },
            false,
            false,
            parse_quote! { ::proto_rs::alloc::vec::Vec<u8> },
            (*inner_ty).clone(),
            false,
        );
    }

    let inner = parse_field_type(inner_ty);
    ParsedFieldType {
        rust_type: ty.clone(),
        proto_type: inner.proto_type.clone(),
        prost_type: inner.prost_type.clone(),
        is_option: false,

        is_message_like: inner.is_message_like,
        is_numeric_scalar: inner.is_numeric_scalar,
        proto_rust_type: inner.proto_rust_type.clone(),
        elem_type: inner.elem_type.clone(),
        is_rust_enum: inner.is_rust_enum,
        map_kind: None,
    }
}

fn parse_zero_copy_type(path: &TypePath, ty: &Type) -> ParsedFieldType {
    let Some(inner_ty) = single_generic(path) else {
        panic!("ZeroCopy must have a single generic argument");
    };

    let inner = parse_field_type(inner_ty);

    ParsedFieldType {
        rust_type: ty.clone(),
        proto_type: inner.proto_type.clone(),
        prost_type: inner.prost_type.clone(),
        is_option: false,
        is_message_like: inner.is_message_like,
        is_numeric_scalar: inner.is_numeric_scalar,
        proto_rust_type: inner.proto_rust_type.clone(),
        elem_type: inner.elem_type.clone(),
        is_rust_enum: inner.is_rust_enum,
        map_kind: None,
    }
}

fn parse_box_like_type(path: &TypePath, ty: &Type) -> ParsedFieldType {
    let Some(inner_ty) = single_generic(path) else {
        panic!("Box-like wrappers must have a single generic argument");
    };

    let mut inner = parse_field_type(inner_ty);
    inner.rust_type = ty.clone();
    inner.elem_type = (*inner_ty).clone();
    inner
}

fn parse_primitive_or_custom(ty: &Type) -> ParsedFieldType {
    match ty {
        Type::Path(path) => {
            if let Some(id) = last_ident(path) {
                return match id.to_string().as_str() {
                    "u8" | "u16" | "u32" => numeric_scalar(ty.clone(), parse_quote! { u32 }, "uint32"),
                    "u64" | "usize" => numeric_scalar(ty.clone(), parse_quote! { u64 }, "uint64"),
                    "i8" | "i16" | "i32" => numeric_scalar(ty.clone(), parse_quote! { i32 }, "int32"),
                    "i64" | "isize" => numeric_scalar(ty.clone(), parse_quote! { i64 }, "int64"),
                    "AtomicBool" => numeric_scalar(ty.clone(), parse_quote! { bool }, "bool"),
                    "AtomicU8" | "AtomicU16" | "AtomicU32" => numeric_scalar(ty.clone(), parse_quote! { u32 }, "uint32"),
                    "AtomicU64" | "AtomicUsize" => numeric_scalar(ty.clone(), parse_quote! { u64 }, "uint64"),
                    "AtomicI8" | "AtomicI16" | "AtomicI32" => numeric_scalar(ty.clone(), parse_quote! { i32 }, "int32"),
                    "AtomicI64" | "AtomicIsize" => numeric_scalar(ty.clone(), parse_quote! { i64 }, "int64"),
                    "f32" => ParsedFieldType::new(ty.clone(), "float", quote! { float }, false, true, parse_quote! { f32 }, ty.clone(), false),
                    "f64" => ParsedFieldType::new(ty.clone(), "double", quote! { double }, false, true, parse_quote! { f64 }, ty.clone(), false),
                    "bool" => numeric_scalar(ty.clone(), parse_quote! { bool }, "bool"),
                    "String" => ParsedFieldType::new(
                        ty.clone(),
                        "string",
                        quote! { string },
                        false,
                        false,
                        parse_quote! { ::proto_rs::alloc::string::String },
                        ty.clone(),
                        false,
                    ),
                    "Bytes" => ParsedFieldType::new(ty.clone(), "bytes", quote! { bytes }, false, false, parse_quote! { ::proto_rs::bytes::Bytes }, ty.clone(), false),
                    _ => parse_custom_type(ty),
                };
            }
            parse_custom_type(ty)
        }
        _ => parse_custom_type(ty),
    }
}

fn parse_map_type(path: &TypePath, ty: &Type, kind: MapKind) -> ParsedFieldType {
    let syn::PathArguments::AngleBracketed(args) = &path.path.segments.last().unwrap().arguments else {
        panic!("Map types must specify key and value generics");
    };

    let mut generics = args.args.iter().filter_map(|arg| match arg {
        GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    });

    let key_ty = generics.next().expect("map key type missing");
    let value_ty = generics.next().expect("map value type missing");

    let key_info = parse_field_type(&key_ty);
    let value_info = parse_field_type(&value_ty);

    let key_proto = key_info.proto_type.clone();
    let value_proto = if value_info.is_message_like {
        let rust_name = rust_type_path_ident(&value_info.proto_rust_type).to_string();
        strip_proto_suffix(&rust_name)
    } else {
        value_info.proto_type.clone()
    };

    let proto_type = format!("map<{key_proto}, {value_proto}>");

    let key_proto_ty = key_info.proto_rust_type.clone();
    let value_proto_ty = value_info.proto_rust_type.clone();
    let proto_rust_type = match kind {
        MapKind::HashMap => parse_quote! { ::std::collections::HashMap<#key_proto_ty, #value_proto_ty> },
        MapKind::BTreeMap => parse_quote! { ::proto_rs::alloc::collections::BTreeMap<#key_proto_ty, #value_proto_ty> },
    };

    ParsedFieldType {
        rust_type: ty.clone(),
        proto_type,
        prost_type: quote! { map },
        is_option: false,

        is_message_like: true,
        is_numeric_scalar: false,
        proto_rust_type,
        elem_type: value_ty.clone(),
        is_rust_enum: false,
        map_kind: Some(kind),
    }
}

fn parse_set_type(path: &TypePath, ty: &Type) -> ParsedFieldType {
    let syn::PathArguments::AngleBracketed(args) = &path.path.segments.last().unwrap().arguments else {
        panic!("Set types must specify element generics");
    };

    let elem_ty = args
        .args
        .iter()
        .find_map(|arg| match arg {
            GenericArgument::Type(ty) => Some(ty.clone()),
            _ => None,
        })
        .expect("set element type missing");

    let inner = parse_field_type(&elem_ty);

    ParsedFieldType {
        rust_type: ty.clone(),
        proto_type: inner.proto_type.clone(),
        prost_type: inner.prost_type.clone(),
        is_option: false,
        is_message_like: inner.is_message_like,
        is_numeric_scalar: inner.is_numeric_scalar,
        proto_rust_type: inner.proto_rust_type.clone(),
        elem_type: elem_ty,
        is_rust_enum: inner.is_rust_enum,
        map_kind: None,
    }
}

fn numeric_scalar(rust: Type, proto: Type, name: &str) -> ParsedFieldType {
    let ident = syn::Ident::new(name, Span::call_site());
    ParsedFieldType::new(rust.clone(), name, quote! { #ident }, false, true, proto, rust, false)
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
    ParsedFieldType::new(ty.clone(), "message", quote! { message }, true, false, proto_ty, ty.clone(), false)
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
