use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;
use syn::ItemEnum;
use syn::parse_quote;
use syn::spanned::Spanned;

use super::build_validate_with_ext_impl;
use super::unified_field_handler::sanitize_enum;
use crate::parse::UnifiedProtoConfig;
use crate::utils::collect_discriminants_for_variants;
use crate::utils::find_marked_default_variant;

pub(super) fn generate_simple_enum_impl(
    input: &DeriveInput,
    item_enum: &ItemEnum,
    data: &syn::DataEnum,
    config: &UnifiedProtoConfig,
) -> TokenStream2 {
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
        variant.discriminant = Some((
            syn::token::Eq {
                spans: [Span::call_site()],
            },
            expr,
        ));
    }

    let raw_from_variant: Vec<_> = ordered_variants
        .iter()
        .zip(discriminants.iter())
        .map(|(variant, value)| {
            let ident = &variant.ident;
            quote! { #name::#ident => #value }
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

    let validate_with_ext_impl = build_validate_with_ext_impl(config);
    let validate_with_ext_proto_impl = if config.has_suns() {
        TokenStream2::new()
    } else {
        validate_with_ext_impl.clone()
    };

    let mut shadow_generics = generics.clone();
    shadow_generics.params.insert(0, parse_quote!('a));
    let (shadow_impl_generics, _shadow_ty_generics, shadow_where_clause) = shadow_generics.split_for_impl();

    let sun_impls = if config.has_suns() {
        let sun_impls = config.suns.iter().map(|sun| {
            let target_ty = &sun.ty;
            quote! {
                impl #impl_generics ::proto_rs::ProtoExt for #target_ty #where_clause {
                    const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::SimpleEnum;
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

                impl #impl_generics ::proto_rs::ProtoDecoder for #target_ty #where_clause {
                    #[inline(always)]
                    fn proto_default() -> Self {
                        let shadow = <#name #ty_generics as ::proto_rs::ProtoDecoder>::proto_default();
                        <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)
                            .expect("failed to build default sun value")
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
                        let mut shadow = <#name #ty_generics as ::proto_rs::ProtoShadowEncode<'_, #target_ty>>::from_sun(value);
                        <#name #ty_generics as ::proto_rs::ProtoDecoder>::merge_field(&mut shadow, tag, wire_type, buf, ctx)?;
                        *value = <#name #ty_generics as ::proto_rs::ProtoShadowDecode<#target_ty>>::to_sun(shadow)?;
                        Ok(())
                    }

                    #[inline(always)]
                    fn merge(
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
                    type Archived<'a> = <#name #ty_generics as ::proto_rs::ProtoArchive>::Archived<'a>;

                    #[inline(always)]
                    fn is_default(&self) -> bool {
                        let shadow = <#name #ty_generics as ::proto_rs::ProtoShadowEncode<'_, #target_ty>>::from_sun(self);
                        <#name #ty_generics as ::proto_rs::ProtoArchive>::is_default(&shadow)
                    }

                    #[inline(always)]
                    fn len(archived: &Self::Archived<'_>) -> usize {
                        <#name #ty_generics as ::proto_rs::ProtoArchive>::len(archived)
                    }

                    #[inline(always)]
                    unsafe fn encode<const TAG: u32>(archived: Self::Archived<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                        <#name #ty_generics as ::proto_rs::ProtoArchive>::encode::<TAG>(archived, buf);
                    }

                    #[inline(always)]
                    fn archive<const TAG: u32>(&self) -> Self::Archived<'_> {
                        let shadow = <#name #ty_generics as ::proto_rs::ProtoShadowEncode<'_, #target_ty>>::from_sun(self);
                        <#name #ty_generics as ::proto_rs::ProtoArchive>::archive::<TAG>(&shadow)
                    }
                }
            }
        });
        quote! { #( #sun_impls )* }
    } else {
        quote! {}
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

    quote! {
        #enum_item
        #try_from_impl

        impl #impl_generics ::proto_rs::ProtoExt for #name #ty_generics #where_clause {
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::SimpleEnum;
        }

        impl #shadow_impl_generics ::proto_rs::ProtoShadowEncode<'a, #name #ty_generics> for i32 #shadow_where_clause {
            #[inline(always)]
            fn from_sun(value: &'a #name #ty_generics) -> Self {
                match *value {
                    #(#raw_from_variant,)*
                }
            }
        }

        impl #impl_generics ::proto_rs::ProtoArchive for #name #ty_generics #where_clause {
            type Archived<'a> = i32;

            #[inline(always)]
            fn is_default(&self) -> bool {
                matches!(*self, Self::#default_ident)
            }

            #[inline(always)]
            fn len(archived: &Self::Archived<'_>) -> usize {
                <i32 as ::proto_rs::ProtoArchive>::len(archived)
            }

            #[inline(always)]
            unsafe fn encode<const TAG: u32>(archived: Self::Archived<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                if TAG != 0 {
                    ::proto_rs::encoding::encode_key(TAG, ::proto_rs::encoding::WireType::Varint, buf);
                }
                <i32 as ::proto_rs::ProtoArchive>::encode::<0>(archived, buf);
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self) -> Self::Archived<'_> {
                match *self {
                    #(#raw_from_variant,)*
                }
            }
        }

        impl #impl_generics ::proto_rs::ProtoEncode for #name #ty_generics #where_clause {
            type Shadow<'a> = i32;
        }

        impl #impl_generics ::proto_rs::ProtoDecoder for #name #ty_generics #where_clause {
            #[inline(always)]
            fn proto_default() -> Self {
                Self::#default_ident
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
                if tag == 1 {
                    Self::merge(value, wire_type, buf, ctx)
                } else {
                    ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx)
                }
            }

            #[inline(always)]
            fn merge(&mut self, wire_type: ::proto_rs::encoding::WireType, buf: &mut impl ::proto_rs::bytes::Buf, ctx: ::proto_rs::encoding::DecodeContext) -> Result<(), ::proto_rs::DecodeError> {
                let mut raw = 0i32;
                <i32 as ::proto_rs::ProtoDecoder>::merge(&mut raw, wire_type, buf, ctx)?;
                *self = Self::try_from(raw)?;
                Ok(())
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

        #sun_impls
    }
}
