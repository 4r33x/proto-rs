//! Centralized utilities for proto macro code generation

use proc_macro2::Span;
use proc_macro2::TokenStream;
use syn::DataEnum;
use syn::Expr;
use syn::Field;
use syn::GenericArgument;
use syn::Lit;
use syn::PathArguments;
use syn::Type;
use syn::TypePath;
use syn::spanned::Spanned;

pub mod string_helpers;
pub mod type_info;

pub use string_helpers::*;
pub use type_info::ParsedFieldType;
pub use type_info::is_bytes_array;
pub use type_info::is_bytes_vec;
pub use type_info::parse_field_type;

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
                    if meta.input.peek(syn::Token![=])
                        && let Some(fun) = parse_string_value(&meta)
                    {
                        cfg.skip_deser_fn = Some(fun);
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

pub fn vec_inner_type(ty: &Type) -> Option<Type> {
    if let Type::Path(path) = ty
        && let Some(seg) = path.path.segments.last()
        && seg.ident == "Vec"
        && let PathArguments::AngleBracketed(args) = &seg.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner.clone());
    }
    None
}

pub fn is_complex_type(ty: &Type) -> bool {
    parse_field_type(ty).is_message_like
}

pub struct MethodInfo {
    pub name: syn::Ident,
    pub request_type: Box<Type>,
    pub response_type: Box<Type>,
    pub is_streaming: bool,
    pub stream_type_name: Option<syn::Ident>,
    pub inner_response_type: Option<Type>,
    pub user_method_signature: TokenStream,
}

pub fn collect_enum_discriminants(data: &DataEnum) -> Result<Vec<i32>, syn::Error> {
    let mut values = Vec::with_capacity(data.variants.len());
    let mut next_value: i32 = 0;

    for variant in data.variants.iter() {
        let value = if let Some((_, expr)) = &variant.discriminant {
            let parsed = eval_discriminant(expr)?;
            next_value = parsed.checked_add(1).ok_or_else(|| syn::Error::new_spanned(&variant.ident, "enum discriminant overflowed i32 range"))?;
            parsed
        } else {
            let value = next_value;
            next_value = next_value
                .checked_add(1)
                .ok_or_else(|| syn::Error::new_spanned(&variant.ident, "enum discriminant overflowed i32 range"))?;
            value
        };

        values.push(value);
    }

    if !values.contains(&0) {
        return Err(syn::Error::new(data.variants.span(), "proto enums must contain a variant with discriminant 0"));
    }

    Ok(values)
}

fn eval_discriminant(expr: &Expr) -> Result<i32, syn::Error> {
    match expr {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            Lit::Int(lit_int) => lit_int.base10_parse::<i32>().map_err(|_| syn::Error::new(lit_int.span(), "enum discriminant must fit in i32")),
            _ => Err(syn::Error::new(expr.span(), "unsupported enum discriminant literal")),
        },
        Expr::Unary(expr_unary) => {
            use syn::UnOp;
            match expr_unary.op {
                UnOp::Neg(_) => {
                    let value = eval_discriminant(&expr_unary.expr)?;
                    value.checked_neg().ok_or_else(|| syn::Error::new(expr.span(), "enum discriminant must fit in i32"))
                }
                _ => Err(syn::Error::new(expr.span(), "unsupported enum discriminant expression")),
            }
        }
        Expr::Group(group) => eval_discriminant(&group.expr),
        Expr::Paren(paren) => eval_discriminant(&paren.expr),
        _ => Err(syn::Error::new(expr.span(), "unsupported enum discriminant expression")),
    }
}
