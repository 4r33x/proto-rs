//! Handler for simple enums (unit variants only) with ProtoExt support

use proc_macro2::TokenStream;
use quote::quote;
use syn::DataEnum;
use syn::DeriveInput;

pub fn handle_enum(input: DeriveInput, data: &DataEnum) -> TokenStream {
    let name = &input.ident;
    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto_message")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    // Build original variants (without proto attributes)
    let original_variants: Vec<_> = data
        .variants
        .iter()
        .map(|v| {
            let variant_attrs: Vec<_> = v.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
            let ident = &v.ident;
            quote! {
                #(#variant_attrs)*
                #ident
            }
        })
        .collect();

    // Generate TryFrom arms
    let try_from_arms: Vec<_> = data
        .variants
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let ident = &v.ident;
            let i = i as i32;
            quote! { #i => Ok(Self::#ident) }
        })
        .collect();

    // First variant is the default
    let first_variant = &data.variants.first().expect("Enum must have at least one variant").ident;

    quote! {
        #(#attrs)*
        #vis enum #name #generics {
            #(#original_variants),*
        }

        impl #generics ::proto_rs::ProtoExt for #name #generics {
            #[inline]
            fn proto_default() -> Self {
                Self::#first_variant
            }

            fn encode_raw(&self, buf: &mut impl ::bytes::BufMut) {
                let value = *self as i32;
                if value != 0 {
                    ::proto_rs::encoding::int32::encode(1, &value, buf);
                }
            }

            fn merge_field(
                &mut self,
                tag: u32,
                wire_type: ::proto_rs::encoding::WireType,
                buf: &mut impl ::bytes::Buf,
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

        impl #generics ::proto_rs::MessageField for #name #generics {}

        impl #generics TryFrom<i32> for #name #generics {
            type Error = ::proto_rs::DecodeError;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                match value {
                    #(#try_from_arms,)*
                    _ => Err(::proto_rs::DecodeError::new("Invalid enum value")),
                }
            }
        }
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
