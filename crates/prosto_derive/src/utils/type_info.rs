//! Unified type analysis and conversion

use proc_macro2::TokenStream;
use quote::quote;
use syn::GenericArgument;
use syn::PathArguments;
use syn::Type;
use syn::TypePath;

// ============================================================================
// TYPE PREDICATES
// ============================================================================

/// Check if type is bytes array ([u8; N])
pub fn is_bytes_array(ty: &Type) -> bool {
    matches!(ty, Type::Array(arr) if is_u8_element(&arr.elem))
}

/// Check if type is Vec<u8>
pub fn is_bytes_vec(ty: &Type) -> bool {
    vec_inner_type(ty).map(|inner| is_u8_element(&inner)).unwrap_or(false)
}

pub fn is_u8_element(ty: &Type) -> bool {
    if let Type::Path(path) = ty
        && let Some(seg) = path.path.segments.last()
    {
        return seg.ident == "u8";
    }
    false
}

/// Check if type is Option<T>
pub fn is_option_type(ty: &Type) -> bool {
    is_wrapper_type(ty, "Option")
}

fn is_wrapper_type(ty: &Type, wrapper_name: &str) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        path.segments.last().map(|s| s.ident == wrapper_name).unwrap_or(false)
    } else {
        false
    }
}

/// Check if type is a complex (non-primitive) type
pub fn is_complex_type(ty: &Type) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segment = match path.segments.last() {
                Some(s) => s,
                None => return true,
            };

            let type_name = segment.ident.to_string();

            // Handle wrappers recursively
            if matches!(type_name.as_str(), "Option" | "Vec" | "Box") {
                return extract_inner_from_generic(segment).map(is_complex_type).unwrap_or(false);
            }

            // Primitives are NOT complex
            !is_primitive_name(&type_name)
        }
        _ => true,
    }
}

fn is_primitive_name(type_name: &str) -> bool {
    matches!(
        type_name,
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "f32" | "f64" | "bool" | "String" | "Bytes"
    )
}

// ============================================================================
// TYPE EXTRACTION
// ============================================================================

/// Extract inner type from Option<T>
pub fn extract_option_inner_type(ty: &Type) -> &Type {
    extract_wrapper_inner_type(ty, "Option").unwrap_or(ty)
}

fn extract_wrapper_inner_type<'a>(ty: &'a Type, wrapper: &str) -> Option<&'a Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == wrapper
    {
        return extract_inner_from_generic(segment);
    }
    None
}

fn extract_inner_from_generic(segment: &syn::PathSegment) -> Option<&Type> {
    if let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner);
    }
    None
}

/// Extract inner type from Vec<T> (returns Option<Type>)
pub fn vec_inner_type(ty: &Type) -> Option<Type> {
    extract_wrapper_inner_type(ty, "Vec").cloned()
}

/// Get array element type
pub fn array_elem_type(ty: &Type) -> Option<Type> {
    if let Type::Array(type_array) = ty { Some((*type_array.elem).clone()) } else { None }
}

/// Get the last identifier from a type path (handles nested generics)
pub fn rust_type_path_ident(ty: &Type) -> &syn::Ident {
    match ty {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last().expect("Empty type path");
            let ident = &segment.ident;

            // Recursively unwrap wrappers
            if matches!(ident.to_string().as_str(), "Vec" | "Option" | "Box")
                && let Some(inner) = extract_inner_from_generic(segment)
            {
                return rust_type_path_ident(inner);
            }

            ident
        }
        Type::Array(arr) => rust_type_path_ident(&arr.elem),
        Type::Reference(r) => rust_type_path_ident(&r.elem),
        Type::Group(g) => rust_type_path_ident(&g.elem),
        _ => panic!("Unsupported type structure: {:?}", quote!(#ty)),
    }
}

// ============================================================================
// TYPE CONVERSION
// ============================================================================

/// Get the proto-equivalent Rust type (handles size conversions)
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

    // Handle type paths
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        let converted = convert_primitive_type(&segment.ident.to_string());
        if let Some(proto_type) = converted {
            return proto_type;
        }
    }

    // Default: pass through
    quote! { #ty }
}

fn convert_primitive_type(type_name: &str) -> Option<TokenStream> {
    match type_name {
        "u8" | "u16" => Some(quote! { u32 }),
        "i8" | "i16" => Some(quote! { i32 }),
        "usize" => Some(quote! { u64 }),
        "isize" => Some(quote! { i64 }),
        "u128" | "i128" => Some(quote! { ::std::vec::Vec<u8> }),
        _ => None,
    }
}

