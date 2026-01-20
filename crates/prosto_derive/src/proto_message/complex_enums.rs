use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;
use syn::GenericParam;
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
use super::unified_field_handler::field_proto_default_expr;
use super::unified_field_handler::needs_encode_conversion;
use super::unified_field_handler::needs_decode_conversion;
use super::unified_field_handler::parse_path_string;
use super::unified_field_handler::sanitize_enum;
use super::unified_field_handler::encode_conversion_expr;
use crate::parse::UnifiedProtoConfig;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;
use crate::utils::resolved_field_type;
use crate::utils::find_marked_default_variant;

pub(super) fn generate_complex_enum_impl(
    input: &DeriveInput,
    item_enum: &ItemEnum,
    data: &syn::DataEnum,
    config: &UnifiedProtoConfig,
) -> syn::Result<TokenStream2> {
    let enum_item = sanitize_enum(item_enum.clone());

    let name = &input.ident;
    let generics = &input.generics;

    let default_index = find_marked_default_variant(data)?.unwrap_or(0);
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
    let bounded_generics = add_proto_wire_bounds(generics, bound_fields.iter().copied());
    let (impl_generics, ty_generics, where_clause) = bounded_generics.split_for_impl();

    let merge_field_arms = variants.iter().map(|variant| build_variant_merge_arm(name, variant)).collect::<Vec<_>>();

    let default_expr = build_variant_default_expr(&variants[default_index]);

    let proto_ext_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoExt for #name #ty_generics #where_clause {
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
        }
    };

    let validate_with_ext_impl = build_validate_with_ext_impl(config);

    let proto_decoder_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoDecoder for #name #ty_generics #where_clause {
            #[inline(always)]
            fn proto_default() -> Self {
                #default_expr
            }

            #[inline(always)]
            fn clear(&mut self) {
                *self = Self::proto_default();
            }

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
    };

    let proto_shadow_decode_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoShadowDecode<Self> for #name #ty_generics #where_clause {
            #[inline(always)]
            fn to_sun(self) -> Result<Self, ::proto_rs::DecodeError> {
                Ok(self)
            }
        }
    };

    let message_validation = if let Some(validator_fn) = &config.validator {
        let validator_path: syn::Path = syn::parse_str(validator_fn).expect("invalid validator function path");
        quote! {
            #validator_path(&mut value)?;
        }
    } else {
        quote! {}
    };

    let post_decode_impl = if config.validator.is_none() {
        quote! {}
    } else {
        quote! {
            #[inline(always)]
            fn post_decode(mut value: Self::ShadowDecoded) -> Result<Self, ::proto_rs::DecodeError> {
                #message_validation
                Ok(value)
            }
        }
    };

    let proto_decode_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoDecode for #name #ty_generics #where_clause {
            type ShadowDecoded = Self;

            #post_decode_impl
            #validate_with_ext_impl
        }
    };

    let mut shadow_encode_generics = bounded_generics.clone();
    shadow_encode_generics.params.insert(0, parse_quote!('__proto_a));
    let (shadow_encode_impl_generics, shadow_encode_ty_generics, shadow_encode_where_clause) = shadow_encode_generics.split_for_impl();
    let proto_shadow_encode_impl = quote! {
        impl #shadow_encode_impl_generics ::proto_rs::ProtoShadowEncode<'__proto_a, #name #ty_generics>
            for &'__proto_a #name #ty_generics #where_clause
        {
            #[inline(always)]
            fn from_sun(value: &'__proto_a #name #ty_generics) -> Self {
                value
            }
        }
    };

    let archived_enum_ident = Ident::new(&format!("__proto_rs_{}Archived", name), name.span());
    let archived_type_args = build_archived_type_args(&bounded_generics, quote! { '__proto_x });
    let archived_type_args_with_a = build_archived_type_args(&bounded_generics, quote! { '__proto_a });
    let mut archived_struct_defs = Vec::new();
    let mut archived_enum_variants = Vec::new();
    let mut archive_match_arms = Vec::new();
    let mut len_match_arms = Vec::new();
    let mut encode_match_arms = Vec::new();
    let mut is_default_match_arms = Vec::new();

    for variant in &variants {
        let ident = variant.ident;
        let tag = variant.tag;
        match &variant.kind {
            VariantKind::Unit => {
                archived_enum_variants.push(quote! { #ident });
                archive_match_arms.push(quote! { #name::#ident => #archived_enum_ident::#ident });
                len_match_arms.push(quote! { #archived_enum_ident::#ident => ::proto_rs::encoding::key_len(#tag) + 1 });
                encode_match_arms.push(quote! {
                    #archived_enum_ident::#ident => {
                        ::proto_rs::encoding::encode_key(
                            #tag,
                            ::proto_rs::encoding::WireType::LengthDelimited,
                            buf,
                        );
                        ::proto_rs::encoding::encode_varint(0, buf);
                    }
                });
                is_default_match_arms.push(build_variant_is_default_arm(name, variant));
            }
            VariantKind::Tuple { field } => {
                if field.field.config.skip {
                    archived_enum_variants.push(quote! { #ident });
                    archive_match_arms.push(quote! { #name::#ident(..) => #archived_enum_ident::#ident });
                    len_match_arms.push(quote! { #archived_enum_ident::#ident => ::proto_rs::encoding::key_len(#tag) + 1 });
                    encode_match_arms.push(quote! {
                        #archived_enum_ident::#ident => {
                            ::proto_rs::encoding::encode_key(
                                #tag,
                                ::proto_rs::encoding::WireType::LengthDelimited,
                                buf,
                            );
                            ::proto_rs::encoding::encode_varint(0, buf);
                        }
                    });
                } else {
                    let proto_ty = &field.field.proto_ty;
                    let binding_ident = &field.binding_ident;
                    let binding_pattern = quote! { #binding_ident };
                    let access = quote! { #binding_ident };
                    archived_enum_variants.push(quote! {
                        #ident(::proto_rs::ArchivedProtoInner<'__proto_a, #tag, #proto_ty>)
                    });
                    if needs_encode_conversion(&field.field.config, &field.field.parsed) {
                        let converted_ident = Ident::new(
                            &format!("__proto_rs_variant_{}_converted", ident.to_string().to_lowercase()),
                            field.field.field.span(),
                        );
                        let converted = encode_conversion_expr(&field.field, &access);
                        archive_match_arms.push(quote! {
                            #name::#ident(#binding_pattern) => {
                                let #converted_ident: #proto_ty = #converted;
                                let archived = ::proto_rs::ArchivedProtoInner::<#tag, #proto_ty>::new(&#converted_ident);
                                #archived_enum_ident::#ident(archived)
                            }
                        });
                    } else {
                        archive_match_arms.push(quote! {
                            #name::#ident(#binding_pattern) => {
                                let archived = ::proto_rs::ArchivedProtoInner::<#tag, #proto_ty>::new(#binding_ident);
                                #archived_enum_ident::#ident(archived)
                            }
                        });
                    }
                    len_match_arms.push(quote! { #archived_enum_ident::#ident(archived) => archived.len() });
                    encode_match_arms.push(quote! { #archived_enum_ident::#ident(archived) => archived.encode(buf) });
                }
                is_default_match_arms.push(build_variant_is_default_arm(name, variant));
            }
            VariantKind::Struct { fields } => {
                if fields.is_empty() {
                    archived_enum_variants.push(quote! { #ident });
                    archive_match_arms.push(quote! { #name::#ident { .. } => #archived_enum_ident::#ident });
                    len_match_arms.push(quote! { #archived_enum_ident::#ident => ::proto_rs::encoding::key_len(#tag) + 1 });
                    encode_match_arms.push(quote! {
                        #archived_enum_ident::#ident => {
                            ::proto_rs::encoding::encode_key(
                                #tag,
                                ::proto_rs::encoding::WireType::LengthDelimited,
                                buf,
                            );
                            ::proto_rs::encoding::encode_varint(0, buf);
                        }
                    });
                } else {
                    let archived_struct_name = Ident::new(
                        &format!("__proto_rs_{}{}Archived", name, ident),
                        ident.span(),
                    );
                    let archived_struct_fields: Vec<_> = fields
                        .iter()
                        .filter_map(|info| {
                            let tag = info.tag?;
                            let field_name = Ident::new(&format!("f{tag}"), info.field.span());
                            let proto_ty = &info.proto_ty;
                            Some(quote! {
                                #field_name: ::proto_rs::ArchivedProtoInner<'__proto_a, #tag, #proto_ty>
                            })
                        })
                        .collect();
                    archived_struct_defs.push(quote! {
                        #[allow(non_camel_case_types)]
                        struct #archived_struct_name #shadow_encode_ty_generics #shadow_encode_where_clause {
                            #(#archived_struct_fields),*
                        }
                    });
                    archived_enum_variants.push(quote! { #ident(#archived_struct_name #archived_type_args_with_a) });

                    let bindings = build_struct_field_bindings(fields);
                    let archive_fields: Vec<_> = fields
                        .iter()
                        .filter_map(|info| {
                            let tag = info.tag?;
                            let field_ident = info.field.ident.as_ref().expect("named field");
                            let field_name = Ident::new(&format!("f{tag}"), info.field.span());
                            let access = quote! { #field_ident };
                            let proto_ty = &info.proto_ty;
                            if needs_encode_conversion(&info.config, &info.parsed) {
                                let converted_ident = Ident::new(
                                    &format!("__proto_rs_{}_{}_converted", ident.to_string().to_lowercase(), info.index),
                                    info.field.span(),
                                );
                                let converted = encode_conversion_expr(info, &access);
                                Some(quote! {
                                    let #converted_ident: #proto_ty = #converted;
                                    let #field_name = ::proto_rs::ArchivedProtoInner::<#tag, #proto_ty>::new(&#converted_ident);
                                })
                            } else {
                                Some(quote! {
                                    let #field_name = ::proto_rs::ArchivedProtoInner::<#tag, #proto_ty>::new(#field_ident);
                                })
                            }
                        })
                        .collect();

                    let archived_inits: Vec<_> = fields
                        .iter()
                        .filter_map(|info| info.tag.map(|tag| Ident::new(&format!("f{tag}"), info.field.span())))
                        .map(|name| quote! { #name })
                        .collect();

                    archive_match_arms.push(quote! {
                        #name::#ident { #(#bindings),* } => {
                            #(#archive_fields)*
                            #archived_enum_ident::#ident(#archived_struct_name { #(#archived_inits),* })
                        }
                    });

                    let msg_len_terms: Vec<_> = fields
                        .iter()
                        .filter_map(|info| info.tag.map(|tag| Ident::new(&format!("f{tag}"), info.field.span())))
                        .map(|name| quote! { archived.#name.len() })
                        .collect();
                    let msg_len_expr = if msg_len_terms.is_empty() {
                        quote! { 0 }
                    } else {
                        quote! { 0 #(+ #msg_len_terms)* }
                    };
                    len_match_arms.push(quote! {
                        #archived_enum_ident::#ident(archived) => {
                            let msg_len = #msg_len_expr;
                            ::proto_rs::encoding::key_len(#tag)
                                + ::proto_rs::encoding::encoded_len_varint(msg_len as u64)
                                + msg_len
                        }
                    });

                    let encode_fields: Vec<_> = fields
                        .iter()
                        .filter_map(|info| info.tag.map(|tag| Ident::new(&format!("f{tag}"), info.field.span())))
                        .map(|name| quote! { archived.#name.encode(buf); })
                        .collect();
                    encode_match_arms.push(quote! {
                        #archived_enum_ident::#ident(archived) => {
                            let msg_len = #msg_len_expr;
                            ::proto_rs::encoding::encode_key(
                                #tag,
                                ::proto_rs::encoding::WireType::LengthDelimited,
                                buf,
                            );
                            ::proto_rs::encoding::encode_varint(msg_len as u64, buf);
                            #(#encode_fields)*
                        }
                    });
                }
                is_default_match_arms.push(build_variant_is_default_arm(name, variant));
            }
        }
    }

    let archived_enum_def = quote! {
        #[allow(non_camel_case_types)]
        enum #archived_enum_ident #shadow_encode_ty_generics #shadow_encode_where_clause {
            #(#archived_enum_variants),*
        }
    };

    let proto_archive_impl = quote! {
        #(#archived_struct_defs)*
        #archived_enum_def

        impl #shadow_encode_impl_generics ::proto_rs::ProtoArchive for &'__proto_a #name #ty_generics #where_clause {
            type Archived<'__proto_x> = #archived_enum_ident #archived_type_args;

            #[inline(always)]
            fn is_default(&self) -> bool {
                match self {
                    #(#is_default_match_arms,)*
                }
            }

            #[inline(always)]
            fn len(archived: &Self::Archived<'_>) -> usize {
                match archived {
                    #(#len_match_arms,)*
                }
            }

            #[inline(always)]
            unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                match archived {
                    #(#encode_match_arms,)*
                }
            }

            #[inline(always)]
            fn archive(&self) -> Self::Archived<'_> {
                match self {
                    #(#archive_match_arms,)*
                }
            }
        }
    };

    let proto_archive_value_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoArchive for #name #ty_generics #where_clause {
            type Archived<'__proto_a> = <&'__proto_a #name #ty_generics as ::proto_rs::ProtoArchive>::Archived<'__proto_a>;

            #[inline(always)]
            fn is_default(&self) -> bool {
                <&Self as ::proto_rs::ProtoArchive>::is_default(&self)
            }

            #[inline(always)]
            fn len(archived: &Self::Archived<'_>) -> usize {
                <&Self as ::proto_rs::ProtoArchive>::len(archived)
            }

            #[inline(always)]
            unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                <&Self as ::proto_rs::ProtoArchive>::encode(archived, buf)
            }

            #[inline(always)]
            fn archive(&self) -> Self::Archived<'_> {
                match self {
                    #(#archive_match_arms,)*
                }
            }
        }
    };
    let proto_encode_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoEncode for #name #ty_generics #where_clause {
            type Shadow<'__proto_a> = &'__proto_a Self;
        }
    };

    Ok(quote! {
        #enum_item
        #proto_ext_impl
        #proto_decoder_impl
        #proto_shadow_decode_impl
        #proto_decode_impl
        #proto_shadow_encode_impl
        #proto_archive_impl
        #proto_archive_value_impl
        #proto_encode_impl
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

fn build_variant_is_default_arm(enum_ident: &Ident, variant: &VariantInfo<'_>) -> TokenStream2 {
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
            if variant.is_default {
                if field.field.config.skip {
                    quote! { #enum_ident::#ident(..) => true }
                } else {
                    let binding_ident = &field.binding_ident;
                    let access = quote! { #binding_ident };
                    let ty = &field.field.proto_ty;
                    if needs_encode_conversion(&field.field.config, &field.field.parsed) {
                        let converted_ident = Ident::new(
                            &format!("__proto_rs_variant_{}_default", ident.to_string().to_lowercase()),
                            field.field.field.span(),
                        );
                        let converted = encode_conversion_expr(&field.field, &access);
                        quote! {
                            #enum_ident::#ident(#binding_ident) => {
                                let #converted_ident: #ty = #converted;
                                <#ty as ::proto_rs::ProtoArchive>::is_default(&#converted_ident)
                            }
                        }
                    } else {
                        quote! {
                            #enum_ident::#ident(#binding_ident) => {
                                <#ty as ::proto_rs::ProtoArchive>::is_default(#binding_ident)
                            }
                        }
                    }
                }
            } else {
                quote! { #enum_ident::#ident(..) => false }
            }
        }
        VariantKind::Struct { fields } => {
            if variant.is_default {
                if fields.is_empty() {
                    quote! { #enum_ident::#ident { .. } => true }
                } else {
                    let bindings = build_struct_field_bindings(fields);

                    let checks = fields.iter().filter_map(|info| {
                        let ty = &info.proto_ty;
                        let tag = info.tag?;
                        let field_ident = info.field.ident.as_ref().expect("named field");
                        let access = quote! { #field_ident };
                        if needs_encode_conversion(&info.config, &info.parsed) {
                            let converted_ident = Ident::new(
                                &format!("__proto_rs_variant_{}_{}_default", ident.to_string().to_lowercase(), info.index),
                                info.field.span(),
                            );
                            let converted = encode_conversion_expr(info, &access);
                            Some(quote! {
                                {
                                    let _ = #tag;
                                    let #converted_ident: #ty = #converted;
                                    if !<#ty as ::proto_rs::ProtoArchive>::is_default(&#converted_ident) {
                                        return false;
                                    }
                                }
                            })
                        } else {
                            Some(quote! {
                                {
                                    let _ = #tag;
                                    if !<#ty as ::proto_rs::ProtoArchive>::is_default(#field_ident) {
                                        return false;
                                    }
                                }
                            })
                        }
                    });

                    quote! {
                        #enum_ident::#ident { #(#bindings),* } => {
                            #(#checks;)*
                            true
                        }
                    }
                }
            } else {
                quote! { #enum_ident::#ident { .. } => false }
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
                    let mut #tmp_ident: #decode_ty = <#decode_ty as ::proto_rs::ProtoDecoder>::proto_default();
                    <#decode_ty as ::proto_rs::ProtoDecoder>::merge(
                        &mut #tmp_ident,
                        wire_type,
                        buf,
                        ctx,
                    )?;
                    #assign
                }
            } else {
                let ty = &field.field.field.ty;
                quote! {
                    <#ty as ::proto_rs::ProtoDecoder>::merge(
                        &mut #binding_ident,
                        wire_type,
                        buf,
                        ctx,
                    )?;
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
                    let validation = if let Some(validator_fn) = &info.config.validator {
                        let validator_path = parse_path_string(info.field, validator_fn);
                        quote! {
                            #validator_path(&mut #field_ident)?;
                        }
                    } else {
                        quote! {}
                    };
                    if needs_decode_conversion(&info.config, &info.parsed) {
                        let tmp_ident = Ident::new(&format!("__proto_rs_variant_field_{}_tmp", info.index), info.field.span());
                        let decode_ty = &info.decode_ty;
                        let access = quote! { #field_ident };
                        let assign = decode_conversion_assign(info, &access, &tmp_ident);
                        Some(quote! {
                            #field_tag => {
                                let mut #tmp_ident: #decode_ty = <#decode_ty as ::proto_rs::ProtoDecoder>::proto_default();
                                <#decode_ty as ::proto_rs::ProtoDecoder>::merge(
                                    &mut #tmp_ident,
                                    field_wire_type,
                                    buf,
                                    inner_ctx,
                                )?;
                                #assign
                                #validation
                            }
                        })
                    } else {
                        let ty = &info.field.ty;
                        Some(quote! {
                            #field_tag => {
                                <#ty as ::proto_rs::ProtoDecoder>::merge(
                                    &mut #field_ident,
                                    field_wire_type,
                                    buf,
                                    inner_ctx,
                                )?;
                                #validation
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
                    #assign_variant
                    Ok(())
                }
            }
        }
    }
}

fn build_archived_type_args(generics: &syn::Generics, lifetime: TokenStream2) -> TokenStream2 {
    let args: Vec<TokenStream2> = generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Type(ty) => {
                let ident = &ty.ident;
                quote! { #ident }
            }
            GenericParam::Lifetime(lifetime_param) => {
                let ident = &lifetime_param.lifetime;
                quote! { #ident }
            }
            GenericParam::Const(const_param) => {
                let ident = &const_param.ident;
                quote! { #ident }
            }
        })
        .collect();

    if args.is_empty() {
        quote! { <#lifetime> }
    } else {
        quote! { <#lifetime, #(#args),*> }
    }
}
