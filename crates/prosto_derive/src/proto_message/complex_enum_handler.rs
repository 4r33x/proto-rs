//! Handler for complex enums (with associated data)

use proc_macro2::TokenStream;
use quote::quote;
use syn::DataEnum;
use syn::DeriveInput;
use syn::Fields;
use syn::FieldsNamed;
use syn::FieldsUnnamed;
use syn::Type;

use crate::utils::field_handling::FieldHandler;
use crate::utils::field_handling::FromProtoConversion;
use crate::utils::parse_field_config;
use crate::utils::type_info::*;

pub fn handle_complex_enum(input: DeriveInput, data: &DataEnum) -> TokenStream {
    let name = &input.ident;
    let proto_name = syn::Ident::new(&format!("{}Proto", name), name.span());
    let error_name = syn::Ident::new(&format!("{}ConversionError", name), name.span());

    let oneof_mod_name = syn::Ident::new(&crate::utils::to_snake_case(&proto_name.to_string()), name.span());
    let oneof_enum_name = syn::Ident::new("Value", name.span());

    // Collections for generated code
    let mut oneof_variants = Vec::new();
    let mut tags = Vec::new();
    let mut to_proto_arms = Vec::new();
    let mut from_proto_arms = Vec::new();
    let mut nested_message_structs = Vec::new();

    // Process each variant
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

    // Build original enum variants
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
    let oneof_path = format!("{}::Value", crate::utils::to_snake_case(&proto_name.to_string()));

    // Generate final code
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
            MissingField { field: String },
            VariantConversion { variant: String, source: Box<dyn std::error::Error> },
            FieldConversion { field: String, source: Box<dyn std::error::Error> },
        }

        impl std::fmt::Display for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::MissingValue => write!(f, "Missing oneof value in Proto message"),
                    Self::MissingField { field } => write!(f, "Missing required field: {}", field),
                    Self::VariantConversion { variant, source } =>
                        write!(f, "Error converting variant {}: {}", variant, source),
                    Self::FieldConversion { field, source } =>
                        write!(f, "Error converting field {}: {}", field, source),
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
                            #error_name::MissingField { field } => #error_name::MissingField {
                                field: field.clone(),
                            },
                            #error_name::VariantConversion { variant, .. } =>
                                #error_name::VariantConversion {
                                    variant: variant.clone(),
                                    source: Box::new(std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        "Conversion error"
                                    )),
                                },
                            #error_name::FieldConversion { field, .. } =>
                                #error_name::FieldConversion {
                                    field: field.clone(),
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
    proto_name: &syn::Ident,
    oneof_mod_name: &syn::Ident,
    oneof_enum_name: &syn::Ident,
    variant: &syn::Variant,
    tag: usize,
    nested_message_structs: &mut Vec<TokenStream>,
    oneof_variants: &mut Vec<TokenStream>,
    to_proto_arms: &mut Vec<TokenStream>,
    from_proto_arms: &mut Vec<TokenStream>,
) {
    let variant_ident = &variant.ident;
    let empty_msg_name = format!("{}{}", proto_name, variant_ident);
    let empty_msg_ident = syn::Ident::new(&empty_msg_name, variant_ident.span());

    nested_message_structs.push(quote! {
        #[derive(::prost::Message, Clone, PartialEq)]
        pub struct #empty_msg_ident {}
    });

    oneof_variants.push(quote! {
        #[prost(message, tag = #tag)]
        #variant_ident(super::#empty_msg_ident)
    });

    to_proto_arms.push(quote! {
        #name::#variant_ident => #oneof_mod_name::#oneof_enum_name::#variant_ident(#empty_msg_ident {})
    });

    from_proto_arms.push(quote! {
        #oneof_mod_name::#oneof_enum_name::#variant_ident(_) => #name::#variant_ident
    });
}