/// Check if type needs .into() conversion for to_proto
pub fn needs_into_conversion(ty: &Type) -> bool {
    type_needs_conversion(ty, &["u8", "u16", "i8", "i16", "usize", "isize"])
}

/// Check if type needs .try_into() conversion for from_proto
pub fn needs_try_into_conversion(ty: &Type) -> bool {
    type_needs_conversion(ty, &["u8", "u16", "i8", "i16"])
}

fn type_needs_conversion(ty: &Type, type_names: &[&str]) -> bool {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.last().map(|s| type_names.contains(&s.ident.to_string().as_str())).unwrap_or(false)
    } else {
        false
    }
}

/// Generate to_proto conversion for primitives
pub fn generate_primitive_to_proto(ident: &syn::Ident, ty: &Type) -> TokenStream {
    // Handle arrays
    if let Type::Array(_) = ty {
        if is_bytes_array(ty) {
            return quote! { #ident: self.#ident.to_vec() };
        }

        if let Some(elem_ty) = array_elem_type(ty)
            && needs_into_conversion(&elem_ty)
        {
            return quote! { #ident: self.#ident.iter().map(|v| (*v).into()).collect() };
        }

        return quote! { #ident: self.#ident.to_vec() };
    }

    // Handle primitives
    if needs_into_conversion(ty) {
        quote! { #ident: self.#ident.into() }
    } else {
        quote! { #ident: self.#ident.clone() }
    }
}

/// Generate from_proto conversion for primitives
pub fn generate_primitive_from_proto(ident: &syn::Ident, ty: &Type, error_name: &syn::Ident) -> TokenStream {
    use crate::utils::generate_field_error;

    // Handle arrays
    if let Type::Array(_) = ty {
        return generate_array_from_proto(ident, ty, error_name);
    }

    // Handle primitives
    if needs_try_into_conversion(ty) {
        let error_handler = generate_field_error(ident, error_name);
        quote! {
            #ident: proto.#ident.try_into()
                #error_handler
        }
    } else {
        quote! { #ident: proto.#ident }
    }
}

fn generate_array_from_proto(ident: &syn::Ident, ty: &Type, error_name: &syn::Ident) -> TokenStream {
    use crate::utils::generate_field_error;

    let array_error = quote! {
        .map_err(|_| #error_name::FieldConversion {
            field: stringify!(#ident).to_string(),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid array length"
            )),
        })?
    };

    if is_bytes_array(ty) {
        return quote! {
            #ident: proto.#ident.as_slice().try_into() #array_error
        };
    }

    if let Some(elem_ty) = array_elem_type(ty)
        && needs_try_into_conversion(&elem_ty)
    {
        let error_handler = generate_field_error(ident, error_name);
        return quote! {
            #ident: {
                let vec: Vec<_> = proto.#ident.iter()
                    .map(|v| (*v).try_into())
                    .collect::<Result<_, _>>()
                    #error_handler;
                vec.as_slice().try_into() #array_error
            }
        };
    }

    quote! {
        #ident: proto.#ident.as_slice().try_into() #array_error
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_is_bytes_array() {
        let ty: Type = parse_quote! { [u8; 32] };
        assert!(is_bytes_array(&ty));

        let ty: Type = parse_quote! { [u16; 32] };
        assert!(!is_bytes_array(&ty));
    }

    #[test]
    fn test_is_bytes_vec() {
        let ty: Type = parse_quote! { Vec<u8> };
        assert!(is_bytes_vec(&ty));

        let ty: Type = parse_quote! { Vec<u32> };
        assert!(!is_bytes_vec(&ty));
    }

    #[test]
    fn test_wrapper_detection() {
        let ty: Type = parse_quote! { Option<u32> };
        assert!(is_option_type(&ty));
    }

    #[test]
    fn test_type_conversion() {
        assert!(needs_into_conversion(&parse_quote! { u8 }));
        assert!(needs_try_into_conversion(&parse_quote! { u16 }));
        assert!(!needs_into_conversion(&parse_quote! { u32 }));
    }

    #[test]
    fn test_is_complex_type() {
        assert!(!is_complex_type(&parse_quote! { u32 }));
        assert!(!is_complex_type(&parse_quote! { String }));
        assert!(is_complex_type(&parse_quote! { MyCustomType }));
        assert!(!is_complex_type(&parse_quote! { Vec<u32> }));
        assert!(is_complex_type(&parse_quote! { Vec<MyCustomType> }));
    }
}
