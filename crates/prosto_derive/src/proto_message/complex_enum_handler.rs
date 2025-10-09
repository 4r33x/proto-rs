use proc_macro2::TokenStream;
use quote::quote;
use syn::DataEnum;
use syn::DeriveInput;
use syn::Fields;
use syn::FieldsNamed;
use syn::FieldsUnnamed;
use syn::Type;
use syn::TypeArray;

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
                    &mut nested_message_structs,
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

/// Helper to create nested message wrapper for Vec/Option/Array types in oneof
fn create_nested_wrapper(
    proto_name: &syn::Ident,
    variant_ident: &syn::Ident,
    field_ty_tokens: TokenStream,
    prost_attr: TokenStream,
    tag: usize,
    nested_message_structs: &mut Vec<TokenStream>,
    oneof_variants: &mut Vec<TokenStream>,
) -> syn::Ident {
    let nested_msg_name = format!("{}{}", proto_name, variant_ident);
    let nested_msg_ident = syn::Ident::new(&nested_msg_name, variant_ident.span());

    nested_message_structs.push(quote! {
        #[derive(::prost::Message, Clone, PartialEq)]
        pub struct #nested_msg_ident {
            #prost_attr
            pub value: #field_ty_tokens
        }
    });

    oneof_variants.push(quote! {
        #[prost(message, tag = #tag)]
        #variant_ident(super::#nested_msg_ident)
    });

    nested_msg_ident
}

/// Handle array fields with special attributes (message, rust_enum, proto_enum)
fn handle_array_with_attribute(
    name: &syn::Ident,
    proto_name: &syn::Ident,
    oneof_mod_name: &syn::Ident,
    oneof_enum_name: &syn::Ident,
    error_name: &syn::Ident,
    variant_ident: &syn::Ident,
    elem_ty: &Type,
    field_config: &FieldConfig,
    tag: usize,
    nested_message_structs: &mut Vec<TokenStream>,
    oneof_variants: &mut Vec<TokenStream>,
    to_proto_arms: &mut Vec<TokenStream>,
    from_proto_arms: &mut Vec<TokenStream>,
) -> bool {
    // Handle [Message; N] with #[proto(message)]
    if field_config.is_message {
        let field_ty_tokens = quote! { ::std::vec::Vec<#elem_ty> };
        let prost_attr = quote! { #[prost(message, repeated, tag = 1)] };

        let nested_msg_ident = create_nested_wrapper(proto_name, variant_ident, field_ty_tokens, prost_attr, tag, nested_message_structs, oneof_variants);

        // #[proto(message)] means type is already prost::Message - just clone it
        to_proto_arms.push(quote! {
            #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(
                #nested_msg_ident {
                    value: inner.to_vec()
                }
            )
        });

        // No conversion needed - just convert Vec to array
        from_proto_arms.push(quote! {
            #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                let converted = inner.value.try_into()
                    .map_err(|v: Vec<_>| #error_name::VariantConversion {
                        variant: stringify!(#variant_ident).to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Invalid array length: got {}", v.len())
                        )),
                    })?;
                #name::#variant_ident(converted)
            }
        });
        return true;
    }

    // Handle [RustEnum; N] with #[proto(rust_enum)]
    if field_config.is_rust_enum {
        let elem_ident = rust_type_path_ident(elem_ty);
        let proto_enum_name = format!("{}Proto", elem_ident);
        let proto_enum_ident = syn::Ident::new(&proto_enum_name, elem_ident.span());

        let field_ty_tokens = quote! { ::std::vec::Vec<i32> };
        let prost_attr = quote! { #[prost(enumeration = #proto_enum_name, repeated, tag = 1)] };

        let nested_msg_ident = create_nested_wrapper(proto_name, variant_ident, field_ty_tokens, prost_attr, tag, nested_message_structs, oneof_variants);

        to_proto_arms.push(quote! {
            #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(
                #nested_msg_ident {
                    value: inner.iter().map(|v| v.to_proto() as i32).collect()
                }
            )
        });

        from_proto_arms.push(quote! {
            #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                let vec: Vec<_> = inner.value.into_iter()
                    .map(|v| #proto_enum_ident::try_from(v)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                        .and_then(|proto_enum| #elem_ty::from_proto(proto_enum)))
                    .collect::<Result<_, _>>()
                    .map_err(|e| #error_name::VariantConversion {
                        variant: stringify!(#variant_ident).to_string(),
                        source: e,
                    })?;
                let converted = vec.try_into()
                    .map_err(|v: Vec<_>| #error_name::VariantConversion {
                        variant: stringify!(#variant_ident).to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Invalid array length: got {}", v.len())
                        )),
                    })?;
                #name::#variant_ident(converted)
            }
        });
        return true;
    }

    // Handle [ProtoEnum; N] with #[proto(enum)]
    if field_config.is_proto_enum {
        let elem_ident = rust_type_path_ident(elem_ty);
        let enum_name = elem_ident.to_string();

        let field_ty_tokens = quote! { ::std::vec::Vec<i32> };
        let prost_attr = quote! { #[prost(enumeration = #enum_name, repeated, tag = 1)] };

        let nested_msg_ident = create_nested_wrapper(proto_name, variant_ident, field_ty_tokens, prost_attr, tag, nested_message_structs, oneof_variants);

        to_proto_arms.push(quote! {
            #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(
                #nested_msg_ident {
                    value: inner.iter().map(|v| (*v) as i32).collect()
                }
            )
        });

        from_proto_arms.push(quote! {
            #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                let vec: Vec<_> = inner.value.into_iter()
                    .map(|v| #elem_ty::try_from(v))
                    .collect::<Result<_, _>>()
                    .map_err(|e| #error_name::VariantConversion {
                        variant: stringify!(#variant_ident).to_string(),
                        source: Box::new(e),
                    })?;
                let converted = vec.try_into()
                    .map_err(|v: Vec<_>| #error_name::VariantConversion {
                        variant: stringify!(#variant_ident).to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Invalid array length: got {}", v.len())
                        )),
                    })?;
                #name::#variant_ident(converted)
            }
        });
        return true;
    }

    false
}

