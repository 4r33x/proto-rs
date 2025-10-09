use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Fields;
use syn::Type;

use crate::utils::*;

pub fn handle_struct(input: DeriveInput, data: &syn::DataStruct) -> TokenStream {
    match &data.fields {
        Fields::Named(_) => handle_named_struct(input, data),
        Fields::Unnamed(_) => handle_tuple_struct(input, data),
        Fields::Unit => handle_unit_struct(input),
    }
}

fn handle_unit_struct(input: DeriveInput) -> TokenStream {
    let name = &input.ident;
    let proto_name = syn::Ident::new(&format!("{}Proto", name), name.span());
    let error_name = syn::Ident::new(&format!("{}ConversionError", name), name.span());
    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    quote! {
        #(#attrs)*
        #vis struct #name #generics;

        #[derive(Debug)]
        #vis enum #error_name {
            MissingField { field: String },
            FieldConversion { field: String, source: Box<dyn std::error::Error> },
        }

        impl std::fmt::Display for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::MissingField { field } => write!(f, "Missing required field: {}", field),
                    Self::FieldConversion { field, source } => write!(f, "Error converting field {}: {}", field, source),
                }
            }
        }
        impl std::error::Error for #error_name {}

        #[derive(::prost::Message, Clone, PartialEq)]
        #vis struct #proto_name #generics {}

        impl HasProto for #name #generics {
            type Proto = #proto_name #generics;

            fn to_proto(&self) -> Self::Proto {
                #proto_name {}
            }

            fn from_proto(_proto: Self::Proto) -> Result<Self, Box<dyn std::error::Error>> {
                Ok(Self)
            }
        }

        impl From<#name #generics> for #proto_name #generics {
            fn from(value: #name #generics) -> Self {
                value.to_proto()
            }
        }

        impl TryFrom<#proto_name #generics> for #name #generics {
            type Error = #error_name;

            fn try_from(proto: #proto_name #generics) -> Result<Self, Self::Error> {
                Self::from_proto(proto).map_err(|e| #error_name::FieldConversion {
                    field: "unknown".to_string(),
                    source: e,
                })
            }
        }
    }
}

