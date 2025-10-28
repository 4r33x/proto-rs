use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;
use syn::Ident;
use syn::ItemEnum;
use syn::Lit;
use syn::spanned::Spanned;

use super::unified_field_handler::FieldAccess;
use super::unified_field_handler::FieldInfo;
use super::unified_field_handler::assign_tags;
use super::unified_field_handler::build_encode_stmts;
use super::unified_field_handler::build_encoded_len_terms;
use super::unified_field_handler::build_is_default_checks;
use super::unified_field_handler::compute_decode_ty;
use super::unified_field_handler::compute_proto_ty;
use super::unified_field_handler::decode_conversion_assign;
use super::unified_field_handler::field_proto_default_expr;
use super::unified_field_handler::generate_proto_shadow_impl;
use super::unified_field_handler::needs_decode_conversion;
use super::unified_field_handler::parse_path_string;
use super::unified_field_handler::sanitize_enum;
use crate::parse::UnifiedProtoConfig;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;

pub(super) fn generate_complex_enum_impl(input: &DeriveInput, item_enum: &ItemEnum, data: &syn::DataEnum, config: &UnifiedProtoConfig) -> syn::Result<TokenStream2> {
    let enum_item = sanitize_enum(item_enum.clone());

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let default_index = crate::utils::find_marked_default_variant(data)?.unwrap_or(0);
    let mut variants = collect_variant_infos(data)?;
    if variants.is_empty() {
        return Err(syn::Error::new(input.ident.span(), "proto_message enum must contain at least one variant"));
    }
    if default_index >= variants.len() {
        return Err(syn::Error::new(input.ident.span(), "#[default] variant index is out of bounds"));
    }
    variants[default_index].is_default = true;

    let proto_shadow_impl = if config.sun.is_some() {
        quote! {}
    } else {
        generate_proto_shadow_impl(name, generics)
    };

    let target_ty = if let Some(sun) = &config.sun {
        let ty = &sun.ty;
        quote! { #ty }
    } else {
        quote! { #name #ty_generics }
    };
    let shadow_ty = quote! { #name #ty_generics };

    let merge_field_arms = variants.iter().map(|variant| build_variant_merge_arm(name, variant)).collect::<Vec<_>>();

    let default_expr = build_variant_default_expr(&variants[default_index]);
    let is_default_match_arms = variants.iter().map(build_variant_is_default_arm).collect::<Vec<_>>();
    let encoded_len_arms = variants.iter().map(build_variant_encoded_len_arm).collect::<Vec<_>>();
    let encode_arms = variants.iter().map(build_variant_encode_arm).collect::<Vec<_>>();

    let proto_ext_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoExt for #target_ty #where_clause {
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
                    #(#merge_field_arms,)*
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }
        }
    };

    let proto_wire_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoWire for #name #ty_generics #where_clause {
            type EncodeInput<'b> = &'b Self;
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;

            #[inline(always)]
            fn proto_default() -> Self {
                #default_expr
            }

            #[inline(always)]
            fn clear(&mut self) {
                *self = Self::proto_default();
            }

            #[inline(always)]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                match **value {
                    #(#is_default_match_arms,)*
                }
            }

            #[inline(always)]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                match **value {
                    #(#encoded_len_arms,)*
                }
            }

            #[inline(always)]
            fn encode_raw_unchecked(
                value: Self::EncodeInput<'_>,
                buf: &mut impl ::proto_rs::bytes::BufMut,
            ) {
                match *value {
                    #(#encode_arms,)*
                }
            }

            #[inline(always)]
            fn decode_into(
                wire_type: ::proto_rs::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                ::proto_rs::encoding::check_wire_type(
                    ::proto_rs::encoding::WireType::LengthDelimited,
                    wire_type,
                )?;
                *value = <Self as ::proto_rs::ProtoExt>::decode_length_delimited(buf, ctx)?;
                Ok(())
            }
        }
    };

    Ok(quote! {
        #enum_item
        #proto_shadow_impl
        #proto_ext_impl
        #proto_wire_impl
    })
}

#[derive(Clone)]
enum VariantKind<'a> {
    Unit,
    Tuple { field: TupleVariantInfo<'a> },
    Struct { fields: Vec<FieldInfo<'a>> },
}

#[derive(Clone)]
struct VariantInfo<'a> {
    ident: &'a Ident,
    tag: u32,
    kind: VariantKind<'a>,
    is_default: bool,
}

#[derive(Clone)]
struct TupleVariantInfo<'a> {
    field: FieldInfo<'a>,
    binding_ident: Ident,
}

