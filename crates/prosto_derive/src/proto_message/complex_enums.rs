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
use super::unified_field_handler::compute_decode_ty;
use super::unified_field_handler::compute_proto_ty;
use super::unified_field_handler::decode_conversion_assign;
use super::unified_field_handler::encode_input_binding;
use super::unified_field_handler::field_proto_default_expr;
use super::unified_field_handler::generate_delegating_proto_wire_impl;
use super::unified_field_handler::generate_proto_shadow_impl;
use super::unified_field_handler::generate_sun_proto_ext_impl;
use super::unified_field_handler::needs_decode_conversion;
use super::unified_field_handler::parse_path_string;
use super::unified_field_handler::sanitize_enum;
use crate::parse::UnifiedProtoConfig;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;
use crate::utils::resolved_field_type;

pub(super) fn generate_complex_enum_impl(input: &DeriveInput, item_enum: &ItemEnum, data: &syn::DataEnum, config: &UnifiedProtoConfig) -> syn::Result<TokenStream2> {
    let enum_item = sanitize_enum(item_enum.clone());

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let default_index = crate::utils::find_marked_default_variant(data)?.unwrap_or(0);
    let mut variants = collect_variant_infos(data, config)?;
    if variants.is_empty() {
        return Err(syn::Error::new(input.ident.span(), "proto_message enum must contain at least one variant"));
    }
    if default_index >= variants.len() {
        return Err(syn::Error::new(input.ident.span(), "#[default] variant index is out of bounds"));
    }
    variants[default_index].is_default = true;

    let proto_shadow_impl = generate_proto_shadow_impl(name, generics);

    let shadow_ty = quote! { #name #ty_generics };

    let merge_field_arms = variants.iter().map(|variant| build_variant_merge_arm(name, variant)).collect::<Vec<_>>();

    let default_expr = build_variant_default_expr(&variants[default_index]);
    let is_default_match_arms = variants.iter().map(build_variant_is_default_arm).collect::<Vec<_>>();
    let encoded_len_arms = variants.iter().map(build_variant_encoded_len_arm).collect::<Vec<_>>();
    let encode_arms = variants.iter().map(build_variant_encode_arm).collect::<Vec<_>>();

    let decode_into_body = if let Some(sun) = config.suns.first() {
        let target_ty = &sun.ty;
        if sun.by_ref {
            quote! {
                let decoded = <#target_ty as ::proto_rs::ProtoExt>::decode_length_delimited(buf, ctx)?;
                *value = <Self as ::proto_rs::ProtoShadow<#target_ty>>::from_sun(&decoded);
                Ok(())
            }
        } else {
            quote! {
                let decoded = <#target_ty as ::proto_rs::ProtoExt>::decode_length_delimited(buf, ctx)?;
                *value = <Self as ::proto_rs::ProtoShadow<#target_ty>>::from_sun(decoded);
                Ok(())
            }
        }
    } else {
        quote! {
            *value = <Self as ::proto_rs::ProtoExt>::decode_length_delimited(buf, ctx)?;
            Ok(())
        }
    };

    let proto_ext_impl = if config.has_suns() {
        let impls: Vec<_> = config
            .suns
            .iter()
            .map(|sun| {
                let target_ty = &sun.ty;
                generate_sun_proto_ext_impl(&shadow_ty, target_ty, &merge_field_arms, &quote! {})
            })
            .collect();
        quote! { #(#impls)* }
    } else {
        quote! {
            impl #impl_generics ::proto_rs::ProtoExt for #name #ty_generics #where_clause {
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
        }
    };

    let encode_input_ty = if let Some(sun) = config.suns.first() {
        let target_ty = &sun.ty;
        quote! { <Self as ::proto_rs::ProtoShadow<#target_ty>>::View<'b> }
    } else {
        quote! { <Self as ::proto_rs::ProtoShadow<Self>>::View<'b> }
    };

    let proto_wire_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoWire for #name #ty_generics #where_clause {
            type EncodeInput<'b> = #encode_input_ty;
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
                match &*value {
                    #(#is_default_match_arms,)*
                }
            }

            #[inline(always)]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                match &*value {
                    #(#encoded_len_arms,)*
                }
            }

            #[inline(always)]
            fn encode_raw_unchecked(
                value: Self::EncodeInput<'_>,
                buf: &mut impl ::proto_rs::bytes::BufMut,
            ) {
                match value {
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
                #decode_into_body
            }
        }
    };

    let delegating_impls = if config.has_suns() {
        let shadow_ty = quote! { #name #ty_generics };
        let impls: Vec<_> = config
            .suns
            .iter()
            .map(|sun| generate_delegating_proto_wire_impl(&shadow_ty, &sun.ty))
            .collect();

        quote! { #(#impls)* }
    } else {
        quote! {}
    };

    Ok(quote! {
        #enum_item
        #proto_shadow_impl
        #proto_ext_impl
        #proto_wire_impl
        #delegating_impls
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
                let effective_ty = resolved_field_type(field, &config);
                let parsed = parse_field_type(&effective_ty);
                let proto_ty = compute_proto_ty(field, &config, &parsed, &effective_ty);
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

// Helper: Generate encoding body for empty variants (Unit or empty Struct)
fn build_empty_variant_encode_body(tag: u32) -> TokenStream2 {
    quote! {
        ::proto_rs::encoding::encode_key(
            #tag,
            ::proto_rs::encoding::WireType::LengthDelimited,
            buf,
        );
        ::proto_rs::encoding::encode_varint(0, buf);
    }
}

// Helper: Generate encoded length for empty variants (Unit or empty Struct)
fn build_empty_variant_encoded_len(tag: u32) -> TokenStream2 {
    quote! { ::proto_rs::encoding::key_len(#tag) + 1 }
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
                if field.field.config.skip {
                    quote! { Self::#ident(..) => true }
                } else {
                    let binding_ident = &field.binding_ident;
                    let binding = encode_input_binding(&field.field, &quote! { #binding_ident });
                    let prelude = binding.prelude.into_iter();
                    let value = binding.value;
                    let ty = &field.field.proto_ty;
                    quote! {
                        Self::#ident(#binding_ident) => {
                            #( #prelude )*
                            <#ty as ::proto_rs::ProtoWire>::is_default_impl(&#value)
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
                    let bindings = build_struct_field_bindings(fields);

                    let checks = fields.iter().filter_map(|info| {
                        let ty = &info.proto_ty;
                        let tag = info.tag?;
                        let field_ident = info.field.ident.as_ref().expect("named field");
                        let binding = encode_input_binding(info, &quote! { #field_ident });
                        let prelude = binding.prelude.into_iter();
                        let value = binding.value;
                        Some(quote! {
                            {
                                let _ = #tag;
                                #( #prelude )*
                                if !<#ty as ::proto_rs::ProtoWire>::is_default_impl(&#value) {
                                    return false;
                                }
                            }
                        })
                    });

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
        VariantKind::Unit => {
            let len_expr = build_empty_variant_encoded_len(tag);
            quote! { Self::#ident => #len_expr }
        }
        VariantKind::Tuple { field } => {
            if field.field.config.skip {
                quote! { Self::#ident(..) => 0 }
            } else {
                let binding_ident = &field.binding_ident;
                let binding = encode_input_binding(&field.field, &TokenStream2::new());
                let encode_prelude = binding.prelude.into_iter();
                let encode_value = binding.value;
                let ty = &field.field.proto_ty;
                quote! {
                    Self::#ident(#binding_ident) => {
                        #( #encode_prelude )*
                        let wire = <#ty as ::proto_rs::ProtoWire>::WIRE_TYPE;
                        let body_len = unsafe { <#ty as ::proto_rs::ProtoWire>::encoded_len_impl_raw(&#encode_value) };
                        let key_len = ::proto_rs::encoding::key_len(#tag);
                        let len = match wire {
                            ::proto_rs::encoding::WireType::LengthDelimited => {
                                body_len + ::proto_rs::encoding::encoded_len_varint(body_len as u64)
                            }
                            _ => body_len,
                        };
                        key_len + len
                    }
                }
            }
        }
        VariantKind::Struct { fields } => {
            if fields.is_empty() {
                let len_expr = build_empty_variant_encoded_len(tag);
                quote! {
                    Self::#ident { .. } => #len_expr
                }
            } else {
                let bindings = build_struct_field_bindings(fields);
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
        VariantKind::Unit => {
            let encode_body = build_empty_variant_encode_body(tag);
            quote! {
                Self::#ident => {
                    #encode_body
                }
            }
        }
        VariantKind::Tuple { field } => {
            let binding_ident = &field.binding_ident;
            let encode_binding = encode_input_binding(&field.field, &TokenStream2::new());
            let encode_prelude = encode_binding.prelude.into_iter();
            let encode_value = encode_binding.value;
            let ty = &field.field.proto_ty;
            let encode_body = if field.field.config.skip {
                quote! {}
            } else {
                quote! {
                    #( #encode_prelude )*
                    let wire = <#ty as ::proto_rs::ProtoWire>::WIRE_TYPE;
                    ::proto_rs::encoding::encode_key(#tag, wire, buf);
                    if wire == ::proto_rs::encoding::WireType::LengthDelimited {
                        let len = unsafe { <#ty as ::proto_rs::ProtoWire>::encoded_len_impl_raw(&#encode_value) };
                        ::proto_rs::encoding::encode_varint(len as u64, buf);
                    }
                    <#ty as ::proto_rs::ProtoWire>::encode_raw_unchecked(#encode_value, buf);
                }
            };
            let binding_pattern = if field.field.config.skip {
                quote! { .. }
            } else {
                quote! { #binding_ident }
            };
            quote! {
                Self::#ident(#binding_pattern) => {
                    #encode_body
                }
            }
        }
        VariantKind::Struct { fields } => {
            if fields.is_empty() {
                let encode_body = build_empty_variant_encode_body(tag);
                quote! {
                    Self::#ident { .. } => {
                        #encode_body
                    }
                }
            } else {
                let bindings = build_struct_field_bindings(fields);
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
            let decode_stmt = if field.field.config.skip {
                quote! {
                    ::proto_rs::encoding::skip_field(wire_type, #tag, buf, ctx)?;
                }
            } else if needs_decode_conversion(&field.field.config, &field.field.parsed) {
                let tmp_ident = Ident::new(&format!("__proto_rs_variant_field_{}_tmp", field.field.index), field.field.field.span());
                let decode_ty = &field.field.decode_ty;
                let assign = decode_conversion_assign(&field.field, &quote! { #binding_ident }, &tmp_ident);
                quote! {
                    let mut #tmp_ident: #decode_ty = <#decode_ty as ::proto_rs::ProtoWire>::proto_default();
                    <#decode_ty as ::proto_rs::ProtoWire>::decode_into(
                        wire_type,
                        &mut #tmp_ident,
                        buf,
                        ctx,
                    )?;
                    #assign
                }
            } else {
                let ty = &field.field.field.ty;
                quote! {
                    <#ty as ::proto_rs::ProtoWire>::decode_into(
                        wire_type,
                        &mut #binding_ident,
                        buf,
                        ctx,
                    )?;
                }
            };

            let post_hook = if field.field.config.skip {
                field.field.config.skip_deser_fn.as_ref().map(|fun| {
                    let fun_path = parse_path_string(field.field.field, fun);
                    let skip_binding_ident = Ident::new(&format!("__proto_rs_variant_{}_skip_binding", ident.to_string().to_lowercase()), field.field.field.span());
                    let computed_ident = Ident::new(&format!("__proto_rs_variant_{}_computed", ident.to_string().to_lowercase()), field.field.field.span());
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
            let post_hooks = fields
                .iter()
                .filter_map(|info| {
                    if !info.config.skip {
                        return None;
                    }
                    let fun = info.config.skip_deser_fn.as_ref()?;
                    let field_ident = info.field.ident.as_ref().expect("named field");
                    let fun_path = parse_path_string(info.field, fun);
                    let skip_binding_ident = Ident::new(&format!("__proto_rs_variant_{}_{}_skip_binding", ident.to_string().to_lowercase(), info.index), info.field.span());
                    let computed_ident = Ident::new(&format!("__proto_rs_variant_{}_{}_computed", ident.to_string().to_lowercase(), info.index), info.field.span());
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