/// Handle unnamed variants - properly using FieldHandler
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

    // Handle skip fields in unnamed variants
    if field_config.skip {
        // For skip fields, generate an empty nested message
        let nested_msg_name = format!("{}{}", proto_name, variant_ident);
        let nested_msg_ident = syn::Ident::new(&nested_msg_name, variant_ident.span());

        nested_message_structs.push(quote! {
            #[derive(::prost::Message, Clone, PartialEq)]
            pub struct #nested_msg_ident {}
        });

        oneof_variants.push(quote! {
            #[prost(message, tag = #tag)]
            #variant_ident(super::#nested_msg_ident)
        });

        // to_proto: Don't pass the field value
        to_proto_arms.push(quote! {
            #name::#variant_ident(_) =>
                #oneof_mod_name::#oneof_enum_name::#variant_ident(#nested_msg_ident {})
        });

        // from_proto: Use skip function or Default
        if let Some(ref skip_fn) = field_config.skip_deser_fn {
            let skip_fn_ident: syn::Ident = syn::parse_str(skip_fn).expect("Invalid skip function name");
            from_proto_arms.push(quote! {
                #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                    let value = #skip_fn_ident(&inner);
                    #name::#variant_ident(value)
                }
            });
        } else {
            from_proto_arms.push(quote! {
                #oneof_mod_name::#oneof_enum_name::#variant_ident(_) =>
                    #name::#variant_ident(Default::default())
            });
        }
        return;
    }

    // Special case: [u8; N] can be directly in oneof
    if let Type::Array(_type_array) = field_ty
        && is_bytes_array(field_ty)
    {
        oneof_variants.push(quote! {
            #[prost(bytes, tag = #tag)]
            #variant_ident(::std::vec::Vec<u8>)
        });

        to_proto_arms.push(quote! {
            #name::#variant_ident(inner) =>
                #oneof_mod_name::#oneof_enum_name::#variant_ident(inner.to_vec())
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

    // For all other cases, create a nested message wrapper
    let nested_msg_name = format!("{}{}", proto_name, variant_ident);
    let nested_msg_ident = syn::Ident::new(&nested_msg_name, variant_ident.span());
    let value_ident = syn::Ident::new("value", variant_ident.span());

    // Use FieldHandler to properly handle the field
    let handler = FieldHandler::new(
        field,
        &value_ident,
        1, // tag inside nested message
        error_name,
        format!("{}::value", variant_ident),
    );

    let result = handler.generate();

    // Create nested message with the proto field
    if let Some(prost_field) = result.prost_field {
        nested_message_structs.push(quote! {
            #[derive(::prost::Message, Clone, PartialEq)]
            pub struct #nested_msg_ident {
                #prost_field,
            }
        });
    }

    // Add to oneof
    oneof_variants.push(quote! {
        #[prost(message, tag = #tag)]
        #variant_ident(super::#nested_msg_ident)
    });

    // Generate to_proto conversion
    if let Some(to_proto) = result.to_proto {
        // Extract value expression and replace self.value with inner
        let to_value = extract_and_adjust_value(&to_proto, &value_ident, quote! { inner });

        to_proto_arms.push(quote! {
            #name::#variant_ident(inner) => #oneof_mod_name::#oneof_enum_name::#variant_ident(
                #nested_msg_ident { value: #to_value }
            )
        });
    }

    // Generate from_proto conversion
    match result.from_proto {
        FromProtoConversion::Normal(from_proto) => {
            // Extract value expression and adjust proto.value to inner.value
            let from_value = extract_conversion_value_adjusted(&from_proto, &value_ident, quote! { inner.value });

            from_proto_arms.push(quote! {
                #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                    #name::#variant_ident(#from_value)
                }
            });
        }
        _ => panic!("Enum variants don't support skip attributes"),
    }
}