/// Handle Vec/Option wrapper types with special attributes
fn handle_wrapper_with_attribute(
    name: &syn::Ident,
    proto_name: &syn::Ident,
    oneof_mod_name: &syn::Ident,
    oneof_enum_name: &syn::Ident,
    error_name: &syn::Ident,
    variant_ident: &syn::Ident,
    field_ty: &Type,
    is_option: bool,
    is_repeated: bool,
    field_config: &FieldConfig,
    tag: usize,
    nested_message_structs: &mut Vec<TokenStream>,
    oneof_variants: &mut Vec<TokenStream>,
    to_proto_arms: &mut Vec<TokenStream>,
    from_proto_arms: &mut Vec<TokenStream>,
) -> bool {
    let variant_name_str = variant_ident.to_string();

    // Handle rust_enum wrappers (Option<RustEnum> or Vec<RustEnum>)
    if field_config.is_rust_enum {
        let (enum_type, _, _) = extract_wrapper_info(field_ty);
        let enum_ident = rust_type_path_ident(&enum_type);
        let proto_enum_name = format!("{}Proto", enum_ident);
        let proto_enum_ident = syn::Ident::new(&proto_enum_name, enum_ident.span());

        let field_ty_tokens = if is_option {
            quote! { Option<i32> }
        } else {
            quote! { Vec<i32> }
        };

        let prost_attr = if is_repeated {
            quote! { #[prost(enumeration = #proto_enum_name, repeated, tag = 1)] }
        } else {
            quote! { #[prost(enumeration = #proto_enum_name, optional, tag = 1)] }
        };

        let nested_msg_ident = create_nested_wrapper(proto_name, variant_ident, field_ty_tokens, prost_attr, tag, nested_message_structs, oneof_variants);

        let to_value = if is_option {
            quote! { inner.as_ref().map(|v| v.to_proto() as i32) }
        } else {
            quote! { inner.iter().map(|v| v.to_proto() as i32).collect() }
        };

        to_proto_arms.push(quote! {
            #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(
                #nested_msg_ident { value: #to_value }
            )
        });

        let from_value = if is_option {
            quote! {
                #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                    let converted = inner.value
                        .map(|v| #proto_enum_ident::try_from(v)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                            .and_then(|proto_enum| #enum_type::from_proto(proto_enum)))
                        .transpose()
                        .map_err(|e| #error_name::VariantConversion {
                            variant: #variant_name_str.to_string(),
                            source: e,
                        })?;
                    #name::#variant_ident(converted)
                }
            }
        } else {
            quote! {
                #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                    let converted = inner.value.into_iter()
                        .map(|v| #proto_enum_ident::try_from(v)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                            .and_then(|proto_enum| #enum_type::from_proto(proto_enum)))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| #error_name::VariantConversion {
                            variant: #variant_name_str.to_string(),
                            source: e,
                        })?;
                    #name::#variant_ident(converted)
                }
            }
        };

        from_proto_arms.push(from_value);
        return true;
    }

    // Handle proto_enum wrappers (Option<ProtoEnum> or Vec<ProtoEnum>)
    if field_config.is_proto_enum {
        let (enum_type, _, _) = extract_wrapper_info(field_ty);
        let enum_ident = rust_type_path_ident(&enum_type);
        let enum_name = enum_ident.to_string();

        let field_ty_tokens = if is_option {
            quote! { Option<i32> }
        } else {
            quote! { Vec<i32> }
        };

        let prost_attr = if is_repeated {
            quote! { #[prost(enumeration = #enum_name, repeated, tag = 1)] }
        } else {
            quote! { #[prost(enumeration = #enum_name, optional, tag = 1)] }
        };

        let nested_msg_ident = create_nested_wrapper(proto_name, variant_ident, field_ty_tokens, prost_attr, tag, nested_message_structs, oneof_variants);

        let to_value = if is_option {
            quote! { inner.map(|v| v as i32) }
        } else {
            quote! { inner.iter().map(|v| *v as i32).collect() }
        };

        to_proto_arms.push(quote! {
            #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(
                #nested_msg_ident { value: #to_value }
            )
        });

        let from_value = if is_option {
            quote! {
                #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                    let converted = inner.value
                        .map(|v| #enum_type::try_from(v))
                        .transpose()
                        .map_err(|e| #error_name::VariantConversion {
                            variant: #variant_name_str.to_string(),
                            source: Box::new(e),
                        })?;
                    #name::#variant_ident(converted)
                }
            }
        } else {
            quote! {
                #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                    let converted = inner.value.into_iter()
                        .map(|v| #enum_type::try_from(v))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| #error_name::VariantConversion {
                            variant: #variant_name_str.to_string(),
                            source: Box::new(e),
                        })?;
                    #name::#variant_ident(converted)
                }
            }
        };

        from_proto_arms.push(from_value);
        return true;
    }

    false
}

