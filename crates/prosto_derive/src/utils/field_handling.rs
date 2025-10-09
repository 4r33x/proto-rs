//! Unified field handling for structs and enums

use proc_macro2::TokenStream;
use quote::quote;
use syn::Field;
use syn::Type;
use syn::TypeArray;

use super::array_handling::ArrayFieldHandler;
use super::enum_handling::EnumType;
use super::type_info::*;
use crate::utils::FieldConfig;
use crate::utils::ParsedFieldType;
use crate::utils::generate_field_error;
use crate::utils::generate_missing_field_error;
use crate::utils::parse_field_config;
use crate::utils::parse_field_type;

pub struct FieldHandler<'a> {
    field: &'a Field,
    field_name: &'a syn::Ident,
    field_tag: usize,
    field_config: FieldConfig,
    error_name: &'a syn::Ident,
    context: String,
}

impl<'a> FieldHandler<'a> {
    pub fn new(field: &'a Field, field_name: &'a syn::Ident, field_tag: usize, error_name: &'a syn::Ident, context: String) -> Self {
        let field_config = parse_field_config(field);
        Self {
            field,
            field_name,
            field_tag,
            field_config,
            error_name,
            context,
        }
    }

    /// Generate complete field handling (prost field + conversions)
    pub fn generate(&self) -> FieldGenerationResult {
        let field_ty = &self.field.ty;

        if self.field_config.skip {
            return self.handle_skip_field();
        }

        if self.is_custom_conversion() {
            return self.handle_custom_conversion();
        }

        if let Type::Array(type_array) = field_ty {
            return self.handle_array_field(type_array);
        }

        if let Some(enum_type) = EnumType::from_field(field_ty, &self.field_config) {
            return self.handle_enum_field(enum_type, field_ty);
        }

        self.handle_standard_field(field_ty)
    }

    fn is_custom_conversion(&self) -> bool {
        self.field_config.into_type.is_some() && self.field_config.into_fn.is_some() && self.field_config.from_fn.is_some()
    }

