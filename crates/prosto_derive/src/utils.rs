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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtoRename {
    pub proto_type: String,
    pub is_optional: Option<bool>,
    pub is_repeated: Option<bool>,
}

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
    pub treat_as: Option<String>,
    pub skip: bool,
    pub skip_deser_fn: Option<String>, // run after full decode
    pub is_rust_enum: bool,            // treat T as Rust enum -> i32 on wire
    pub is_message: bool,              // force message semantics
    pub is_proto_enum: bool,           // prost-like enum (i32 backing)
    pub import_path: Option<String>,
    pub getter: Option<String>,
    pub custom_tag: Option<usize>,
    pub rename: Option<ProtoRename>,
    pub validator: Option<String>, // field-level validation function
}

pub fn parse_field_config(field: &Field) -> FieldConfig {
    let mut cfg = FieldConfig::default();

    for attr in &field.attrs {
        if !attr.path().is_ident("proto") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
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
                Some("getter") => cfg.getter = parse_string_value(&meta),
                Some("into") => cfg.into_type = parse_string_value(&meta),
                Some("from") => cfg.from_type = parse_string_value(&meta),
                Some("into_fn") => cfg.into_fn = parse_string_value(&meta),
                Some("from_fn") => cfg.from_fn = parse_string_value(&meta),
                Some("try_from_fn") => cfg.try_from_fn = parse_string_value(&meta),
                Some("treat_as") => cfg.treat_as = parse_string_value(&meta),
                Some("import_path") => cfg.import_path = parse_string_value(&meta),
                Some("tag") => cfg.custom_tag = parse_usize_value(&meta),
                Some("rename") => {
                    let tokens: TokenStream = meta.value().expect("rename expects a value").parse().expect("failed to parse rename attribute");
                    cfg.rename = Some(parse_proto_rename(field, tokens));
                }
                Some("validator") => cfg.validator = parse_string_or_path_value(&meta),
                _ => return Err(meta.error("unknown #[proto(...)] attribute")),
            }
            Ok(())
        })
        .expect("failed to parse #[proto(...)] attributes");
    }

    cfg
}

fn parse_proto_rename(field: &Field, tokens: TokenStream) -> ProtoRename {
    use proc_macro2::TokenStream as TokenStream2;

    let tokens2: TokenStream2 = tokens;

    if let Ok(lit) = syn::parse2::<Lit>(tokens2.clone())
        && let Lit::Str(value) = lit
    {
        return parse_proto_rename_string(field, value.value());
    }

    if let Ok(ty) = syn::parse2::<Type>(tokens2.clone()) {
        return parse_proto_rename_type(&ty);
    }

    if let Ok(path) = syn::parse2::<syn::Path>(tokens2.clone()) {
        let path_str = path.segments.iter().map(|seg| seg.ident.to_string()).collect::<Vec<_>>().join("::");
        return parse_proto_rename_string(field, path_str);
    }

    panic!(
        "invalid value for #[proto(rename = ...)] on field {}",
        field.ident.as_ref().map_or_else(|| "<tuple field>".to_string(), ToString::to_string)
    );
}

fn parse_proto_rename_type(ty: &Type) -> ProtoRename {
    let (is_optional, is_repeated, inner_ty) = extract_field_wrapper_info(ty);
    let proto_type = canonicalize_proto_type_from_type(&inner_ty);

    ProtoRename {
        proto_type,
        is_optional: is_optional.then_some(true),
        is_repeated: is_repeated.then_some(true),
    }
}

fn parse_proto_rename_string(field: &Field, raw: String) -> ProtoRename {
    let mut is_optional = None;
    let mut is_repeated = None;
    let mut base_tokens = Vec::new();

    for token in raw.split_whitespace() {
        match token {
            "optional" => is_optional = Some(true),
            "repeated" => is_repeated = Some(true),
            _ => base_tokens.push(token),
        }
    }

    let base = base_tokens.join(" ");
    assert!(
        !base.is_empty(),
        "#[proto(rename = ...)] on field {} requires a target type",
        field.ident.as_ref().map_or_else(|| "<tuple field>".to_string(), ToString::to_string)
    );

    let proto_type = canonicalize_proto_type_from_str(&base).unwrap_or_else(|| canonicalize_proto_type_from_type_str(&base));

    ProtoRename { proto_type, is_optional, is_repeated }
}

fn canonicalize_proto_type_from_str(base: &str) -> Option<String> {
    if is_known_proto_scalar(base) { Some(base.to_string()) } else { None }
}

fn canonicalize_proto_type_from_type_str(base: &str) -> String {
    syn::parse_str::<Type>(base).map_or_else(|_| base.to_string(), |ty| canonicalize_proto_type_from_type(&ty))
}

fn canonicalize_proto_type_from_type(ty: &Type) -> String {
    if let Some(name) = proto_scalar_ident(ty)
        && is_known_proto_scalar(&name)
    {
        return name;
    }

    if is_bytes_vec(ty) || is_bytes_array(ty) {
        return "bytes".to_string();
    }

    let parsed = parse_field_type(ty);
    if parsed.map_kind.is_some() {
        return parsed.proto_type;
    }

    if parsed.is_message_like {
        let base_name = rust_type_path_ident(&parsed.proto_rust_type).to_string();
        strip_proto_suffix(&base_name)
    } else if parsed.proto_type == "message" {
        rust_type_path_ident(ty).to_string()
    } else {
        parsed.proto_type
    }
}