/// Refactored handle_unnamed_variant function
#[allow(clippy::too_many_arguments)]
fn handle_unnamed_variant(
    name: &syn::Ident,
    proto_name: &syn::Ident,
    oneof_mod_name: &syn::Ident,
    oneof_enum_name: &syn::Ident,
    error_name: &syn::Ident,
    variant: &syn::Variant,
    fields: &FieldsUnnamed,
    tag: usize,
    nested_message_structs: &mut Vec<TokenStream>,
    oneof_variants: &mut Vec<TokenStream>,
    to_proto_arms: &mut Vec<TokenStream>,
    from_proto_arms: &mut Vec<TokenStream>,
) {
    if fields.unnamed.len() != 1 {
        panic!("Complex enum unnamed variants must have exactly one field");
    }

    let variant_ident = &variant.ident;
    let field = fields.unnamed.first().unwrap();
    let field_ty = &field.ty;
    let field_config = parse_field_config(field);

    // ====== HANDLE ARRAYS FIRST ======
    if let Type::Array(type_array) = field_ty {
        let elem_ty = &*type_array.elem;

        // Special case: [u8; N] -> bytes (can be in oneof directly)
        if is_bytes_array(field_ty) {
            let oneof_field_ty = quote! { ::std::vec::Vec<u8> };
            let prost_attr = quote! { #[prost(bytes, tag = #tag)] };

            oneof_variants.push(quote! {
                #prost_attr
                #variant_ident(#oneof_field_ty)
            });

            to_proto_arms.push(quote! {
                #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(inner.to_vec())
            });

            from_proto_arms.push(quote! {
                #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                    let converted = inner.as_slice().try_into()
                        .map_err(|_| #error_name::VariantConversion {
                            variant: stringify!(#variant_ident).to_string(),
                            source: Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Invalid byte array length"
                            )),
                        })?;
                    #name::#variant_ident(converted)
                }
            });
            return;
        }

        // Check for special attributes (message, rust_enum, proto_enum)
        if handle_array_with_attribute(
            name,
            proto_name,
            oneof_mod_name,
            oneof_enum_name,
            error_name,
            variant_ident,
            elem_ty,
            &field_config,
            tag,
            nested_message_structs,
            oneof_variants,
            to_proto_arms,
            from_proto_arms,
        ) {
            return;
        }

        // Default array handling (no special attributes)
        let parsed_elem = parse_field_type(elem_ty);
        let prost_type = &parsed_elem.prost_type;

        let field_ty_tokens = if parsed_elem.is_message_like {
            let proto_type = &parsed_elem.proto_rust_type;
            quote! { ::std::vec::Vec<#proto_type> }
        } else {
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
            quote! { ::std::vec::Vec<#proto_elem_ty> }
        };

        let nested_msg_name = format!("{}{}", proto_name, variant_ident);
        let nested_msg_ident = syn::Ident::new(&nested_msg_name, variant_ident.span());

        nested_message_structs.push(quote! {
            #[derive(::prost::Message, Clone, PartialEq)]
            pub struct #nested_msg_ident {
                #[prost(#prost_type, repeated, tag = 1)]
                pub value: #field_ty_tokens
            }
        });

        oneof_variants.push(quote! {
            #[prost(message, tag = #tag)]
            #variant_ident(super::#nested_msg_ident)
        });

        // Generate to_proto conversion using 'inner' as source
        let to_value = if parsed_elem.is_message_like {
            quote! { inner.iter().map(|v| v.to_proto()).collect() }
        } else {
            let needs_into = if let Type::Path(type_path) = elem_ty {
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
                quote! { inner.iter().map(|v| (*v).into()).collect() }
            } else {
                quote! { inner.to_vec() }
            }
        };

        to_proto_arms.push(quote! {
            #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(
                #nested_msg_ident { value: #to_value }
            )
        });

        // Generate from_proto conversion
        let variant_name_str = variant_ident.to_string();
        let from_value = if parsed_elem.is_message_like {
            quote! {
                #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                    let vec: Vec<_> = inner.value.into_iter()
                        .map(|v| v.try_into())
                        .collect::<Result<_, _>>()
                        .map_err(|e| #error_name::VariantConversion {
                            variant: #variant_name_str.to_string(),
                            source: Box::new(e),
                        })?;
                    let converted = vec.try_into()
                        .map_err(|v: Vec<_>| #error_name::VariantConversion {
                            variant: #variant_name_str.to_string(),
                            source: Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("Invalid array length: got {}", v.len())
                            )),
                        })?;
                    #name::#variant_ident(converted)
                }
            }
        } else {
            let needs_try_into = if let Type::Path(type_path) = elem_ty {
                type_path
                    .path
                    .segments
                    .last()
                    .map(|s| matches!(s.ident.to_string().as_str(), "u8" | "u16" | "i8" | "i16"))
                    .unwrap_or(false)
            } else {
                false
            };

            if needs_try_into {
                quote! {
                    #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                        let vec: Vec<_> = inner.value.iter()
                            .map(|v| (*v).try_into())
                            .collect::<Result<_, _>>()
                            .map_err(|e| #error_name::VariantConversion {
                                variant: #variant_name_str.to_string(),
                                source: Box::new(e),
                            })?;
                        let converted = vec.try_into()
                            .map_err(|v: Vec<_>| #error_name::VariantConversion {
                                variant: #variant_name_str.to_string(),
                                source: Box::new(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("Invalid array length: got {}", v.len())
                                )),
                            })?;
                        #name::#variant_ident(converted)
                    }
                }
            } else {
                quote! {
                    #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                        let converted = inner.value.try_into()
                            .map_err(|v: Vec<_>| #error_name::VariantConversion {
                                variant: #variant_name_str.to_string(),
                                source: Box::new(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("Invalid array length: got {}", v.len())
                                )),
                            })?;
                        #name::#variant_ident(converted)
                    }
                }
            }
        };

        from_proto_arms.push(from_value);
        return;
    }

    // ====== HANDLE VEC/OPTION WRAPPERS ======
    let parsed = parse_field_type(field_ty);
    if (parsed.is_repeated || parsed.is_option) && !field_config.is_rust_enum && !field_config.is_proto_enum {
        let inner_ty = if parsed.is_option {
            extract_option_inner_type(field_ty)
        } else {
            extract_vec_inner_type(field_ty)
        };
        let inner_parsed = parse_field_type(inner_ty);

        let nested_msg_name = format!("{}{}", proto_name, variant_ident);
        let nested_msg_ident = syn::Ident::new(&nested_msg_name, variant_ident.span());

        let field_ty_tokens = if inner_parsed.is_message_like {
            let proto_type = &inner_parsed.proto_rust_type;
            if parsed.is_option {
                quote! { ::core::option::Option<#proto_type> }
            } else {
                quote! { ::std::vec::Vec<#proto_type> }
            }
        } else {
            let proto_elem = if let Type::Path(type_path) = inner_ty {
                if let Some(segment) = type_path.path.segments.last() {
                    match segment.ident.to_string().as_str() {
                        "u8" | "u16" => quote! { u32 },
                        "i8" | "i16" => quote! { i32 },
                        "usize" => quote! { u64 },
                        "isize" => quote! { i64 },
                        _ => quote! { #inner_ty },
                    }
                } else {
                    quote! { #inner_ty }
                }
            } else {
                quote! { #inner_ty }
            };

            if parsed.is_option {
                quote! { ::core::option::Option<#proto_elem> }
            } else {
                quote! { ::std::vec::Vec<#proto_elem> }
            }
        };

        let prost_type_tokens = &inner_parsed.prost_type;
        let prost_attr = if parsed.is_repeated {
            quote! { #[prost(#prost_type_tokens, repeated, tag = 1)] }
        } else {
            quote! { #[prost(#prost_type_tokens, optional, tag = 1)] }
        };

        nested_message_structs.push(quote! {
            #[derive(::prost::Message, Clone, PartialEq)]
            pub struct #nested_msg_ident {
                #prost_attr
                pub value: #field_ty_tokens
            }
        });

        oneof_variants.push(quote! {
            #[prost(message, tag = #tag)]
            #variant_ident(super::#nested_msg_ident)
        });

        // to_proto conversion
        let to_value = if inner_parsed.is_message_like {
            if parsed.is_option {
                quote! { inner.as_ref().map(|v| v.to_proto()) }
            } else {
                quote! { inner.iter().map(|v| v.to_proto()).collect() }
            }
        } else {
            let needs_conversion = if let Type::Path(type_path) = inner_ty {
                type_path
                    .path
                    .segments
                    .last()
                    .map(|s| matches!(s.ident.to_string().as_str(), "u8" | "u16" | "i8" | "i16" | "usize" | "isize"))
                    .unwrap_or(false)
            } else {
                false
            };

            if needs_conversion {
                if parsed.is_option {
                    quote! { inner.map(|v| v.into()) }
                } else {
                    quote! { inner.iter().map(|v| (*v).into()).collect() }
                }
            } else {
                quote! { inner.clone() }
            }
        };

        to_proto_arms.push(quote! {
            #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(
                #nested_msg_ident {
                    value: #to_value
                }
            )
        });

        // from_proto conversion
        let variant_name_str = variant_ident.to_string();
        let from_value = if inner_parsed.is_message_like {
            if parsed.is_option {
                quote! {
                    #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                        let converted = match inner.value {
                            Some(v) => Some(v.try_into()
                                .map_err(|e| #error_name::VariantConversion {
                                    variant: #variant_name_str.to_string(),
                                    source: Box::new(e),
                                })?),
                            None => None,
                        };
                        #name::#variant_ident(converted)
                    }
                }
            } else {
                quote! {
                    #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                        let converted = inner.value.into_iter()
                            .map(|v| v.try_into())
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| #error_name::VariantConversion {
                                variant: #variant_name_str.to_string(),
                                source: Box::new(e),
                            })?;
                        #name::#variant_ident(converted)
                    }
                }
            }
        } else {
            let needs_conversion = if let Type::Path(type_path) = inner_ty {
                type_path
                    .path
                    .segments
                    .last()
                    .map(|s| matches!(s.ident.to_string().as_str(), "u8" | "u16" | "i8" | "i16"))
                    .unwrap_or(false)
            } else {
                false
            };

            if needs_conversion {
                if parsed.is_option {
                    quote! {
                        #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                            let converted = inner.value
                                .map(|v| v.try_into())
                                .transpose()
                                .map_err(|e| #error_name::VariantConversion {
                                    variant: #variant_name_str.to_string(),
                                    source: Box::new(e),
                                })?;
                            #name::#variant_ident(converted)
                        }
                    }
                } else {
                    quote! {
                        #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                            let converted = inner.value.iter()
                                .map(|v| (*v).try_into())
                                .collect::<Result<Vec<_>, _>>()
                                .map_err(|e| #error_name::VariantConversion {
                                    variant: #variant_name_str.to_string(),
                                    source: Box::new(e),
                                })?;
                            #name::#variant_ident(converted)
                        }
                    }
                }
            } else {
                quote! {
                    #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                        #name::#variant_ident(inner.value)
                    }
                }
            }
        };

        from_proto_arms.push(from_value);
        return;
    }

    // ====== HANDLE UNWRAPPED ENUMS (Direct enum, not Vec/Option) ======
    if field_config.is_rust_enum {
        let (enum_type, is_option, is_repeated) = extract_wrapper_info(field_ty);

        // If wrapped, use the helper
        if (is_option || is_repeated)
            && handle_wrapper_with_attribute(
                name,
                proto_name,
                oneof_mod_name,
                oneof_enum_name,
                error_name,
                variant_ident,
                field_ty,
                is_option,
                is_repeated,
                &field_config,
                tag,
                nested_message_structs,
                oneof_variants,
                to_proto_arms,
                from_proto_arms,
            )
        {
            return;
        }

        // Direct enum (not wrapped)
        let enum_ident = rust_type_path_ident(&enum_type);
        let proto_enum_name = format!("{}Proto", enum_ident);
        let proto_enum_ident = syn::Ident::new(&proto_enum_name, enum_ident.span());

        let oneof_field_ty = quote! { i32 };
        let enum_path = format!("super::{}", proto_enum_name);
        let prost_attr = quote! { #[prost(enumeration = #enum_path, tag = #tag)] };

        oneof_variants.push(quote! {
            #prost_attr
            #variant_ident(#oneof_field_ty)
        });

        to_proto_arms.push(quote! {
            #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(inner.to_proto() as i32)
        });

        let variant_name_str = variant_ident.to_string();
        from_proto_arms.push(quote! {
            #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                let proto_enum = #proto_enum_ident::try_from(inner)
                    .map_err(|e| #error_name::VariantConversion {
                        variant: #variant_name_str.to_string(),
                        source: Box::new(e),
                    })?;
                let converted = #enum_type::from_proto(proto_enum)
                    .map_err(|e| #error_name::VariantConversion {
                        variant: #variant_name_str.to_string(),
                        source: e,
                    })?;
                #name::#variant_ident(converted)
            }
        });
        return;
    }

    if field_config.is_proto_enum {
        let (enum_type, is_option, is_repeated) = extract_wrapper_info(field_ty);

        // If wrapped, use the helper
        if (is_option || is_repeated)
            && handle_wrapper_with_attribute(
                name,
                proto_name,
                oneof_mod_name,
                oneof_enum_name,
                error_name,
                variant_ident,
                field_ty,
                is_option,
                is_repeated,
                &field_config,
                tag,
                nested_message_structs,
                oneof_variants,
                to_proto_arms,
                from_proto_arms,
            )
        {
            return;
        }

        // Direct enum (not wrapped)
        let enum_ident = rust_type_path_ident(&enum_type);
        let enum_name = enum_ident.to_string();

        let oneof_field_ty = quote! { i32 };
        let enum_path = format!("super::{}", enum_name);
        let prost_attr = quote! { #[prost(enumeration = #enum_path, tag = #tag)] };

        oneof_variants.push(quote! {
            #prost_attr
            #variant_ident(#oneof_field_ty)
        });

        to_proto_arms.push(quote! {
            #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(*inner as i32)
        });

        let variant_name_str = variant_ident.to_string();
        from_proto_arms.push(quote! {
            #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                let converted = #enum_type::try_from(inner)
                    .map_err(|e| #error_name::VariantConversion {
                        variant: #variant_name_str.to_string(),
                        source: Box::new(e),
                    })?;
                #name::#variant_ident(converted)
            }
        });
        return;
    }

    // ====== HANDLE REGULAR NON-WRAPPED TYPES ======
    // At this point: not an array, not a Vec/Option wrapper, not an enum attribute

    let parsed = parse_field_type(field_ty);

    // Build oneof field type tokens for prost (direct non-wrapped types)
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

