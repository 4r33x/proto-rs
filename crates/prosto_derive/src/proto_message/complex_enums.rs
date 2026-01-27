use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;
use syn::Ident;
use syn::ItemEnum;
use syn::Lit;
use syn::parse_quote;
use syn::spanned::Spanned;

use super::build_validate_with_ext_impl;
use super::generic_bounds::add_proto_wire_bounds;
use super::unified_field_handler::FieldAccess;
use super::unified_field_handler::FieldInfo;
use super::unified_field_handler::assign_tags;
use super::unified_field_handler::compute_decode_ty;
use super::unified_field_handler::compute_proto_ty;
use super::unified_field_handler::decode_conversion_assign;
use super::unified_field_handler::encode_conversion_expr;
use super::unified_field_handler::field_proto_default_expr;
use super::unified_field_handler::needs_decode_conversion;
use super::unified_field_handler::needs_encode_conversion;
use super::unified_field_handler::parse_path_string;
use super::unified_field_handler::sanitize_enum;
use crate::parse::UnifiedProtoConfig;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;
use crate::utils::resolved_field_type;

pub(super) fn generate_complex_enum_impl(
    input: &DeriveInput,
    item_enum: &ItemEnum,
    data: &syn::DataEnum,
    config: &UnifiedProtoConfig,
) -> syn::Result<TokenStream2> {
    let enum_item = sanitize_enum(item_enum.clone());

    let name = &input.ident;
    let generics = &input.generics;

    let default_index = crate::utils::find_marked_default_variant(data)?.unwrap_or(0);
    let mut variants = collect_variant_infos(data, config)?;
    if variants.is_empty() {
        return Err(syn::Error::new(
            input.ident.span(),
            "proto_message enum must contain at least one variant",
        ));
    }
    if default_index >= variants.len() {
        return Err(syn::Error::new(input.ident.span(), "#[default] variant index is out of bounds"));
    }
    variants[default_index].is_default = true;

    let bound_fields = collect_variant_fields(&variants);
    let bounded_generics = add_proto_wire_bounds(generics, bound_fields);
    let (impl_generics, ty_generics, where_clause) = bounded_generics.split_for_impl();

    let merge_field_arms = variants.iter().map(|variant| build_variant_merge_arm(name, variant)).collect::<Vec<_>>();
    let default_expr = build_variant_default_expr(&variants[default_index], name);
    let is_default_arms = variants.iter().map(|variant| build_variant_is_default_arm(variant, name)).collect::<Vec<_>>();
    let encode_arms = variants.iter().map(|variant| build_variant_encode_arm(variant, name)).collect::<Vec<_>>();

    let validate_with_ext_impl = build_validate_with_ext_impl(config);
    let validate_with_ext_proto_impl = if config.has_suns() {
        TokenStream2::new()
    } else {
        validate_with_ext_impl.clone()
    };

    let mut shadow_generics = bounded_generics.clone();
    shadow_generics.params.insert(0, parse_quote!('a));
    let (shadow_impl_generics, _shadow_ty_generics, shadow_where_clause) = shadow_generics.split_for_impl();

    let sun_impls = if config.has_suns() {
        let sun_impls = config.suns.iter().map(|sun| {
            let target_ty = &sun.ty;
            quote! {
                impl #impl_generics ::proto_rs::ProtoExt for #target_ty #where_clause {
                    const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
                }

                impl #impl_generics ::proto_rs::ProtoEncode for #target_ty #where_clause {
                    type Shadow<'a> = #name #ty_generics;
                }

                impl #impl_generics ::proto_rs::ProtoDecode for #target_ty #where_clause {
                    type ShadowDecoded = #name #ty_generics;

                    #[inline(always)]
                    fn post_decode(value: Self::ShadowDecoded) -> Result<Self, ::proto_rs::DecodeError> {
                        <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(value)
                    }

                    #validate_with_ext_impl
                }

                impl #impl_generics ::proto_rs::ProtoDefault for #target_ty #where_clause {
                    #[inline(always)]
                    fn proto_default() -> Self {
                        let shadow = <#name #ty_generics as ::proto_rs::ProtoDefault>::proto_default();
                        <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)
                            .expect("failed to build default sun value")
                    }
                }

                impl #impl_generics ::proto_rs::ProtoFieldMerge for #target_ty #where_clause {
                    #[inline(always)]
                    fn merge_value(
                        &mut self,
                        wire_type: ::proto_rs::encoding::WireType,
                        buf: &mut impl ::proto_rs::bytes::Buf,
                        ctx: ::proto_rs::encoding::DecodeContext,
                    ) -> Result<(), ::proto_rs::DecodeError> {
                        let mut shadow = <#name #ty_generics as ::proto_rs::ProtoShadowEncode<'_, #target_ty>>::from_sun(self);
                        <#name #ty_generics as ::proto_rs::ProtoDecoder>::merge(&mut shadow, wire_type, buf, ctx)?;
                        *self = <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)?;
                        Ok(())
                    }
                }

                impl #impl_generics ::proto_rs::ProtoArchive for #target_ty #where_clause {
                    #[inline(always)]
                    fn is_default(&self) -> bool {
                        let shadow = <#name #ty_generics as ::proto_rs::ProtoShadowEncode<'_, #target_ty>>::from_sun(self);
                        <#name #ty_generics as ::proto_rs::ProtoArchive>::is_default(&shadow)
                    }

                    #[inline(always)]
                    fn archive<const TAG: u32>(&self, w: &mut impl ::proto_rs::RevWriter) {
                        let shadow = <#name #ty_generics as ::proto_rs::ProtoShadowEncode<'_, #target_ty>>::from_sun(self);
                        <#name #ty_generics as ::proto_rs::ProtoArchive>::archive::<TAG>(&shadow, w)
                    }
                }
            }
        });
        quote! { #( #sun_impls )* }
    } else {
        quote! {}
    };

    Ok(quote! {
        #enum_item

        impl #impl_generics ::proto_rs::ProtoExt for #name #ty_generics #where_clause {
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
        }

        impl #impl_generics ::proto_rs::ProtoDecoder for #name #ty_generics #where_clause {
            #[inline(always)]
            fn merge_field(
                value: &mut Self,
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

        impl #impl_generics ::proto_rs::ProtoDefault for #name #ty_generics #where_clause {
            #[inline(always)]
            fn proto_default() -> Self {
                #default_expr
            }
        }

        impl #impl_generics ::proto_rs::ProtoDecode for #name #ty_generics #where_clause {
            type ShadowDecoded = Self;
            #validate_with_ext_proto_impl
        }

        impl #impl_generics ::proto_rs::ProtoShadowDecode<#name #ty_generics> for #name #ty_generics #where_clause {
            #[inline(always)]
            fn to_sun(self) -> Result<#name #ty_generics, ::proto_rs::DecodeError> {
                Ok(self)
            }
        }

        impl #shadow_impl_generics ::proto_rs::ProtoShadowEncode<'a, #name #ty_generics> for &'a #name #ty_generics #shadow_where_clause {
            #[inline(always)]
            fn from_sun(value: &'a #name #ty_generics) -> Self {
                value
            }
        }

        impl #shadow_impl_generics ::proto_rs::ProtoArchive for &'a #name #ty_generics #shadow_where_clause {
            #[inline(always)]
            fn is_default(&self) -> bool {
                match *self {
                    #(#is_default_arms,)*
                }
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl ::proto_rs::RevWriter) {
                let mark = w.mark();
                match *self {
                    #(#encode_arms,)*
                }
                if TAG != 0 {
                    let payload_len = w.written_since(mark);
                    w.put_varint(payload_len as u64);
                    ::proto_rs::ArchivedProtoField::<TAG, Self>::put_key(w);
                }
            }
        }

        impl #impl_generics ::proto_rs::ProtoArchive for #name #ty_generics #where_clause {
            #[inline(always)]
            fn is_default(&self) -> bool {
                <&Self as ::proto_rs::ProtoArchive>::is_default(&self)
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl ::proto_rs::RevWriter) {
                <&Self as ::proto_rs::ProtoArchive>::archive::<TAG>(&self, w)
            }
        }

        impl #impl_generics ::proto_rs::ProtoEncode for #name #ty_generics #where_clause {
            type Shadow<'a> = &'a #name #ty_generics;
        }

        #sun_impls
    })
}

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
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

