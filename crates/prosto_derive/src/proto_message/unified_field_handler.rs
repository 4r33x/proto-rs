use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Attribute;
use syn::Field;
use syn::Ident;
use syn::ItemEnum;
use syn::Path;
use syn::Type;
use syn::parse_quote;
use syn::spanned::Spanned;

use crate::utils::FieldConfig;
use crate::utils::ParsedFieldType;
use crate::utils::is_option_type;

#[derive(Clone)]
pub struct FieldInfo<'a> {
    pub index: usize,
    pub field: &'a Field,
    pub access: FieldAccess<'a>,
    pub config: FieldConfig,
    pub tag: Option<u32>,
    pub parsed: ParsedFieldType,
    pub proto_ty: Type,
    pub decode_ty: Type,
}

#[derive(Clone)]
pub enum FieldAccess<'a> {
    Named(&'a Ident),
    Tuple(usize),
    Direct(TokenStream2),
}

impl FieldAccess<'_> {
    pub fn ident(&self) -> Option<&Ident> {
        match self {
            FieldAccess::Named(id) => Some(id),
            FieldAccess::Tuple(_) | FieldAccess::Direct(_) => None,
        }
    }

    pub fn access_tokens(&self, base: TokenStream2) -> TokenStream2 {
        match self {
            FieldAccess::Named(ident) => quote! { #base.#ident },
            FieldAccess::Tuple(idx) => {
                let index = syn::Index::from(*idx);
                quote! { #base.#index }
            }
            FieldAccess::Direct(tokens) => tokens.clone(),
        }
    }
}

fn parse_type_string(field: &Field, value: &str) -> Type {
    syn::parse_str::<Type>(value).unwrap_or_else(|_| {
        let name = field.ident.as_ref().map_or_else(|| "<tuple field>".to_string(), ToString::to_string);
        panic!("invalid type in #[proto] attribute on field {name}")
    })
}

pub fn parse_path_string(field: &Field, value: &str) -> Path {
    syn::parse_str::<Path>(value).unwrap_or_else(|_| {
        let name = field.ident.as_ref().map_or_else(|| "<tuple field>".to_string(), ToString::to_string);
        panic!("invalid function path in #[proto] attribute on field {name}")
    })
}

fn is_numeric_enum(config: &FieldConfig, parsed: &ParsedFieldType) -> bool {
    config.is_rust_enum || config.is_proto_enum || parsed.is_rust_enum
}

pub fn compute_proto_ty(field: &Field, config: &FieldConfig, parsed: &ParsedFieldType, effective_ty: &Type) -> Type {
    if let Some(into_ty) = &config.into_type {
        parse_type_string(field, into_ty)
    } else if is_numeric_enum(config, parsed) {
        parse_quote! { i32 }
    } else {
        effective_ty.clone()
    }
}

pub fn compute_decode_ty(field: &Field, config: &FieldConfig, parsed: &ParsedFieldType, proto_ty: &Type) -> Type {
    if let Some(from_ty) = &config.from_type {
        parse_type_string(field, from_ty)
    } else if let Some(into_ty) = &config.into_type {
        parse_type_string(field, into_ty)
    } else if is_numeric_enum(config, parsed) {
        parse_quote! { i32 }
    } else {
        proto_ty.clone()
    }
}

pub fn needs_encode_conversion(config: &FieldConfig, parsed: &ParsedFieldType) -> bool {
    config.into_type.is_some() || config.into_fn.is_some() || is_numeric_enum(config, parsed)
}

pub fn needs_decode_conversion(config: &FieldConfig, parsed: &ParsedFieldType) -> bool {
    config.from_type.is_some()
        || config.from_fn.is_some()
        || config.try_from_fn.is_some()
        || config.into_type.is_some()
        || is_numeric_enum(config, parsed)
}

pub(super) fn uses_proto_wire_directly(info: &FieldInfo<'_>) -> bool {
    !info.config.skip
        && !needs_encode_conversion(&info.config, &info.parsed)
        && info.config.from_type.is_none()
        && info.config.from_fn.is_none()
        && info.config.try_from_fn.is_none()
}

pub fn strip_proto_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs.iter().filter(|attr| !attr.path().is_ident("proto_message") && !attr.path().is_ident("proto")).cloned().collect()
}

pub fn sanitize_enum(mut item: ItemEnum) -> ItemEnum {
    item.attrs = strip_proto_attrs(&item.attrs);
    for variant in &mut item.variants {
        variant.attrs = strip_proto_attrs(&variant.attrs);
        match &mut variant.fields {
            syn::Fields::Named(fields) => {
                for field in &mut fields.named {
                    field.attrs = strip_proto_attrs(&field.attrs);
                }
            }
            syn::Fields::Unnamed(fields) => {
                for field in &mut fields.unnamed {
                    field.attrs = strip_proto_attrs(&field.attrs);
                }
            }
            syn::Fields::Unit => {}
        }
    }
    item
}