/// Build prost field tokens for array field handling.
/// Returns (prost_field_tokens, field_meta_parsed).
fn build_prost_field_for_array(field_name: &syn::Ident, field_tag: usize, array_type: &TypeArray, cfg: &FieldConfig) -> (TokenStream, ParsedFieldType) {
    let elem_ty = &*array_type.elem;
    // Build a Type::Array from the existing TypeArray (TypeArray implements Clone)
    let array_ty = Type::Array(array_type.clone());

    // Handle bytes array specially (e.g. [u8; N])
    if is_bytes_array(&array_ty) {
        let prost = quote! {
            #[prost(bytes, tag = #field_tag)]
            pub #field_name: ::std::vec::Vec<u8>
        };
        let parsed = ParsedFieldType {
            rust_type: array_ty.clone(),
            proto_type: "bytes".to_string(),
            prost_type: quote! { bytes },
            is_option: false,
            is_repeated: true,
            is_message_like: false,
            proto_rust_type: elem_ty.clone(),
        };
        return (prost, parsed);
    }

    // If user marked #[proto(message)] on array -> repeated message of elem type
    if cfg.is_message {
        let prost = quote! {
            #[prost(message, repeated, tag = #field_tag)]
            pub #field_name: ::std::vec::Vec<#elem_ty>
        };

        let parsed = ParsedFieldType {
            rust_type: array_ty.clone(),
            proto_type: "message".to_string(),
            prost_type: quote! { message },
            is_option: false,
            is_repeated: true,
            is_message_like: true,
            proto_rust_type: elem_ty.clone(),
        };

        return (prost, parsed);
    }

    // #[proto(rust_enum)] on array
    if cfg.is_rust_enum {
        let elem_ident = rust_type_path_ident(elem_ty);
        let proto_enum_name = format!("{}Proto", elem_ident);
        let prost = quote! {
            #[prost(enumeration = #proto_enum_name, repeated, tag = #field_tag)]
            pub #field_name: ::std::vec::Vec<i32>  // Changed from i32 to Vec<i32>
        };

        let parsed = ParsedFieldType {
            rust_type: array_ty.clone(),
            proto_type: "enum".to_string(),
            prost_type: quote! { enumeration },
            is_option: false,
            is_repeated: true,
            is_message_like: false,
            proto_rust_type: elem_ty.clone(),
        };

        return (prost, parsed);
    }

    // #[proto(enum)] on array
    if cfg.is_proto_enum {
        let elem_ident = rust_type_path_ident(elem_ty);
        let enum_name = elem_ident.to_string();
        let prost = quote! {
            #[prost(enumeration = #enum_name, repeated, tag = #field_tag)]
            pub #field_name: ::std::vec::Vec<i32>
        };

        let parsed = ParsedFieldType {
            rust_type: array_ty.clone(),
            proto_type: "enum".to_string(),
            prost_type: quote! { enumeration },
            is_option: false,
            is_repeated: true,
            is_message_like: false,
            proto_rust_type: elem_ty.clone(),
        };

        return (prost, parsed);
    }

    // Default array -> repeated T mapping with possible primitive conversions
    let parsed_elem = parse_field_type(elem_ty);
    let prost_type = &parsed_elem.prost_type;
    let prost_attr = quote! { #[prost(#prost_type, repeated, tag = #field_tag)] };

    let field_ty_tokens = if parsed_elem.is_message_like {
        let proto_type = &parsed_elem.proto_rust_type;
        quote! { ::std::vec::Vec<#proto_type> }
    } else {
        // map some smaller integer types to protobufs
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
        quote! { ::std::vec::Vec<#proto_elem_ty> }
    };

    let prost = quote! {
        #prost_attr
        pub #field_name: #field_ty_tokens
    };

    let parsed = ParsedFieldType {
        rust_type: array_ty.clone(),
        proto_type: parsed_elem.proto_type.clone(),
        prost_type: parsed_elem.prost_type.clone(),
        is_option: false,
        is_repeated: true,
        is_message_like: parsed_elem.is_message_like,
        proto_rust_type: parsed_elem.proto_rust_type.clone(),
    };

    (prost, parsed)
}

/// Build `to_proto` conversion snippet for array field.
fn build_conversion_to_for_array(name: &syn::Ident, elem_ty: &Type, cfg: &FieldConfig, parsed_elem: &ParsedFieldType) -> TokenStream {
    // bytes array
    if is_bytes_array(elem_ty) {
        return quote! { #name: #name.to_vec() };
    }

    if cfg.is_message {
        return quote! { #name: #name.to_vec() };
    }

    if cfg.is_rust_enum {
        return quote! { #name: #name.iter().map(|v| v.to_proto() as i32).collect() };
    }

    if cfg.is_proto_enum {
        return quote! { #name: #name.iter().map(|v| (*v) as i32).collect() };
    }

    if parsed_elem.is_message_like {
        return quote! { #name: #name.iter().map(|v| v.to_proto()).collect() };
    }

    // Primitive mapping and `into` conversions for small ints
    let needs_into = if let Type::Path(type_path) = elem_ty {
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
        quote! { #name: #name.iter().map(|v| (*v).into()).collect() }
    } else {
        quote! { #name: #name.to_vec() }
    }
}

/// Build `from_proto` conversion snippet for array field.
fn build_conversion_from_for_array(name: &syn::Ident, elem_ty: &Type, cfg: &FieldConfig, error_name: &syn::Ident, variant_name_str: &str, parsed_elem: &ParsedFieldType) -> TokenStream {
    // bytes array
    if is_bytes_array(elem_ty) {
        return quote! {
            #name: inner.#name.as_slice().try_into()
                .map_err(|_| #error_name::VariantConversion {
                    variant: #variant_name_str.to_string(),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid byte array length"
                    )),
                })?
        };
    }

    if cfg.is_message {
        return quote! {
            #name: inner.#name.try_into()
                .map_err(|v: Vec<_>| #error_name::VariantConversion {
                    variant: #variant_name_str.to_string(),
                    source: Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Invalid array length: got {}", v.len())
                    )),
                })?
        };
    }

    if cfg.is_rust_enum {
        let elem_ident = rust_type_path_ident(elem_ty);
        let proto_enum_name = format!("{}Proto", elem_ident);
        let proto_enum_ident = syn::Ident::new(&proto_enum_name, elem_ident.span());
        return quote! {
            #name: {
                let vec: Vec<_> = inner.#name.into_iter()
                    .map(|v| #proto_enum_ident::try_from(v)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                        .and_then(|proto_enum| #elem_ty::from_proto(proto_enum)))
                    .collect::<Result<_, _>>()
                    .map_err(|e| #error_name::VariantConversion {
                        variant: #variant_name_str.to_string(),
                        source: e,
                    })?;
                vec.try_into()
                    .map_err(|v: Vec<_>| #error_name::VariantConversion {
                        variant: #variant_name_str.to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Invalid array length: got {}", v.len())
                        )),
                    })?
            }
        };
    }

    if cfg.is_proto_enum {
        return quote! {
            #name: {
                let vec: Vec<_> = inner.#name.into_iter()
                    .map(|v| #elem_ty::try_from(v))
                    .collect::<Result<_, _>>()
                    .map_err(|e| #error_name::VariantConversion {
                        variant: #variant_name_str.to_string(),
                        source: Box::new(e),
                    })?;
                vec.try_into()
                    .map_err(|v: Vec<_>| #error_name::VariantConversion {
                        variant: #variant_name_str.to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Invalid array length: got {}", v.len())
                        )),
                    })?
            }
        };
    }

    // generic message-like element
    if parsed_elem.is_message_like {
        return quote! {
            #name: {
                let vec: Vec<_> = inner.#name.into_iter()
                    .map(|v| v.try_into())
                    .collect::<Result<_, _>>()
                    .map_err(|e| #error_name::VariantConversion {
                        variant: #variant_name_str.to_string(),
                        source: Box::new(e),
                    })?;
                vec.try_into()
                    .map_err(|v: Vec<_>| #error_name::VariantConversion {
                        variant: #variant_name_str.to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Invalid array length: got {}", v.len())
                        )),
                    })?
            }
        };
    }

    // primitive try_into conversions for small ints
    let needs_try_into = if let Type::Path(type_path) = elem_ty {
        type_path
            .path
            .segments
            .last()
            .map(|s| matches!(s.ident.to_string().as_str(), "u8" | "u16" | "i8" | "i16"))
            .unwrap_or(false)
    } else {
        false
    };

    if needs_try_into {
        return quote! {
            #name: {
                let vec: Vec<_> = inner.#name.iter()
                    .map(|v| (*v).try_into())
                    .collect::<Result<_, _>>()
                    .map_err(|e| #error_name::VariantConversion {
                        variant: #variant_name_str.to_string(),
                        source: Box::new(e),
                    })?;
                vec.try_into()
                    .map_err(|v: Vec<_>| #error_name::VariantConversion {
                        variant: #variant_name_str.to_string(),
                        source: Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Invalid array length: got {}", v.len())
                        )),
                    })?
            }
        };
    }

    quote! {
        #name: inner.#name.try_into()
            .map_err(|v: Vec<_>| #error_name::VariantConversion {
                variant: #variant_name_str.to_string(),
                source: Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid array length: got {}", v.len())
                )),
            })?
    }
}