fn collect_variant_infos<'a>(data: &'a syn::DataEnum, _config: &'a UnifiedProtoConfig) -> syn::Result<Vec<VariantInfo<'a>>> {
    let mut used_tags = BTreeSet::new();
    let mut variants = Vec::new();

    for (idx, variant) in data.variants.iter().enumerate() {
        let tag = resolve_variant_tag(variant, idx + 1)?;
        if !used_tags.insert(tag) {
            return Err(syn::Error::new(
                variant.ident.span(),
                format!("duplicate proto(tag) attribute for enum variant: {tag}"),
            ));
        }

        let kind = match &variant.fields {
            syn::Fields::Unit => VariantKind::Unit,
            syn::Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    return Err(syn::Error::new(
                        variant.ident.span(),
                        "complex enum tuple variants must contain exactly one field",
                    ));
                }

                let field = &fields.unnamed[0];
                let config = parse_field_config(field);
                let effective_ty = resolved_field_type(field, &config);
                let parsed = parse_field_type(&effective_ty);
                let proto_ty = compute_proto_ty(field, &config, &parsed, &effective_ty);
                let decode_ty = compute_decode_ty(field, &config, &parsed, &proto_ty);
                let binding_ident = Ident::new(
                    &format!("__proto_rs_variant_{}_value", variant.ident.to_string().to_lowercase()),
                    field.span(),
                );

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
                    field: TupleVariantInfo {
                        field: field_info,
                        binding_ident,
                    },
                }
            }
            syn::Fields::Named(fields_named) => {
                let mut infos: Vec<_> = fields_named
                    .named
                    .iter()
                    .enumerate()
                    .map(|(field_idx, field)| {
                        let config = parse_field_config(field);
                        let effective_ty = resolved_field_type(field, &config);
                        let parsed = parse_field_type(&effective_ty);
                        let proto_ty = compute_proto_ty(field, &config, &parsed, &effective_ty);
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

fn collect_variant_fields<'a>(variants: &'a [VariantInfo<'a>]) -> Vec<&'a FieldInfo<'a>> {
    let mut fields = Vec::new();
    for variant in variants {
        match &variant.kind {
            VariantKind::Unit => {}
            VariantKind::Tuple { field } => {
                fields.push(&field.field);
            }
            VariantKind::Struct { fields: struct_fields } => {
                fields.extend(struct_fields.iter());
            }
        }
    }
    fields
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
                    Lit::Str(str_lit) => str_lit
                        .value()
                        .parse::<usize>()
                        .map_err(|_| syn::Error::new(str_lit.span(), "proto tag must be a positive integer"))?,
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
        return Err(syn::Error::new(
            variant.ident.span(),
            "proto enum variant tags must be greater than or equal to 1",
        ));
    }

    let tag_u32 = u32::try_from(tag).map_err(|_| syn::Error::new(variant.ident.span(), "proto tag overflowed u32"))?;
    Ok(tag_u32)
}

// Helper: Generate encoding body for empty variants (Unit or empty Struct)
fn build_empty_variant_encode_body(tag: u32) -> TokenStream2 {
    quote! {
        w.put_varint(0);
        ::proto_rs::ArchivedProtoField::<#tag, ()>::put_key(w);
    }
}

// Helper: Generate binding patterns for struct fields (handles skip attributes)
fn build_struct_field_bindings<'a>(fields: &'a [FieldInfo<'a>]) -> impl Iterator<Item = TokenStream2> + 'a {
    fields.iter().map(|info| {
        let field_ident = info.field.ident.as_ref().expect("named field");
        if info.config.skip {
            quote! { #field_ident: _ }
        } else {
            quote! { #field_ident }
        }
    })
}

fn build_variant_default_expr(variant: &VariantInfo<'_>, enum_ident: &Ident) -> TokenStream2 {
    let ident = variant.ident;
    match &variant.kind {
        VariantKind::Unit => quote! { #enum_ident::#ident },
        VariantKind::Tuple { field } => {
            let default_expr = field_proto_default_expr(&field.field);
            quote! { #enum_ident::#ident(#default_expr) }
        }
        VariantKind::Struct { fields } => {
            if fields.is_empty() {
                quote! { #enum_ident::#ident }
            } else {
                let inits = fields.iter().map(|info| {
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    let expr = field_proto_default_expr(info);
                    quote! { #field_ident: #expr }
                });
                quote! { #enum_ident::#ident { #(#inits),* } }
            }
        }
    }
}

fn build_variant_is_default_arm(variant: &VariantInfo<'_>, enum_ident: &Ident) -> TokenStream2 {
    let ident = variant.ident;
    match &variant.kind {
        VariantKind::Unit => {
            if variant.is_default {
                quote! { #enum_ident::#ident => true }
            } else {
                quote! { #enum_ident::#ident => false }
            }
        }
        VariantKind::Tuple { field } => {
            if !variant.is_default {
                return quote! { #enum_ident::#ident(..) => false };
            }
            if field.field.config.skip {
                quote! { #enum_ident::#ident(..) => true }
            } else {
                let binding_ident = &field.binding_ident;
                let field_ty = &field.field.field.ty;
                let ref_expr = quote! { &#binding_ident };
                let check_expr = if needs_encode_conversion(&field.field.config, &field.field.parsed) {
                    let converted = encode_conversion_expr(&field.field, &ref_expr);
                    quote! { ::proto_rs::ProtoArchive::is_default(&#converted) }
                } else {
                    let shadow_ty = quote! { <#field_ty as ::proto_rs::ProtoEncode>::Shadow<'_> };
                    quote! {
                        {
                            let shadow = <#shadow_ty as ::proto_rs::ProtoShadowEncode<'_, #field_ty>>::from_sun(#ref_expr);
                            ::proto_rs::ProtoArchive::is_default(&shadow)
                        }
                    }
                };
                quote! {
                    #enum_ident::#ident(#binding_ident) => { #check_expr }
                }
            }
        }
        VariantKind::Struct { fields } => {
            if !variant.is_default {
                return quote! { #enum_ident::#ident { .. } => false };
            }
            if fields.is_empty() {
                quote! { #enum_ident::#ident { .. } => true }
            } else {
                let bindings = build_struct_field_bindings(fields);
                let checks = fields.iter().filter_map(|info| {
                    if info.config.skip {
                        return None;
                    }
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    let field_ty = &info.field.ty;
                    let ref_expr = quote! { &#field_ident };
                    let check_expr = if needs_encode_conversion(&info.config, &info.parsed) {
                        let converted = encode_conversion_expr(info, &ref_expr);
                        quote! { ::proto_rs::ProtoArchive::is_default(&#converted) }
                    } else {
                        let shadow_ty = quote! { <#field_ty as ::proto_rs::ProtoEncode>::Shadow<'_> };
                        quote! {
                            {
                                let shadow = <#shadow_ty as ::proto_rs::ProtoShadowEncode<'_, #field_ty>>::from_sun(#ref_expr);
                                ::proto_rs::ProtoArchive::is_default(&shadow)
                            }
                        }
                    };
                    Some(check_expr)
                });
                quote! {
                    #enum_ident::#ident { #(#bindings),* } => {
                        true #(&& #checks)*
                    }
                }
            }
        }
    }
}

fn build_variant_encode_arm(variant: &VariantInfo<'_>, enum_ident: &Ident) -> TokenStream2 {
    let ident = variant.ident;
    let tag = variant.tag;
    match &variant.kind {
        VariantKind::Unit => {
            let encode_body = build_empty_variant_encode_body(tag);
            quote! {
                #enum_ident::#ident => {
                    #encode_body
                }
            }
        }
        VariantKind::Tuple { field } => {
            let binding_ident = &field.binding_ident;
            if field.field.config.skip {
                return quote! {
                    #enum_ident::#ident(..) => {}
                };
            }
            let field_ty = &field.field.field.ty;
            let ref_expr = quote! { &#binding_ident };
            let shadow_ty = if needs_encode_conversion(&field.field.config, &field.field.parsed) {
                let proto_ty = &field.field.proto_ty;
                quote! { #proto_ty }
            } else {
                quote! { <#field_ty as ::proto_rs::ProtoEncode>::Shadow<'_> }
            };
            let shadow_expr = if needs_encode_conversion(&field.field.config, &field.field.parsed) {
                encode_conversion_expr(&field.field, &ref_expr)
            } else {
                quote! { <#shadow_ty as ::proto_rs::ProtoShadowEncode<'_, #field_ty>>::from_sun(#ref_expr) }
            };
            quote! {
                #enum_ident::#ident(#binding_ident) => {
                    let __proto_rs_shadow = #shadow_expr;
                    // Use new_always to preserve variant selection even when payload is default
                    ::proto_rs::ArchivedProtoField::<#tag, #shadow_ty>::new_always(&__proto_rs_shadow, w);
                }
            }
        }
        VariantKind::Struct { fields } => {
            let bindings = build_struct_field_bindings(fields);
            let field_encodes = fields.iter().rev().filter_map(|info| {
                if info.config.skip {
                    return None;
                }
                let field_ident = info.field.ident.as_ref().expect("named field");
                let field_tag = info.tag.expect("tag required");
                let field_ty = &info.field.ty;
                let ref_expr = quote! { &#field_ident };
                let shadow_ty = if needs_encode_conversion(&info.config, &info.parsed) {
                    let proto_ty = &info.proto_ty;
                    quote! { #proto_ty }
                } else {
                    quote! { <#field_ty as ::proto_rs::ProtoEncode>::Shadow<'_> }
                };
                let shadow_expr = if needs_encode_conversion(&info.config, &info.parsed) {
                    encode_conversion_expr(info, &ref_expr)
                } else {
                    quote! { <#shadow_ty as ::proto_rs::ProtoShadowEncode<'_, #field_ty>>::from_sun(#ref_expr) }
                };
                let shadow_ident = syn::Ident::new(
                    &format!("__proto_rs_variant_{}_shadow_{}", ident.to_string().to_lowercase(), info.index),
                    info.field.span(),
                );
                Some(quote! {
                    let #shadow_ident = #shadow_expr;
                    ::proto_rs::ArchivedProtoField::<#field_tag, #shadow_ty>::archive(&#shadow_ident, w);
                })
            });
            quote! {
                #enum_ident::#ident { #(#bindings),* } => {
                    let mark = w.mark();
                    #(#field_encodes)*
                    let payload_len = w.written_since(mark);
                    w.put_varint(payload_len as u64);
                    ::proto_rs::ArchivedProtoField::<#tag, Self>::put_key(w);
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
                    // No need to check limit_reached() for unit variants - no recursion happens
                    let len = ::proto_rs::encoding::decode_varint(buf)?;
                    if len > buf.remaining() as u64 {
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
            let decode_stmt = if field.field.config.skip {
                quote! {
                    ::proto_rs::encoding::skip_field(wire_type, #tag, buf, ctx)?;
                }
            } else if needs_decode_conversion(&field.field.config, &field.field.parsed) {
                let tmp_ident = Ident::new(
                    &format!("__proto_rs_variant_field_{}_tmp", field.field.index),
                    field.field.field.span(),
                );
                let decode_ty = &field.field.decode_ty;
                let assign = decode_conversion_assign(&field.field, &quote! { #binding_ident }, &tmp_ident);
                quote! {
                    let mut #tmp_ident: #decode_ty = <#decode_ty as ::proto_rs::ProtoDefault>::proto_default();
                    <#decode_ty as ::proto_rs::ProtoFieldMerge>::merge_value(&mut #tmp_ident, wire_type, buf, ctx)?;
                    #assign
                }
            } else {
                let ty = &field.field.field.ty;
                quote! {
                    <#ty as ::proto_rs::ProtoFieldMerge>::merge_value(&mut #binding_ident, wire_type, buf, ctx)?;
                }
            };

            let post_hook = if field.field.config.skip {
                field.field.config.skip_deser_fn.as_ref().map(|fun| {
                    let fun_path = parse_path_string(field.field.field, fun);
                    let skip_binding_ident = Ident::new(
                        &format!("__proto_rs_variant_{}_skip_binding", ident.to_string().to_lowercase()),
                        field.field.field.span(),
                    );
                    let computed_ident = Ident::new(
                        &format!("__proto_rs_variant_{}_computed", ident.to_string().to_lowercase()),
                        field.field.field.span(),
                    );
                    quote! {
                        let #computed_ident = #fun_path(value);
                        if let #name::#ident(#skip_binding_ident) = value {
                            *#skip_binding_ident = #computed_ident;
                        }
                    }
                })
            } else {
                None
            };

            let assign_variant = if let Some(post_hook) = post_hook {
                quote! {
                    *value = #name::#ident(#binding_ident);
                    #post_hook
                }
            } else {
                quote! { *value = #name::#ident(#binding_ident); }
            };

            quote! {
                #tag => {
                    let mut #binding_ident = #binding_default;
                    #decode_stmt
                    #assign_variant
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
                                let mut #tmp_ident: #decode_ty = <#decode_ty as ::proto_rs::ProtoDefault>::proto_default();
                                <#decode_ty as ::proto_rs::ProtoFieldMerge>::merge_value(&mut #tmp_ident, field_wire_type, buf, inner_ctx)?;
                                #assign
                            }
                        })
                    } else {
                        let ty = &info.field.ty;
                        Some(quote! {
                            #field_tag => {
                                <#ty as ::proto_rs::ProtoFieldMerge>::merge_value(&mut #field_ident, field_wire_type, buf, inner_ctx)?;
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
            let post_hooks = fields
                .iter()
                .filter_map(|info| {
                    if !info.config.skip {
                        return None;
                    }
                    let fun = info.config.skip_deser_fn.as_ref()?;
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    let fun_path = parse_path_string(info.field, fun);
                    let skip_binding_ident = Ident::new(
                        &format!(
                            "__proto_rs_variant_{}_{}_skip_binding",
                            ident.to_string().to_lowercase(),
                            info.index
                        ),
                        info.field.span(),
                    );
                    let computed_ident = Ident::new(
                        &format!("__proto_rs_variant_{}_{}_computed", ident.to_string().to_lowercase(), info.index),
                        info.field.span(),
                    );
                    Some(quote! {
                        let #computed_ident = #fun_path(value);
                        if let #name::#ident { #field_ident: #skip_binding_ident, .. } = value {
                            *#skip_binding_ident = #computed_ident;
                        }
                    })
                })
                .collect::<Vec<_>>();
            let assign_variant = if post_hooks.is_empty() {
                quote! { *value = #construct_expr; }
            } else {
                quote! {
                    *value = #construct_expr;
                    #(#post_hooks)*
                }
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
                    // Check limit once at recursion boundary, then use entered context for fields
                    ctx.limit_reached()?;
                    let inner_ctx = ctx.enter_recursion();
                    let len = ::proto_rs::encoding::decode_varint(buf)?;
                    if len > buf.remaining() as u64 {
                        return Err(::proto_rs::DecodeError::new("buffer underflow"));
                    }
                    let limit = buf.remaining() - len as usize;
                    #(#field_inits)*
                    #decode_loop
                    #assign_variant
                    Ok(())
                }
            }
        }
    }
}
