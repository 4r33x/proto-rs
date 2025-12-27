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
    config.from_type.is_some() || config.from_fn.is_some() || config.try_from_fn.is_some() || config.into_type.is_some() || is_numeric_enum(config, parsed)
}

fn uses_proto_wire_directly(info: &FieldInfo<'_>) -> bool {
    !info.config.skip && !needs_encode_conversion(&info.config, &info.parsed) && info.config.from_type.is_none() && info.config.from_fn.is_none() && info.config.try_from_fn.is_none()
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

pub fn generate_proto_shadow_impl(name: &Ident, generics: &syn::Generics) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics ::proto_rs::ProtoShadow<Self> for #name #ty_generics #where_clause {
            type Sun<'a> = &'a Self;
            type OwnedSun = Self;
            type View<'a> = &'a Self;

            #[inline(always)]
            fn to_sun(self) -> Result<Self::OwnedSun, ::proto_rs::DecodeError> {
                Ok(self)
            }

            #[inline(always)]
            fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
                value
            }
        }
    }
}

pub struct EncodeBinding {
    pub prelude: Option<TokenStream2>,
    pub value: TokenStream2,
}

pub fn encode_input_binding(field: &FieldInfo<'_>, base: &TokenStream2) -> EncodeBinding {
    let proto_ty = &field.proto_ty;
    let access_expr = if let Some(getter) = &field.config.getter {
        let base_str = base.to_string();
        let getter_expr = getter.replace('$', &base_str);
        syn::parse_str::<TokenStream2>(&getter_expr).unwrap_or_else(|_| {
            panic!(
                "invalid getter expression in #[proto(getter = ...)] on field {}",
                field.field.ident.as_ref().map_or_else(|| "<tuple field>".to_string(), ToString::to_string)
            )
        })
    } else {
        match &field.access {
            FieldAccess::Direct(tokens) => tokens.clone(),
            _ => field.access.access_tokens(base.clone()),
        }
    };

    if is_papaya_hash_map_type(&field.field.ty) {
        let tmp_ident = Ident::new(&format!("__proto_rs_field_{}_papaya", field.index), field.field.span());
        let prelude = quote! {
            let #tmp_ident = ::proto_rs::papaya_map_encode_input(&#access_expr);
        };
        return EncodeBinding {
            prelude: Some(prelude),
            value: quote! { #tmp_ident },
        };
    }

    if is_papaya_hash_set_type(&field.field.ty) {
        let tmp_ident = Ident::new(&format!("__proto_rs_field_{}_papaya", field.index), field.field.span());
        let prelude = quote! {
            let #tmp_ident = ::proto_rs::papaya_set_encode_input(&#access_expr);
        };
        return EncodeBinding {
            prelude: Some(prelude),
            value: quote! { #tmp_ident },
        };
    }

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
            if is_value_encode_type(&field.parsed.elem_type) {
                quote! { (#access_expr).clone() }
            } else {
                quote! { (#access_expr).as_ref().map(|inner| inner) }
            }
        } else if field.config.getter.is_some() {
            if is_value_encode_type(proto_ty) {
                quote! {{
                    let borrowed: &#proto_ty = #access_expr;
                    *borrowed
                }}
            } else {
                access_expr.clone()
            }
        } else if matches!(field.access, FieldAccess::Direct(_)) {
            if is_value_encode_type(proto_ty) {
                quote! {{
                    let borrowed: &#proto_ty = ::core::borrow::Borrow::borrow(&#access_expr);
                    *borrowed
                }}
            } else {
                access_expr.clone()
            }
        } else if is_value_encode_type(proto_ty) {
            access_expr.clone()
        } else {
            quote! { &(#access_expr) }
        };
        EncodeBinding { prelude: None, value: init_expr }
    }
}

fn is_papaya_hash_map_type(ty: &Type) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };

    let mut segments = type_path.path.segments.iter();
    let Some(last) = segments.next_back() else {
        return false;
    };

    if last.ident != "HashMap" {
        return false;
    }

    segments.any(|seg| seg.ident == "papaya")
}

fn is_papaya_hash_set_type(ty: &Type) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };

    let mut segments = type_path.path.segments.iter();
    let Some(last) = segments.next_back() else { return false };

    if last.ident != "HashSet" {
        return false;
    }

    segments.any(|seg| seg.ident == "papaya")
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
        quote! { <#ty as ::proto_rs::ProtoWire>::proto_default() }
    } else {
        quote! { ::core::default::Default::default() }
    }
}

fn encode_conversion_expr(field: &FieldInfo<'_>, access: &TokenStream2) -> TokenStream2 {
    if is_numeric_enum(&field.config, &field.parsed) {
        quote! { (#access) as i32 }
    } else if let Some(fun) = &field.config.into_fn {
        let fun_path = parse_path_string(field.field, fun);
        quote! { #fun_path(&(#access)) }
    } else if field.config.into_type.is_some() {
        let ty = &field.proto_ty;
        quote! { <#ty as ::core::convert::From<_>>::from((#access).clone()) }
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
                        let mut #tmp_ident: #decode_ty = <#decode_ty as ::proto_rs::ProtoWire>::proto_default();
                        <#decode_ty as ::proto_rs::ProtoWire>::decode_into(
                            wire_type,
                            &mut #tmp_ident,
                            buf,
                            ctx,
                        )?;
                        #assign
                        #validation
                        Ok(())
                    }
                })
            } else {
                let field_ty = &info.field.ty;
                Some(quote! {
                    #tag => {
                        <#field_ty as ::proto_rs::ProtoWire>::decode_into(
                            wire_type,
                            &mut #access,
                            buf,
                            ctx,
                        )?;
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
                quote! { <#ty as ::proto_rs::ProtoWire>::clear(&mut #access) }
            } else {
                quote! { #access = ::core::default::Default::default() }
            }
        })
        .collect()
}

pub fn build_is_default_checks(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            info.tag?;
            let ty = &info.proto_ty;
            let binding = encode_input_binding(info, base);
            let prelude = binding.prelude.into_iter();
            let value = binding.value;
            Some(quote! {
                {
                    #( #prelude )*
                    if !<#ty as ::proto_rs::ProtoWire>::is_default_impl(&#value) {
                        return false;
                    }
                }
            })
        })
        .collect()
}