/// Helper to generate prost field for rust_enum type
fn generate_rust_enum_prost_field(field_name: &syn::Ident, field_tag: usize, proto_enum_name: &str, is_option: bool, is_repeated: bool) -> TokenStream {
    let field_ty_tokens = if is_option {
        quote! { Option<i32> }
    } else if is_repeated {
        quote! { Vec<i32> }
    } else {
        quote! { i32 }
    };

    let prost_attr = if is_repeated {
        quote! { #[prost(enumeration = #proto_enum_name, repeated, tag = #field_tag)] }
    } else if is_option {
        quote! { #[prost(enumeration = #proto_enum_name, optional, tag = #field_tag)] }
    } else {
        quote! { #[prost(enumeration = #proto_enum_name, tag = #field_tag)] }
    };

    quote! {
        #prost_attr
        pub #field_name: #field_ty_tokens
    }
}

/// Helper to generate prost field for proto_enum type
fn generate_proto_enum_prost_field(field_name: &syn::Ident, field_tag: usize, enum_name: &str, is_option: bool, is_repeated: bool) -> TokenStream {
    let field_ty_tokens = if is_option {
        quote! { Option<i32> }
    } else if is_repeated {
        quote! { Vec<i32> }
    } else {
        quote! { i32 }
    };

    let prost_attr = if is_repeated {
        quote! { #[prost(enumeration = #enum_name, repeated, tag = #field_tag)] }
    } else if is_option {
        quote! { #[prost(enumeration = #enum_name, optional, tag = #field_tag)] }
    } else {
        quote! { #[prost(enumeration = #enum_name, tag = #field_tag)] }
    };

    quote! {
        #prost_attr
        pub #field_name: #field_ty_tokens
    }
}

/// Helper to generate to_proto conversion for rust_enum
fn generate_rust_enum_to_proto(field_name: &syn::Ident, is_option: bool, is_repeated: bool) -> TokenStream {
    if is_option {
        quote! { #field_name: #field_name.as_ref().map(|v| v.to_proto() as i32) }
    } else if is_repeated {
        quote! { #field_name: #field_name.iter().map(|v| v.to_proto() as i32).collect() }
    } else {
        quote! { #field_name: #field_name.to_proto() as i32 }
    }
}

/// Helper to generate to_proto conversion for proto_enum
fn generate_proto_enum_to_proto(field_name: &syn::Ident, is_option: bool, is_repeated: bool) -> TokenStream {
    if is_option {
        quote! { #field_name: #field_name.map(|v| v as i32) }
    } else if is_repeated {
        quote! { #field_name: #field_name.iter().map(|v| *v as i32).collect() }
    } else {
        quote! { #field_name: (*#field_name) as i32 }
    }
}

/// Helper to generate from_proto conversion for rust_enum
fn generate_rust_enum_from_proto(field_name: &syn::Ident, enum_ident: &syn::Ident, is_option: bool, is_repeated: bool, error_name: &syn::Ident, variant_name_str: &str) -> TokenStream {
    if is_option {
        quote! {
            #field_name: inner.#field_name
                .map(|v| #enum_ident::try_from(v))
                .transpose()
                .map_err(|e| #error_name::VariantConversion {
                    variant: #variant_name_str.to_string(),
                    source: Box::new(e),
                })?
        }
    } else if is_repeated {
        quote! {
            #field_name: inner.#field_name
                .into_iter()
                .map(|v| #enum_ident::try_from(v))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| #error_name::VariantConversion {
                    variant: #variant_name_str.to_string(),
                    source: Box::new(e),
                })?
        }
    } else {
        quote! {
            #field_name: #enum_ident::try_from(inner.#field_name)
                .map_err(|e| #error_name::VariantConversion {
                    variant: #variant_name_str.to_string(),
                    source: Box::new(e),
                })?
        }
    }
}

/// Helper to generate from_proto conversion for proto_enum
fn generate_proto_enum_from_proto(field_name: &syn::Ident, enum_ident: &syn::Ident, is_option: bool, is_repeated: bool, error_name: &syn::Ident, variant_name_str: &str) -> TokenStream {
    if is_option {
        quote! {
            #field_name: inner.#field_name
                .map(|v| #enum_ident::try_from(v))
                .transpose()
                .map_err(|e| #error_name::VariantConversion {
                    variant: #variant_name_str.to_string(),
                    source: Box::new(e),
                })?
        }
    } else if is_repeated {
        quote! {
            #field_name: inner.#field_name
                .into_iter()
                .map(|v| #enum_ident::try_from(v))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| #error_name::VariantConversion {
                    variant: #variant_name_str.to_string(),
                    source: Box::new(e),
                })?
        }
    } else {
        quote! {
            #field_name: #enum_ident::try_from(inner.#field_name)
                .map_err(|e| #error_name::VariantConversion {
                    variant: #variant_name_str.to_string(),
                    source: Box::new(e),
                })?
        }
    }
}

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
    nested_message_structs: &mut Vec<TokenStream>,
    oneof_variants: &mut Vec<TokenStream>,
    to_proto_arms: &mut Vec<TokenStream>,
    from_proto_arms: &mut Vec<TokenStream>,
) {
    let variant_ident = &variant.ident;
    let nested_msg_name = format!("{}{}", proto_name, variant_ident);
    let nested_msg_ident = syn::Ident::new(&nested_msg_name, variant_ident.span());

    let mut prost_fields = Vec::new();
    let mut nested_fields_meta: Vec<(syn::Ident, ParsedFieldType, FieldConfig, syn::Type)> = Vec::new();
    let mut skip_with_fn: Vec<(syn::Ident, TokenStream)> = Vec::new();
    let mut skip_with_default: Vec<syn::Ident> = Vec::new();

    let mut field_tag = 0usize;
    for field in fields_named.named.iter() {
        let field_name = field.ident.as_ref().unwrap().clone();
        let field_ty = &field.ty;
        let field_config = parse_field_config(field);

        // Skip handling
        if field_config.skip {
            if let Some(ref deser_fn) = field_config.skip_deser_fn {
                let deser_fn_ident: syn::Ident = syn::parse_str(deser_fn).expect("Invalid deser function name");
                skip_with_fn.push((field_name.clone(), quote! { let #field_name = #deser_fn_ident(&inner); }));
            } else {
                skip_with_default.push(field_name.clone());
            }
            continue;
        }

        field_tag += 1;

        let ty_for_parsing = if let Some(ref into_type) = field_config.into_type {
            syn::parse_str::<Type>(into_type).unwrap_or_else(|_| field_ty.clone())
        } else {
            field_ty.clone()
        };

        // ====== ARRAYS FIRST - MUST BE CHECKED BEFORE ENUM ATTRIBUTES ======
        if let Type::Array(type_array) = &ty_for_parsing {
            let elem_ty = &*type_array.elem;

            if field_config.is_rust_enum {
                let elem_ident = rust_type_path_ident(elem_ty);
                let proto_enum_name = format!("{}Proto", elem_ident);

                prost_fields.push(quote! {
                    #[prost(enumeration = #proto_enum_name, repeated, tag = #field_tag)]
                    pub #field_name: ::std::vec::Vec<i32>
                });

                let parsed = ParsedFieldType {
                    rust_type: field_ty.clone(),
                    proto_type: "enum".to_string(),
                    prost_type: quote! { enumeration },
                    is_option: false,
                    is_repeated: true,
                    is_message_like: false,
                    proto_rust_type: elem_ty.clone(),
                };

                nested_fields_meta.push((field_name.clone(), parsed, field_config.clone(), field_ty.clone()));
                continue;
            }

            if field_config.is_proto_enum {
                let elem_ident = rust_type_path_ident(elem_ty);
                let enum_name = elem_ident.to_string();

                prost_fields.push(quote! {
                    #[prost(enumeration = #enum_name, repeated, tag = #field_tag)]
                    pub #field_name: ::std::vec::Vec<i32>
                });

                let parsed = ParsedFieldType {
                    rust_type: field_ty.clone(),
                    proto_type: "enum".to_string(),
                    prost_type: quote! { enumeration },
                    is_option: false,
                    is_repeated: true,
                    is_message_like: false,
                    proto_rust_type: elem_ty.clone(),
                };

                nested_fields_meta.push((field_name.clone(), parsed, field_config.clone(), field_ty.clone()));
                continue;
            }

            // General array handling
            let (prost_field_ts, parsed_meta) = build_prost_field_for_array(&field_name, field_tag, type_array, &field_config);
            prost_fields.push(prost_field_ts);
            nested_fields_meta.push((field_name.clone(), parsed_meta, field_config.clone(), field_ty.clone()));
            continue;
        }

        // ====== NOW CHECK ENUM ATTRIBUTES FOR NON-ARRAYS ======
        if field_config.is_rust_enum {
            let (_, is_option, is_repeated) = extract_wrapper_info(field_ty);
            let enum_ident = rust_type_path_ident(&ty_for_parsing);
            let proto_enum_name = format!("{}Proto", enum_ident);

            prost_fields.push(generate_rust_enum_prost_field(&field_name, field_tag, &proto_enum_name, is_option, is_repeated));

            let simple_parsed = ParsedFieldType {
                rust_type: field_ty.clone(),
                proto_type: "enum".to_string(),
                prost_type: quote! { enumeration },
                is_option,
                is_repeated,
                is_message_like: false,
                proto_rust_type: field_ty.clone(),
            };

            nested_fields_meta.push((field_name.clone(), simple_parsed, field_config.clone(), field_ty.clone()));
            continue;
        }

        if field_config.is_proto_enum {
            let (_, is_option, is_repeated) = extract_wrapper_info(field_ty);
            let enum_ident = rust_type_path_ident(&ty_for_parsing);
            let enum_name = enum_ident.to_string();

            prost_fields.push(generate_proto_enum_prost_field(&field_name, field_tag, &enum_name, is_option, is_repeated));

            let simple_parsed = ParsedFieldType {
                rust_type: field_ty.clone(),
                proto_type: "enum".to_string(),
                prost_type: quote! { enumeration },
                is_option,
                is_repeated,
                is_message_like: false,
                proto_rust_type: field_ty.clone(),
            };

            nested_fields_meta.push((field_name.clone(), simple_parsed, field_config.clone(), field_ty.clone()));
            continue;
        }

        // Handle Vec<u8> and [u8; N] -> bytes
        if is_bytes_array(&ty_for_parsing) || is_bytes_vec(&ty_for_parsing) {
            prost_fields.push(quote! {
                #[prost(bytes, tag = #field_tag)]
                pub #field_name: ::std::vec::Vec<u8>
            });
            nested_fields_meta.push((field_name.clone(), parse_field_type(&ty_for_parsing), field_config.clone(), field_ty.clone()));
            continue;
        }

        // Handle regular fields
        let parsed = parse_field_type(&ty_for_parsing);

        let field_ty_tokens = if field_config.into_type.is_some() {
            quote! { #ty_for_parsing }
        } else if parsed.is_option || parsed.is_repeated {
            let inner_ty = if parsed.is_option {
                extract_option_inner_type(&ty_for_parsing)
            } else {
                extract_vec_inner_type(&ty_for_parsing)
            };

            let inner_parsed = parse_field_type(inner_ty);

            let proto_elem_ty = if field_config.is_message {
                quote! { #inner_ty }
            } else if inner_parsed.is_message_like {
                let proto_type = &inner_parsed.proto_rust_type;
                quote! { #proto_type }
            } else if let Type::Path(type_path) = inner_ty {
                if let Some(segment) = type_path.path.segments.last() {
                    match segment.ident.to_string().as_str() {
                        "u8" | "u16" => quote! { u32 },
                        "i8" | "i16" => quote! { i32 },
                        "usize" => quote! { u64 },
                        "isize" => quote! { i64 },
                        _ => quote! { #inner_ty },
                    }
                } else {
                    quote! { #inner_ty }
                }
            } else {
                quote! { #inner_ty }
            };

            if parsed.is_option {
                quote! { ::core::option::Option<#proto_elem_ty> }
            } else {
                quote! { ::std::vec::Vec<#proto_elem_ty> }
            }
        } else {
            get_proto_field_type(&parsed, field_ty, &field_config)
        };

        let prost_attr = if field_config.into_type.is_some() {
            let prost_type_tokens = &parsed.prost_type;
            quote! { #prost_type_tokens }
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

    // Build nested prost message struct
    nested_message_structs.push(quote! {
        #[derive(::prost::Message, Clone, PartialEq)]
        pub struct #nested_msg_ident {
            #(#prost_fields),*
        }
    });

    // oneof variant entry
    oneof_variants.push(quote! {
        #[prost(message, tag = #tag)]
        #variant_ident(super::#nested_msg_ident)
    });

    // Build to_proto arm
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

    let field_conversions_to: Vec<_> = nested_fields_meta
        .iter()
        .map(|(name, parsed, cfg, field_ty)| {
            // Handle arrays FIRST
            if let Type::Array(type_array) = field_ty {
                let elem_ty = &*type_array.elem;
                let parsed_elem = parse_field_type(elem_ty);
                return Some(build_conversion_to_for_array(name, elem_ty, cfg, &parsed_elem));
            }

            // Enum conversions
            if cfg.is_rust_enum {
                let (_, is_option, is_repeated) = extract_wrapper_info(field_ty);
                return Some(generate_rust_enum_to_proto(name, is_option, is_repeated));
            }

            if cfg.is_proto_enum {
                let (_, is_option, is_repeated) = extract_wrapper_info(field_ty);
                return Some(generate_proto_enum_to_proto(name, is_option, is_repeated));
            }

            // Primitive conversions
            if !cfg.is_rust_enum && !cfg.is_proto_enum && (parsed.is_option || parsed.is_repeated) && !parsed.is_message_like {
                let inner_ty = if parsed.is_option {
                    extract_option_inner_type(field_ty)
                } else {
                    extract_vec_inner_type(field_ty)
                };

                let needs_conversion = if let Type::Path(type_path) = inner_ty {
                    type_path
                        .path
                        .segments
                        .last()
                        .map(|s| matches!(s.ident.to_string().as_str(), "u8" | "u16" | "i8" | "i16" | "usize" | "isize"))
                        .unwrap_or(false)
                } else {
                    false
                };

                if needs_conversion {
                    if parsed.is_option {
                        return Some(quote! { #name: #name.map(|v| v.into()) });
                    } else {
                        return Some(quote! { #name: #name.iter().map(|v| (*v).into()).collect() });
                    }
                }
            }

            // Custom conversion
            if cfg.into_fn.is_some() {
                let into_fn: syn::Ident = syn::parse_str(cfg.into_fn.as_ref().unwrap()).unwrap();
                return Some(quote! { #name: #into_fn(&#name) });
            }

            // Message-like handling
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

    // Build from_proto arm
    let variant_name_str = variant_ident.to_string();

    let field_conversions_from: Vec<_> = nested_fields_meta
        .iter()
        .map(|(name, parsed, cfg, field_ty)| {
            // Arrays first
            if let Type::Array(type_array) = field_ty {
                let elem_ty = &*type_array.elem;
                let parsed_elem = parse_field_type(elem_ty);
                return build_conversion_from_for_array(name, elem_ty, cfg, error_name, &variant_name_str, &parsed_elem);
            }

            // Enum conversions
            if cfg.is_rust_enum {
                let (enum_type, is_option, is_repeated) = extract_wrapper_info(field_ty);
                let enum_ident = rust_type_path_ident(&enum_type);
                return generate_rust_enum_from_proto(name, enum_ident, is_option, is_repeated, error_name, &variant_name_str);
            }

            if cfg.is_proto_enum {
                let (enum_type, is_option, is_repeated) = extract_wrapper_info(field_ty);
                let enum_ident = rust_type_path_ident(&enum_type);
                return generate_proto_enum_from_proto(name, enum_ident, is_option, is_repeated, error_name, &variant_name_str);
            }

            // Primitive conversions
            if !cfg.is_rust_enum && !cfg.is_proto_enum && (parsed.is_option || parsed.is_repeated) && !parsed.is_message_like {
                let inner_ty = if parsed.is_option {
                    extract_option_inner_type(field_ty)
                } else {
                    extract_vec_inner_type(field_ty)
                };

                let needs_conversion = if let Type::Path(type_path) = inner_ty {
                    type_path
                        .path
                        .segments
                        .last()
                        .map(|s| matches!(s.ident.to_string().as_str(), "u8" | "u16" | "i8" | "i16"))
                        .unwrap_or(false)
                } else {
                    false
                };

                if needs_conversion {
                    if parsed.is_option {
                        return quote! {
                            #name: inner.#name
                                .map(|v| v.try_into())
                                .transpose()
                                .map_err(|e| #error_name::VariantConversion {
                                    variant: #variant_name_str.to_string(),
                                    source: Box::new(e),
                                })?
                        };
                    } else {
                        return quote! {
                            #name: inner.#name.iter()
                                .map(|v| (*v).try_into())
                                .collect::<Result<Vec<_>, _>>()
                                .map_err(|e| #error_name::VariantConversion {
                                    variant: #variant_name_str.to_string(),
                                    source: Box::new(e),
                                })?
                        };
                    }
                }
            }

            // Custom from_fn
            if cfg.from_fn.is_some() {
                let from_fn: syn::Ident = syn::parse_str(cfg.from_fn.as_ref().unwrap()).unwrap();
                return quote! { #name: #from_fn(inner.#name) };
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

            quote! { #name: inner.#name }
        })
        .collect();

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
