//! Centralized type conversion logic to eliminate duplication

use syn::Type;
use syn::parse_quote;

use super::type_info::is_bytes_array;

/// Get the proto-equivalent Rust type (handles size conversions)
/// Maps Rust types to their protobuf-compatible equivalents
pub fn get_proto_rust_type(ty: &Type) -> Type {
    // Handle arrays
    if let Type::Array(type_array) = ty {
        let elem_ty = &*type_array.elem;
        if is_bytes_array(ty) {
            return parse_quote! { ::std::vec::Vec<u8> };
        }
        let elem_proto = get_proto_rust_type(elem_ty);
        return parse_quote! { ::std::vec::Vec<#elem_proto> };
    }

    // Handle primitives with size conversions
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return match segment.ident.to_string().as_str() {
            // Types that need conversion to larger proto types
            "u8" | "u16" => parse_quote! { u32 },
            "i8" | "i16" => parse_quote! { i32 },
            "usize" => parse_quote! { u64 },
            "isize" => parse_quote! { i64 },

            // Types that are too large for proto primitives
            "u128" | "i128" => parse_quote! { ::std::vec::Vec<u8> },

            // Types that don't need conversion - pass through as-is
            "u32" | "u64" | "i32" | "i64" | "f32" | "f64" | "bool" | "String" => ty.clone(),

            // Custom types pass through
            _ => ty.clone(),
        };
    }

    // Default: pass through
    ty.clone()
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

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use super::*;

    fn ty_to_string(ty: &Type) -> String {
        quote!(#ty).to_string()
    }

    #[test]
    fn test_get_proto_rust_type_conversions() {
        // Types that need conversion
        let ty: Type = parse_quote! { u8 };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "u32");

        let ty: Type = parse_quote! { u16 };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "u32");

        let ty: Type = parse_quote! { i8 };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "i32");

        let ty: Type = parse_quote! { i16 };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "i32");

        let ty: Type = parse_quote! { usize };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "u64");

        let ty: Type = parse_quote! { isize };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "i64");
    }

    #[test]
    fn test_get_proto_rust_type_no_conversion() {
        // Types that don't need conversion
        let ty: Type = parse_quote! { u32 };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "u32");

        let ty: Type = parse_quote! { u64 };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "u64");

        let ty: Type = parse_quote! { i32 };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "i32");

        let ty: Type = parse_quote! { i64 };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "i64");

        let ty: Type = parse_quote! { f32 };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "f32");

        let ty: Type = parse_quote! { f64 };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "f64");

        let ty: Type = parse_quote! { bool };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "bool");

        let ty: Type = parse_quote! { String };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), "String");
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
    fn test_bytes_array() {
        let ty: Type = parse_quote! { [u8; 32] };
        let proto = get_proto_rust_type(&ty);
        assert_eq!(ty_to_string(&proto), ":: std :: vec :: Vec < u8 >");
    }

    #[test]
    fn test_array_of_primitives() {
        let ty: Type = parse_quote! { [u16; 32] };
        let proto = get_proto_rust_type(&ty);
        let result = ty_to_string(&proto);
        assert!(result.contains("Vec"));
        assert!(result.contains("u32")); // u16 -> u32
    }
}