    fn handle_skip_field(&self) -> FieldGenerationResult {
        let field_name = self.field_name;

        if let Some(ref deser_fn) = self.field_config.skip_deser_fn {
            let deser_fn_ident: syn::Ident = syn::parse_str(deser_fn).expect("Invalid deser function name");

            FieldGenerationResult {
                prost_field: None,
                to_proto: None,
                from_proto: FromProtoConversion::SkipWithFn {
                    computation: quote! { let #field_name = #deser_fn_ident(&proto); },
                    field_name: field_name.clone(),
                },
            }
        } else {
            FieldGenerationResult {
                prost_field: None,
                to_proto: None,
                from_proto: FromProtoConversion::SkipDefault(field_name.clone()),
            }
        }
    }

    fn handle_custom_conversion(&self) -> FieldGenerationResult {
        let field_name = self.field_name;
        let field_tag = self.field_tag;

        let into_type: Type = syn::parse_str(self.field_config.into_type.as_ref().unwrap()).expect("Invalid into type");

        let into_fn: syn::Ident = syn::parse_str(self.field_config.into_fn.as_ref().unwrap()).expect("Invalid into_fn");

        let from_fn: syn::Ident = syn::parse_str(self.field_config.from_fn.as_ref().unwrap()).expect("Invalid from_fn");

        let parsed = parse_field_type(&into_type);
        let prost_type_tokens = &parsed.prost_type;

        let prost_field = quote! {
            #[prost(#prost_type_tokens, tag = #field_tag)]
            pub #field_name: #into_type
        };

        let to_proto = quote! { #field_name: #into_fn(&self.#field_name) };
        let from_proto = quote! { #field_name: #from_fn(proto.#field_name) };

        FieldGenerationResult {
            prost_field: Some(prost_field),
            to_proto: Some(to_proto),
            from_proto: FromProtoConversion::Normal(from_proto),
        }
    }

    fn handle_array_field(&self, type_array: &TypeArray) -> FieldGenerationResult {
        let handler = ArrayFieldHandler::new(self.field_name, self.field_tag, type_array, &self.field_config, self.error_name, self.context.clone());

        let (prost_field, parsed) = handler.generate_prost_field();
        let to_proto = handler.generate_to_proto(&parsed);
        let from_proto = handler.generate_from_proto(&parsed);

        FieldGenerationResult {
            prost_field: Some(prost_field),
            to_proto: Some(to_proto),
            from_proto: FromProtoConversion::Normal(from_proto),
        }
    }

    fn handle_enum_field(&self, enum_type: EnumType, field_ty: &Type) -> FieldGenerationResult {
        let field_name = self.field_name;
        let field_tag = self.field_tag;
        let error_name = self.error_name;
        let context = &self.context;

        let (_, is_option, is_repeated) = super::enum_handling::extract_wrapper_info(field_ty);

        let prost_field = enum_type.generate_prost_field(field_name, field_tag, is_option, is_repeated);
        let to_proto = enum_type.generate_to_proto(field_name, is_option, is_repeated);
        let from_proto = enum_type.generate_from_proto(field_name, is_option, is_repeated, error_name, context);

        FieldGenerationResult {
            prost_field: Some(prost_field),
            to_proto: Some(to_proto),
            from_proto: FromProtoConversion::Normal(from_proto),
        }
    }

    fn handle_standard_field(&self, field_ty: &Type) -> FieldGenerationResult {
        let field_name = self.field_name;
        let field_tag = self.field_tag;
        let error_name = self.error_name;

        let parsed = parse_field_type(field_ty);
        let proto_field_ty = self.get_proto_field_type(&parsed, field_ty);

        let prost_attr = if parsed.is_repeated {
            let prost_type = &parsed.prost_type;
            quote! { #[prost(#prost_type, repeated, tag = #field_tag)] }
        } else if parsed.is_option || parsed.is_message_like {
            let prost_type = &parsed.prost_type;
            quote! { #[prost(#prost_type, optional, tag = #field_tag)] }
        } else {
            let prost_type = &parsed.prost_type;
            quote! { #[prost(#prost_type, tag = #field_tag)] }
        };

        let prost_field = quote! {
            #prost_attr
            pub #field_name: #proto_field_ty
        };

        let to_proto = self.generate_standard_to_proto(&parsed, field_ty);
        let from_proto = self.generate_standard_from_proto(&parsed, field_ty, error_name);

        FieldGenerationResult {
            prost_field: Some(prost_field),
            to_proto: Some(to_proto),
            from_proto: FromProtoConversion::Normal(from_proto),
        }
    }

    fn get_proto_field_type(&self, parsed: &ParsedFieldType, field_ty: &Type) -> TokenStream {
        if parsed.is_repeated {
            if let Some(inner_ty) = vec_inner_type(field_ty) {
                if is_bytes_vec(field_ty) {
                    return quote! { ::std::vec::Vec<u8> };
                } else if self.field_config.is_message {
                    return quote! { ::std::vec::Vec<#inner_ty> };
                } else if is_complex_type(&inner_ty) {
                    let inner_proto = &parsed.proto_rust_type;
                    return quote! { ::std::vec::Vec<#inner_proto> };
                } else {
                    let inner_proto = get_proto_rust_type(&inner_ty);
                    return quote! { ::std::vec::Vec<#inner_proto> };
                }
            }
        } else if parsed.is_option {
            let inner_ty = extract_option_inner_type(field_ty);
            if self.field_config.is_message {
                return quote! { ::core::option::Option<#inner_ty> };
            } else if is_complex_type(inner_ty) {
                let inner_parsed = parse_field_type(inner_ty);
                let inner_proto = &inner_parsed.proto_rust_type;
                return quote! { ::core::option::Option<#inner_proto> };
            } else {
                let proto_inner = get_proto_rust_type(inner_ty);
                return quote! { ::core::option::Option<#proto_inner> };
            }
        } else if parsed.is_message_like {
            if self.field_config.is_message {
                return quote! { ::core::option::Option<#field_ty> };
            } else {
                let proto_type = &parsed.proto_rust_type;
                return quote! { ::core::option::Option<#proto_type> };
            }
        }

        get_proto_rust_type(field_ty)
    }

    fn generate_standard_to_proto(&self, parsed: &ParsedFieldType, field_ty: &Type) -> TokenStream {
        let field_name = self.field_name;

        if parsed.is_repeated {
            if let Some(inner_ty) = vec_inner_type(field_ty) {
                if is_bytes_vec(field_ty) || self.field_config.is_message {
                    return quote! { #field_name: self.#field_name.clone() };
                } else if is_complex_type(&inner_ty) {
                    return quote! { #field_name: self.#field_name.iter().map(|v| v.to_proto()).collect() };
                } else if needs_into_conversion(&inner_ty) {
                    return quote! { #field_name: self.#field_name.iter().map(|v| (*v).into()).collect() };
                }
            }
            return quote! { #field_name: self.#field_name.clone() };
        } else if parsed.is_option {
            let inner_ty = extract_option_inner_type(field_ty);
            if self.field_config.is_message {
                return quote! { #field_name: self.#field_name.clone() };
            } else if is_complex_type(inner_ty) {
                return quote! { #field_name: self.#field_name.as_ref().map(|v| v.to_proto()) };
            } else if needs_into_conversion(inner_ty) {
                return quote! { #field_name: self.#field_name.map(|v| v.into()) };
            }
            return quote! { #field_name: self.#field_name.clone() };
        } else if parsed.is_message_like {
            if self.field_config.is_message {
                return quote! { #field_name: Some(self.#field_name.clone()) };
            } else {
                return quote! { #field_name: Some(self.#field_name.to_proto()) };
            }
        }

        generate_primitive_to_proto(field_name, field_ty)
    }

    fn generate_standard_from_proto(&self, parsed: &ParsedFieldType, field_ty: &Type, error_name: &syn::Ident) -> TokenStream {
        let field_name = self.field_name;

        if parsed.is_repeated {
            if let Some(inner_ty) = vec_inner_type(field_ty) {
                if is_bytes_vec(field_ty) || self.field_config.is_message {
                    return quote! { #field_name: proto.#field_name };
                } else if is_complex_type(&inner_ty) {
                    let error_handler = generate_field_error(field_name, error_name);
                    return quote! {
                        #field_name: proto.#field_name
                            .into_iter()
                            .map(|v| v.try_into())
                            .collect::<Result<_, _>>()
                            #error_handler
                    };
                } else if needs_try_into_conversion(&inner_ty) {
                    let error_handler = generate_field_error(field_name, error_name);
                    return quote! {
                        #field_name: proto.#field_name
                            .iter()
                            .map(|v| (*v).try_into())
                            .collect::<Result<_, _>>()
                            #error_handler
                    };
                }
            }
            return quote! { #field_name: proto.#field_name };
        } else if parsed.is_option {
            let inner_ty = extract_option_inner_type(field_ty);
            if self.field_config.is_message {
                return quote! { #field_name: proto.#field_name };
            } else if is_complex_type(inner_ty) || needs_try_into_conversion(inner_ty) {
                let error_handler = generate_field_error(field_name, error_name);
                return quote! {
                    #field_name: proto.#field_name
                        .map(|v| v.try_into())
                        .transpose()
                        #error_handler
                };
            }
            return quote! { #field_name: proto.#field_name };
        } else if parsed.is_message_like {
            let missing_error = generate_missing_field_error(field_name, error_name);
            if self.field_config.is_message {
                return quote! {
                    #field_name: proto.#field_name
                        #missing_error
                };
            } else {
                let conversion_error = generate_field_error(field_name, error_name);
                return quote! {
                    #field_name: proto.#field_name
                        #missing_error
                        .try_into()
                        #conversion_error
                };
            }
        }

        generate_primitive_from_proto(field_name, field_ty, error_name)
    }
}

pub struct FieldGenerationResult {
    pub prost_field: Option<TokenStream>,
    pub to_proto: Option<TokenStream>,
    pub from_proto: FromProtoConversion,
}

pub enum FromProtoConversion {
    Normal(TokenStream),
    SkipDefault(syn::Ident),
    SkipWithFn { computation: TokenStream, field_name: syn::Ident },
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_field_handler_primitive() {
        let field: Field = parse_quote! {
            pub id: u64
        };
        let field_name: syn::Ident = parse_quote! { id };
        let error_name: syn::Ident = parse_quote! { MyError };

        let handler = FieldHandler::new(&field, &field_name, 1, &error_name, "id".to_string());
        let result = handler.generate();

        assert!(result.prost_field.is_some());
        assert!(result.to_proto.is_some());
    }

    #[test]
    fn test_field_handler_skip() {
        let field: Field = parse_quote! {
            #[proto(skip)]
            pub internal: String
        };
        let field_name: syn::Ident = parse_quote! { internal };
        let error_name: syn::Ident = parse_quote! { MyError };

        let handler = FieldHandler::new(&field, &field_name, 1, &error_name, "internal".to_string());
        let result = handler.generate();

        assert!(result.prost_field.is_none());
        assert!(result.to_proto.is_none());

        matches!(result.from_proto, FromProtoConversion::SkipDefault(_));
    }
}
