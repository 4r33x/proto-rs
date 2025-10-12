//! Unified enum handling for rust_enum and proto_enum attributes

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::utils::FieldConfig;
use crate::utils::rust_type_path_ident;

/// Enum type information - abstracts over rust_enum and proto_enum
#[derive(Debug, Clone)]
pub enum EnumType {
    /// Rust enum that needs conversion via Proto enum
    /// Example: #[proto(rust_enum)] status: Status
    RustEnum { enum_ident: syn::Ident, proto_enum_name: String },
    /// Proto enum that's already i32 compatible
    /// Example: #[proto(enum)] status: StatusProto
    ProtoEnum { enum_ident: syn::Ident, enum_name: String },
}

impl EnumType {
    /// Create EnumType from field type and config
    pub fn from_field(field_ty: &Type, field_config: &FieldConfig) -> Option<Self> {
        if !field_config.is_rust_enum && !field_config.is_proto_enum {
            return None;
        }

        let (base_type, _, _) = extract_wrapper_info(field_ty);
        let enum_ident = rust_type_path_ident(&base_type).clone();

        if field_config.is_rust_enum {
            Some(EnumType::RustEnum {
                proto_enum_name: format!("{}Proto", enum_ident),
                enum_ident,
            })
        } else if field_config.is_proto_enum {
            Some(EnumType::ProtoEnum {
                enum_name: enum_ident.to_string(),
                enum_ident,
            })
        } else {
            None
        }
    }