fn handle_tuple_struct(input: DeriveInput, data: &syn::DataStruct) -> TokenStream {
    let name = &input.ident;
    let proto_name = syn::Ident::new(&format!("{}Proto", name), name.span());
    let error_name = syn::Ident::new(&format!("{}ConversionError", name), name.span());

    let Fields::Unnamed(fields) = &data.fields else {
        panic!("Expected unnamed fields");
    };

    let mut proto_fields = Vec::new();
    let mut to_proto_values = Vec::new();
    let mut from_proto_values = Vec::new();

    for (idx, field) in fields.unnamed.iter().enumerate() {
        let field_num = (idx + 1) as u32;
        let field_name = syn::Ident::new(&format!("field_{}", idx), name.span());
        let field_idx = syn::Index::from(idx);
        let ty = &field.ty;

        // Special handling for [u8; N] -> bytes
        if is_bytes_array(ty) {
            proto_fields.push(quote! {
                #[prost(bytes, tag = #field_num)]
                pub #field_name: ::std::vec::Vec<u8>,
            });

            to_proto_values.push(quote! { #field_name: self.#field_idx.to_vec() });
            from_proto_values.push(quote! {
                proto.#field_name.as_slice().try_into()
                    .map_err(|_| #error_name::FieldConversion {
                        field: stringify!(#field_name).to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid byte array length"
                        ))
                    })?
            });
            continue;
        }

        // Special handling for other arrays [T; N] -> repeated T
        if let Type::Array(type_array) = ty {
            let elem_ty = &*type_array.elem;
            let parsed_elem = parse_field_type(elem_ty);

            // Get the proto type for the element
            let proto_elem_ty = if let Type::Path(type_path) = elem_ty {
                if let Some(segment) = type_path.path.segments.last() {
                    match segment.ident.to_string().as_str() {
                        "u8" | "u16" => quote! { u32 },
                        "i8" | "i16" => quote! { i32 },
                        "usize" => quote! { u64 },
                        "isize" => quote! { i64 },
                        _ => quote! { #elem_ty },
                    }
                } else {
                    quote! { #elem_ty }
                }
            } else {
                quote! { #elem_ty }
            };

            let prost_type = &parsed_elem.prost_type;

            proto_fields.push(quote! {
                #[prost(#prost_type, repeated, tag = #field_num)]
                pub #field_name: ::std::vec::Vec<#proto_elem_ty>,
            });

            // Check if element needs conversion
            let needs_elem_into = if let Type::Path(type_path) = elem_ty {
                type_path
                    .path
                    .segments
                    .last()
                    .map(|s| matches!(s.ident.to_string().as_str(), "u8" | "u16" | "i8" | "i16" | "usize" | "isize"))
                    .unwrap_or(false)
            } else {
                false
            };

            if needs_elem_into {
                to_proto_values.push(quote! {
                    #field_name: self.#field_idx.iter().map(|v| (*v).into()).collect()
                });
            } else {
                to_proto_values.push(quote! {
                    #field_name: self.#field_idx.to_vec()
                });
            }

            let needs_elem_try_into = if let Type::Path(type_path) = elem_ty {
                type_path
                    .path
                    .segments
                    .last()
                    .map(|s| matches!(s.ident.to_string().as_str(), "u8" | "u16" | "i8" | "i16"))
                    .unwrap_or(false)
            } else {
                false
            };

            if needs_elem_try_into {
                from_proto_values.push(quote! {
                    {
                        let converted: Result<Vec<_>, _> = proto.#field_name.iter()
                            .map(|v| (*v).try_into())
                            .collect();
                        converted
                            .map_err(|e| #error_name::FieldConversion {
                                field: stringify!(#field_name).to_string(),
                                source: Box::new(e),
                            })?
                            .as_slice()
                            .try_into()
                            .map_err(|_| #error_name::FieldConversion {
                                field: stringify!(#field_name).to_string(),
                                source: Box::new(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "Invalid array length"
                                ))
                            })?
                    }
                });
            } else {
                from_proto_values.push(quote! {
                    proto.#field_name.as_slice().try_into()
                        .map_err(|_| #error_name::FieldConversion {
                            field: stringify!(#field_name).to_string(),
                            source: Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Invalid array length"
                            ))
                        })?
                });
            }
            continue;
        }

        // Handle Vec<u8> -> bytes
        if is_bytes_vec(ty) {
            proto_fields.push(quote! {
                #[prost(bytes, tag = #field_num)]
                pub #field_name: ::std::vec::Vec<u8>,
            });
            to_proto_values.push(quote! { #field_name: self.#field_idx.clone() });
            from_proto_values.push(quote! { proto.#field_name });
            continue;
        }

        // Handle regular types (primitives, messages, etc.)
        let parsed = parse_field_type(ty);

        let proto_field_ty = if parsed.is_message_like {
            let proto_rust_type = &parsed.proto_rust_type;
            quote! { ::core::option::Option<#proto_rust_type> }
        } else {
            // Map Rust types to their proto equivalents
            if let syn::Type::Path(type_path) = ty {
                if let Some(segment) = type_path.path.segments.last() {
                    match segment.ident.to_string().as_str() {
                        "u8" | "u16" => quote! { u32 },
                        "i8" | "i16" => quote! { i32 },
                        "usize" => quote! { u64 },
                        "isize" => quote! { i64 },
                        "u128" | "i128" => quote! { ::std::vec::Vec<u8> },
                        _ => quote! { #ty },
                    }
                } else {
                    quote! { #ty }
                }
            } else {
                quote! { #ty }
            }
        };

        let prost_attr = if parsed.is_message_like {
            let prost_type = &parsed.prost_type;
            quote! { #[prost(#prost_type, optional, tag = #field_num)] }
        } else {
            let prost_type = &parsed.prost_type;
            quote! { #[prost(#prost_type, tag = #field_num)] }
        };

        proto_fields.push(quote! {
            #prost_attr
            pub #field_name: #proto_field_ty,
        });

        let to_value = if parsed.is_message_like {
            quote! { #field_name: Some(self.#field_idx.to_proto()) }
        } else {
            // Check if type needs conversion to proto type
            let needs_into = if let syn::Type::Path(type_path) = ty {
                type_path
                    .path
                    .segments
                    .last()
                    .map(|s| matches!(s.ident.to_string().as_str(), "u8" | "u16" | "i8" | "i16" | "usize" | "isize"))
                    .unwrap_or(false)
            } else {
                false
            };

            if needs_into {
                quote! { #field_name: self.#field_idx.into() }
            } else {
                quote! { #field_name: self.#field_idx.clone() }
            }
        };
        to_proto_values.push(to_value);

        let from_value = if parsed.is_message_like {
            quote! {
                proto.#field_name
                    .ok_or_else(|| #error_name::MissingField { field: stringify!(#field_name).to_string() })?
                    .try_into()
                    .map_err(|e| #error_name::FieldConversion {
                        field: stringify!(#field_name).to_string(),
                        source: Box::new(e),
                    })?
            }
        } else {
            // Check if type needs conversion (u8, u16, i8, i16, etc.)
            let needs_conversion = matches!(
                ty,
                syn::Type::Path(p) if p.path.segments.last()
                    .map(|s| matches!(s.ident.to_string().as_str(), "u8" | "u16" | "i8" | "i16"))
                    .unwrap_or(false)
            );

            if needs_conversion {
                quote! {
                    proto.#field_name.try_into()
                        .map_err(|e| #error_name::FieldConversion {
                            field: stringify!(#field_name).to_string(),
                            source: Box::new(e),
                        })?
                }
            } else {
                quote! { proto.#field_name }
            }
        };
        from_proto_values.push(from_value);
    }

    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    let field_types: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();

    quote! {
        #(#attrs)*
        #vis struct #name #generics(#(pub #field_types),*);

        #[derive(Debug)]
        #vis enum #error_name {
            MissingField { field: String },
            FieldConversion { field: String, source: Box<dyn std::error::Error> },
        }

        impl std::fmt::Display for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::MissingField { field } => write!(f, "Missing required field: {}", field),
                    Self::FieldConversion { field, source } => write!(f, "Error converting field {}: {}", field, source),
                }
            }
        }
        impl std::error::Error for #error_name {}

        #[derive(::prost::Message, Clone, PartialEq)]
        #vis struct #proto_name #generics {
            #(#proto_fields)*
        }

        impl HasProto for #name #generics {
            type Proto = #proto_name #generics;

            fn to_proto(&self) -> Self::Proto {
                #proto_name {
                    #(#to_proto_values),*
                }
            }

            fn from_proto(proto: Self::Proto) -> Result<Self, Box<dyn std::error::Error>> {
                Ok(Self(
                    #(#from_proto_values),*
                ))
            }
        }

        impl From<#name #generics> for #proto_name #generics {
            fn from(value: #name #generics) -> Self {
                value.to_proto()
            }
        }

        impl TryFrom<#proto_name #generics> for #name #generics {
            type Error = #error_name;

            fn try_from(proto: #proto_name #generics) -> Result<Self, Self::Error> {
                Self::from_proto(proto).map_err(|e| #error_name::FieldConversion {
                    field: "unknown".to_string(),
                    source: e,
                })
            }
        }
    }
}

pub fn handle_named_struct(input: DeriveInput, data: &syn::DataStruct) -> TokenStream {
    let name = &input.ident;
    let proto_name = syn::Ident::new(&format!("{}Proto", name), name.span());
    let error_name = syn::Ident::new(&format!("{}ConversionError", name), name.span());

    let mut proto_fields = Vec::new();
    let mut to_proto_fields = Vec::new();
    let mut from_proto_fields = Vec::new();
    let mut skip_computations = Vec::new();

    let mut field_num = 0;
    if let Fields::Named(fields) = &data.fields {
        for field in &fields.named {
            let ident = field.ident.as_ref().unwrap();
            let field_config = parse_field_config(field);

            if field_config.skip && field_config.skip_deser_fn.is_some() {
                let deser_fn: syn::Ident = syn::parse_str(field_config.skip_deser_fn.as_ref().unwrap()).expect("Invalid deser function name");
                skip_computations.push(quote! {
                    let #ident = #deser_fn(&proto);
                });
                from_proto_fields.push(quote! {
                    #ident
                });
                continue;
            }

            if field_config.skip {
                // Add value for skipped fields in from_proto
                if field_config.skip_deser_fn.is_none() {
                    from_proto_fields.push(quote! {
                        #ident: Default::default()
                    });
                }
                continue;
            }

            field_num += 1;

            // Check if this field has custom conversion
            let has_custom_conversion = field_config.into_type.is_some() && field_config.into_fn.is_some() && field_config.from_fn.is_some();

            let (proto_field_ty, prost_attr, to_proto_value, from_proto_value) = if has_custom_conversion {
                handle_custom_conversion_field(&field_config, field_num, ident)
            } else if field_config.is_rust_enum {
                handle_rust_enum_field(&field.ty, field_num, ident, &error_name)
            } else if field_config.is_proto_enum {
                handle_proto_enum_field(&field.ty, field_num, ident, &error_name)
            } else {
                handle_standard_field(&field.ty, &field_config, field_num, ident, &error_name)
            };

            proto_fields.push(quote! {
                #prost_attr
                pub #ident: #proto_field_ty,
            });
            to_proto_fields.push(to_proto_value);
            from_proto_fields.push(from_proto_value);
        }
    }

    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    let mut fields_named_idents = Vec::new();
    let mut fields_named_types = Vec::new();

    if let Fields::Named(fields) = &data.fields {
        for field in &fields.named {
            let ident = field.ident.as_ref().unwrap();
            let ty = &field.ty;
            fields_named_idents.push(ident);
            fields_named_types.push(ty);
        }
    }

    quote! {
    // Original struct untouched
    #(#attrs)*
    #vis struct #name #generics {
        #(
            pub #fields_named_idents: #fields_named_types,
        )*
    }

            // Conversion error type
            #[derive(Debug)]
            #vis enum #error_name {
                MissingField { field: String },
                FieldConversion { field: String, source: Box<dyn std::error::Error> },
            }

            impl std::fmt::Display for #error_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        Self::MissingField { field } => write!(f, "Missing required field: {}", field),
                        Self::FieldConversion { field, source } => write!(f, "Error converting field {}: {}", field, source),
                    }
                }
            }
            impl std::error::Error for #error_name {}

            // Proto struct with prost attributes
            #[derive(::prost::Message, Clone, PartialEq)]
            #vis struct #proto_name #generics {
                #(#proto_fields)*
            }

            // HasProto impl
            impl HasProto for #name #generics {
                type Proto = #proto_name #generics;

                fn to_proto(&self) -> Self::Proto {
                    #proto_name {
                        #(#to_proto_fields),*
                    }
                }

                fn from_proto(proto: Self::Proto) -> Result<Self, Box<dyn std::error::Error>> {
                    // Compute skipped fields before destructuring proto
                    #(#skip_computations)*

                    Ok(Self {
                        #(#from_proto_fields),*
                    })
                }
            }

            impl From<#name #generics> for #proto_name #generics {
                fn from(value: #name #generics) -> Self {
                    value.to_proto()
                }
            }

            impl TryFrom<#proto_name #generics> for #name #generics {
                type Error = #error_name;

                fn try_from(proto: #proto_name #generics) -> Result<Self, Self::Error> {
                    Self::from_proto(proto).map_err(|e| #error_name::FieldConversion {
                        field: "unknown".to_string(),
                        source: e,
                    })
                }
            }
        }
}

