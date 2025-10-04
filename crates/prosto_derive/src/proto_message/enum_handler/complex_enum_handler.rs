use proc_macro::TokenStream;
use quote::quote;
use syn::DataEnum;
use syn::DeriveInput;
use syn::Fields;
use syn::Type;

use crate::proto_message::utils::ProtoConfig;
use crate::utils::strip_proto_suffix;
use crate::utils::*;
use crate::write_file::register_import;
use crate::write_file::write_proto_file;

struct NestedMessage {
    proto_def: String,
    fields: Vec<NestedField>,
}

struct NestedField {
    name: syn::Ident,
    ty: syn::Type,
    parsed: ParsedFieldType,
    config: FieldConfig,
}

pub fn handle_complex_enum(input: DeriveInput, data: &DataEnum, config: ProtoConfig) -> TokenStream {
    let name = &input.ident;
    let proto_name = syn::Ident::new(&format!("{}Proto", name), name.span());
    let error_name = syn::Ident::new(&format!("{}ConversionError", name), name.span());

    let oneof_mod_name = syn::Ident::new(&to_snake_case(&proto_name.to_string()), name.span());
    let oneof_enum_name = syn::Ident::new("Value", name.span());

    let mut proto_fields = String::new();
    let mut oneof_variants = Vec::new();
    let mut tags = Vec::new();
    let mut to_proto_arms = Vec::new();
    let mut from_proto_arms = Vec::new();
    let mut nested_messages = Vec::new();
    let mut nested_message_structs = Vec::new();

    for (idx, variant) in data.variants.iter().enumerate() {
        let tag = idx + 1;
        tags.push(tag);

        let variant_ident = &variant.ident;
        let field_name_snake = to_snake_case(&variant_ident.to_string());

        match &variant.fields {
            // Unit variant: First
            Fields::Unit => {
                let empty_msg_name = format!("{}{}", proto_name, variant_ident);
                proto_fields.push_str(&format!("    {} {} = {};\n", empty_msg_name, field_name_snake, tag));

                let empty_msg_proto = format!("message {} {{}}\n\n", empty_msg_name);
                let empty_msg_ident = syn::Ident::new(&empty_msg_name, variant_ident.span());

                nested_messages.push(NestedMessage {
                    proto_def: empty_msg_proto,
                    fields: Vec::new(),
                });

                nested_message_structs.push(quote! {
                    #[derive(::prost::Message, Clone, PartialEq)]
                    pub struct #empty_msg_ident {}
                });

                let prost_attr = quote! { #[prost(message, tag = #tag)] };
                oneof_variants.push(quote! {
                    #prost_attr
                    #variant_ident(super::#empty_msg_ident)
                });

                to_proto_arms.push(quote! {
                    #name::#variant_ident => #oneof_mod_name::#oneof_enum_name::#variant_ident(#empty_msg_ident {})
                });

                from_proto_arms.push(quote! {
                    #oneof_mod_name::#oneof_enum_name::#variant_ident(_) => #name::#variant_ident
                });
            }

            // Unnamed variant: Second(Address)
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    panic!("Complex enum unnamed variants must have exactly one field");
                }

                let field_ty = &fields.unnamed.first().unwrap().ty;
                let parsed = parse_field_type(field_ty);

                let proto_type_str = if parsed.is_message_like {
                    let rust_name = rust_type_path_ident(&parsed.proto_rust_type).to_string();
                    strip_proto_suffix(&rust_name)
                } else {
                    parsed.proto_type.clone()
                };

                proto_fields.push_str(&format!("    {} {} = {};\n", proto_type_str, field_name_snake, tag));

                let oneof_field_ty = if parsed.is_message_like {
                    let proto_type = &parsed.proto_rust_type;
                    quote! { super::#proto_type }
                } else {
                    let ty = &parsed.rust_type;
                    quote! { #ty }
                };

                let prost_type_tokens = &parsed.prost_type;
                let prost_attr = quote! { #[prost(#prost_type_tokens, tag = #tag)] };

                oneof_variants.push(quote! {
                    #prost_attr
                    #variant_ident(#oneof_field_ty)
                });

                let to_value = if parsed.is_message_like {
                    quote! { inner.to_proto() }
                } else {
                    quote! { inner.clone() }
                };

                to_proto_arms.push(quote! {
                    #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(#to_value)
                });

                let from_value = if parsed.is_message_like {
                    let variant_name_str = variant_ident.to_string();
                    quote! {
                        #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                            let converted = #field_ty::from_proto(inner)
                                .map_err(|e| #error_name::VariantConversion {
                                    variant: #variant_name_str.to_string(),
                                    source: e,
                                })?;
                            #name::#variant_ident(converted)
                        }
                    }
                } else {
                    quote! {
                        #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => #name::#variant_ident(inner)
                    }
                };

                from_proto_arms.push(from_value);
            }

            // Named variant: Third { id: u64, address: Address }
            Fields::Named(fields) => {
                let nested_msg_name = format!("{}{}", proto_name, variant_ident);
                let nested_msg_ident = syn::Ident::new(&nested_msg_name, variant_ident.span());

                let mut nested_proto_fields = String::new();
                let mut nested_field_info = Vec::new();
                let mut prost_fields = Vec::new();
                let mut skip_with_fn = Vec::new();
                let mut skip_with_default = Vec::new();

                let mut field_tag = 0;
                for field in fields.named.iter() {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_ty = &field.ty;
                    let field_config = parse_field_config(field);

                    // Handle skip fields
                    if field_config.skip {
                        if let Some(ref deser_fn) = field_config.skip_deser_fn {
                            let deser_fn_ident: syn::Ident = syn::parse_str(deser_fn).expect("Invalid deser function name");
                            skip_with_fn.push((
                                field_name.clone(),
                                quote! {
                                    let #field_name = #deser_fn_ident(&inner);
                                },
                            ));
                        } else {
                            skip_with_default.push(field_name.clone());
                        }
                        continue; // Skip this field in proto
                    }

                    field_tag += 1;

                    // Determine the type to use for proto generation
                    let ty_for_parsing = if let Some(ref into_type) = field_config.into_type {
                        syn::parse_str::<Type>(into_type).unwrap_or_else(|_| field_ty.clone())
                    } else {
                        field_ty.clone()
                    };

                    let parsed = parse_field_type(&ty_for_parsing);

                    // Determine proto type string for .proto file
                    let proto_type_str = if field_config.is_rust_enum || field_config.is_proto_enum {
                        // Enum handling
                        let (base_enum_type, _, _) = extract_wrapper_info(&ty_for_parsing);
                        let enum_name = rust_type_path_ident(&base_enum_type).to_string();

                        if let Some(ref import_path) = field_config.import_path {
                            let qualified_name = format!("{}.{}", import_path, enum_name);
                            if parsed.is_repeated {
                                format!("repeated {}", qualified_name)
                            } else if parsed.is_option {
                                format!("optional {}", qualified_name)
                            } else {
                                qualified_name
                            }
                        } else if parsed.is_repeated {
                            format!("repeated {}", enum_name)
                        } else if parsed.is_option {
                            format!("optional {}", enum_name)
                        } else {
                            enum_name
                        }
                    } else if let Some(ref import_path) = field_config.import_path {
                        // Imported message with import_path
                        let base_type_name = if field_config.is_message {
                            rust_type_path_ident(&extract_wrapper_info(&ty_for_parsing).0).to_string()
                        } else {
                            rust_type_path_ident(&parsed.proto_rust_type).to_string()
                        };
                        let qualified_name = format!("{}.{}", import_path, base_type_name);

                        if parsed.is_repeated {
                            format!("repeated {}", qualified_name)
                        } else if parsed.is_option {
                            format!("optional {}", qualified_name)
                        } else {
                            qualified_name
                        }
                    } else if field_config.is_message || parsed.is_message_like {
                        let base_type = if field_config.is_message {
                            rust_type_path_ident(&extract_wrapper_info(&ty_for_parsing).0).to_string()
                        } else {
                            let rust_name = rust_type_path_ident(&parsed.proto_rust_type).to_string();
                            strip_proto_suffix(&rust_name)
                        };
                        if parsed.is_repeated {
                            format!("repeated {}", base_type)
                        } else if parsed.is_option {
                            format!("optional {}", base_type)
                        } else {
                            base_type
                        }
                    } else {
                        // Primitive type
                        if parsed.is_repeated {
                            format!("repeated {}", parsed.proto_type)
                        } else if parsed.is_option {
                            format!("optional {}", parsed.proto_type)
                        } else {
                            parsed.proto_type.clone()
                        }
                    };

                    nested_proto_fields.push_str(&format!("  {} {} = {};\n", proto_type_str, field_name, field_tag));

                    // Register import if needed
                    if let Some(ref import_path) = field_config.import_path {
                        register_import(&config.file_name, import_path);
                    }

                    nested_field_info.push(NestedField {
                        name: field_name.clone(),
                        ty: field_ty.clone(),
                        parsed: parsed.clone(),
                        config: field_config.clone(),
                    });

                    // Determine the Rust type for the proto struct field
                    let field_ty_tokens = if field_config.into_type.is_some() {
                        // Custom conversion - use the into_type
                        quote! { #ty_for_parsing }
                    } else if field_config.is_rust_enum || field_config.is_proto_enum {
                        // Enum fields use i32
                        if parsed.is_option {
                            quote! { Option<i32> }
                        } else if parsed.is_repeated {
                            quote! { Vec<i32> }
                        } else {
                            quote! { i32 }
                        }
                    } else {
                        get_proto_field_type(&parsed, field_ty, &field_config)
                    };

                    // Determine prost attribute
                    let prost_attr = if field_config.into_type.is_some() {
                        // Custom conversion
                        let prost_type_tokens = &parsed.prost_type;
                        quote! { #prost_type_tokens }
                    } else if field_config.is_rust_enum || field_config.is_proto_enum {
                        // Enum with enumeration attribute
                        let (base_enum_type, _, _) = extract_wrapper_info(&ty_for_parsing);
                        let enum_name = rust_type_path_ident(&base_enum_type).to_string();
                        if parsed.is_repeated {
                            quote! { enumeration = #enum_name, repeated }
                        } else if parsed.is_option {
                            quote! { enumeration = #enum_name, optional }
                        } else {
                            quote! { enumeration = #enum_name }
                        }
                    } else if parsed.is_message_like {
                        let base_type = &parsed.prost_type; // "message"
                        if parsed.is_repeated {
                            quote! { #base_type, repeated }
                        } else if parsed.is_option {
                            quote! { #base_type, optional }
                        } else {
                            quote! { #base_type }
                        }
                    } else {
                        // Primitive type
                        let base_type = &parsed.prost_type;
                        if parsed.is_repeated {
                            quote! { #base_type, repeated }
                        } else if parsed.is_option {
                            quote! { #base_type, optional }
                        } else {
                            quote! { #base_type }
                        }
                    };

                    prost_fields.push(quote! {
                        #[prost(#prost_attr, tag = #field_tag)]
                        pub #field_name: #field_ty_tokens
                    });
                }

                let nested_proto_def = format!("message {} {{\n{}}}\n\n", nested_msg_name, nested_proto_fields);

                nested_messages.push(NestedMessage {
                    proto_def: nested_proto_def,
                    fields: nested_field_info,
                });

                nested_message_structs.push(quote! {
                    #[derive(::prost::Message, Clone, PartialEq)]
                    pub struct #nested_msg_ident {
                        #(#prost_fields),*
                    }
                });

                proto_fields.push_str(&format!("    {} {} = {};\n", nested_msg_name, field_name_snake, tag));

                let prost_attr = quote! { #[prost(message, tag = #tag)] };
                oneof_variants.push(quote! {
                    #prost_attr
                    #variant_ident(super::#nested_msg_ident)
                });

                // Generate to_proto conversion for named variant
                // Create field patterns - skipped fields use `field: _` to avoid unused warnings
                let field_patterns: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| {
                        let field_name = f.ident.as_ref().unwrap();
                        let field_config = parse_field_config(f);
                        if field_config.skip {
                            quote! { #field_name: _ }
                        } else {
                            quote! { #field_name }
                        }
                    })
                    .collect();

                let field_conversions_to: Vec<_> = nested_messages
                    .last()
                    .unwrap()
                    .fields
                    .iter()
                    .map(|nf| {
                        let field_name = &nf.name;

                        // Custom conversion
                        if nf.config.into_fn.is_some() {
                            let into_fn: syn::Ident = syn::parse_str(nf.config.into_fn.as_ref().unwrap()).unwrap();
                            return quote! { #field_name: #into_fn(&#field_name) };
                        }

                        // Rust enum conversion
                        if nf.config.is_rust_enum {
                            if nf.parsed.is_option {
                                return quote! { #field_name: #field_name.as_ref().map(|v| v.to_proto() as i32) };
                            } else if nf.parsed.is_repeated {
                                return quote! { #field_name: #field_name.iter().map(|v| v.to_proto() as i32).collect() };
                            } else {
                                return quote! { #field_name: #field_name.to_proto() as i32 };
                            }
                        }

                        // Proto enum conversion
                        if nf.config.is_proto_enum {
                            if nf.parsed.is_option {
                                return quote! { #field_name: #field_name.map(|v| v as i32) };
                            } else if nf.parsed.is_repeated {
                                return quote! { #field_name: #field_name.iter().cloned().map(|v| v as i32).collect() };
                            } else {
                                return quote! { #field_name: (*#field_name) as i32 };
                            }
                        }

                        // Message handling
                        if nf.parsed.is_message_like || nf.config.is_message {
                            if nf.parsed.is_option {
                                if nf.config.is_message {
                                    quote! { #field_name: #field_name.clone() }
                                } else {
                                    quote! { #field_name: #field_name.as_ref().map(|v| v.to_proto()) }
                                }
                            } else if nf.parsed.is_repeated {
                                if nf.config.is_message {
                                    quote! { #field_name: #field_name.clone() }
                                } else {
                                    quote! { #field_name: #field_name.iter().map(|v| v.to_proto()).collect() }
                                }
                            } else if nf.config.is_message {
                                quote! { #field_name: Some(#field_name.clone()) }
                            } else {
                                quote! { #field_name: Some(#field_name.to_proto()) }
                            }
                        } else {
                            // Primitives
                            quote! { #field_name: #field_name.clone() }
                        }
                    })
                    .collect();

                to_proto_arms.push(quote! {
                    #name::#variant_ident { #(#field_patterns),* } => {
                        #oneof_mod_name::#oneof_enum_name::#variant_ident(
                            #nested_msg_ident {
                                #(#field_conversions_to),*
                            }
                        )
                    }
                });

                // Generate from_proto conversion for named variant
                let variant_name_str = variant_ident.to_string();

                let field_conversions_from: Vec<_> = nested_messages
                    .last()
                    .unwrap()
                    .fields
                    .iter()
                    .map(|nf| {
                        let field_name = &nf.name;
                        let field_ty = &nf.ty;

                        // Custom conversion
                        if nf.config.from_fn.is_some() {
                            let from_fn: syn::Ident = syn::parse_str(nf.config.from_fn.as_ref().unwrap()).unwrap();
                            return quote! { #field_name: #from_fn(inner.#field_name) };
                        }

                        // Rust enum conversion
                        if nf.config.is_rust_enum {
                            let (enum_type, is_option, is_repeated) = extract_wrapper_info(field_ty);
                            let enum_ident = rust_type_path_ident(&enum_type);

                            if is_option {
                                return quote! {
                                    #field_name: inner.#field_name
                                        .map(|v| #enum_ident::try_from(v))
                                        .transpose()
                                        .map_err(|e| #error_name::VariantConversion {
                                            variant: #variant_name_str.to_string(),
                                            source: Box::new(e),
                                        })?
                                };
                            } else if is_repeated {
                                return quote! {
                                    #field_name: inner.#field_name
                                        .into_iter()
                                        .map(|v| #enum_ident::try_from(v))
                                        .collect::<Result<Vec<_>, _>>()
                                        .map_err(|e| #error_name::VariantConversion {
                                            variant: #variant_name_str.to_string(),
                                            source: Box::new(e),
                                        })?
                                };
                            } else {
                                return quote! {
                                    #field_name: #enum_ident::try_from(inner.#field_name)
                                        .map_err(|e| #error_name::VariantConversion {
                                            variant: #variant_name_str.to_string(),
                                            source: Box::new(e),
                                        })?
                                };
                            }
                        }

                        // Proto enum conversion
                        if nf.config.is_proto_enum {
                            let (enum_type, is_option, is_repeated) = extract_wrapper_info(field_ty);
                            let enum_ident = rust_type_path_ident(&enum_type);

                            if is_option {
                                return quote! {
                                    #field_name: inner.#field_name
                                        .map(|v| #enum_ident::try_from(v))
                                        .transpose()
                                        .map_err(|e| #error_name::VariantConversion {
                                            variant: #variant_name_str.to_string(),
                                            source: Box::new(e),
                                        })?
                                };
                            } else if is_repeated {
                                return quote! {
                                    #field_name: inner.#field_name
                                        .into_iter()
                                        .map(|v| #enum_ident::try_from(v))
                                        .collect::<Result<Vec<_>, _>>()
                                        .map_err(|e| #error_name::VariantConversion {
                                            variant: #variant_name_str.to_string(),
                                            source: Box::new(e),
                                        })?
                                };
                            } else {
                                return quote! {
                                    #field_name: #enum_ident::try_from(inner.#field_name)
                                        .map_err(|e| #error_name::VariantConversion {
                                            variant: #variant_name_str.to_string(),
                                            source: Box::new(e),
                                        })?
                                };
                            }
                        }

                        // Message handling
                        if nf.parsed.is_message_like || nf.config.is_message {
                            if nf.parsed.is_option {
                                if nf.config.is_message {
                                    quote! { #field_name: inner.#field_name }
                                } else {
                                    let inner_ty = extract_option_inner_type(field_ty);
                                    quote! {
                                        #field_name: match inner.#field_name {
                                            Some(v) => Some(#inner_ty::from_proto(v)
                                                .map_err(|e| #error_name::VariantConversion {
                                                    variant: #variant_name_str.to_string(),
                                                    source: e,
                                                })?),
                                            None => None,
                                        }
                                    }
                                }
                            } else if nf.parsed.is_repeated {
                                if nf.config.is_message {
                                    quote! { #field_name: inner.#field_name }
                                } else {
                                    let inner_ty = extract_vec_inner_type(field_ty);
                                    quote! {
                                        #field_name: inner.#field_name.into_iter()
                                            .map(|v| #inner_ty::from_proto(v))
                                            .collect::<Result<Vec<_>, _>>()
                                            .map_err(|e| #error_name::VariantConversion {
                                                variant: #variant_name_str.to_string(),
                                                source: e,
                                            })?
                                    }
                                }
                            } else if nf.config.is_message {
                                quote! {
                                    #field_name: inner.#field_name
                                        .ok_or_else(|| #error_name::VariantConversion {
                                            variant: #variant_name_str.to_string(),
                                            source: Box::new(std::io::Error::new(
                                                std::io::ErrorKind::InvalidData,
                                                format!("Missing required field: {}", stringify!(#field_name))
                                            )),
                                        })?
                                }
                            } else {
                                quote! {
                                    #field_name: #field_ty::from_proto(
                                        inner.#field_name.ok_or_else(|| #error_name::VariantConversion {
                                            variant: #variant_name_str.to_string(),
                                            source: Box::new(std::io::Error::new(
                                                std::io::ErrorKind::InvalidData,
                                                format!("Missing required field: {}", stringify!(#field_name))
                                            )),
                                        })?
                                    )
                                    .map_err(|e| #error_name::VariantConversion {
                                        variant: #variant_name_str.to_string(),
                                        source: e,
                                    })?
                                }
                            }
                        } else {
                            // Primitives
                            quote! { #field_name: inner.#field_name }
                        }
                    })
                    .collect();

                let skip_field_assignments: Vec<_> = skip_with_fn
                    .iter()
                    .map(|(field_name, _)| {
                        quote! { #field_name }
                    })
                    .collect();

                let skip_default_assignments: Vec<_> = skip_with_default
                    .iter()
                    .map(|field_name| {
                        quote! { #field_name: Default::default() }
                    })
                    .collect();

                let skip_computations: Vec<_> = skip_with_fn.iter().map(|(_, computation)| computation.clone()).collect();

                from_proto_arms.push(quote! {
                    #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                        #(#skip_computations)*
                        #name::#variant_ident {
                            #(#field_conversions_from,)*
                            #(#skip_field_assignments,)*
                            #(#skip_default_assignments),*
                        }
                    }
                });
            }
        }
    }

    // Write proto file with nested messages
    let mut full_proto_def = String::new();
    for nested_msg in &nested_messages {
        full_proto_def.push_str(&nested_msg.proto_def);
    }
    full_proto_def.push_str(&format!("message {} {{\n  oneof value {{\n{}}}\n}}\n\n", proto_name, proto_fields));

    write_proto_file(&config.file_name, &full_proto_def);

    // Generate code
    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto_message")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    let original_variants: Vec<_> = data
        .variants
        .iter()
        .map(|v| {
            let attrs: Vec<_> = v.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
            let ident = &v.ident;

            // Filter proto attributes from fields as well
            let fields = match &v.fields {
                Fields::Named(fields_named) => {
                    let filtered_fields: Vec<_> = fields_named
                        .named
                        .iter()
                        .map(|f| {
                            let field_attrs: Vec<_> = f.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
                            let field_ident = &f.ident;
                            let field_ty = &f.ty;
                            // Don't add pub visibility - enum fields inherit from enum
                            quote! { #(#field_attrs)* #field_ident: #field_ty }
                        })
                        .collect();
                    quote! { { #(#filtered_fields),* } }
                }
                Fields::Unnamed(fields_unnamed) => {
                    let filtered_fields: Vec<_> = fields_unnamed
                        .unnamed
                        .iter()
                        .map(|f| {
                            let field_attrs: Vec<_> = f.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
                            let field_ty = &f.ty;
                            quote! { #(#field_attrs)* #field_ty }
                        })
                        .collect();
                    quote! { ( #(#filtered_fields),* ) }
                }
                Fields::Unit => quote! {},
            };

            quote! { #(#attrs)* #ident #fields }
        })
        .collect();

    let tags_str = tags.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ");
    let oneof_path = format!("{}::Value", to_snake_case(&proto_name.to_string()));

    quote! {
        // Nested message structs (for unit and named variants)
        #(#nested_message_structs)*

        // Original enum
        #(#attrs)*
        #vis enum #name #generics {
            #(#original_variants),*
        }

        // Proto message with oneof
        #[derive(::prost::Message, Clone, PartialEq)]
        #vis struct #proto_name #generics {
            #[prost(oneof = #oneof_path, tags = #tags_str)]
            pub value: ::core::option::Option<#oneof_mod_name::#oneof_enum_name #generics>
        }

        // Oneof enum module
        pub mod #oneof_mod_name {
            #[derive(::prost::Oneof, Clone, PartialEq)]
            pub enum #oneof_enum_name {
                #(#oneof_variants),*
            }
        }

        // Error type
        #[derive(Debug)]
        #vis enum #error_name {
            MissingValue,
            VariantConversion { variant: String, source: Box<dyn std::error::Error> },
        }

        impl std::fmt::Display for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::MissingValue => write!(f, "Missing oneof value in Proto message"),
                    Self::VariantConversion { variant, source } =>
                        write!(f, "Error converting variant {}: {}", variant, source),
                }
            }
        }

        impl std::error::Error for #error_name {}

        // HasProto implementation
        impl #generics HasProto for #name #generics {
            type Proto = #proto_name #generics;

            fn to_proto(&self) -> Self::Proto {
                let value = match self {
                    #(#to_proto_arms),*
                };
                #proto_name { value: Some(value) }
            }

            fn from_proto(proto: Self::Proto) -> Result<Self, Box<dyn std::error::Error>> {
                match proto.value {
                    Some(oneof_value) => Ok(match oneof_value {
                        #(#from_proto_arms),*
                    }),
                    None => Err(Box::new(#error_name::MissingValue)),
                }
            }
        }

        impl #generics From<#name #generics> for #proto_name #generics {
            fn from(value: #name #generics) -> Self {
                value.to_proto()
            }
        }

        impl #generics TryFrom<#proto_name #generics> for #name #generics {
            type Error = #error_name;

            fn try_from(proto: #proto_name #generics) -> Result<Self, Self::Error> {
                Self::from_proto(proto).map_err(|e| {
                    if let Some(conv_err) = e.downcast_ref::<#error_name>() {
                        match conv_err {
                            #error_name::MissingValue => #error_name::MissingValue,
                            #error_name::VariantConversion { variant, .. } =>
                                #error_name::VariantConversion {
                                    variant: variant.clone(),
                                    source: Box::new(std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        "Conversion error"
                                    )),
                                },
                        }
                    } else {
                        #error_name::MissingValue
                    }
                })
            }
        }
    }
    .into()
}

// Helper function to extract inner type from Option<T>
fn extract_option_inner_type(ty: &syn::Type) -> &syn::Type {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        return inner;
    }
    ty
}

// Helper function to extract inner type from Vec<T>
fn extract_vec_inner_type(ty: &syn::Type) -> &syn::Type {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Vec"
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        return inner;
    }
    ty
}

// Helper to extract wrapper info (returns base type, is_option, is_repeated)
fn extract_wrapper_info(ty: &Type) -> (Type, bool, bool) {
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

fn get_proto_field_type(parsed: &ParsedFieldType, field_ty: &Type, field_config: &FieldConfig) -> proc_macro2::TokenStream {
    if field_config.is_message {
        // Imported message - still needs proper wrapping for prost
        if parsed.is_option {
            // Already wrapped in Option
            quote! { #field_ty }
        } else if parsed.is_repeated {
            // Vec<T> stays as is
            quote! { #field_ty }
        } else {
            // Non-wrapped message needs Option wrapper for prost
            quote! { ::core::option::Option<#field_ty> }
        }
    } else if parsed.is_message_like {
        if parsed.is_option {
            // Extract inner type from Option<T> and convert to proto
            let inner_ty = extract_option_inner_type(field_ty);
            let inner_parsed = parse_field_type(inner_ty);
            let proto_type = &inner_parsed.proto_rust_type;
            quote! { ::core::option::Option<#proto_type> }
        } else if parsed.is_repeated {
            // Extract inner type from Vec<T> and convert to proto
            let inner_ty = extract_vec_inner_type(field_ty);
            let inner_parsed = parse_field_type(inner_ty);
            let proto_type = &inner_parsed.proto_rust_type;
            quote! { ::std::vec::Vec<#proto_type> }
        } else {
            // Direct message type (no wrapper) - prost requires message fields to be wrapped in Option
            let proto_type = &parsed.proto_rust_type;
            quote! { ::core::option::Option<#proto_type> }
        }
    } else {
        // For primitives: use the original field_ty which preserves Option<>/Vec<> wrappers
        quote! { #field_ty }
    }
}