    /// Generate prost field attribute
    /// Handles optional, repeated, and required fields
    pub fn generate_prost_field(&self, field_name: &syn::Ident, field_tag: usize, is_option: bool, is_repeated: bool) -> TokenStream {
        let field_ty_tokens = match (is_option, is_repeated) {
            (true, false) => quote! { Option<i32> },
            (false, true) => quote! { Vec<i32> },
            (false, false) => quote! { i32 },
            (true, true) => panic!("Field cannot be both optional and repeated"),
        };

        let enum_path = match self {
            EnumType::RustEnum { proto_enum_name, .. } => proto_enum_name,
            EnumType::ProtoEnum { enum_name, .. } => enum_name,
        };

        let prost_attr = if is_repeated {
            quote! { #[prost(enumeration = #enum_path, repeated, packed = "true", tag = #field_tag)] }
        } else if is_option {
            quote! { #[prost(enumeration = #enum_path, optional, tag = #field_tag)] }
        } else {
            quote! { #[prost(enumeration = #enum_path, tag = #field_tag)] }
        };

        quote! {
            #prost_attr
            pub #field_name: #field_ty_tokens
        }
    }

    /// Generate to_proto conversion
    /// RustEnum: calls .to_proto() and casts to i32
    /// ProtoEnum: casts directly to i32
    pub fn generate_to_proto(&self, field_name: &syn::Ident, is_option: bool, is_repeated: bool) -> TokenStream {
        match self {
            EnumType::RustEnum { .. } => {
                if is_option {
                    quote! { #field_name: self.#field_name.as_ref().map(|v| v.to_proto() as i32) }
                } else if is_repeated {
                    quote! { #field_name: self.#field_name.iter().map(|v| v.to_proto() as i32).collect() }
                } else {
                    quote! { #field_name: self.#field_name.to_proto() as i32 }
                }
            }
            EnumType::ProtoEnum { .. } => {
                if is_option {
                    quote! { #field_name: self.#field_name.map(|v| v as i32) }
                } else if is_repeated {
                    quote! { #field_name: self.#field_name.iter().map(|v| *v as i32).collect() }
                } else {
                    quote! { #field_name: self.#field_name as i32 }
                }
            }
        }
    }

    /// Generate from_proto conversion with error handling
    /// RustEnum: converts via Proto enum type then to Rust enum
    /// ProtoEnum: converts directly from i32
    pub fn generate_from_proto(
        &self,
        field_name: &syn::Ident,
        is_option: bool,
        is_repeated: bool,
        error_name: &syn::Ident,
        _context: &str,
    ) -> TokenStream {
        match self {
            EnumType::RustEnum { enum_ident, proto_enum_name } => {
                let proto_ident = syn::Ident::new(proto_enum_name, enum_ident.span());

                if is_option {
                    quote! {
                        #field_name: proto.#field_name
                            .map(|v| #proto_ident::try_from(v)
                                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                                .and_then(|proto_enum| #enum_ident::from_proto(proto_enum)))
                            .transpose()
                            .map_err(|e| #error_name::FieldConversion {
                                field: stringify!(#field_name).to_string(),
                                source: e,
                            })?
                    }
                } else if is_repeated {
                    quote! {
                        #field_name: proto.#field_name
                            .into_iter()
                            .map(|v| #proto_ident::try_from(v)
                                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                                .and_then(|proto_enum| #enum_ident::from_proto(proto_enum)))
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| #error_name::FieldConversion {
                                field: stringify!(#field_name).to_string(),
                                source: e,
                            })?
                    }
                } else {
                    quote! {
                        #field_name: {
                            let proto_enum = #proto_ident::try_from(proto.#field_name)
                                .map_err(|e| #error_name::FieldConversion {
                                    field: stringify!(#field_name).to_string(),
                                    source: Box::new(e),
                                })?;
                            #enum_ident::from_proto(proto_enum)
                                .map_err(|e| #error_name::FieldConversion {
                                    field: stringify!(#field_name).to_string(),
                                    source: e,
                                })?
                        }
                    }
                }
            }
            EnumType::ProtoEnum { enum_ident, .. } => {
                if is_option {
                    quote! {
                        #field_name: proto.#field_name
                            .map(|v| #enum_ident::try_from(v))
                            .transpose()
                            .map_err(|e| #error_name::FieldConversion {
                                field: stringify!(#field_name).to_string(),
                                source: Box::new(e),
                            })?
                    }
                } else if is_repeated {
                    quote! {
                        #field_name: proto.#field_name
                            .into_iter()
                            .map(|v| #enum_ident::try_from(v))
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| #error_name::FieldConversion {
                                field: stringify!(#field_name).to_string(),
                                source: Box::new(e),
                            })?
                    }
                } else {
                    quote! {
                        #field_name: #enum_ident::try_from(proto.#field_name)
                            .map_err(|e| #error_name::FieldConversion {
                                field: stringify!(#field_name).to_string(),
                                source: Box::new(e),
                            })?
                    }
                }
            }
        }
    }
}

/// Helper to extract wrapper info (returns base type, is_option, is_repeated)
pub fn extract_wrapper_info(ty: &Type) -> (Type, bool, bool) {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        if segment.ident == "Option" {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
            {
                return (inner.clone(), true, false);
            }
        } else if segment.ident == "Vec"
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
        {
            return (inner.clone(), false, true);
        }
    }
    (ty.clone(), false, false)
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_extract_wrapper_info() {
        let ty: Type = parse_quote! { Status };
        let (_base, is_opt, is_rep) = extract_wrapper_info(&ty);
        assert!(!is_opt);
        assert!(!is_rep);

        let ty: Type = parse_quote! { Option<Status> };
        let (_base, is_opt, is_rep) = extract_wrapper_info(&ty);
        assert!(is_opt);
        assert!(!is_rep);

        let ty: Type = parse_quote! { Vec<Status> };
        let (_base, is_opt, is_rep) = extract_wrapper_info(&ty);
        assert!(!is_opt);
        assert!(is_rep);
    }

    #[test]
    fn test_enum_type_from_field_rust_enum() {
        let ty: Type = parse_quote! { Status };
        let config = FieldConfig {
            is_rust_enum: true,
            ..Default::default()
        };

        let enum_type = EnumType::from_field(&ty, &config).unwrap();
        match enum_type {
            EnumType::RustEnum { proto_enum_name, .. } => {
                assert_eq!(proto_enum_name, "StatusProto");
            }
            _ => panic!("Expected RustEnum"),
        }
    }

    #[test]
    fn test_enum_type_from_field_proto_enum() {
        let ty: Type = parse_quote! { StatusProto };
        let config = FieldConfig {
            is_proto_enum: true,
            ..Default::default()
        };

        let enum_type = EnumType::from_field(&ty, &config).unwrap();
        match enum_type {
            EnumType::ProtoEnum { enum_name, .. } => {
                assert_eq!(enum_name, "StatusProto");
            }
            _ => panic!("Expected ProtoEnum"),
        }
    }
}
