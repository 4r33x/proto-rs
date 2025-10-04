use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::Fields;
use syn::Type;

use crate::proto_message::utils::ProtoConfig;
use crate::utils::*;
use crate::write_file::write_proto_file;

pub fn handle_struct(input: DeriveInput, data: &syn::DataStruct, config: ProtoConfig) -> TokenStream {
    let name = &input.ident;
    let proto_name = syn::Ident::new(&format!("{}Proto", name), name.span());
    let error_name = syn::Ident::new(&format!("{}ConversionError", name), name.span());

    let proto_def = generate_struct_proto(&name.to_string(), &data.fields);

    write_proto_file(&config.file_name, &proto_def);

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
                // Custom conversion path
                let into_type_str = field_config.into_type.as_ref().unwrap();
                let into_type: Type = syn::parse_str(into_type_str).expect("Invalid into type");
                let into_fn_str = field_config.into_fn.as_ref().unwrap();
                let into_fn: syn::Ident = syn::parse_str(into_fn_str).expect("Invalid into_fn");
                let from_fn_str = field_config.from_fn.as_ref().unwrap();
                let from_fn: syn::Ident = syn::parse_str(from_fn_str).expect("Invalid from_fn");

                // Parse the into_type to get prost attribute
                let parsed = parse_field_type(&into_type);

                let proto_ty = quote! { #into_type };
                let prost_type_tokens = &parsed.prost_type;
                let prost_attr = quote! { #[prost(#prost_type_tokens, tag = #field_num)] };
                let to_value = quote! { #ident: #into_fn(&self.#ident) };
                let from_value = quote! { #ident: #from_fn(proto.#ident) };

                (proto_ty, prost_attr, to_value, from_value)
            } else if field_config.is_rust_enum {
                // Handle enum fields - use i32 in proto, enumeration attribute
                // Extract the enum type from Option<T> or Vec<T> if needed
                let (enum_type, is_option, is_repeated) = if is_option_type(&field.ty) {
                    // Option<Status>
                    if let Type::Path(type_path) = &field.ty {
                        if let Some(segment) = type_path.path.segments.last() {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                    (inner_ty.clone(), true, false)
                                } else {
                                    panic!("Invalid Option type for enum field");
                                }
                            } else {
                                panic!("Invalid Option type for enum field");
                            }
                        } else {
                            panic!("Invalid Option type for enum field");
                        }
                    } else {
                        panic!("Invalid Option type for enum field");
                    }
                } else if let Some(inner_ty) = vec_inner_type(&field.ty) {
                    // Vec<Status>
                    (inner_ty, false, true)
                } else {
                    // Status
                    (field.ty.clone(), false, false)
                };

                // Proto field type is i32 based
                let proto_field_ty = if is_option {
                    quote! { Option<i32> }
                } else if is_repeated {
                    quote! { Vec<i32> }
                } else {
                    quote! { i32 }
                };

                // Prost attribute with enumeration
                let enum_ident = rust_type_path_ident(&enum_type);
                let enum_name = enum_ident.to_string();
                let prost_attr = if is_repeated {
                    quote! { #[prost(enumeration = #enum_name, repeated, tag = #field_num)] }
                } else if is_option {
                    quote! { #[prost(enumeration = #enum_name, optional, tag = #field_num)] }
                } else {
                    quote! { #[prost(enumeration = #enum_name, tag = #field_num)] }
                };
                // Conversion logic
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
            } else if field_config.is_proto_enum {
                // NEW: Handle proto-native enum fields
                // These are already prost::Enumeration types, use as-is

                let (enum_type, is_option, is_repeated) = if is_option_type(&field.ty) {
                    // Option<TestEnum>
                    if let Type::Path(type_path) = &field.ty {
                        if let Some(segment) = type_path.path.segments.last() {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                    (inner_ty.clone(), true, false)
                                } else {
                                    panic!("Invalid Option type for proto enum field");
                                }
                            } else {
                                panic!("Invalid Option type for proto enum field");
                            }
                        } else {
                            panic!("Invalid Option type for proto enum field");
                        }
                    } else {
                        panic!("Invalid Option type for proto enum field");
                    }
                } else if let Some(inner_ty) = vec_inner_type(&field.ty) {
                    // Vec<TestEnum>
                    (inner_ty, false, true)
                } else {
                    // TestEnum
                    (field.ty.clone(), false, false)
                };

                // Proto field type - proto enums are stored as i32
                let proto_field_ty = if is_option {
                    quote! { Option<i32> }
                } else if is_repeated {
                    quote! { Vec<i32> }
                } else {
                    quote! { i32 }
                };

                // Prost attribute with enumeration
                let enum_ident = rust_type_path_ident(&enum_type);
                let enum_name = enum_ident.to_string();
                let prost_attr = if is_repeated {
                    quote! { #[prost(enumeration = #enum_name, repeated, tag = #field_num)] }
                } else if is_option {
                    quote! { #[prost(enumeration = #enum_name, optional, tag = #field_num)] }
                } else {
                    quote! { #[prost(enumeration = #enum_name, tag = #field_num)] }
                };

                // Conversion logic - proto-native enums are already i32 compatible
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
            } else {
                // Standard conversion path
                let ParsedFieldType {
                    rust_type,
                    prost_type,
                    is_option,
                    is_repeated,
                    is_message_like,
                    proto_rust_type,
                    proto_type: _,
                } = parse_field_type(&field.ty);

                // Proto field type - check is_repeated, then is_option, then is_message_like
                let proto_field_ty = if is_repeated {
                    if let Some(inner_ty) = vec_inner_type(&field.ty) {
                        if is_bytes_vec(&field.ty) {
                            quote! { ::std::vec::Vec<u8> }
                        } else {
                            // Check if this is an imported message
                            if field_config.is_message {
                                // Use the type as-is, no Proto suffix
                                quote! { ::std::vec::Vec<#inner_ty> }
                            } else {
                                let inner_proto = convert_field_type_to_proto(&inner_ty);
                                quote! { ::std::vec::Vec<#inner_proto> }
                            }
                        }
                    } else {
                        quote! { #rust_type }
                    }
                } else if is_option {
                    if let Type::Path(type_path) = &field.ty {
                        if let Some(segment) = type_path.path.segments.last() {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                    if field_config.is_message {
                                        // Use the type as-is, no Proto suffix
                                        quote! { ::core::option::Option<#inner_ty> }
                                    } else if is_complex_type(inner_ty) {
                                        let inner_proto = convert_field_type_to_proto(inner_ty);
                                        quote! { ::core::option::Option<#inner_proto> }
                                    } else {
                                        let ty = &field.ty;
                                        quote! { #ty }
                                    }
                                } else {
                                    let ty = &field.ty;
                                    quote! { #ty }
                                }
                            } else {
                                let ty = &field.ty;
                                quote! { #ty }
                            }
                        } else {
                            let ty = &field.ty;
                            quote! { #ty }
                        }
                    } else {
                        let ty = &field.ty;
                        quote! { #ty }
                    }
                } else if is_message_like {
                    if field_config.is_message {
                        // Use the type as-is, no Proto suffix
                        let ty = &field.ty;
                        quote! { ::core::option::Option<#ty> }
                    } else {
                        quote! { ::core::option::Option<#proto_rust_type> }
                    }
                } else {
                    let ty = &field.ty;
                    quote! { #ty }
                };

                // prost attribute
                let prost_attr = if is_repeated {
                    quote! { #[prost(#prost_type, repeated, tag = #field_num)] }
                } else if is_option || is_message_like {
                    quote! { #[prost(#prost_type, optional, tag = #field_num)] }
                } else {
                    quote! { #[prost(#prost_type, tag = #field_num)] }
                };

                // to_proto conversion
                let to_proto_value = if is_repeated {
                    if let Some(inner_ty) = vec_inner_type(&field.ty) {
                        if is_bytes_vec(&field.ty) {
                            quote! { #ident: self.#ident.clone() }
                        } else if field_config.is_message {
                            // Imported message - no conversion needed
                            quote! { #ident: self.#ident.clone() }
                        } else if is_complex_type(&inner_ty) {
                            quote! { #ident: self.#ident.iter().map(|v| v.to_proto()).collect() }
                        } else {
                            quote! { #ident: self.#ident.clone() }
                        }
                    } else {
                        quote! { #ident: self.#ident.clone() }
                    }
                } else if is_option {
                    if let Type::Path(type_path) = &field.ty {
                        if let Some(segment) = type_path.path.segments.last() {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                    if field_config.is_message {
                                        // Imported message - no conversion needed
                                        quote! { #ident: self.#ident.clone() }
                                    } else if is_complex_type(inner_ty) {
                                        quote! { #ident: self.#ident.as_ref().map(|v| v.to_proto()) }
                                    } else {
                                        quote! { #ident: self.#ident.clone() }
                                    }
                                } else {
                                    quote! { #ident: self.#ident.clone() }
                                }
                            } else {
                                quote! { #ident: self.#ident.clone() }
                            }
                        } else {
                            quote! { #ident: self.#ident.clone() }
                        }
                    } else {
                        quote! { #ident: self.#ident.clone() }
                    }
                } else if is_message_like {
                    if field_config.is_message {
                        // Imported message - wrap in Some but no conversion
                        quote! { #ident: Some(self.#ident.clone()) }
                    } else {
                        quote! { #ident: Some(self.#ident.to_proto()) }
                    }
                } else {
                    quote! { #ident: self.#ident.clone() }
                };

                // from_proto conversion
                let from_proto_value = if is_repeated {
                    if let Some(inner_ty) = vec_inner_type(&field.ty) {
                        if is_bytes_vec(&field.ty) {
                            quote! { #ident: proto.#ident }
                        } else if field_config.is_message {
                            // Imported message - no conversion needed
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
                        } else {
                            quote! { #ident: proto.#ident }
                        }
                    } else {
                        quote! { #ident: proto.#ident }
                    }
                } else if is_option {
                    if let Type::Path(type_path) = &field.ty {
                        if let Some(segment) = type_path.path.segments.last() {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                    if field_config.is_message {
                                        // Imported message - no conversion needed
                                        quote! { #ident: proto.#ident }
                                    } else if is_complex_type(inner_ty) {
                                        quote! {
                                            #ident: proto.#ident
                                                .map(|v| v.try_into())
                                                .transpose()
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
                            } else {
                                quote! { #ident: proto.#ident }
                            }
                        } else {
                            quote! { #ident: proto.#ident }
                        }
                    } else {
                        quote! { #ident: proto.#ident }
                    }
                } else if is_message_like {
                    if field_config.is_message {
                        // Imported message - unwrap but no conversion
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
                    quote! { #ident: proto.#ident }
                };

                (proto_field_ty, prost_attr, to_proto_value, from_proto_value)
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
    .into()
}