pub fn build_encoded_len_terms(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            let tag = info.tag?;
            let ty = &info.proto_ty;
            let binding = encode_input_binding(info, base);
            let prelude = binding.prelude.into_iter();
            let value = binding.value;
            Some(quote! {{
                #( #prelude )*
                <#ty as ::proto_rs::ProtoWire>::encoded_len_tagged_impl(&#value, #tag)
            }})
        })
        .collect()
}

pub fn build_encode_stmts(fields: &[FieldInfo<'_>], base: &TokenStream2) -> Vec<TokenStream2> {
    fields
        .iter()
        .filter_map(|info| {
            let tag = info.tag?;
            let ty = &info.proto_ty;
            let binding = encode_input_binding(info, base);
            let prelude = binding.prelude.into_iter();
            let value = binding.value;
            Some(quote! {
                {
                    #( #prelude )*
                   <#ty as ::proto_rs::ProtoWire>::encode_with_tag(#tag, #value, buf)
                }
            })
        })
        .collect()
}

/// Generate delegating `ProtoWire` implementation for a sun type
/// This eliminates code duplication across structs, enums, and complex enums
pub fn generate_delegating_proto_wire_impl(shadow_ty: &TokenStream2, target_ty: &syn::Type) -> TokenStream2 {
    quote! {
        impl ::proto_rs::ProtoWire for #target_ty {
            type EncodeInput<'a> = <#shadow_ty as ::proto_rs::ProtoShadow<#target_ty>>::Sun<'a>;
            const KIND: ::proto_rs::ProtoKind = <#shadow_ty as ::proto_rs::ProtoWire>::KIND;

            #[inline(always)]
            fn proto_default() -> Self {
                <#shadow_ty as ::proto_rs::ProtoShadow<#target_ty>>::to_sun(
                    <#shadow_ty as ::proto_rs::ProtoWire>::proto_default(),
                )
                .expect("default shadow must be decodable")
            }

            #[inline(always)]
            fn clear(&mut self) {
                *self = Self::proto_default();
            }

            #[inline(always)]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                let shadow = <#shadow_ty as ::proto_rs::ProtoShadow<#target_ty>>::from_sun(*value);
                <#shadow_ty as ::proto_rs::ProtoWire>::is_default_impl(&shadow)
            }

            #[inline(always)]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                let shadow = <#shadow_ty as ::proto_rs::ProtoShadow<#target_ty>>::from_sun(*value);
                <#shadow_ty as ::proto_rs::ProtoWire>::encoded_len_impl_raw(&shadow)
            }

            #[inline(always)]
            fn encode_raw_unchecked(
                value: Self::EncodeInput<'_>,
                buf: &mut impl ::proto_rs::bytes::BufMut,
            ) {
                let shadow = <#shadow_ty as ::proto_rs::ProtoShadow<#target_ty>>::from_sun(value);
                <#shadow_ty as ::proto_rs::ProtoWire>::encode_raw_unchecked(shadow, buf)
            }

            #[inline(always)]
            fn decode_into(
                wire_type: ::proto_rs::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                let mut shadow = <#shadow_ty as ::proto_rs::ProtoWire>::proto_default();
                <#shadow_ty as ::proto_rs::ProtoWire>::decode_into(wire_type, &mut shadow, buf, ctx)?;
                *value = <#shadow_ty as ::proto_rs::ProtoShadow<#target_ty>>::to_sun(shadow)?;
                Ok(())
            }
        }
    }
}

/// Generate sun-based `ProtoExt` implementation
/// This eliminates code duplication across different type handlers
pub fn generate_sun_proto_ext_impl(
    shadow_ty: &TokenStream2,
    target_ty: &syn::Type,
    decode_arms: &[TokenStream2],
    post_decode_impl: &TokenStream2,
    validate_with_ext_impl: &TokenStream2,
) -> TokenStream2 {
    quote! {
        impl ::proto_rs::ProtoExt for #target_ty {
            type Shadow<'b> = #shadow_ty where Self: 'b;

            #[inline(always)]
            fn merge_field(
                value: &mut Self::Shadow<'_>,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                match tag {
                    #(#decode_arms,)*
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }

            #post_decode_impl
            #validate_with_ext_impl
        }
    }
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
        assert!(rendered.contains("Borrow :: borrow"), "binding should borrow before copying: {rendered}");
    }
}
