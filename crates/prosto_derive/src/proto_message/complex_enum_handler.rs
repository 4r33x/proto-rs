use proc_macro2::TokenStream;
use quote::quote;
use syn::DataEnum;
use syn::DeriveInput;
use syn::Fields;
use syn::FieldsNamed;
use syn::FieldsUnnamed;
use syn::Type;

use crate::utils::*;

pub fn handle_complex_enum(input: DeriveInput, data: &DataEnum) -> TokenStream {
    let name = &input.ident;
    let proto_name = syn::Ident::new(&format!("{}Proto", name), name.span());
    let error_name = syn::Ident::new(&format!("{}ConversionError", name), name.span());

    let oneof_mod_name = syn::Ident::new(&to_snake_case(&proto_name.to_string()), name.span());
    let oneof_enum_name = syn::Ident::new("Value", name.span());

    // Collections used for Rust/prost codegen
    let mut oneof_variants = Vec::new();
    let mut tags = Vec::new();
    let mut to_proto_arms = Vec::new();
    let mut from_proto_arms = Vec::new();
    let mut nested_message_structs = Vec::new();

    // iterate variants and fill the above collections using small helpers
    for (idx, variant) in data.variants.iter().enumerate() {
        let tag = idx + 1;
        tags.push(tag);
        match &variant.fields {
            Fields::Unit => {
                handle_unit_variant(
                    name,
                    &proto_name,
                    &oneof_mod_name,
                    &oneof_enum_name,
                    variant,
                    tag,
                    &mut nested_message_structs,
                    &mut oneof_variants,
                    &mut to_proto_arms,
                    &mut from_proto_arms,
                );
            }
            Fields::Unnamed(fields_unnamed) => {
                handle_unnamed_variant(
                    name,
                    &proto_name,
                    &oneof_mod_name,
                    &oneof_enum_name,
                    &error_name,
                    variant,
                    fields_unnamed,
                    tag,
                    &mut oneof_variants,
                    &mut to_proto_arms,
                    &mut from_proto_arms,
                );
            }
            Fields::Named(fields_named) => {
                handle_named_variant(
                    name,
                    &proto_name,
                    &oneof_mod_name,
                    &oneof_enum_name,
                    &error_name,
                    variant,
                    fields_named,
                    tag,
                    &mut nested_message_structs,
                    &mut oneof_variants,
                    &mut to_proto_arms,
                    &mut from_proto_arms,
                );
            }
        }
    }

    // Build original enum variants without proto attrs
    let attrs: Vec<_> = input.attrs.iter().filter(|a| !a.path().is_ident("proto_message")).collect();
    let vis = &input.vis;
    let generics = &input.generics;

    let original_variants: Vec<_> = data
        .variants
        .iter()
        .map(|v| {
            let attrs: Vec<_> = v.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
            let ident = &v.ident;

            let fields = match &v.fields {
                Fields::Named(fields_named) => {
                    let filtered_fields: Vec<_> = fields_named
                        .named
                        .iter()
                        .map(|f| {
                            let field_attrs: Vec<_> = f.attrs.iter().filter(|a| !a.path().is_ident("proto")).collect();
                            let field_ident = &f.ident;
                            let field_ty = &f.ty;
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

    // Final TokenStream (prost structs, oneof enum, conversions, errors, HasProto)
    quote! {
        // Nested message structs (for unit and named variants)
        #(#nested_message_structs)*

        // Original enum (without proto attributes)
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
}

/// Handle unit variants (Variant)
#[allow(clippy::too_many_arguments)]
fn handle_unit_variant(
    name: &syn::Ident,
    _proto_name: &syn::Ident,
    oneof_mod_name: &syn::Ident,
    oneof_enum_name: &syn::Ident,
    variant: &syn::Variant,
    tag: usize,
    nested_message_structs: &mut Vec<proc_macro2::TokenStream>,
    oneof_variants: &mut Vec<proc_macro2::TokenStream>,
    to_proto_arms: &mut Vec<proc_macro2::TokenStream>,
    from_proto_arms: &mut Vec<proc_macro2::TokenStream>,
) {
    let variant_ident = &variant.ident;
    let empty_msg_name = format!("{}{}", _proto_name, variant_ident);
    let empty_msg_ident = syn::Ident::new(&empty_msg_name, variant_ident.span());

    // prost empty nested message struct
    nested_message_structs.push(quote! {
        #[derive(::prost::Message, Clone, PartialEq)]
        pub struct #empty_msg_ident {}
    });

    // oneof variant (message)
    let prost_attr = quote! { #[prost(message, tag = #tag)] };
    oneof_variants.push(quote! {
        #prost_attr
        #variant_ident(super::#empty_msg_ident)
    });

    // to_proto arm
    to_proto_arms.push(quote! {
        #name::#variant_ident => #oneof_mod_name::#oneof_enum_name::#variant_ident(#empty_msg_ident {})
    });

    // from_proto arm
    from_proto_arms.push(quote! {
        #oneof_mod_name::#oneof_enum_name::#variant_ident(_) => #name::#variant_ident
    });
}

/// Handle unnamed (single-field tuple) variants (Variant(T))
#[allow(clippy::too_many_arguments)]
fn handle_unnamed_variant(
    name: &syn::Ident,
    _proto_name: &syn::Ident,
    oneof_mod_name: &syn::Ident,
    oneof_enum_name: &syn::Ident,
    error_name: &syn::Ident,
    variant: &syn::Variant,
    fields: &FieldsUnnamed,
    tag: usize,
    oneof_variants: &mut Vec<proc_macro2::TokenStream>,
    to_proto_arms: &mut Vec<proc_macro2::TokenStream>,
    from_proto_arms: &mut Vec<proc_macro2::TokenStream>,
) {
    if fields.unnamed.len() != 1 {
        panic!("Complex enum unnamed variants must have exactly one field");
    }

    let variant_ident = &variant.ident;
    let field_ty = &fields.unnamed.first().unwrap().ty;
    let parsed = parse_field_type(field_ty);

    // Build oneof field type tokens for prost
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

    // to_proto arm
    let to_value = if parsed.is_message_like {
        quote! { inner.to_proto() }
    } else {
        quote! { inner.clone() }
    };

    to_proto_arms.push(quote! {
        #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(#to_value)
    });

    // from_proto arm
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

/// Handle named-field variants (Variant { a: T, b: U })
#[allow(clippy::too_many_arguments)]
fn handle_named_variant(
    name: &syn::Ident,
    proto_name: &syn::Ident,
    oneof_mod_name: &syn::Ident,
    oneof_enum_name: &syn::Ident,
    error_name: &syn::Ident,
    variant: &syn::Variant,
    fields_named: &FieldsNamed,
    tag: usize,
    nested_message_structs: &mut Vec<proc_macro2::TokenStream>,
    oneof_variants: &mut Vec<proc_macro2::TokenStream>,
    to_proto_arms: &mut Vec<proc_macro2::TokenStream>,
    from_proto_arms: &mut Vec<proc_macro2::TokenStream>,
) {
    let variant_ident = &variant.ident;
    let nested_msg_name = format!("{}{}", proto_name, variant_ident);
    let nested_msg_ident = syn::Ident::new(&nested_msg_name, variant_ident.span());

    // Collect prost fields for the nested message Rust struct
    let mut prost_fields = Vec::new();
    // Keep a list of metadata for each field used to generate conversions below:
    // (field_ident, Option<ParsedFieldType>, FieldConfig, field_ty)
    let mut nested_fields_meta: Vec<(syn::Ident, ParsedFieldType, FieldConfig, syn::Type)> = Vec::new();

    // Skip handling helpers
    let mut skip_with_fn: Vec<(syn::Ident, proc_macro2::TokenStream)> = Vec::new();
    let mut skip_with_default: Vec<syn::Ident> = Vec::new();

    let mut field_tag = 0usize;
    for field in fields_named.named.iter() {
        let field_name = field.ident.as_ref().unwrap().clone();
        let field_ty = &field.ty;
        let field_config = parse_field_config(field);

        // Handle skip fields (we keep metadata but don't emit prost lines for them)
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

            continue;
        }

        field_tag += 1;

        // If there's an `into_type` override, use it for type parsing decisions
        let ty_for_parsing = if let Some(ref into_type) = field_config.into_type {
            syn::parse_str::<Type>(into_type).unwrap_or_else(|_| field_ty.clone())
        } else {
            field_ty.clone()
        };

        let parsed = parse_field_type(&ty_for_parsing);

        // Determine Rust type tokens for the prost struct field
        let field_ty_tokens = if field_config.into_type.is_some() {
            quote! { #ty_for_parsing }
        } else if field_config.is_rust_enum || field_config.is_proto_enum {
            // enums are represented as i32 / Option<i32> / Vec<i32> in prost structs
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

        // Build prost attribute for this field
        let prost_attr = if field_config.into_type.is_some() {
            let prost_type_tokens = &parsed.prost_type;
            quote! { #prost_type_tokens }
        } else if field_config.is_rust_enum || field_config.is_proto_enum {
            let (base_enum_type, _, _) = extract_wrapper_info(&ty_for_parsing);
            let enum_name = rust_type_path_ident(&base_enum_type).to_string();
            if parsed.is_repeated {
                quote! { enumeration = #enum_name, repeated }
            } else if parsed.is_option {
                quote! { enumeration = #enum_name, optional }
            } else {
                quote! { enumeration = #enum_name }
            }
        } else {
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

        nested_fields_meta.push((field_name.clone(), parsed.clone(), field_config.clone(), field_ty.clone()));
    }

    // Build nested prost message struct (for the oneof variant)
    nested_message_structs.push(quote! {
        #[derive(::prost::Message, Clone, PartialEq)]
        pub struct #nested_msg_ident {
            #(#prost_fields),*
        }
    });

    // oneof variant entry pointing to nested message
    let prost_attr = quote! { #[prost(message, tag = #tag)] };
    oneof_variants.push(quote! {
        #prost_attr
        #variant_ident(super::#nested_msg_ident)
    });

    // to_proto arm: build conversion from enum variant fields -> nested proto struct
    let field_patterns: Vec<_> = fields_named
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

    // build conversions into nested message fields
    let field_conversions_to: Vec<_> = nested_fields_meta
        .iter()
        .map(|(name, parsed_opt, cfg, _field_ty)| {
            let parsed = parsed_opt;
            // custom conversion via into_fn
            if cfg.into_fn.is_some() {
                let into_fn: syn::Ident = syn::parse_str(cfg.into_fn.as_ref().unwrap()).unwrap();
                return Some(quote! { #name: #into_fn(&#name) });
            }

            // rust enum -> to_proto() as i32
            if cfg.is_rust_enum {
                if parsed.is_option {
                    return Some(quote! { #name: #name.as_ref().map(|v| v.to_proto() as i32) });
                } else if parsed.is_repeated {
                    return Some(quote! { #name: #name.iter().map(|v| v.to_proto() as i32).collect() });
                } else {
                    return Some(quote! { #name: #name.to_proto() as i32 });
                }
            }

            // proto enum -> cast to i32
            if cfg.is_proto_enum {
                if parsed.is_option {
                    return Some(quote! { #name: #name.map(|v| v as i32) });
                } else if parsed.is_repeated {
                    return Some(quote! { #name: #name.iter().cloned().map(|v| v as i32).collect() });
                } else {
                    return Some(quote! { #name: (*#name) as i32 });
                }
            }

            // message-like handling
            if parsed.is_message_like || cfg.is_message {
                if parsed.is_option {
                    if cfg.is_message {
                        return Some(quote! { #name: #name.clone() });
                    } else {
                        return Some(quote! { #name: #name.as_ref().map(|v| v.to_proto()) });
                    }
                } else if parsed.is_repeated {
                    if cfg.is_message {
                        return Some(quote! { #name: #name.clone() });
                    } else {
                        return Some(quote! { #name: #name.iter().map(|v| v.to_proto()).collect() });
                    }
                } else if cfg.is_message {
                    return Some(quote! { #name: Some(#name.clone()) });
                } else {
                    return Some(quote! { #name: Some(#name.to_proto()) });
                }
            }

            // primitives
            Some(quote! { #name: #name.clone() })
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

    // from_proto arm: convert nested proto -> original enum named variant
    let variant_name_str = variant_ident.to_string();

    let field_conversions_from: Vec<_> = nested_fields_meta
        .iter()
        .map(|(name, parsed_opt, cfg, field_ty)| {
            let parsed = parsed_opt;

            // custom from_fn
            if cfg.from_fn.is_some() {
                let from_fn: syn::Ident = syn::parse_str(cfg.from_fn.as_ref().unwrap()).unwrap();
                return quote! { #name: #from_fn(inner.#name) };
            }

            // rust enum conversion
            if cfg.is_rust_enum {
                let (enum_type, is_option, is_repeated) = extract_wrapper_info(field_ty);
                let enum_ident = rust_type_path_ident(&enum_type);
                if is_option {
                    return quote! {
                        #name: inner.#name
                            .map(|v| #enum_ident::try_from(v))
                            .transpose()
                            .map_err(|e| #error_name::VariantConversion {
                                variant: #variant_name_str.to_string(),
                                source: Box::new(e),
                            })?
                    };
                } else if is_repeated {
                    return quote! {
                        #name: inner.#name
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
                        #name: #enum_ident::try_from(inner.#name)
                            .map_err(|e| #error_name::VariantConversion {
                                variant: #variant_name_str.to_string(),
                                source: Box::new(e),
                            })?
                    };
                }
            }

            // proto enum conversion
            if cfg.is_proto_enum {
                let (enum_type, is_option, is_repeated) = extract_wrapper_info(field_ty);
                let enum_ident = rust_type_path_ident(&enum_type);
                if is_option {
                    return quote! {
                        #name: inner.#name
                            .map(|v| #enum_ident::try_from(v))
                            .transpose()
                            .map_err(|e| #error_name::VariantConversion {
                                variant: #variant_name_str.to_string(),
                                source: Box::new(e),
                            })?
                    };
                } else if is_repeated {
                    return quote! {
                        #name: inner.#name
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
                        #name: #enum_ident::try_from(inner.#name)
                            .map_err(|e| #error_name::VariantConversion {
                                variant: #variant_name_str.to_string(),
                                source: Box::new(e),
                            })?
                    };
                }
            }

            // Message handling
            if parsed.is_message_like || cfg.is_message {
                if parsed.is_option {
                    if cfg.is_message {
                        return quote! { #name: inner.#name };
                    } else {
                        let inner_ty = extract_option_inner_type(field_ty);
                        return quote! {
                            #name: match inner.#name {
                                Some(v) => Some(#inner_ty::from_proto(v)
                                    .map_err(|e| #error_name::VariantConversion {
                                        variant: #variant_name_str.to_string(),
                                        source: e,
                                    })?),
                                None => None,
                            }
                        };
                    }
                } else if parsed.is_repeated {
                    if cfg.is_message {
                        return quote! { #name: inner.#name };
                    } else {
                        let inner_ty = extract_vec_inner_type(field_ty);
                        return quote! {
                            #name: inner.#name.into_iter()
                                .map(|v| #inner_ty::from_proto(v))
                                .collect::<Result<Vec<_>, _>>()
                                .map_err(|e| #error_name::VariantConversion {
                                    variant: #variant_name_str.to_string(),
                                    source: e,
                                })?
                        };
                    }
                } else if cfg.is_message {
                    return quote! {
                        #name: inner.#name
                            .ok_or_else(|| #error_name::VariantConversion {
                                variant: #variant_name_str.to_string(),
                                source: Box::new(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("Missing required field: {}", stringify!(#name))
                                )),
                            })?
                    };
                } else {
                    return quote! {
                        #name: #field_ty::from_proto(
                            inner.#name.ok_or_else(|| #error_name::VariantConversion {
                                variant: #variant_name_str.to_string(),
                                source: Box::new(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("Missing required field: {}", stringify!(#name))
                                )),
                            })?
                        )
                        .map_err(|e| #error_name::VariantConversion {
                            variant: #variant_name_str.to_string(),
                            source: e,
                        })?
                    };
                }
            }

            // Primitives
            quote! { #name: inner.#name }
        })
        .collect();

    // skip/computation tokens
    let skip_computations: Vec<_> = skip_with_fn.iter().map(|(_, computation)| computation.clone()).collect();
    let skip_field_assignments: Vec<_> = skip_with_fn.iter().map(|(field_name, _)| quote! { #field_name }).collect();
    let skip_default_assignments: Vec<_> = skip_with_default.iter().map(|field_name| quote! { #field_name: Default::default() }).collect();

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

// --- retained helpers from the original file (unchanged) ---

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
