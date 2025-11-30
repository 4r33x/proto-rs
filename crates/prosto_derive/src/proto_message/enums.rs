use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;
use syn::ItemEnum;
use syn::parse_quote;
use syn::spanned::Spanned;

use super::unified_field_handler::generate_delegating_proto_wire_impl;
use super::unified_field_handler::generate_proto_shadow_impl;
use super::unified_field_handler::generate_sun_proto_ext_impl;
use super::unified_field_handler::sanitize_enum;
use crate::parse::UnifiedProtoConfig;
use crate::utils::collect_discriminants_for_variants;
use crate::utils::find_marked_default_variant;

pub(super) fn generate_simple_enum_impl(input: &DeriveInput, item_enum: &ItemEnum, data: &syn::DataEnum, config: &UnifiedProtoConfig) -> TokenStream2 {
    let mut enum_item = sanitize_enum(item_enum.clone());

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let ordered_variants: Vec<&syn::Variant> = (0..data.variants.len()).map(|idx| &data.variants[idx]).collect();
    let mut discriminants = match collect_discriminants_for_variants(&ordered_variants) {
        Ok(values) => values,
        Err(err) => return err.to_compile_error(),
    };

    let marked_default = match find_marked_default_variant(data) {
        Ok(value) => value,
        Err(err) => return err.to_compile_error(),
    };

    let Some(zero_index) = discriminants.iter().position(|&value| value == 0) else {
        return syn::Error::new(data.variants.span(), "proto enums must contain a variant with discriminant 0").to_compile_error();
    };

    let default_index = marked_default.unwrap_or(zero_index);
    if default_index != zero_index {
        let default_value = discriminants[default_index];
        discriminants[default_index] = 0;
        discriminants[zero_index] = default_value;
    }
    let default_ident = &data.variants[default_index].ident;

    enum_item.attrs.push(parse_quote!(#[repr(i32)]));
    for (variant, value) in enum_item.variants.iter_mut().zip(discriminants.iter()) {
        let expr: syn::Expr = parse_quote!(#value);
        variant.discriminant = Some((syn::token::Eq { spans: [Span::call_site()] }, expr));
    }

    let raw_from_variant: Vec<_> = ordered_variants
        .iter()
        .zip(discriminants.iter())
        .map(|(variant, value)| {
            let ident = &variant.ident;
            quote! { Self::#ident => #value }
        })
        .collect();

    let try_from_arms: Vec<_> = ordered_variants
        .iter()
        .zip(discriminants.iter())
        .map(|(variant, value)| {
            let ident = &variant.ident;
            quote! { #value => Ok(Self::#ident) }
        })
        .collect();

    let proto_shadow_impl = generate_proto_shadow_impl(name, generics);

    let shadow_ty = quote! { #name #ty_generics };

    let decode_arms = vec![quote! {
        1 => {
            let mut raw = 0i32;
            <i32 as ::proto_rs::ProtoWire>::decode_into(
                wire_type,
                &mut raw,
                buf,
                ctx,
            )?;
            *value = <Self::Shadow<'_> as ::core::convert::TryFrom<i32>>::try_from(raw)?;
            Ok(())
        }
    }];

    let proto_ext_impl = if config.has_suns() {
        let impls: Vec<_> = config
            .suns
            .iter()
            .map(|sun| {
                let target_ty = &sun.ty;
                generate_sun_proto_ext_impl(&shadow_ty, target_ty, &decode_arms, &quote! {})
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
                        1 => {
                            let mut raw = 0i32;
                            <i32 as ::proto_rs::ProtoWire>::decode_into(
                                wire_type,
                                &mut raw,
                                buf,
                                ctx,
                            )?;
                            *value = <Self::Shadow<'_> as ::core::convert::TryFrom<i32>>::try_from(raw)?;
                            Ok(())
                        }
                        _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                    }
                }
            }
        }
    };

    let proto_wire_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoWire for #name #ty_generics #where_clause {
            type EncodeInput<'b> = &'b Self;
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::SimpleEnum;

            #[inline(always)]
            fn proto_default() -> Self {
                Self::#default_ident
            }

            #[inline(always)]
            fn clear(&mut self) {
                *self = Self::proto_default();
            }

            #[inline(always)]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                matches!(**value, Self::#default_ident)
            }

            #[inline(always)]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                let raw = match **value {
                    #(#raw_from_variant,)*
                };
                <i32 as ::proto_rs::ProtoWire>::encoded_len_impl_raw(&raw)
            }

            #[inline(always)]
            fn encode_raw_unchecked(
                value: Self::EncodeInput<'_>,
                buf: &mut impl ::proto_rs::bytes::BufMut,
            ) {
                let raw = match *value {
                    #(#raw_from_variant,)*
                };
                <i32 as ::proto_rs::ProtoWire>::encode_raw_unchecked(raw, buf);
            }

            #[inline(always)]
            fn decode_into(
                wire_type: ::proto_rs::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                let mut raw = 0i32;
                <i32 as ::proto_rs::ProtoWire>::decode_into(wire_type, &mut raw, buf, ctx)?;
                *value = Self::try_from(raw)?;
                Ok(())
            }
        }
    };

    let try_from_impl = quote! {
        impl #impl_generics ::core::convert::TryFrom<i32> for #name #ty_generics #where_clause {
            type Error = ::proto_rs::DecodeError;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                match value {
                    #(#try_from_arms,)*
                    _ => Err(::proto_rs::DecodeError::new("invalid enum value")),
                }
            }
        }
    };

    let delegating_impls = if config.has_suns() {
        let shadow_ty = quote! { #name #ty_generics };
        let impls: Vec<_> = config.suns.iter().map(|sun| generate_delegating_proto_wire_impl(&shadow_ty, &sun.ty)).collect();

        quote! { #(#impls)* }
    } else {
        quote! {}
    };

    quote! {
        #enum_item
        #proto_shadow_impl
        #proto_ext_impl
        #proto_wire_impl
        #try_from_impl
        #delegating_impls
    }
}
