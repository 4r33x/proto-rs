//! Centralized array handling logic

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;
use syn::TypeArray;

use super::enum_handling::EnumType;
use super::type_info::is_bytes_array;
use crate::utils::FieldConfig;
use crate::utils::ParsedFieldType;
use crate::utils::get_proto_rust_type;
use crate::utils::needs_into_conversion;
use crate::utils::needs_try_into_conversion;
use crate::utils::parse_field_type;

pub struct ArrayFieldHandler<'a> {
    field_name: &'a syn::Ident,
    field_tag: usize,
    type_array: &'a TypeArray,
    field_config: &'a FieldConfig,
    error_name: &'a syn::Ident,
    context: String,
}

impl<'a> ArrayFieldHandler<'a> {
    pub fn new(field_name: &'a syn::Ident, field_tag: usize, type_array: &'a TypeArray, field_config: &'a FieldConfig, error_name: &'a syn::Ident, context: String) -> Self {
        Self {
            field_name,
            field_tag,
            type_array,
            field_config,
            error_name,
            context,
        }
    }

    /// Generate prost field for array
    pub fn generate_prost_field(&self) -> (TokenStream, ParsedFieldType) {
        let elem_ty = &*self.type_array.elem;
        let array_ty = Type::Array(self.type_array.clone());

        // Special case: [u8; N] -> bytes
        if is_bytes_array(&array_ty) {
            return self.generate_bytes_array_field();
        }

        // Check for enum attributes
        if let Some(enum_type) = EnumType::from_field(&array_ty, self.field_config) {
            return self.generate_enum_array_field(enum_type, elem_ty);
        }

        // Handle message arrays
        if self.field_config.is_message {
            return self.generate_message_array_field(elem_ty);
        }

        // Default array handling
        self.generate_default_array_field(elem_ty)
    }

    fn generate_bytes_array_field(&self) -> (TokenStream, ParsedFieldType) {
        let field_name = self.field_name;
        let field_tag = self.field_tag;
        let array_ty = Type::Array(self.type_array.clone());

        let prost = quote! {
            #[prost(bytes, tag = #field_tag)]
            pub #field_name: ::std::vec::Vec<u8>
        };

        let parsed = ParsedFieldType {
            rust_type: array_ty,
            proto_type: "bytes".to_string(),
            prost_type: quote! { bytes },
            is_option: false,
            is_repeated: true,
            is_message_like: false,
            proto_rust_type: parse_field_type(&self.type_array.elem).proto_rust_type,
        };

        (prost, parsed)
    }

    fn generate_enum_array_field(&self, enum_type: EnumType, elem_ty: &Type) -> (TokenStream, ParsedFieldType) {
        let field_name = self.field_name;
        let field_tag = self.field_tag;
        let array_ty = Type::Array(self.type_array.clone());

        // Generate prost field using EnumType but mark as repeated
        let prost = enum_type.generate_prost_field(field_name, field_tag, false, true);

        let parsed = ParsedFieldType {
            rust_type: array_ty,
            proto_type: "enum".to_string(),
            prost_type: quote! { enumeration },
            is_option: false,
            is_repeated: true,
            is_message_like: false,
            proto_rust_type: elem_ty.clone(),
        };

        (prost, parsed)
    }

    fn generate_message_array_field(&self, elem_ty: &Type) -> (TokenStream, ParsedFieldType) {
        let field_name = self.field_name;
        let field_tag = self.field_tag;
        let array_ty = Type::Array(self.type_array.clone());

        let prost = quote! {
            #[prost(message, repeated, tag = #field_tag)]
            pub #field_name: ::std::vec::Vec<#elem_ty>
        };

        let parsed = ParsedFieldType {
            rust_type: array_ty,
            proto_type: "message".to_string(),
            prost_type: quote! { message },
            is_option: false,
            is_repeated: true,
            is_message_like: true,
            proto_rust_type: elem_ty.clone(),
        };

        (prost, parsed)
    }