/// Get the proto field type for a Rust type, handling size conversions
fn get_proto_rust_type(ty: &Type) -> proc_macro2::TokenStream {
    // Handle arrays (non-u8)
    if let Type::Array(type_array) = ty {
        let elem_ty = &*type_array.elem;
        if !is_bytes_array(ty) {
            let elem_proto = get_proto_rust_type(elem_ty);
            return quote! { ::std::vec::Vec<#elem_proto> };
        } else {
            return quote! { ::std::vec::Vec<u8> };
        }
    }

    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return match segment.ident.to_string().as_str() {
            "u8" | "u16" => quote! { u32 },
            "i8" | "i16" => quote! { i32 },
            "usize" => quote! { u64 },
            "isize" => quote! { i64 },
            "u128" | "i128" => quote! { ::std::vec::Vec<u8> },
            _ => quote! { #ty },
        };
    }
    quote! { #ty }
}

/// Check if a type needs .into() conversion for to_proto
fn needs_into_conversion(ty: &Type) -> bool {
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

/// Check if a type needs .try_into() conversion for from_proto
fn needs_try_into_conversion(ty: &Type) -> bool {
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

/// Generate conversion logic for primitives with size differences
fn generate_primitive_to_proto(ident: &syn::Ident, ty: &Type) -> proc_macro2::TokenStream {
    // Handle arrays (non-u8)
    if let Type::Array(type_array) = ty {
        if !is_bytes_array(ty) {
            let elem_ty = &*type_array.elem;
            if needs_into_conversion(elem_ty) {
                return quote! { #ident: self.#ident.iter().map(|v| (*v).into()).collect() };
            } else {
                return quote! { #ident: self.#ident.to_vec() };
            }
        } else {
            return quote! { #ident: self.#ident.to_vec() };
        }
    }

    if needs_into_conversion(ty) {
        quote! { #ident: self.#ident.into() }
    } else {
        quote! { #ident: self.#ident.clone() }
    }
}

/// Generate conversion logic for primitives with size differences
fn generate_primitive_from_proto(ident: &syn::Ident, ty: &Type, error_name: &syn::Ident) -> proc_macro2::TokenStream {
    // Handle arrays (non-u8)
    if let Type::Array(type_array) = ty {
        if !is_bytes_array(ty) {
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
        } else {
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
    }

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

fn handle_custom_conversion_field(
    field_config: &FieldConfig,
    field_num: i32,
    ident: &syn::Ident,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let into_type: Type = syn::parse_str(field_config.into_type.as_ref().unwrap()).expect("Invalid into type");
    let into_fn: syn::Ident = syn::parse_str(field_config.into_fn.as_ref().unwrap()).expect("Invalid into_fn");
    let from_fn: syn::Ident = syn::parse_str(field_config.from_fn.as_ref().unwrap()).expect("Invalid from_fn");

    let parsed = parse_field_type(&into_type);
    let proto_ty = quote! { #into_type };
    let prost_type_tokens = &parsed.prost_type;
    let prost_attr = quote! { #[prost(#prost_type_tokens, tag = #field_num)] };
    let to_value = quote! { #ident: #into_fn(&self.#ident) };
    let from_value = quote! { #ident: #from_fn(proto.#ident) };

    (proto_ty, prost_attr, to_value, from_value)
}

fn handle_rust_enum_field(
    field_ty: &Type,
    field_num: i32,
    ident: &syn::Ident,
    error_name: &syn::Ident,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let (enum_type, is_option, is_repeated) = extract_wrapper_info(field_ty);
    let enum_ident = rust_type_path_ident(&enum_type);
    let enum_name = enum_ident.to_string();

    let proto_field_ty = if is_option {
        quote! { Option<i32> }
    } else if is_repeated {
        quote! { Vec<i32> }
    } else {
        quote! { i32 }
    };

    let prost_attr = if is_repeated {
        quote! { #[prost(enumeration = #enum_name, repeated, tag = #field_num)] }
    } else if is_option {
        quote! { #[prost(enumeration = #enum_name, optional, tag = #field_num)] }
    } else {
        quote! { #[prost(enumeration = #enum_name, tag = #field_num)] }
    };

    let to_proto_value = if is_option {
        quote! { #ident: self.#ident.as_ref().map(|v| v.to_proto() as i32) }
    } else if is_repeated {
        quote! { #ident: self.#ident.iter().map(|v| v.to_proto() as i32).collect() }
    } else {
        quote! { #ident: self.#ident.to_proto() as i32 }
    };

    let from_proto_value = if is_option {
        quote! {
            #ident: proto.#ident
                .map(|v| #enum_ident::try_from(v))
                .transpose()
                .map_err(|e| #error_name::FieldConversion {
                    field: stringify!(#ident).to_string(),
                    source: Box::new(e),
                })?
        }
    } else if is_repeated {
        quote! {
            #ident: proto.#ident
                .into_iter()
                .map(|v| #enum_ident::try_from(v))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| #error_name::FieldConversion {
                    field: stringify!(#ident).to_string(),
                    source: Box::new(e),
                })?
        }
    } else {
        quote! {
            #ident: #enum_ident::try_from(proto.#ident)
                .map_err(|e| #error_name::FieldConversion {
                    field: stringify!(#ident).to_string(),
                    source: Box::new(e),
                })?
        }
    };

    (proto_field_ty, prost_attr, to_proto_value, from_proto_value)
}

fn handle_proto_enum_field(
    field_ty: &Type,
    field_num: i32,
    ident: &syn::Ident,
    error_name: &syn::Ident,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let (enum_type, is_option, is_repeated) = extract_wrapper_info(field_ty);
    let enum_ident = rust_type_path_ident(&enum_type);
    let enum_name = enum_ident.to_string();

    let proto_field_ty = if is_option {
        quote! { Option<i32> }
    } else if is_repeated {
        quote! { Vec<i32> }
    } else {
        quote! { i32 }
    };

    let prost_attr = if is_repeated {
        quote! { #[prost(enumeration = #enum_name, repeated, tag = #field_num)] }
    } else if is_option {
        quote! { #[prost(enumeration = #enum_name, optional, tag = #field_num)] }
    } else {
        quote! { #[prost(enumeration = #enum_name, tag = #field_num)] }
    };

    let to_proto_value = if is_option {
        quote! { #ident: self.#ident.map(|v| v as i32) }
    } else if is_repeated {
        quote! { #ident: self.#ident.iter().map(|v| *v as i32).collect() }
    } else {
        quote! { #ident: self.#ident as i32 }
    };

    let from_proto_value = if is_option {
        quote! {
            #ident: proto.#ident
                .map(|v| #enum_ident::try_from(v))
                .transpose()
                .map_err(|e| #error_name::FieldConversion {
                    field: stringify!(#ident).to_string(),
                    source: Box::new(e),
                })?
        }
    } else if is_repeated {
        quote! {
            #ident: proto.#ident
                .into_iter()
                .map(|v| #enum_ident::try_from(v))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| #error_name::FieldConversion {
                    field: stringify!(#ident).to_string(),
                    source: Box::new(e),
                })?
        }
    } else {
        quote! {
            #ident: #enum_ident::try_from(proto.#ident)
                .map_err(|e| #error_name::FieldConversion {
                    field: stringify!(#ident).to_string(),
                    source: Box::new(e),
                })?
        }
    };

    (proto_field_ty, prost_attr, to_proto_value, from_proto_value)
}

fn handle_standard_field(
    field_ty: &Type,
    field_config: &FieldConfig,
    field_num: i32,
    ident: &syn::Ident,
    error_name: &syn::Ident,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream) {
    // Handle arrays specially - treat them like repeated fields
    if let Type::Array(type_array) = field_ty {
        return handle_array_field(type_array, field_num, ident, error_name);
    }

    let parsed = parse_field_type(field_ty);

    let proto_field_ty = if parsed.is_repeated {
        handle_repeated_field_type(field_ty, field_config, &parsed)
    } else if parsed.is_option {
        handle_option_field_type(field_ty, field_config, &parsed)
    } else if parsed.is_message_like {
        if field_config.is_message {
            quote! { ::core::option::Option<#field_ty> }
        } else {
            let proto_rust_type = &parsed.proto_rust_type;
            quote! { ::core::option::Option<#proto_rust_type> }
        }
    } else {
        get_proto_rust_type(field_ty)
    };

    let prost_attr = if parsed.is_repeated {
        let prost_type = &parsed.prost_type;
        quote! { #[prost(#prost_type, repeated, tag = #field_num)] }
    } else if parsed.is_option || parsed.is_message_like {
        let prost_type = &parsed.prost_type;
        quote! { #[prost(#prost_type, optional, tag = #field_num)] }
    } else {
        let prost_type = &parsed.prost_type;
        quote! { #[prost(#prost_type, tag = #field_num)] }
    };

    let to_proto_value = generate_to_proto_conversion(field_ty, field_config, &parsed, ident);
    let from_proto_value = generate_from_proto_conversion(field_ty, field_config, &parsed, ident, error_name);

    (proto_field_ty, prost_attr, to_proto_value, from_proto_value)
}

fn handle_array_field(
    type_array: &syn::TypeArray,
    field_num: i32,
    ident: &syn::Ident,
    error_name: &syn::Ident,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let elem_ty = &*type_array.elem;

    // Special case for [u8; N] - treat as bytes
    if let Type::Path(elem_path) = elem_ty
        && let Some(segment) = elem_path.path.segments.last()
        && segment.ident == "u8"
    {
        let proto_field_ty = quote! { ::std::vec::Vec<u8> };
        let prost_attr = quote! { #[prost(bytes, tag = #field_num)] };
        let to_value = quote! { #ident: self.#ident.to_vec() };
        let from_value = quote! {
            #ident: proto.#ident.as_slice().try_into()
                .map_err(|_| #error_name::FieldConversion {
                    field: stringify!(#ident).to_string(),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid byte array length"
                    ))
                })?
        };
        return (proto_field_ty, prost_attr, to_value, from_value);
    }

    // For other arrays, treat as repeated
    let parsed_elem = parse_field_type(elem_ty);

    // Check if element is a message-like type
    let is_message = parsed_elem.is_message_like;

    // Get the proto type for the element
    let proto_elem_ty = if is_message {
        // Convert to Proto type
        let proto_rust_type = &parsed_elem.proto_rust_type;
        quote! { #proto_rust_type }
    } else {
        get_proto_rust_type(elem_ty)
    };

    let proto_field_ty = quote! { ::std::vec::Vec<#proto_elem_ty> };

    let prost_type = &parsed_elem.prost_type;
    let prost_attr = quote! { #[prost(#prost_type, repeated, tag = #field_num)] };

    // Generate conversions based on element type
    let to_value = if is_message {
        // For message types, call to_proto() on each element
        quote! { #ident: self.#ident.iter().map(|v| v.to_proto()).collect() }
    } else if needs_into_conversion(elem_ty) {
        // For primitives that need conversion
        quote! { #ident: self.#ident.iter().map(|v| (*v).into()).collect() }
    } else {
        // For primitives that don't need conversion
        quote! { #ident: self.#ident.to_vec() }
    };

    let from_value = if is_message {
        // For message types, convert each element and handle errors
        quote! {
            #ident: {
                let vec: Vec<_> = proto.#ident.into_iter()
                    .map(|v| v.try_into())
                    .collect::<Result<_, _>>()
                    .map_err(|e| #error_name::FieldConversion {
                        field: stringify!(#ident).to_string(),
                        source: Box::new(e),
                    })?;
                vec.try_into()
                    .map_err(|v: Vec<_>| #error_name::FieldConversion {
                        field: stringify!(#ident).to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Invalid array length: expected 32, got {}", v.len())
                        ))
                    })?
            }
        }
    } else if needs_try_into_conversion(elem_ty) {
        // For primitives that need try_into
        quote! {
            #ident: {
                let vec: Vec<_> = proto.#ident.iter()
                    .map(|v| (*v).try_into())
                    .collect::<Result<_, _>>()
                    .map_err(|e| #error_name::FieldConversion {
                        field: stringify!(#ident).to_string(),
                        source: Box::new(e),
                    })?;
                vec.try_into()
                    .map_err(|v: Vec<_>| #error_name::FieldConversion {
                        field: stringify!(#ident).to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Invalid array length: expected size, got {}", v.len())
                        ))
                    })?
            }
        }
    } else {
        // For primitives that don't need conversion
        quote! {
            #ident: proto.#ident.try_into()
                .map_err(|v: Vec<_>| #error_name::FieldConversion {
                    field: stringify!(#ident).to_string(),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Invalid array length: expected size, got {}", v.len())
                    ))
                })?
        }
    };

    (proto_field_ty, prost_attr, to_value, from_value)
}

fn handle_repeated_field_type(field_ty: &Type, field_config: &FieldConfig, _parsed: &ParsedFieldType) -> proc_macro2::TokenStream {
    if let Some(inner_ty) = vec_inner_type(field_ty) {
        if is_bytes_vec(field_ty) {
            quote! { ::std::vec::Vec<u8> }
        } else if field_config.is_message {
            quote! { ::std::vec::Vec<#inner_ty> }
        } else if is_complex_type(&inner_ty) {
            let inner_proto = convert_field_type_to_proto(&inner_ty);
            quote! { ::std::vec::Vec<#inner_proto> }
        } else {
            // Handle primitive types with size conversions (u16->u32, etc.)
            let inner_proto = get_proto_rust_type(&inner_ty);
            quote! { ::std::vec::Vec<#inner_proto> }
        }
    } else {
        quote! { #field_ty }
    }
}

fn handle_option_field_type(field_ty: &Type, field_config: &FieldConfig, _parsed: &ParsedFieldType) -> proc_macro2::TokenStream {
    if let Type::Path(type_path) = field_ty
        && let Some(segment) = type_path.path.segments.last()
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
    {
        if field_config.is_message {
            return quote! { ::core::option::Option<#inner_ty> };
        } else if is_complex_type(inner_ty) {
            let inner_proto = convert_field_type_to_proto(inner_ty);
            return quote! { ::core::option::Option<#inner_proto> };
        } else {
            // Handle primitives that need size conversion
            let proto_inner = get_proto_rust_type(inner_ty);
            return quote! { ::core::option::Option<#proto_inner> };
        }
    }
    quote! { #field_ty }
}

fn generate_to_proto_conversion(field_ty: &Type, field_config: &FieldConfig, parsed: &ParsedFieldType, ident: &syn::Ident) -> proc_macro2::TokenStream {
    if parsed.is_repeated {
        if let Some(inner_ty) = vec_inner_type(field_ty) {
            if is_bytes_vec(field_ty) || field_config.is_message {
                quote! { #ident: self.#ident.clone() }
            } else if is_complex_type(&inner_ty) {
                quote! { #ident: self.#ident.iter().map(|v| v.to_proto()).collect() }
            } else if needs_into_conversion(&inner_ty) {
                // Convert each element for primitives that need size conversion
                quote! { #ident: self.#ident.iter().map(|v| (*v).into()).collect() }
            } else {
                quote! { #ident: self.#ident.clone() }
            }
        } else {
            quote! { #ident: self.#ident.clone() }
        }
    } else if parsed.is_option {
        if let Type::Path(type_path) = field_ty
            && let Some(segment) = type_path.path.segments.last()
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
        {
            if field_config.is_message {
                return quote! { #ident: self.#ident.clone() };
            } else if is_complex_type(inner_ty) {
                return quote! { #ident: self.#ident.as_ref().map(|v| v.to_proto()) };
            } else if needs_into_conversion(inner_ty) {
                return quote! { #ident: self.#ident.map(|v| v.into()) };
            }
        }
        quote! { #ident: self.#ident.clone() }
    } else if parsed.is_message_like {
        if field_config.is_message {
            quote! { #ident: Some(self.#ident.clone()) }
        } else {
            quote! { #ident: Some(self.#ident.to_proto()) }
        }
    } else {
        generate_primitive_to_proto(ident, field_ty)
    }
}

fn generate_from_proto_conversion(field_ty: &Type, field_config: &FieldConfig, parsed: &ParsedFieldType, ident: &syn::Ident, error_name: &syn::Ident) -> proc_macro2::TokenStream {
    if parsed.is_repeated {
        if let Some(inner_ty) = vec_inner_type(field_ty) {
            if is_bytes_vec(field_ty) || field_config.is_message {
                quote! { #ident: proto.#ident }
            } else if is_complex_type(&inner_ty) {
                quote! {
                    #ident: proto.#ident
                        .into_iter()
                        .map(|v| v.try_into())
                        .collect::<Result<_, _>>()
                        .map_err(|e| #error_name::FieldConversion {
                            field: stringify!(#ident).to_string(),
                            source: Box::new(e),
                        })?
                }
            } else if needs_try_into_conversion(&inner_ty) {
                // Convert each element for primitives that need size conversion
                quote! {
                    #ident: proto.#ident
                        .iter()
                        .map(|v| (*v).try_into())
                        .collect::<Result<_, _>>()
                        .map_err(|e| #error_name::FieldConversion {
                            field: stringify!(#ident).to_string(),
                            source: Box::new(e),
                        })?
                }
            } else {
                quote! { #ident: proto.#ident }
            }
        } else {
            quote! { #ident: proto.#ident }
        }
    } else if parsed.is_option {
        if let Type::Path(type_path) = field_ty
            && let Some(segment) = type_path.path.segments.last()
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
        {
            if field_config.is_message {
                return quote! { #ident: proto.#ident };
            } else if is_complex_type(inner_ty) {
                return quote! {
                    #ident: proto.#ident
                        .map(|v| v.try_into())
                        .transpose()
                        .map_err(|e| #error_name::FieldConversion {
                            field: stringify!(#ident).to_string(),
                            source: Box::new(e),
                        })?
                };
            } else if needs_try_into_conversion(inner_ty) {
                return quote! {
                    #ident: proto.#ident
                        .map(|v| v.try_into())
                        .transpose()
                        .map_err(|e| #error_name::FieldConversion {
                            field: stringify!(#ident).to_string(),
                            source: Box::new(e),
                        })?
                };
            }
        }
        quote! { #ident: proto.#ident }
    } else if parsed.is_message_like {
        if field_config.is_message {
            quote! {
                #ident: proto.#ident
                    .ok_or_else(|| #error_name::MissingField { field: stringify!(#ident).to_string() })?
            }
        } else {
            quote! {
                #ident: proto.#ident
                    .ok_or_else(|| #error_name::MissingField { field: stringify!(#ident).to_string() })?
                    .try_into()
                    .map_err(|e| #error_name::FieldConversion {
                        field: stringify!(#ident).to_string(),
                        source: Box::new(e),
                    })?
            }
        }
    } else {
        generate_primitive_from_proto(ident, field_ty, error_name)
    }
}
