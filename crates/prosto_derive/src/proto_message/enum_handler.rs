//! Handler for simple enums (unit variants only) with ProtoExt support

use proc_macro2::TokenStream;
use quote::quote;
use syn::DataEnum;
use syn::DeriveInput;

use crate::utils::collect_enum_discriminants;

pub fn handle_enum(input: DeriveInput, data: &DataEnum) -> TokenStream {
    let name = &input.ident;
    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto_message")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    // Build original variants (without proto attributes)
    let discriminants = match collect_enum_discriminants(data) {
        Ok(values) => values,
        Err(err) => return err.to_compile_error(),
    };

    let zero_variant_index = discriminants.iter().position(|&value| value == 0).expect("collect_enum_discriminants guarantees a zero variant");
    let zero_variant_ident = &data.variants[zero_variant_index].ident;

    let original_variants: Vec<_> = data
        .variants
        .iter()
        .map(|v| {
            let variant_attrs: Vec<_> = v.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
            let ident = &v.ident;
            let discriminant = v.discriminant.as_ref().map(|(_, expr)| quote! { = #expr });
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
        .zip(discriminants.iter())
        .map(|(variant, value)| {
            let ident = &variant.ident;
            let value = *value;
            quote! { #value => Ok(Self::#ident) }
        })
        .collect();

    let proto_enum_impl = quote! {
        impl #generics ::proto_rs::ProtoEnum for #name #generics {
            const DEFAULT_VALUE: Self = Self::#zero_variant_ident;

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
            #[inline]
            fn proto_default() -> Self {
                Self::#zero_variant_ident
            }

            fn encode_raw(&self, buf: &mut impl ::proto_rs::bytes::BufMut) {
                let value = *self as i32;
                if value != 0 {
                    ::proto_rs::encoding::int32::encode(1, &value, buf);
                }
            }

            fn merge_field(
                &mut self,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::proto_rs::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                match tag {
                    1 => {
                        let mut value: i32 = 0;
                        ::proto_rs::encoding::int32::merge(wire_type, &mut value, buf, ctx)?;
                        *self = Self::try_from(value)
                            .map_err(|_| ::proto_rs::DecodeError::new("Invalid enum value"))?;
                        Ok(())
                    }
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }

            fn encoded_len(&self) -> usize {
                let value = *self as i32;
                if value != 0 {
                    ::proto_rs::encoding::int32::encoded_len(1, &value)
                } else {
                    0
                }
            }

            fn clear(&mut self) {
                *self = Self::proto_default();
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

        let data = match input.data.clone() {
            syn::Data::Enum(data) => data,
            _ => panic!("Expected enum"),
        };

        let output = handle_enum(input, &data);
        let output_str = output.to_string();

        assert!(output_str.contains("enum Status"));
        assert!(output_str.contains("Pending"));
    }
}
