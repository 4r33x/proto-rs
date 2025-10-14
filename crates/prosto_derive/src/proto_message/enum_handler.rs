//! Handler for simple enums (unit variants only) with `ProtoExt` support

use proc_macro2::TokenStream;
use quote::quote;
use syn::DataEnum;
use syn::DeriveInput;
use syn::spanned::Spanned;

use crate::utils::collect_discriminants_for_variants;
use crate::utils::find_marked_default_variant;

pub fn handle_enum(input: DeriveInput, data: &DataEnum) -> TokenStream {
    let name = &input.ident;
    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto_message")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    // Build original variants (without proto attributes)
    let marked_default = match find_marked_default_variant(data) {
        Ok(idx) => idx,
        Err(err) => return err.to_compile_error(),
    };

    let mut order: Vec<usize> = (0..data.variants.len()).collect();
    if let Some(idx) = marked_default
        && idx < order.len()
    {
        order.remove(idx);
        order.insert(0, idx);
    }

    let ordered_variants: Vec<&syn::Variant> = order.iter().map(|&idx| &data.variants[idx]).collect();
    let ordered_discriminants = match collect_discriminants_for_variants(&ordered_variants) {
        Ok(values) => values,
        Err(err) => return err.to_compile_error(),
    };

    if let Some(idx) = marked_default
        && ordered_discriminants.first().copied().unwrap_or_default() != 0
    {
        let variant = &data.variants[idx];
        return syn::Error::new(variant.span(), "enum #[default] variant must have discriminant 0").to_compile_error();
    }

    if !ordered_discriminants.contains(&0) {
        return syn::Error::new(data.variants.span(), "proto enums must contain a variant with discriminant 0").to_compile_error();
    }

    let mut discriminants_by_index = vec![0; data.variants.len()];
    for (pos, &variant_idx) in order.iter().enumerate() {
        discriminants_by_index[variant_idx] = ordered_discriminants[pos];
    }

    let default_variant_index = marked_default.unwrap_or_else(|| {
        discriminants_by_index
            .iter()
            .enumerate()
            .find(|(_, value)| **value == 0)
            .map(|(idx, _)| idx)
            .expect("validated that a zero discriminant exists")
    });
    let default_variant_ident = &data.variants[default_variant_index].ident;

    let original_variants: Vec<_> = order
        .iter()
        .map(|&idx| {
            let variant = &data.variants[idx];
            let variant_attrs: Vec<_> = variant.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
            let ident = &variant.ident;
            let discriminant = variant.discriminant.as_ref().map(|(_, expr)| quote! { = #expr });
            quote! {
                #(#variant_attrs)*
                #ident #discriminant
            }
        })
        .collect();

    // Generate TryFrom arms
    let try_from_arms: Vec<_> = data
        .variants
        .iter()
        .enumerate()
        .map(|(idx, variant)| {
            let ident = &variant.ident;
            let value = discriminants_by_index[idx];
            quote! { #value => Ok(Self::#ident) }
        })
        .collect();

    let proto_enum_impl = quote! {
        impl #generics ::proto_rs::ProtoEnum for #name #generics {
            const DEFAULT_VALUE: Self = Self::#default_variant_ident;

            fn from_i32(value: i32) -> Result<Self, ::proto_rs::DecodeError> {
                Self::try_from(value)
            }

            fn to_i32(self) -> i32 {
                self as i32
            }
        }
    };

    quote! {
        #(#attrs)*
        #vis enum #name #generics {
            #(#original_variants),*
        }

        impl #generics ::proto_rs::ProtoExt for #name #generics {
            type Shadow = Self;

            #[inline]
            fn proto_default() -> Self::Shadow {
                Self::#default_variant_ident
            }

            fn encode_shadow(shadow: &Self::Shadow, buf: &mut impl ::proto_rs::bytes::BufMut) {
                let value = *shadow as i32;
                if value != 0 {
                    ::proto_rs::encoding::int32::encode(1, &value, buf);
                }
            }

            fn merge_field(
                shadow: &mut Self::Shadow,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                match tag {
                    1 => {
                        let mut value: i32 = 0;
                        ::proto_rs::encoding::int32::merge(wire_type, &mut value, buf, ctx)?;
                        *shadow = Self::try_from(value)
                            .map_err(|_| ::proto_rs::DecodeError::new("Invalid enum value"))?;
                        Ok(())
                    }
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }

            fn encoded_len_shadow(shadow: &Self::Shadow) -> usize {
                let value = *shadow as i32;
                if value != 0 {
                    ::proto_rs::encoding::int32::encoded_len(1, &value)
                } else {
                    0
                }
            }

            fn clear_shadow(shadow: &mut Self::Shadow) {
                *shadow = Self::proto_default();
            }

            fn post_decode(shadow: Self::Shadow) -> Self {
                shadow
            }

            fn cast_shadow(value: &Self) -> Self::Shadow {
                *value
            }

            fn encode_raw(&self, buf: &mut impl ::proto_rs::bytes::BufMut) {
                Self::encode_shadow(self, buf);
            }

            fn encoded_len(&self) -> usize {
                Self::encoded_len_shadow(self)
            }

            fn merge_into(
                value: &mut Self,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                Self::merge_field(value, tag, wire_type, buf, ctx)
            }

            fn merge(
                &mut self,
                mut buf: impl ::proto_rs::bytes::Buf,
            ) -> Result<(), ::proto_rs::DecodeError> {
                let ctx = ::proto_rs::encoding::DecodeContext::default();
                while buf.has_remaining() {
                    let (tag, wire_type) = ::proto_rs::encoding::decode_key(&mut buf)?;
                    Self::merge_field(self, tag, wire_type, &mut buf, ctx)?;
                }
                Ok(())
            }

            fn merge_length_delimited(
                &mut self,
                buf: impl ::proto_rs::bytes::Buf,
            ) -> Result<(), ::proto_rs::DecodeError> {
                Self::merge_length_delimited_shadow(self, buf)
            }

            fn clear(&mut self) {
                Self::clear_shadow(self);
            }
        }

        impl #generics TryFrom<i32> for #name #generics {
            type Error = ::proto_rs::DecodeError;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                match value {
                    #(#try_from_arms,)*
                    _ => Err(::proto_rs::DecodeError::new("Invalid enum value")),
                }
            }
        }

        impl #generics ::proto_rs::RepeatedField for #name #generics {
            fn encode_repeated_field(
                tag: u32,
                values: &[Self],
                buf: &mut impl ::proto_rs::bytes::BufMut,
            ) {
                for value in values {
                    let raw = *value as i32;
                    ::proto_rs::encoding::int32::encode(tag, &raw, buf);
                }
            }

            fn merge_repeated_field(
                wire_type: ::proto_rs::encoding::WireType,
                values: &mut ::std::vec::Vec<Self>,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                if wire_type == ::proto_rs::encoding::WireType::LengthDelimited {
                    ::proto_rs::encoding::merge_loop(values, buf, ctx, |values, buf, ctx| {
                        let mut raw: i32 = 0;
                        ::proto_rs::encoding::int32::merge(
                            ::proto_rs::encoding::WireType::Varint,
                            &mut raw,
                            buf,
                            ctx,
                        )?;
                        values.push(Self::try_from(raw)?);
                        Ok(())
                    })
                } else {
                    ::proto_rs::encoding::check_wire_type(
                        ::proto_rs::encoding::WireType::Varint,
                        wire_type,
                    )?;
                    let mut raw: i32 = 0;
                    ::proto_rs::encoding::int32::merge(wire_type, &mut raw, buf, ctx)?;
                    values.push(Self::try_from(raw)?);
                    Ok(())
                }
            }

            fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
                let mut total = 0usize;
                for value in values {
                    let raw = *value as i32;
                    total += ::proto_rs::encoding::int32::encoded_len(tag, &raw);
                }
                total
            }
        }

        impl #generics ::proto_rs::SingularField for #name #generics {
            fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl ::proto_rs::bytes::BufMut) {
                let raw: i32 = (*value) as i32;
                if raw != 0 {
                    ::proto_rs::encoding::int32::encode(tag, &raw, buf);
                }
            }

            fn merge_singular_field(
                wire_type: ::proto_rs::encoding::WireType,
                value: &mut Self,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                let mut raw: i32 = 0;
                ::proto_rs::encoding::int32::merge(wire_type, &mut raw, buf, ctx)?;
                *value = Self::try_from(raw)?;
                Ok(())
            }

            fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
                let raw: i32 = (*value) as i32;
                if raw != 0 {
                    ::proto_rs::encoding::int32::encoded_len(tag, &raw)
                } else {
                    0
                }
            }
        }

        #proto_enum_impl
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_simple_enum_generation() {
        let input: DeriveInput = parse_quote! {
            #[derive(Debug)]
            pub enum Status {
                Pending,
                Active,
                Completed,
            }
        };

        let syn::Data::Enum(data) = input.data.clone() else { panic!("Expected enum") };

        let output = handle_enum(input, &data);
        let output_str = output.to_string();

        assert!(output_str.contains("enum Status"));
        assert!(output_str.contains("Pending"));
    }
}