pub fn assign_tags(mut fields: Vec<FieldInfo<'_>>) -> Vec<FieldInfo<'_>> {
    let mut used = BTreeSet::new();
    let mut next = 1u32;

    for info in &mut fields {
        if info.config.skip {
            continue;
        }

        let tag = if let Some(custom) = info.config.custom_tag {
            assert!(custom != 0, "proto field tags must be >= 1");
            let custom_u32: u32 = custom.try_into().expect("proto field tag overflowed u32");
            assert!(used.insert(custom_u32), "duplicate proto field tag: {custom}");
            custom_u32
        } else {
            while used.contains(&next) {
                next = next.checked_add(1).expect("proto field tag overflowed u32");
            }
            let assigned = next;
            used.insert(assigned);
            next = next.checked_add(1).expect("proto field tag overflowed u32");
            assigned
        };

        info.tag = Some(tag);
    }

    fields
}

pub struct EncodeBinding {
    pub prelude: Option<TokenStream2>,
    pub value: TokenStream2,
}

pub fn encode_input_binding(field: &FieldInfo<'_>, base: &TokenStream2) -> EncodeBinding {
    let proto_ty = &field.proto_ty;
    let access_expr = if let Some(getter) = &field.config.getter {
        parse_getter_expr(getter, base, field.field)
    } else {
        match &field.access {
            FieldAccess::Direct(tokens) => tokens.clone(),
            _ => field.access.access_tokens(base.clone()),
        }
    };

    if needs_encode_conversion(&field.config, &field.parsed) {
        let tmp_ident = Ident::new(&format!("__proto_rs_field_{}_converted", field.index), field.field.span());
        let converted = encode_conversion_expr(field, &access_expr);
        if is_value_encode_type(proto_ty) {
            let prelude = quote! {
                let #tmp_ident: #proto_ty = #converted;
            };
            EncodeBinding {
                prelude: Some(prelude),
                value: quote! { #tmp_ident },
            }
        } else {
            let prelude = quote! {
                let #tmp_ident: #proto_ty = #converted;
            };
            EncodeBinding {
                prelude: Some(prelude),
                value: quote! { &#tmp_ident },
            }
        }
    } else {
        let init_expr = if is_option_type(&field.field.ty) {
            quote! { (#access_expr).as_ref() }
        } else {
            quote! { #access_expr }
        };
        EncodeBinding {
            prelude: None,
            value: init_expr,
        }
    }
}

fn parse_getter_expr(getter: &str, base: &TokenStream2, field: &Field) -> TokenStream2 {
    let base_str = base.to_string();
    let getter_expr = getter.replace('$', &base_str);
    syn::parse_str::<TokenStream2>(&getter_expr).unwrap_or_else(|_| {
        let name = field.ident.as_ref().map_or_else(|| "<tuple field>".to_string(), ToString::to_string);
        panic!("invalid getter expression in #[proto(getter = ...)] on field {name}")
    })
}

pub fn is_value_encode_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(type_path)
    if type_path.qself.is_none()
        && type_path.path.segments.len() == 1
        && matches!(type_path.path.segments[0].ident.to_string().as_str(),
            "bool" | "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
            "f32" | "f64"
        ))
}

pub fn build_proto_default_expr(fields: &[FieldInfo<'_>], original: &syn::Fields) -> TokenStream2 {
    match original {
        syn::Fields::Unit => quote! { Self },
        syn::Fields::Unnamed(_) => {
            if fields.is_empty() {
                quote! { Self }
            } else {
                let defaults = fields.iter().map(field_proto_default_expr);
                quote! { Self( #(#defaults),* ) }
            }
        }
        syn::Fields::Named(_) => {
            if fields.is_empty() {
                quote! { Self { } }
            } else {
                let defaults = fields.iter().map(|info| {
                    let ident = info.access.ident().expect("expected named field ident");
                    let expr = field_proto_default_expr(info);
                    quote! { #ident: #expr }
                });
                quote! { Self { #(#defaults),* } }
            }
        }
    }
}

pub fn field_proto_default_expr(info: &FieldInfo<'_>) -> TokenStream2 {
    if uses_proto_wire_directly(info) {
        let ty = &info.field.ty;
        quote! { <#ty as ::proto_rs::ProtoDecoder>::proto_default() }
    } else {
        quote! { ::core::default::Default::default() }
    }
}

pub fn encode_conversion_expr(field: &FieldInfo<'_>, access: &TokenStream2) -> TokenStream2 {
    if is_numeric_enum(&field.config, &field.parsed) {
        quote! { (*(#access)) as i32 }
    } else if let Some(fun) = &field.config.into_fn {
        let fun_path = parse_path_string(field.field, fun);
        quote! { #fun_path(#access) }
    } else if field.config.into_type.is_some() {
        let ty = &field.proto_ty;
        quote! { <#ty as ::core::convert::From<_>>::from((*(#access)).clone()) }
    } else {
        access.clone()
    }
}

pub fn decode_conversion_assign(info: &FieldInfo<'_>, access: &TokenStream2, tmp_ident: &Ident) -> TokenStream2 {
    if is_numeric_enum(&info.config, &info.parsed) {
        let field_ty = &info.field.ty;
        quote! {
            #access = <#field_ty as ::core::convert::TryFrom<i32>>::try_from(#tmp_ident)
                .map_err(::core::convert::Into::into)?;
        }
    } else if let Some(fun) = &info.config.from_fn {
        let fun_path = parse_path_string(info.field, fun);
        quote! {
            #access = #fun_path(#tmp_ident);
        }
    } else if let Some(fun) = &info.config.try_from_fn {
        let fun_path = parse_path_string(info.field, fun);
        quote! {
            #access = #fun_path(#tmp_ident).map_err(::core::convert::Into::into)?;
        }
    } else {
        let field_ty = &info.field.ty;
        quote! {
            #access = <#field_ty as ::core::convert::From<_>>::from(#tmp_ident);
        }
    }
}

pub fn build_post_decode_hooks(fields: &[FieldInfo<'_>]) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            let fun = info.config.skip_deser_fn.as_ref()?;
            if !info.config.skip {
                return None;
            }
            let fun_path = parse_path_string(info.field, fun);
            let access = info.access.access_tokens(quote! { shadow });
            Some(quote! {
                {
                    let __proto_rs_tmp = #fun_path(&mut shadow);
                    #access = __proto_rs_tmp;
                }
            })
        })
        .collect()
}

pub fn build_decode_match_arms(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            let tag = info.tag?;
            let access = info.access.access_tokens(base.clone());

            // Generate field validation if validator is specified
            let validation = if let Some(validator_fn) = &info.config.validator {
                let validator_path = parse_path_string(info.field, validator_fn);
                quote! {
                    #validator_path(&mut #access)?;
                }
            } else {
                quote! {}
            };

            if needs_decode_conversion(&info.config, &info.parsed) {
                let tmp_ident = Ident::new(&format!("__proto_rs_field_{}_tmp", info.index), info.field.span());
                let decode_ty = &info.decode_ty;
                let assign = decode_conversion_assign(info, &access, &tmp_ident);
                Some(quote! {
                    #tag => {
                        let mut #tmp_ident: #decode_ty = <#decode_ty as ::proto_rs::ProtoDecoder>::proto_default();
                        <#decode_ty as ::proto_rs::ProtoDecoder>::merge(&mut #tmp_ident, wire_type, buf, ctx)?;
                        #assign
                        #validation
                        Ok(())
                    }
                })
            } else {
                let field_ty = &info.field.ty;
                Some(quote! {
                    #tag => {
                        <#field_ty as ::proto_rs::ProtoDecoder>::merge(&mut #access, wire_type, buf, ctx)?;
                        #validation
                        Ok(())
                    }
                })
            }
        })
        .collect()
}

pub fn build_clear_stmts(fields: &[FieldInfo<'_>], self_tokens: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .map(|info| {
            let access = info.access.access_tokens(self_tokens.clone());
            if uses_proto_wire_directly(info) {
                let ty = &info.field.ty;
                quote! { <#ty as ::proto_rs::ProtoDecoder>::clear(&mut #access) }
            } else {
                quote! { #access = ::core::default::Default::default() }
            }
        })
        .collect()
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::parse_field_config;
    use crate::utils::parse_field_type;

    #[test]
    fn direct_scalar_field_derefs_binding() {
        let field: Field = syn::parse_quote! {
            #[proto(tag = 1)]
            value: u32
        };

        let config = parse_field_config(&field);
        let effective_ty = crate::utils::resolved_field_type(&field, &config);
        let parsed = parse_field_type(&effective_ty);
        let proto_ty = compute_proto_ty(&field, &config, &parsed, &effective_ty);
        let decode_ty = compute_decode_ty(&field, &config, &parsed, &proto_ty);

        let info = FieldInfo {
            index: 0,
            field: &field,
            access: FieldAccess::Direct(quote! { value }),
            config,
            tag: Some(1),
            parsed,
            proto_ty,
            decode_ty,
        };

        let binding = encode_input_binding(&info, &TokenStream2::new());
        assert!(binding.prelude.is_none());
        let rendered = binding.value.to_string();
        assert!(
            rendered.contains("Borrow :: borrow"),
            "binding should borrow before copying: {rendered}"
        );
    }
}
