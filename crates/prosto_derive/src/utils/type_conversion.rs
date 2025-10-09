//! Centralized type conversion logic to eliminate duplication

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;
use syn::TypePath;

use super::type_info::is_bytes_array;

/// Get the proto-equivalent Rust type (handles size conversions)
/// Maps Rust types to their protobuf-compatible equivalents
pub fn get_proto_rust_type(ty: &Type) -> TokenStream {
    // Handle arrays
    if let Type::Array(type_array) = ty {
        let elem_ty = &*type_array.elem;
        if is_bytes_array(ty) {
            return quote! { ::std::vec::Vec<u8> };
        }
        let elem_proto = get_proto_rust_type(elem_ty);
        return quote! { ::std::vec::Vec<#elem_proto> };
    }

    // Handle primitives with size conversions
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return match segment.ident.to_string().as_str() {
                // Types that need conversion to larger proto types
                "u8" | "u16" => quote! { u32 },
                "i8" | "i16" => quote! { i32 },
                "usize" => quote! { u64 },
                "isize" => quote! { i64 },

                // Types that are too large for proto primitives
                "u128" | "i128" => quote! { ::std::vec::Vec<u8> },

                // Types that don't need conversion - pass through as-is
                "u32" | "u64" | "i32" | "i64" | "f32" | "f64" | "bool" | "String" => {
                    quote! { #ty }
                }

                // Custom types pass through
                _ => quote! { #ty },
            };
        }
    }

    // Default: pass through
    quote! { #ty }
}

/// Check if type needs .into() conversion for to_proto
/// Returns true for types that are smaller than their proto representation
pub fn needs_into_conversion(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        return type_path
            .path
            .segments
            .last()
            .map(|s| matches!(s.ident.to_string().as_str(), "u8" | "u16" | "i8" | "i16" | "usize" | "isize"))
            .unwrap_or(false);
    }
    false
}

/// Check if type needs .try_into() conversion for from_proto
/// Returns true for types that need downcasting from proto representation
pub fn needs_try_into_conversion(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        return type_path
            .path
            .segments
            .last()
            .map(|s| matches!(s.ident.to_string().as_str(), "u8" | "u16" | "i8" | "i16"))
            .unwrap_or(false);
    }
    false
}

/// Generate to_proto conversion for primitives
/// Handles both direct access and array conversions
pub fn generate_primitive_to_proto(ident: &syn::Ident, ty: &Type) -> TokenStream {
    // Handle arrays
    if let Type::Array(type_array) = ty {
        if is_bytes_array(ty) {
            return quote! { #ident: #ident.to_vec() };
        }
        let elem_ty = &*type_array.elem;
        if needs_into_conversion(elem_ty) {
            return quote! { #ident: #ident.iter().map(|v| (*v).into()).collect() };
        } else {
            return quote! { #ident: #ident.to_vec() };
        }
    }

    // Handle primitives
    if needs_into_conversion(ty) {
        quote! { #ident: self.#ident.into() }
    } else {
        quote! { #ident: self.#ident.clone() }
    }
}

/// Generate from_proto conversion for primitives
/// Handles both direct access and array conversions with error handling
pub fn generate_primitive_from_proto(ident: &syn::Ident, ty: &Type, error_name: &syn::Ident) -> TokenStream {
    // Handle arrays
    if let Type::Array(type_array) = ty {
        if is_bytes_array(ty) {
            return quote! {
                #ident: proto.#ident.as_slice().try_into()
                    .map_err(|_| #error_name::FieldConversion {
                        field: stringify!(#ident).to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid byte array length"
                        ))
                    })?
            };
        }

        let elem_ty = &*type_array.elem;
        if needs_try_into_conversion(elem_ty) {
            return quote! {
                #ident: {
                    let converted: Result<Vec<_>, _> = proto.#ident.iter()
                        .map(|v| (*v).try_into())
                        .collect();
                    converted
                        .map_err(|e| #error_name::FieldConversion {
                            field: stringify!(#ident).to_string(),
                            source: Box::new(e),
                        })?
                        .as_slice()
                        .try_into()
                        .map_err(|_| #error_name::FieldConversion {
                            field: stringify!(#ident).to_string(),
                            source: Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Invalid array length"
                            ))
                        })?
                }
            };
        } else {
            return quote! {
                #ident: proto.#ident.as_slice().try_into()
                    .map_err(|_| #error_name::FieldConversion {
                        field: stringify!(#ident).to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid array length"
                        ))
                    })?
            };
        }
    }

    // Handle primitives
    if needs_try_into_conversion(ty) {
        quote! {
            #ident: proto.#ident.try_into()
                .map_err(|e| #error_name::FieldConversion {
                    field: stringify!(#ident).to_string(),
                    source: Box::new(e),
                })?
        }
    } else {
        quote! { #ident: proto.#ident }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_get_proto_rust_type_conversions() {
        // Types that need conversion
        let ty: Type = parse_quote! { u8 };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "u32");

        let ty: Type = parse_quote! { u16 };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "u32");

        let ty: Type = parse_quote! { i8 };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "i32");

        let ty: Type = parse_quote! { i16 };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "i32");

        let ty: Type = parse_quote! { usize };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "u64");

        let ty: Type = parse_quote! { isize };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "i64");
    }

    #[test]
    fn test_get_proto_rust_type_no_conversion() {
        // Types that don't need conversion
        let ty: Type = parse_quote! { u32 };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "u32");

        let ty: Type = parse_quote! { u64 };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "u64");

        let ty: Type = parse_quote! { i32 };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "i32");

        let ty: Type = parse_quote! { i64 };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "i64");

        let ty: Type = parse_quote! { f32 };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "f32");

        let ty: Type = parse_quote! { f64 };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "f64");

        let ty: Type = parse_quote! { bool };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "bool");

        let ty: Type = parse_quote! { String };
        assert_eq!(get_proto_rust_type(&ty).to_string(), "String");
    }

    #[test]
    fn test_needs_into_conversion() {
        let ty: Type = parse_quote! { u8 };
        assert!(needs_into_conversion(&ty));

        let ty: Type = parse_quote! { u32 };
        assert!(!needs_into_conversion(&ty));

        let ty: Type = parse_quote! { String };
        assert!(!needs_into_conversion(&ty));
    }

    #[test]
    fn test_needs_try_into_conversion() {
        let ty: Type = parse_quote! { u8 };
        assert!(needs_try_into_conversion(&ty));

        let ty: Type = parse_quote! { u32 };
        assert!(!needs_try_into_conversion(&ty));

        // usize needs into but not try_into (u64 -> usize doesn't need try)
        let ty: Type = parse_quote! { usize };
        assert!(!needs_try_into_conversion(&ty));
    }

    #[test]
    fn test_bytes_array() {
        let ty: Type = parse_quote! { [u8; 32] };
        assert_eq!(get_proto_rust_type(&ty).to_string(), ":: std :: vec :: Vec < u8 >");
    }

    #[test]
    fn test_array_of_primitives() {
        let ty: Type = parse_quote! { [u16; 32] };
        let result = get_proto_rust_type(&ty).to_string();
        assert!(result.contains("Vec"));
        assert!(result.contains("u32")); // u16 -> u32
    }
}