    fn generate_default_array_field(&self, elem_ty: &Type) -> (TokenStream, ParsedFieldType) {
        let field_name = self.field_name;
        let field_tag = self.field_tag;
        let array_ty = Type::Array(self.type_array.clone());

        let parsed_elem = parse_field_type(elem_ty);
        let prost_type = &parsed_elem.prost_type;

        let field_ty_tokens = if parsed_elem.is_message_like {
            let proto_type = &parsed_elem.proto_rust_type;
            quote! { ::std::vec::Vec<#proto_type> }
        } else {
            let proto_elem_ty = get_proto_rust_type(elem_ty);
            quote! { ::std::vec::Vec<#proto_elem_ty> }
        };

        let use_packed = !parsed_elem.is_message_like
            && !matches!(parsed_elem.proto_type.as_str(), "string" | "bytes");

        let prost = if use_packed {
            quote! {
                #[prost(#prost_type, repeated, packed = "true", tag = #field_tag)]
                pub #field_name: #field_ty_tokens
            }
        } else {
            quote! {
                #[prost(#prost_type, repeated, tag = #field_tag)]
                pub #field_name: #field_ty_tokens
            }
        };

        let parsed = ParsedFieldType {
            rust_type: array_ty,
            proto_type: parsed_elem.proto_type.clone(),
            prost_type: parsed_elem.prost_type.clone(),
            is_option: false,
            is_repeated: true,
            is_message_like: parsed_elem.is_message_like,
            proto_rust_type: parsed_elem.proto_rust_type,
        };

        (prost, parsed)
    }

    /// Generate to_proto conversion
    pub fn generate_to_proto(&self, parsed: &ParsedFieldType) -> TokenStream {
        let field_name = self.field_name;
        let elem_ty = &*self.type_array.elem;
        let array_ty = Type::Array(self.type_array.clone());

        // Bytes array
        if is_bytes_array(&array_ty) {
            return quote! { #field_name: self.#field_name.to_vec() };
        }

        // Message array (already proto compatible)
        if self.field_config.is_message {
            return quote! { #field_name: self.#field_name.to_vec() };
        }

        // Enum arrays
        if let Some(enum_type) = EnumType::from_field(&array_ty, self.field_config) {
            // Use enum_type to generate conversion but wrap in array context
            return enum_type.generate_to_proto(field_name, false, true);
        }

        // Message-like arrays
        if parsed.is_message_like {
            return quote! { #field_name: self.#field_name.iter().map(|v| v.to_proto()).collect() };
        }

        // Primitive arrays
        if needs_into_conversion(elem_ty) {
            quote! { #field_name: self.#field_name.iter().map(|v| (*v).into()).collect() }
        } else {
            quote! { #field_name: self.#field_name.to_vec() }
        }
    }

    /// Generate from_proto conversion
    pub fn generate_from_proto(&self, parsed: &ParsedFieldType) -> TokenStream {
        let field_name = self.field_name;
        let error_name = self.error_name;
        let context = &self.context;
        let elem_ty = &*self.type_array.elem;
        let array_ty = Type::Array(self.type_array.clone());

        // Bytes array
        if is_bytes_array(&array_ty) {
            return quote! {
                #field_name: proto.#field_name.as_slice().try_into()
                    .map_err(|_| #error_name::FieldConversion {
                        field: #context.to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid byte array length"
                        )),
                    })?
            };
        }

        // Message array
        if self.field_config.is_message {
            return quote! {
                #field_name: proto.#field_name.try_into()
                    .map_err(|v: Vec<_>| #error_name::FieldConversion {
                        field: #context.to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Invalid array length: got {}", v.len())
                        )),
                    })?
            };
        }

        // Enum arrays
        if let Some(enum_type) = EnumType::from_field(&array_ty, self.field_config) {
            let enum_conversion = enum_type.generate_from_proto(field_name, false, true, error_name, context);
            // Extract just the value expression (remove "field_name:" prefix)
            let enum_value = extract_value_from_assignment(&enum_conversion);

            return quote! {
                #field_name: {
                    let vec = #enum_value;
                    vec.try_into()
                        .map_err(|v: Vec<_>| #error_name::FieldConversion {
                            field: #context.to_string(),
                            source: Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("Invalid array length: got {}", v.len())
                            )),
                        })?
                }
            };
        }

        // Message-like arrays
        if parsed.is_message_like {
            return quote! {
                #field_name: {
                    let vec: Vec<_> = proto.#field_name.into_iter()
                        .map(|v| v.try_into())
                        .collect::<Result<_, _>>()
                        .map_err(|e| #error_name::FieldConversion {
                            field: #context.to_string(),
                            source: Box::new(e),
                        })?;
                    vec.try_into()
                        .map_err(|v: Vec<_>| #error_name::FieldConversion {
                            field: #context.to_string(),
                            source: Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("Invalid array length: got {}", v.len())
                            )),
                        })?
                }
            };
        }

        // Primitive arrays with try_into
        if needs_try_into_conversion(elem_ty) {
            return quote! {
                #field_name: {
                    let vec: Vec<_> = proto.#field_name.iter()
                        .map(|v| (*v).try_into())
                        .collect::<Result<_, _>>()
                        .map_err(|e| #error_name::FieldConversion {
                            field: #context.to_string(),
                            source: Box::new(e),
                        })?;
                    vec.try_into()
                        .map_err(|v: Vec<_>| #error_name::FieldConversion {
                            field: #context.to_string(),
                            source: Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("Invalid array length: got {}", v.len())
                            )),
                        })?
                }
            };
        }

        // Default primitive arrays
        quote! {
            #field_name: proto.#field_name.try_into()
                .map_err(|v: Vec<_>| #error_name::FieldConversion {
                    field: #context.to_string(),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Invalid array length: got {}", v.len())
                    )),
                })?
        }
    }
}

/// Extract value expression from a field assignment like "field: value"
fn extract_value_from_assignment(tokens: &TokenStream) -> TokenStream {
    let tokens_str = tokens.to_string();

    // Find the first colon and extract everything after it
    if let Some(colon_pos) = tokens_str.find(':') {
        let value_part = tokens_str[colon_pos + 1..].trim();
        value_part.parse().unwrap_or_else(|_| tokens.clone())
    } else {
        tokens.clone()
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_bytes_array_handling() {
        let type_array: TypeArray = parse_quote! { [u8; 32] };
        let field_name: syn::Ident = parse_quote! { data };
        let error_name: syn::Ident = parse_quote! { MyError };
        let config = FieldConfig::default();

        let handler = ArrayFieldHandler::new(&field_name, 1, &type_array, &config, &error_name, "data".to_string());

        let (prost, parsed) = handler.generate_prost_field();
        assert_eq!(parsed.proto_type, "bytes");
        assert!(prost.to_string().contains("bytes"));
    }

    #[test]
    fn test_primitive_array_handling() {
        let type_array: TypeArray = parse_quote! { [u32; 8] };
        let field_name: syn::Ident = parse_quote! { values };
        let error_name: syn::Ident = parse_quote! { MyError };
        let config = FieldConfig::default();

        let handler = ArrayFieldHandler::new(&field_name, 1, &type_array, &config, &error_name, "values".to_string());

        let (prost, parsed) = handler.generate_prost_field();
        assert_eq!(parsed.proto_type, "uint32");
        assert!(prost.to_string().contains("uint32"));
        assert!(prost.to_string().contains("repeated"));
    }
}