fn collect_variant_infos(data: &syn::DataEnum) -> syn::Result<Vec<VariantInfo<'_>>> {
    let mut used_tags = BTreeSet::new();
    let mut variants = Vec::new();

    for (idx, variant) in data.variants.iter().enumerate() {
        let tag = resolve_variant_tag(variant, idx + 1)?;
        if !used_tags.insert(tag) {
            return Err(syn::Error::new(variant.ident.span(), format!("duplicate proto(tag) attribute for enum variant: {tag}")));
        }

        let kind = match &variant.fields {
            syn::Fields::Unit => VariantKind::Unit,
            syn::Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    return Err(syn::Error::new(variant.ident.span(), "complex enum tuple variants must contain exactly one field"));
                }

                let field = &fields.unnamed[0];
                let config = parse_field_config(field);
                let parsed = parse_field_type(&field.ty);
                let proto_ty = compute_proto_ty(field, &config, &parsed);
                let decode_ty = compute_decode_ty(field, &config, &parsed, &proto_ty);
                let binding_ident = Ident::new(&format!("__proto_rs_variant_{}_value", variant.ident.to_string().to_lowercase()), field.span());

                let mut field_info = FieldInfo {
                    index: 0,
                    field,
                    access: FieldAccess::Direct(quote! { #binding_ident }),
                    config,
                    tag: None,
                    parsed,
                    proto_ty,
                    decode_ty,
                };

                if !field_info.config.skip {
                    let tag = field_info.config.custom_tag.unwrap_or(1);
                    if tag == 0 {
                        return Err(syn::Error::new(field.span(), "proto field tags must be greater than or equal to 1"));
                    }
                    field_info.tag = Some(u32::try_from(tag).map_err(|_| syn::Error::new(field.span(), "proto field tag overflowed u32"))?);
                }

                VariantKind::Tuple {
                    field: TupleVariantInfo { field: field_info, binding_ident },
                }
            }
            syn::Fields::Named(fields_named) => {
                let mut infos: Vec<_> = fields_named
                    .named
                    .iter()
                    .enumerate()
                    .map(|(field_idx, field)| {
                        let config = parse_field_config(field);
                        let parsed = parse_field_type(&field.ty);
                        let proto_ty = compute_proto_ty(field, &config, &parsed);
                        let decode_ty = compute_decode_ty(field, &config, &parsed, &proto_ty);
                        FieldInfo {
                            index: field_idx,
                            field,
                            access: FieldAccess::Direct({
                                let ident = field.ident.as_ref().expect("named variant field");
                                quote! { #ident }
                            }),
                            config,
                            tag: None,
                            parsed,
                            proto_ty,
                            decode_ty,
                        }
                    })
                    .collect();
                infos = assign_tags(infos);
                VariantKind::Struct { fields: infos }
            }
        };

        variants.push(VariantInfo {
            ident: &variant.ident,
            tag,
            kind,
            is_default: false,
        });
    }

    Ok(variants)
}

fn resolve_variant_tag(variant: &syn::Variant, default: usize) -> syn::Result<u32> {
    let mut custom_tag = None;

    for attr in &variant.attrs {
        if !attr.path().is_ident("proto") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.get_ident().is_some_and(|ident| ident == "tag") {
                if custom_tag.is_some() {
                    return Err(syn::Error::new(meta.path.span(), "duplicate proto(tag) attribute for variant"));
                }

                let lit: Lit = meta.value()?.parse()?;
                let value = match lit {
                    Lit::Int(int_lit) => int_lit.base10_parse::<usize>()?,
                    Lit::Str(str_lit) => str_lit.value().parse::<usize>().map_err(|_| syn::Error::new(str_lit.span(), "proto tag must be a positive integer"))?,
                    _ => {
                        return Err(syn::Error::new(lit.span(), "proto tag must be specified as an integer"));
                    }
                };

                custom_tag = Some(value);
            }
            Ok(())
        })?;
    }

    let tag = custom_tag.unwrap_or(default);
    if tag == 0 {
        return Err(syn::Error::new(variant.ident.span(), "proto enum variant tags must be greater than or equal to 1"));
    }

    let tag_u32 = u32::try_from(tag).map_err(|_| syn::Error::new(variant.ident.span(), "proto tag overflowed u32"))?;
    Ok(tag_u32)
}

