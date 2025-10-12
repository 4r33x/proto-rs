//! type_info.rs
//! Lightweight type analysis used by the codegen. 100% `syn` v2 compatible.

use syn::GenericArgument;
use syn::PathArguments;
use syn::Type;

#[derive(Clone)]
pub struct ParsedFieldType {
    /// True if this is `Option<T>`
    pub is_option: bool,
    /// True if this is `Vec<T>`
    pub is_repeated: bool,
    /// True if `T` is treated as a "message" (length-delimited, not scalar)
    pub is_message_like: bool,
    /// True if scalar numeric/bool suitable for packed encoding
    pub is_numeric_scalar: bool,
    /// The element type for `Option<T>`/`Vec<T>`/`[T;N]`, otherwise the type itself
    pub elem_type: Type,
}

impl ParsedFieldType {
    pub fn new(is_option: bool, is_repeated: bool, is_message_like: bool, is_numeric_scalar: bool, elem_type: Type) -> Self {
        Self {
            is_option,
            is_repeated,
            is_message_like,
            is_numeric_scalar,
            elem_type,
        }
    }
}

fn last_path_ident(ty: &Type) -> Option<&syn::Ident> {
    match ty {
        Type::Path(p) => p.path.segments.last().map(|s| &s.ident),
        _ => None,
    }
}

fn path_single_generic(ty: &Type) -> Option<&Type> {
    if let Type::Path(p) = ty {
        if let Some(seg) = p.path.segments.last() {
            if let PathArguments::AngleBracketed(ab) = &seg.arguments {
                if let Some(GenericArgument::Type(inner)) = ab.args.first() {
                    return Some(inner);
                }
            }
        }
    }
    None
}

fn is_bytes_vec_ty(ty: &Type) -> bool {
    if let Type::Path(p) = ty {
        if let Some(seg) = p.path.segments.last() {
            if seg.ident == "Vec" {
                if let PathArguments::AngleBracketed(ab) = &seg.arguments {
                    if let Some(GenericArgument::Type(Type::Path(inner))) = ab.args.first() {
                        return inner.path.segments.last().map(|s| s.ident == "u8").unwrap_or(false);
                    }
                }
            }
        }
    }
    false
}

fn is_numeric_scalar_ty(ty: &Type) -> bool {
    if let Some(id) = last_path_ident(ty) {
        match id.to_string().as_str() {
            "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "bool" | "f32" | "f64" => return true,
            _ => {}
        }
    }
    false
}

fn is_message_like_ty(ty: &Type) -> bool {
    // Strings and bytes are length-delimited but not "message".
    if let Some(id) = last_path_ident(ty) {
        let s = id.to_string();
        if s == "String" || s == "Bytes" {
            return false;
        }
    }
    // Non-scalars are generally message-like unless it's Vec<u8>.
    !is_numeric_scalar_ty(ty) && !is_bytes_vec_ty(ty)
}

/// Parse a field `Type` into a simple shape used by generators.
pub fn parse_field_type(ty: &Type) -> ParsedFieldType {
    // Option<T>
    if let Some(id) = last_path_ident(ty) {
        if id == "Option" {
            if let Some(inner) = path_single_generic(ty) {
                let elem = inner.clone();
                let is_msg = is_message_like_ty(inner);
                let is_num = is_numeric_scalar_ty(inner);
                return ParsedFieldType::new(true, false, is_msg, is_num, elem);
            }
        }
    }

    // Vec<T>
    if let Some(id) = last_path_ident(ty) {
        if id == "Vec" {
            if let Some(inner) = path_single_generic(ty) {
                let elem = inner.clone();
                // Vec<T> is "repeated": if numeric scalar -> packed
                let is_num = is_numeric_scalar_ty(inner);
                let is_msg = is_message_like_ty(inner);
                return ParsedFieldType::new(false, true, is_msg, is_num, elem);
            }
        }
    }

    // Array [T; N] is handled by array-specific logic; here we treat it as scalar of T.
    // Base type
    let elem = ty.clone();
    let is_num = is_numeric_scalar_ty(ty);
    let is_msg = is_message_like_ty(ty);
    ParsedFieldType::new(false, false, is_msg, is_num, elem)
}