/// Handle named variants - properly using FieldHandler for each field
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

    let mut proto_fields = Vec::new();
    let mut to_proto_conversions = Vec::new();
    let mut from_proto_conversions = Vec::new();
    let mut skip_computations = Vec::new(); // For skip with function
    let mut pattern_bindings = Vec::new(); // For match pattern

    // Use FieldHandler for each field
    for (idx, field) in fields_named.named.iter().enumerate() {
        let field_ident = field.ident.as_ref().unwrap();
        let field_num = idx + 1;

        let handler = FieldHandler::new(field, field_ident, field_num, error_name, format!("{}::{}", variant_ident, field_ident));

        let result = handler.generate();

        // Determine pattern binding based on whether field is skipped
        let is_skipped = result.prost_field.is_none();
        if is_skipped {
            // Use _ for skipped fields to avoid unused variable warnings
            pattern_bindings.push(quote! { #field_ident: _ });
        } else {
            // Use field name for non-skipped fields
            pattern_bindings.push(quote! { #field_ident });
        }

        // Add proto field (skip fields won't have this)
        if let Some(prost_field) = result.prost_field {
            proto_fields.push(prost_field);
        }

        // Generate to_proto conversion (skip fields won't have this)
        if let Some(to_proto) = result.to_proto {
            // Adjust self.field to just field (no self in enum variants)
            let adjusted = adjust_for_enum_variant(&to_proto, field_ident);
            to_proto_conversions.push(adjusted);
        }

        // Generate from_proto conversion
        match result.from_proto {
            FromProtoConversion::Normal(from_proto) => {
                // Adjust proto.field to inner.field
                let adjusted = adjust_from_proto_for_variant(&from_proto, field_ident);
                from_proto_conversions.push(quote! { #field_ident: #adjusted });
            }
            FromProtoConversion::SkipDefault(_) => {
                from_proto_conversions.push(quote! { #field_ident: Default::default() });
            }
            FromProtoConversion::SkipWithFn { computation, field_name: _ } => {
                // For enum variants, we need to compute before the variant construction
                // Adjust proto references to inner
                let comp_str = computation.to_string();
                let adjusted_comp = comp_str.replace("proto", "inner");
                let adjusted_computation: TokenStream = adjusted_comp.parse().unwrap_or(computation);

                skip_computations.push(adjusted_computation);
                from_proto_conversions.push(quote! { #field_ident });
            }
        }
    }

    // Create nested message struct
    nested_message_structs.push(quote! {
        #[derive(::prost::Message, Clone, PartialEq)]
        pub struct #nested_msg_ident {
            #(#proto_fields,)*
        }
    });

    // Add to oneof
    oneof_variants.push(quote! {
        #[prost(message, tag = #tag)]
        #variant_ident(super::#nested_msg_ident)
    });

    // Generate to_proto arm
    to_proto_arms.push(quote! {
        #name::#variant_ident { #(#pattern_bindings),* } => {
            #oneof_mod_name::#oneof_enum_name::#variant_ident(
                #nested_msg_ident {
                    #(#to_proto_conversions),*
                }
            )
        }
    });

    // Generate from_proto arm
    let from_proto_arm = if skip_computations.is_empty() {
        quote! {
            #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                #name::#variant_ident {
                    #(#from_proto_conversions),*
                }
            }
        }
    } else {
        quote! {
            #oneof_mod_name::#oneof_enum_name::#variant_ident(inner) => {
                #(#skip_computations)*
                #name::#variant_ident {
                    #(#from_proto_conversions),*
                }
            }
        }
    };

    from_proto_arms.push(from_proto_arm);
}

/// Extract value from field assignment and adjust reference
/// Converts "field: self.field.clone()" with target "inner" to "inner.clone()"
/// Also adds dereference for direct casts since fields are borrowed in patterns
fn extract_and_adjust_value(conversion: &TokenStream, field_name: &syn::Ident, target: TokenStream) -> TokenStream {
    let conv_str = conversion.to_string();

    // Find colon and extract RHS
    if let Some(colon_pos) = conv_str.find(':') {
        let rhs = conv_str[colon_pos + 1..].trim();

        // Replace self.field_name with target
        let field_str = field_name.to_string();
        let target_str = target.to_string();
        let adjusted = rhs.replace(&format!("self . {}", field_str), &target_str).replace(&format!("self.{}", field_str), &target_str);

        // For direct casts in enum variants, we need to dereference
        // Check if we have pattern like "inner as i32"
        let cast_pattern = format!("{} as ", target_str);
        let adjusted = if adjusted.contains(&cast_pattern) {
            // Check it's not inside a closure
            let target_pos = adjusted.find(&cast_pattern).unwrap();
            let before = &adjusted[..target_pos];

            let in_closure = before.contains("map (") || before.contains("map(") || before.contains("|") || before.contains("* ");

            if !in_closure {
                // Direct cast, add dereference
                adjusted.replace(&cast_pattern, &format!("(* {}) as ", target_str))
            } else {
                adjusted
            }
        } else {
            adjusted
        };

        adjusted.parse().unwrap_or(target)
    } else {
        target
    }
}

/// Extract conversion value and adjust proto reference
fn extract_conversion_value_adjusted(conversion: &TokenStream, field_name: &syn::Ident, target: TokenStream) -> TokenStream {
    let conv_str = conversion.to_string();

    // Find colon and extract RHS
    if let Some(colon_pos) = conv_str.find(':') {
        let rhs = conv_str[colon_pos + 1..].trim();

        // Replace proto.field_name with target
        let field_str = field_name.to_string();
        let adjusted = rhs
            .replace(&format!("proto . {}", field_str), &target.to_string())
            .replace(&format!("proto.{}", field_str), &target.to_string());

        adjusted.parse().unwrap_or(target)
    } else {
        target
    }
}

/// Adjust to_proto conversion for enum variant context
/// Converts "field: self.field.value" to "field: field.value"
/// Also adds dereference for direct casts since fields are borrowed in patterns
fn adjust_for_enum_variant(conversion: &TokenStream, field_name: &syn::Ident) -> TokenStream {
    let conv_str = conversion.to_string();
    let field_str = field_name.to_string();

    // Replace self.field with field
    let adjusted = conv_str.replace(&format!("self . {}", field_str), &field_str).replace(&format!("self.{}", field_str), &field_str);

    // For direct casts in enum variants, we need to dereference
    // Pattern: "field as i32" should become "(*field) as i32"
    // But NOT "v as i32" in closures like "map(|v| v as i32)"
    // Check if we have a direct cast: "{field} as " where field is our field name
    let direct_cast_pattern = format!("{} as ", field_str);
    let adjusted = if adjusted.contains(&direct_cast_pattern) {
        // Check it's not inside a closure (not preceded by "|v|" or similar)
        // Simple heuristic: if "map(" appears before our field, it's in a closure
        let field_pos = adjusted.find(&direct_cast_pattern).unwrap();
        let before = &adjusted[..field_pos];

        // If there's a map/closure before our field, don't dereference
        let in_closure = before.contains("map (") || before.contains("map(") || before.contains("|") || before.contains("* ");

        if !in_closure {
            // Direct cast, add dereference
            adjusted.replace(&direct_cast_pattern, &format!("(* {}) as ", field_str))
        } else {
            adjusted
        }
    } else {
        adjusted
    };

    adjusted.parse().unwrap_or_else(|_| conversion.clone())
}

/// Adjust from_proto conversion for enum variant context
/// Extracts value expression and adjusts proto.field to inner.field
fn adjust_from_proto_for_variant(conversion: &TokenStream, field_name: &syn::Ident) -> TokenStream {
    let conv_str = conversion.to_string();
    let field_str = field_name.to_string();

    // The conversion comes as "field_name: <value_expression>"
    // We need to extract just the value expression and adjust references

    // Strategy: Look for the pattern at the START of the string
    // Match "field_name :" or "field_name:" at the beginning (after trimming)
    let trimmed = conv_str.trim();

    let value_expr = if trimmed.starts_with(&format!("{} :", field_str)) {
        // Pattern: "field_name :"
        trimmed[field_str.len() + 2..].trim()
    } else if trimmed.starts_with(&format!("{}:", field_str)) {
        // Pattern: "field_name:"
        trimmed[field_str.len() + 1..].trim()
    } else {
        // Fallback: try to find ": " and take everything after it
        if let Some(colon_space_pos) = trimmed.find(": ") {
            trimmed[colon_space_pos + 2..].trim()
        } else if let Some(colon_pos) = trimmed.find(':') {
            trimmed[colon_pos + 1..].trim()
        } else {
            trimmed
        }
    };

    // Now replace proto references with inner references
    let adjusted = value_expr
        .replace(&format!("proto . {}", field_str), &format!("inner . {}", field_str))
        .replace(&format!("proto.{}", field_str), &format!("inner.{}", field_str))
        // Also handle generic proto references
        .replace("proto .", "inner.");

    adjusted.parse().unwrap_or_else(|e| {
        eprintln!("Failed to parse adjusted conversion:");
        eprintln!("  Original: {}", conv_str);
        eprintln!("  Extracted: {}", value_expr);
        eprintln!("  Adjusted: {}", adjusted);
        eprintln!("  Error: {:?}", e);
        conversion.clone()
    })
}