fn build_variant_default_expr(variant: &VariantInfo<'_>) -> TokenStream2 {
    let ident = variant.ident;
    match &variant.kind {
        VariantKind::Unit => quote! { Self::#ident },
        VariantKind::Tuple { field } => {
            let default_expr = field_proto_default_expr(&field.field);
            quote! { Self::#ident(#default_expr) }
        }
        VariantKind::Struct { fields } => {
            if fields.is_empty() {
                quote! { Self::#ident }
            } else {
                let inits = fields.iter().map(|info| {
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    let expr = field_proto_default_expr(info);
                    quote! { #field_ident: #expr }
                });
                quote! { Self::#ident { #(#inits),* } }
            }
        }
    }
}

fn build_variant_is_default_arm(variant: &VariantInfo<'_>) -> TokenStream2 {
    let ident = variant.ident;
    match &variant.kind {
        VariantKind::Unit => {
            if variant.is_default {
                quote! { Self::#ident => true }
            } else {
                quote! { Self::#ident => false }
            }
        }
        VariantKind::Tuple { field } => {
            if variant.is_default {
                let binding_ident = &field.binding_ident;
                let infos = vec![field.field.clone()];
                let checks = build_is_default_checks(&infos, &TokenStream2::new());
                if checks.is_empty() {
                    quote! { Self::#ident(..) => true }
                } else {
                    quote! {
                        Self::#ident(ref #binding_ident) => {
                            #(#checks;)*
                            true
                        }
                    }
                }
            } else {
                quote! { Self::#ident(..) => false }
            }
        }
        VariantKind::Struct { fields } => {
            if variant.is_default {
                if fields.is_empty() {
                    quote! { Self::#ident { .. } => true }
                } else {
                    let bindings = fields.iter().map(|info| {
                        let field_ident = info.field.ident.as_ref().expect("named field");
                        quote! { #field_ident: ref #field_ident }
                    });
                    let checks = build_is_default_checks(fields, &TokenStream2::new());
                    quote! {
                        Self::#ident { #(#bindings),* } => {
                            #(#checks;)*
                            true
                        }
                    }
                }
            } else {
                quote! { Self::#ident { .. } => false }
            }
        }
    }
}

fn build_variant_encoded_len_arm(variant: &VariantInfo<'_>) -> TokenStream2 {
    let ident = variant.ident;
    let tag = variant.tag;
    match &variant.kind {
        VariantKind::Unit => quote! { Self::#ident => ::proto_rs::encoding::key_len(#tag) + 1 },
        VariantKind::Tuple { field } => {
            let binding_ident = &field.binding_ident;
            let infos = vec![field.field.clone()];
            let terms = build_encoded_len_terms(&infos, &TokenStream2::new());
            let binding_pattern = if field.field.config.skip {
                quote! { .. }
            } else {
                quote! { ref #binding_ident }
            };
            quote! {
                Self::#ident(#binding_pattern) => {
                    let msg_len = 0 #(+ #terms)*;
                    ::proto_rs::encoding::key_len(#tag)
                        + ::proto_rs::encoding::encoded_len_varint(msg_len as u64)
                        + msg_len
                }
            }
        }
        VariantKind::Struct { fields } => {
            if fields.is_empty() {
                quote! {
                    Self::#ident { .. } => ::proto_rs::encoding::key_len(#tag) + 1
                }
            } else {
                let bindings = fields.iter().map(|info| {
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    quote! { #field_ident: ref #field_ident }
                });
                let terms = build_encoded_len_terms(fields, &TokenStream2::new());
                quote! {
                    Self::#ident { #(#bindings),* } => {
                        let msg_len = 0 #(+ #terms)*;
                        ::proto_rs::encoding::key_len(#tag)
                            + ::proto_rs::encoding::encoded_len_varint(msg_len as u64)
                            + msg_len
                    }
                }
            }
        }
    }
}

fn build_variant_encode_arm(variant: &VariantInfo<'_>) -> TokenStream2 {
    let ident = variant.ident;
    let tag = variant.tag;
    match &variant.kind {
        VariantKind::Unit => quote! {
            Self::#ident => {
                ::proto_rs::encoding::encode_key(
                    #tag,
                    ::proto_rs::encoding::WireType::LengthDelimited,
                    buf,
                );
                ::proto_rs::encoding::encode_varint(0, buf);
            }
        },
        VariantKind::Tuple { field } => {
            let binding_ident = &field.binding_ident;
            let infos = vec![field.field.clone()];
            let terms = build_encoded_len_terms(&infos, &TokenStream2::new());
            let encode_stmts = build_encode_stmts(&infos, &TokenStream2::new());
            let binding_pattern = if field.field.config.skip {
                quote! { .. }
            } else {
                quote! { ref #binding_ident }
            };
            quote! {
                Self::#ident(#binding_pattern) => {
                    let msg_len = 0 #(+ #terms)*;
                    ::proto_rs::encoding::encode_key(
                        #tag,
                        ::proto_rs::encoding::WireType::LengthDelimited,
                        buf,
                    );
                    ::proto_rs::encoding::encode_varint(msg_len as u64, buf);
                    #(#encode_stmts)*
                }
            }
        }
        VariantKind::Struct { fields } => {
            if fields.is_empty() {
                quote! {
                    Self::#ident { .. } => {
                        ::proto_rs::encoding::encode_key(
                            #tag,
                            ::proto_rs::encoding::WireType::LengthDelimited,
                            buf,
                        );
                        ::proto_rs::encoding::encode_varint(0, buf);
                    }
                }
            } else {
                let bindings = fields.iter().map(|info| {
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    quote! { #field_ident: ref #field_ident }
                });
                let terms = build_encoded_len_terms(fields, &TokenStream2::new());
                let encode_stmts = build_encode_stmts(fields, &TokenStream2::new());
                quote! {
                    Self::#ident { #(#bindings),* } => {
                        let msg_len = 0 #(+ #terms)*;
                        ::proto_rs::encoding::encode_key(
                            #tag,
                            ::proto_rs::encoding::WireType::LengthDelimited,
                            buf,
                        );
                        ::proto_rs::encoding::encode_varint(msg_len as u64, buf);
                        #(#encode_stmts)*
                    }
                }
            }
        }
    }
}

fn build_variant_merge_arm(name: &Ident, variant: &VariantInfo<'_>) -> TokenStream2 {
    let ident = variant.ident;
    let tag = variant.tag;
    match &variant.kind {
        VariantKind::Unit => {
            quote! {
                #tag => {
                    ::proto_rs::encoding::check_wire_type(
                        ::proto_rs::encoding::WireType::LengthDelimited,
                        wire_type,
                    )?;
                    ctx.limit_reached()?;
                    let len = ::proto_rs::encoding::decode_varint(buf)?;
                    let remaining = buf.remaining();
                    if len > remaining as u64 {
                        return Err(::proto_rs::DecodeError::new("buffer underflow"));
                    }
                    if len != 0 {
                        return Err(::proto_rs::DecodeError::new("expected empty variant payload"));
                    }
                    *value = #name::#ident;
                    Ok(())
                }
            }
        }
        VariantKind::Tuple { field } => {
            let binding_ident = &field.binding_ident;
            let binding_default = field_proto_default_expr(&field.field);
            let mut decode_match = None;
            if let Some(field_tag) = field.field.tag {
                let access = quote! { #binding_ident };
                if needs_decode_conversion(&field.field.config, &field.field.parsed) {
                    let tmp_ident = Ident::new(&format!("__proto_rs_variant_field_{}_tmp", field.field.index), field.field.field.span());
                    let decode_ty = &field.field.decode_ty;
                    let assign = decode_conversion_assign(&field.field, &access, &tmp_ident);
                    decode_match = Some(quote! {
                        #field_tag => {
                            let mut #tmp_ident: #decode_ty = <#decode_ty as ::proto_rs::ProtoWire>::proto_default();
                            <#decode_ty as ::proto_rs::ProtoWire>::decode_into(
                                field_wire_type,
                                &mut #tmp_ident,
                                buf,
                                inner_ctx,
                            )?;
                            #assign
                        }
                    });
                } else {
                    let ty = &field.field.field.ty;
                    decode_match = Some(quote! {
                        #field_tag => {
                            <#ty as ::proto_rs::ProtoWire>::decode_into(
                                field_wire_type,
                                &mut #binding_ident,
                                buf,
                                inner_ctx,
                            )?;
                        }
                    });
                }
            }

            let decode_loop = if let Some(match_arm) = decode_match {
                quote! {
                    while buf.remaining() > limit {
                        let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                        match field_tag {
                            #match_arm
                            _ => ::proto_rs::encoding::skip_field(field_wire_type, field_tag, buf, inner_ctx)?,
                        }
                    }
                }
            } else {
                quote! {
                    while buf.remaining() > limit {
                        let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                        ::proto_rs::encoding::skip_field(field_wire_type, field_tag, buf, inner_ctx)?;
                    }
                }
            };

            let post_hook = if field.field.config.skip {
                if let Some(fun) = &field.field.config.skip_deser_fn {
                    let fun_path = parse_path_string(field.field.field, fun);
                    let skip_binding_ident = Ident::new(&format!("__proto_rs_variant_{}_skip_binding", ident.to_string().to_lowercase()), field.field.field.span());
                    let computed_ident = Ident::new(&format!("__proto_rs_variant_{}_computed", ident.to_string().to_lowercase()), field.field.field.span());
                    quote! {
                        let #computed_ident = #fun_path(&variant_value);
                        if let #name::#ident(ref mut #skip_binding_ident) = variant_value {
                            *#skip_binding_ident = #computed_ident;
                        }
                    }
                } else {
                    quote! {}
                }
            } else {
                quote! {}
            };

            quote! {
                #tag => {
                    ::proto_rs::encoding::check_wire_type(
                        ::proto_rs::encoding::WireType::LengthDelimited,
                        wire_type,
                    )?;
                    ctx.limit_reached()?;
                    let inner_ctx = ctx.enter_recursion();
                    let len = ::proto_rs::encoding::decode_varint(buf)?;
                    let remaining = buf.remaining();
                    if len > remaining as u64 {
                        return Err(::proto_rs::DecodeError::new("buffer underflow"));
                    }
                    let limit = remaining - len as usize;
                    let mut #binding_ident = #binding_default;
                    #decode_loop
                    if buf.remaining() != limit {
                        return Err(::proto_rs::DecodeError::new("delimited length exceeded"));
                    }
                    let mut variant_value = #name::#ident(#binding_ident);
                    #post_hook
                    *value = variant_value;
                    Ok(())
                }
            }
        }
        VariantKind::Struct { fields } => {
            let field_inits = fields.iter().map(|info| {
                let field_ident = info.field.ident.as_ref().expect("named field");
                let init = field_proto_default_expr(info);
                quote! { let mut #field_ident = #init; }
            });
            let decode_match = fields
                .iter()
                .filter_map(|info| {
                    let field_tag = info.tag?;
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    if needs_decode_conversion(&info.config, &info.parsed) {
                        let tmp_ident = Ident::new(&format!("__proto_rs_variant_field_{}_tmp", info.index), info.field.span());
                        let decode_ty = &info.decode_ty;
                        let access = quote! { #field_ident };
                        let assign = decode_conversion_assign(info, &access, &tmp_ident);
                        Some(quote! {
                            #field_tag => {
                                let mut #tmp_ident: #decode_ty = <#decode_ty as ::proto_rs::ProtoWire>::proto_default();
                                <#decode_ty as ::proto_rs::ProtoWire>::decode_into(
                                    field_wire_type,
                                    &mut #tmp_ident,
                                    buf,
                                    inner_ctx,
                                )?;
                                #assign
                            }
                        })
                    } else {
                        let ty = &info.field.ty;
                        Some(quote! {
                            #field_tag => {
                                <#ty as ::proto_rs::ProtoWire>::decode_into(
                                    field_wire_type,
                                    &mut #field_ident,
                                    buf,
                                    inner_ctx,
                                )?;
                            }
                        })
                    }
                })
                .collect::<Vec<_>>();
            let construct_expr = if fields.is_empty() {
                quote! { #name::#ident }
            } else {
                let assigns = fields.iter().map(|info| {
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    quote! { #field_ident }
                });
                quote! { #name::#ident { #(#assigns),* } }
            };
            let decode_loop = if decode_match.is_empty() {
                quote! {
                    while buf.remaining() > limit {
                        let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                        ::proto_rs::encoding::skip_field(field_wire_type, field_tag, buf, inner_ctx)?;
                    }
                }
            } else {
                quote! {
                    while buf.remaining() > limit {
                        let (field_tag, field_wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                        match field_tag {
                            #(#decode_match,)*
                            _ => ::proto_rs::encoding::skip_field(field_wire_type, field_tag, buf, inner_ctx)?,
                        }
                    }
                }
            };
            quote! {
                #tag => {
                    ::proto_rs::encoding::check_wire_type(
                        ::proto_rs::encoding::WireType::LengthDelimited,
                        wire_type,
                    )?;
                    ctx.limit_reached()?;
                    let inner_ctx = ctx.enter_recursion();
                    let len = ::proto_rs::encoding::decode_varint(buf)?;
                    let remaining = buf.remaining();
                    if len > remaining as u64 {
                        return Err(::proto_rs::DecodeError::new("buffer underflow"));
                    }
                    let limit = remaining - len as usize;
                    #(#field_inits)*
                    #decode_loop
                    if buf.remaining() != limit {
                        return Err(::proto_rs::DecodeError::new("delimited length exceeded"));
                    }
                    *value = #construct_expr;
                    Ok(())
                }
            }
        }
    }
}
