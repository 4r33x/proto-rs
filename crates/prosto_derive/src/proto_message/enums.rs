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

    let validate_with_ext_impl = build_validate_with_ext_impl(config);

    // ProtoExt implementation
    let proto_ext_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoExt for #name #ty_generics #where_clause {
            const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::SimpleEnum;
        }
    };

    // ProtoDecoder implementation
    let proto_decoder_impl = quote! {
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
                match tag {
                    1 => {
                        let mut raw = 0i32;
                        <i32 as ::proto_rs::ProtoDecoder>::merge(&mut raw, wire_type, buf, ctx)?;
                        *value = <Self as ::core::convert::TryFrom<i32>>::try_from(raw)?;
                        Ok(())
                    }
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }
        }
    };

    // ProtoShadowDecode implementation (Self -> Self)
    let proto_shadow_decode_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoShadowDecode<Self> for #name #ty_generics #where_clause {
            #[inline(always)]
            fn to_sun(self) -> Result<Self, ::proto_rs::DecodeError> {
                Ok(self)
            }
        }
    };

    // ProtoDecode implementation
    let proto_decode_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoDecode for #name #ty_generics #where_clause {
            type ShadowDecoded = Self;
            #validate_with_ext_impl
        }
    };

    // ProtoShadowEncode implementation (Self borrows to Self)
    let proto_shadow_encode_impl = quote! {
        impl<'__proto_a> #impl_generics ::proto_rs::ProtoShadowEncode<'__proto_a, #name #ty_generics> for #name #ty_generics #where_clause {
            #[inline(always)]
            fn from_sun(value: &'__proto_a #name #ty_generics) -> Self {
                *value
            }
        }
    };

    // ProtoArchive implementation (for Self, returns i32 as archived)
    let proto_archive_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoArchive for #name #ty_generics #where_clause {
            type Archived<'__proto_a> = i32;

            #[inline(always)]
            fn is_default(&self) -> bool {
                matches!(*self, Self::#default_ident)
            }

            #[inline(always)]
            fn len(archived: &Self::Archived<'_>) -> usize {
                <i32 as ::proto_rs::ProtoArchive>::len(archived)
            }

            #[inline(always)]
            unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
                <i32 as ::proto_rs::ProtoArchive>::encode(archived, buf)
            }

            #[inline(always)]
            fn archive(&self) -> Self::Archived<'_> {
                match *self {
                    #(#raw_from_variant,)*
                }
            }
        }
    };

    // ProtoEncode implementation
    let proto_encode_impl = quote! {
        impl #impl_generics ::proto_rs::ProtoEncode for #name #ty_generics #where_clause {
            type Shadow<'__proto_a> = Self;
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

    quote! {
        #enum_item
        #proto_ext_impl
        #proto_decoder_impl
        #proto_shadow_decode_impl
        #proto_decode_impl
        #proto_shadow_encode_impl
        #proto_archive_impl
        #proto_encode_impl
        #try_from_impl
    }
}
