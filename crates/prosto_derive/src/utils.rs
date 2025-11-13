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
pub use type_info::SetKind;
pub use type_info::is_bytes_array;
pub use type_info::is_bytes_vec;
pub use type_info::parse_field_type;

pub fn set_inner_type(ty: &Type) -> Option<(Type, SetKind)> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        let kind = match segment.ident.to_string().as_str() {
            "HashSet" => Some(SetKind::HashSet),
            "BTreeSet" => Some(SetKind::BTreeSet),
            _ => None,
        }?;

        if let PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(GenericArgument::Type(inner)) = args.args.first()
        {
            return Some((inner.clone(), kind));
        }
    }

    None
}

pub fn cache_padded_inner_type(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "CachePadded"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner.clone());
    }

    None
}

pub fn arc_swap_inner_type(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "ArcSwap"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner.clone());
    }

    None
}

#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct FieldConfig {
    pub into_type: Option<String>,
    pub from_type: Option<String>,
    pub into_fn: Option<String>,
    pub from_fn: Option<String>,
    pub try_from_fn: Option<String>,
    pub skip: bool,
    pub skip_deser_fn: Option<String>, // run after full decode
    pub is_rust_enum: bool,            // treat T as Rust enum -> i32 on wire
    pub is_message: bool,              // force message semantics
    pub is_proto_enum: bool,           // prost-like enum (i32 backing)
    pub import_path: Option<String>,
    pub custom_tag: Option<usize>,
    pub is_transparent: bool,
}

pub fn parse_field_config(field: &Field) -> FieldConfig {
    let mut cfg = FieldConfig::default();

    for attr in &field.attrs {
        if !attr.path().is_ident("proto") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            let key = meta.path.get_ident().map(ToString::to_string);

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
                Some("try_from_fn") => cfg.try_from_fn = parse_string_value(&meta),
                Some("import_path") => cfg.import_path = parse_string_value(&meta),
                Some("tag") => cfg.custom_tag = parse_usize_value(&meta),
                Some("transparent") => cfg.is_transparent = true,
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
    path.path.segments.last().map_or_else(|| syn::Ident::new("_", Span::call_site()), |seg| seg.ident.clone())
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

pub fn is_arc_swap_option_type(ty: &Type) -> bool {
    matches!(last_path_segment(ty), Some(seg) if seg.ident == "ArcSwapOption")
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

pub struct MethodInfo {
    pub name: syn::Ident,
    pub request_type: Type,
    pub response_type: Type,
    pub response_return_type: Type,
    pub response_is_result: bool,
    pub is_streaming: bool,
    pub stream_type_name: Option<syn::Ident>,
    pub inner_response_type: Option<Type>,
    pub stream_item_type: Option<Type>,
    pub user_method_signature: TokenStream,
}

fn collect_discriminants_impl(variants: &[&syn::Variant]) -> Result<Vec<i32>, syn::Error> {
    let mut values = Vec::with_capacity(variants.len());
    let mut next_value: i32 = 0;

    for variant in variants {
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

    Ok(values)
}

pub fn collect_discriminants_for_variants(variants: &[&syn::Variant]) -> Result<Vec<i32>, syn::Error> {
    collect_discriminants_impl(variants)
}

pub fn find_marked_default_variant(data: &DataEnum) -> syn::Result<Option<usize>> {
    let mut default_index: Option<usize> = None;

    for (idx, variant) in data.variants.iter().enumerate() {
        if variant.attrs.iter().any(|attr| attr.path().is_ident("default")) {
            if default_index.is_some() {
                return Err(syn::Error::new(variant.span(), "multiple #[default] variants are not allowed"));
            }
            default_index = Some(idx);
        }
    }

    Ok(default_index)
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