fn proto_scalar_ident(ty: &Type) -> Option<String> {
    if let Type::Path(path) = ty {
        if path.qself.is_some() {
            return None;
        }
        let segments: Vec<String> = path.path.segments.iter().map(|seg| seg.ident.to_string()).collect();
        if segments.len() == 1 {
            return Some(segments[0].clone());
        }
    }
    None
}

fn is_known_proto_scalar(name: &str) -> bool {
    matches!(
        name,
        "double" | "float" | "int32" | "int64" | "uint32" | "uint64" | "sint32" | "sint64" | "fixed32" | "fixed64" | "sfixed32" | "sfixed64" | "bool" | "string" | "bytes"
    )
}

fn parse_string_value(meta: &syn::meta::ParseNestedMeta) -> Option<String> {
    meta.value()
        .ok()
        .and_then(|v| v.parse::<Lit>().ok())
        .and_then(|lit| if let syn::Lit::Str(s) = lit { Some(s.value()) } else { None })
}

fn parse_string_or_path_value(meta: &syn::meta::ParseNestedMeta) -> Option<String> {
    let value_parser = meta.value().ok()?;

    // Try parsing as Expr which can be either a Lit or a Path
    if let Ok(expr) = value_parser.parse::<Expr>() {
        match expr {
            // Handle string literals: validator = "validate_fn"
            Expr::Lit(expr_lit) => {
                if let syn::Lit::Str(s) = expr_lit.lit {
                    return Some(s.value());
                }
            }
            // Handle paths: validator = validate_fn
            Expr::Path(expr_path) => {
                let path_str = expr_path.path.segments.iter().map(|seg| seg.ident.to_string()).collect::<Vec<_>>().join("::");
                return Some(path_str);
            }
            _ => {}
        }
    }

    None
}

fn parse_usize_value(meta: &syn::meta::ParseNestedMeta) -> Option<usize> {
    meta.value().ok().and_then(|v| v.parse::<Lit>().ok()).and_then(|lit| match lit {
        syn::Lit::Int(i) => i.base10_parse::<usize>().ok(),
        syn::Lit::Str(s) => s.value().parse::<usize>().ok(),
        _ => None,
    })
}

pub fn resolved_field_type(field: &Field, config: &FieldConfig) -> Type {
    if let Some(treat_as) = &config.treat_as {
        syn::parse_str::<Type>(treat_as).unwrap_or_else(|_| {
            let name = field.ident.as_ref().map_or_else(|| "<tuple field>".to_string(), ToString::to_string);
            panic!("invalid type in #[proto(treat_as = ...)] on field {name}");
        })
    } else {
        field.ty.clone()
    }
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

pub fn extract_field_wrapper_info(ty: &Type) -> (bool, bool, Type) {
    if is_option_type(ty) || is_arc_swap_option_type(ty) {
        if let Type::Path(type_path) = ty
            && let Some(segment) = type_path.path.segments.last()
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
        {
            let (_, inner_repeated, inner_ty) = extract_field_wrapper_info(inner);
            return (true, inner_repeated, inner_ty);
        }
        return (true, false, ty.clone());
    }

    if is_bytes_vec(ty) {
        return (false, false, ty.clone());
    }

    if let Some(inner) = vec_inner_type(ty) {
        let (_, _, inner_ty) = extract_field_wrapper_info(&inner);
        return (false, true, inner_ty);
    }

    if let Some((inner, _)) = set_inner_type(ty) {
        let (_, _, inner_ty) = extract_field_wrapper_info(&inner);
        return (false, true, inner_ty);
    }

    if let Some(inner) = cache_padded_inner_type(ty) {
        let (is_option, is_repeated, inner_ty) = extract_field_wrapper_info(&inner);
        return (is_option, is_repeated, inner_ty);
    }

    if let Some(inner) = arc_swap_inner_type(ty) {
        let (is_option, is_repeated, inner_ty) = extract_field_wrapper_info(&inner);
        return (is_option, is_repeated, inner_ty);
    }

    if let Type::Array(array) = ty {
        if is_bytes_array(ty) {
            return (false, false, ty.clone());
        }
        let (is_option, _, inner_ty) = extract_field_wrapper_info(&array.elem);
        return (is_option, true, inner_ty);
    }

    (false, false, ty.clone())
}

pub struct MethodInfo {
    pub name: syn::Ident,
    pub request_type: Type,
    pub response_type: Type,
    pub response_return_type: Type,
    pub response_is_result: bool,
    pub is_async: bool,
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

#[cfg(test)]
mod tests {
    use std::panic;

    use syn::parse_quote;

    use super::*;

    #[test]
    fn parse_field_config_panics_on_unknown_proto_attribute() {
        let field: syn::Field = parse_quote! { #[proto(unknown)] value: u32 };

        let result = panic::catch_unwind(|| parse_field_config(&field));

        assert!(result.is_err());
    }
}
