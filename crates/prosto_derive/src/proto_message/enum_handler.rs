//! Handler for simple enums (unit variants only)

use proc_macro2::TokenStream;
use quote::quote;
use syn::DataEnum;
use syn::DeriveInput;

pub fn handle_enum(input: DeriveInput, data: &DataEnum) -> TokenStream {
    let name = &input.ident;
    let proto_name = syn::Ident::new(&format!("{}Proto", name), name.span());
    let error_name = syn::Ident::new(&format!("{}ConversionError", name), name.span());

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

    // Build proto enum variants with explicit values
    let proto_variants: Vec<_> = data
        .variants
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let ident = &v.ident;
            let i = i as i32;
            quote! { #ident = #i }
        })
        .collect();

    // Build to_proto conversion arms
    let to_proto_arms: Vec<_> = data
        .variants
        .iter()
        .map(|v| {
            let ident = &v.ident;
            quote! { #name::#ident => #proto_name::#ident }
        })
        .collect();

    // Build from_proto conversion arms
    let from_proto_arms: Vec<_> = data
        .variants
        .iter()
        .map(|v| {
            let ident = &v.ident;
            quote! { #proto_name::#ident => Ok(#name::#ident) }
        })
        .collect();

    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto_message")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    quote! {
        // Original Rust enum (without proto attributes)
        #(#attrs)*
        #vis enum #name #generics {
            #(#original_variants),*
        }

        // Proto enum (prost-compatible)
        #[derive(::prost::Enumeration, Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(i32)]
        #vis enum #proto_name #generics {
            #(#proto_variants),*
        }

        // Conversion error type
        #[derive(Debug)]
        #vis enum #error_name {
            InvalidValue(i32),
        }

        impl std::fmt::Display for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::InvalidValue(v) => write!(f, "Invalid value for enum {}: {}", stringify!(#name), v),
                }
            }
        }

        impl std::error::Error for #error_name {}

        // Conversion methods
        impl #name {
            pub fn to_proto(&self) -> #proto_name {
                match self {
                    #(#to_proto_arms),*
                }
            }

            pub fn from_proto(proto: #proto_name) -> Result<Self, Box<dyn std::error::Error>> {
                match proto {
                    #(#from_proto_arms),*
                }
            }
        }

        // TryFrom<i32> for convenient conversion
        impl TryFrom<i32> for #name {
            type Error = #error_name;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                let proto = #proto_name::try_from(value)
                    .map_err(|_| #error_name::InvalidValue(value))?;
                Self::from_proto(proto)
                    .map_err(|_| #error_name::InvalidValue(value))
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

        // Check that original enum is preserved
        assert!(output_str.contains("enum Status"));
        assert!(output_str.contains("Pending"));

        // Check that proto enum is generated
        assert!(output_str.contains("enum StatusProto"));
        assert!(output_str.contains(":: prost :: Enumeration"));

        // Check conversion methods exist
        assert!(output_str.contains("fn to_proto"));
        assert!(output_str.contains("fn from_proto"));
    }
}
