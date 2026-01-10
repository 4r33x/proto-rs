use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Error;
use syn::Fields;
use syn::ItemEnum;
use syn::ItemStruct;

use crate::emit_proto::generate_complex_enum_proto;
use crate::emit_proto::generate_simple_enum_proto;
use crate::emit_proto::generate_struct_proto;
use crate::parse::UnifiedProtoConfig;
use crate::schema::SchemaTokens;
use crate::schema::assoc_proto_ident_const;
use crate::schema::schema_tokens_for_complex_enum;
use crate::schema::schema_tokens_for_simple_enum;

pub(crate) fn build_validate_with_ext_impl(config: &UnifiedProtoConfig) -> TokenStream2 {
    let Some(validator_fn) = &config.validator_with_ext else {
        return quote! {};
    };
    let validator_path: syn::Path = syn::parse_str(validator_fn).expect("invalid validator_with_ext function path");
    quote! {
        const VALIDATE_WITH_EXT: bool = true;

        #[cfg(feature = "tonic")]
        #[inline(always)]
        fn validate_with_ext(
            value: &mut Self,
            ext: &::tonic::Extensions,
        ) -> Result<(), ::proto_rs::DecodeError> {
            #validator_path(value, ext)
        }
    }
}

mod complex_enums;
mod enums;
mod generic_bounds;
mod structs;
mod unified_field_handler;

use complex_enums::generate_complex_enum_impl;
use enums::generate_simple_enum_impl;
use structs::generate_struct_impl;

pub fn proto_message_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_ts: TokenStream2 = item.clone().into();
    let input: DeriveInput = syn::parse2(item_ts.clone()).expect("proto_message expects a type definition");

    let type_ident = input.ident.to_string();
    let mut config = UnifiedProtoConfig::from_attributes(attr, &type_ident, &input.attrs, &input.data, input.generics.clone());
    let proto_names = config.proto_message_names(&type_ident);
    let generic_params: Vec<syn::Ident> = input.generics.type_params().map(|param| param.ident.clone()).collect();
    if config.transparent && config.proto_path().is_some() {
        return Error::new_spanned(&input.ident, "transparent proto_message types must not be written to .proto files")
            .to_compile_error()
            .into();
    }

    // Get generic type variants (concrete type combinations)
    let generic_variants = match config.generic_type_variants(&input.generics) {
        Ok(variants) => variants,
        Err(err) => return err.to_compile_error().into(),
    };

    let tokens = match input.data {
        Data::Struct(ref data) => {
            let mut schema_tokens_col = quote! {};
            let mut inventory_tokens_col = quote! {};

            // Iterate over each proto name (from suns configuration)
            for proto_name in &proto_names {
                // For each proto name, iterate over generic variants
                for variant in &generic_variants {
                    let message_name = if variant.suffix.is_empty() {
                        proto_name.clone()
                    } else {
                        format!("{}{}", proto_name, variant.suffix)
                    };

                    // Apply generic substitutions to fields
                    let fields = if variant.substitutions.is_empty() {
                        data.fields.clone()
                    } else {
                        crate::generic_substitutions::apply_generic_substitutions_fields(&data.fields, &variant.substitutions)
                    };

                    let proto = generate_struct_proto(&message_name, &fields, &generic_params);
                    // Use _concrete version if we have substitutions
                    let schema_tokens = if variant.substitutions.is_empty() {
                        crate::schema::schema_tokens_for_struct(&input.ident, &message_name, &fields, &config, &message_name)
                    } else {
                        crate::schema::schema_tokens_for_struct_concrete(&input.ident, &message_name, &fields, &config, &message_name)
                    };
                    // Only emit proto file entry for concrete variants (not base generic type)
                    // Base generic type (empty substitutions) is only for Rust client schema
                    let has_type_params = !input.generics.type_params().collect::<Vec<_>>().is_empty();
                    if !has_type_params || !variant.substitutions.is_empty() {
                        config.register_and_emit_proto(&proto);
                    }
                    let SchemaTokens { schema, inventory_submit } = schema_tokens;
                    schema_tokens_col = quote! { #schema #schema_tokens_col};
                    inventory_tokens_col = quote! { #inventory_submit #inventory_tokens_col};
                }
            }

            let item_struct: ItemStruct = syn::parse2(item_ts).expect("failed to parse struct");
            let type_tokens = generate_struct_impl(&input, &item_struct, data, &config);
            quote! {#type_tokens #schema_tokens_col #inventory_tokens_col}
        }
        Data::Enum(ref data) => {
            let is_simple_enum = data.variants.iter().all(|variant| matches!(variant.fields, Fields::Unit));
            let mut schema_tokens_col = quote! {};
            let mut inventory_tokens_col = quote! {};

            // Iterate over each proto name (from suns configuration)
            for proto_name in &proto_names {
                // For each proto name, iterate over generic variants
                for variant in &generic_variants {
                    let message_name = if variant.suffix.is_empty() {
                        proto_name.clone()
                    } else {
                        format!("{}{}", proto_name, variant.suffix)
                    };

                    // Apply generic substitutions to enum data
                    let enum_data = if variant.substitutions.is_empty() {
                        data.clone()
                    } else {
                        crate::generic_substitutions::apply_generic_substitutions_enum(data, &variant.substitutions)
                    };

                    let proto = if is_simple_enum {
                        generate_simple_enum_proto(&message_name, &enum_data)
                    } else {
                        generate_complex_enum_proto(&message_name, &enum_data, &generic_params)
                    };
                    // Use _concrete version if we have substitutions
                    let schema_tokens = if variant.substitutions.is_empty() {
                        if is_simple_enum {
                            schema_tokens_for_simple_enum(&input.ident, &message_name, &enum_data, &config, &message_name)
                        } else {
                            schema_tokens_for_complex_enum(&input.ident, &message_name, &enum_data, &config, &message_name)
                        }
                    } else {
                        if is_simple_enum {
                            crate::schema::schema_tokens_for_simple_enum_concrete(&input.ident, &message_name, &enum_data, &config, &message_name)
                        } else {
                            crate::schema::schema_tokens_for_complex_enum_concrete(&input.ident, &message_name, &enum_data, &config, &message_name)
                        }
                    };
                    // Only emit proto file entry for concrete variants (not base generic type)
                    // Base generic type (empty substitutions) is only for Rust client schema
                    let has_type_params = !input.generics.type_params().collect::<Vec<_>>().is_empty();
                    if !has_type_params || !variant.substitutions.is_empty() {
                        config.register_and_emit_proto(&proto);
                    }
                    let SchemaTokens { schema, inventory_submit } = schema_tokens;
                    schema_tokens_col = quote! { #schema #schema_tokens_col};
                    inventory_tokens_col = quote! { #inventory_submit #inventory_tokens_col};
                }
            }

            let item_enum: ItemEnum = syn::parse2(item_ts).expect("failed to parse enum");
            let type_tokens = if is_simple_enum {
                generate_simple_enum_impl(&input, &item_enum, data, &config)
            } else {
                match generate_complex_enum_impl(&input, &item_enum, data, &config) {
                    Ok(tokens) => tokens,
                    Err(err) => return err.to_compile_error().into(),
                }
            };
            quote! {#type_tokens #schema_tokens_col #inventory_tokens_col}
        }
        Data::Union(_) => Error::new_spanned(&input.ident, "proto_message cannot be used on unions").to_compile_error(),
    };

    // Only generate ProtoIdentifiable if we're not dealing with a generic type that only has concrete variants
    // If we have generic_types configured, we generate ProtoIdentifiable for each concrete variant instead of the generic type
    let has_type_params = input.generics.type_params().next().is_some();
    let has_concrete_variants_only = has_type_params && generic_variants.iter().all(|v| !v.substitutions.is_empty());

    let proto_ident_const = if has_concrete_variants_only {
        // Generate ProtoIdentifiable for each concrete variant
        let mut impls = Vec::new();
        for variant in &generic_variants {
            let message_name = if variant.suffix.is_empty() {
                proto_names.first().map_or_else(|| input.ident.to_string(), ToString::to_string)
            } else {
                format!("{}{}", proto_names.first().map_or_else(|| input.ident.to_string(), ToString::to_string), variant.suffix)
            };

            // Build the concrete type by substituting generic parameters
            let type_ident = &input.ident;
            let type_with_concrete_args = if variant.substitutions.is_empty() {
                quote! { #type_ident }
            } else {
                // Get the concrete type arguments from substitutions
                let type_args: Vec<_> = input
                    .generics
                    .type_params()
                    .map(|param| {
                        variant.substitutions.get(&param.ident.to_string()).map_or_else(
                            || {
                                let ident = &param.ident;
                                quote! { #ident }
                            },
                            |ty| quote! { #ty },
                        )
                    })
                    .collect();
                quote! { #type_ident<#(#type_args),*> }
            };

            let (proto_package, proto_file_path) = config.proto_path().map_or_else(
                || (String::new(), String::new()),
                |path| {
                    let file_name = std::path::Path::new(path).file_name().and_then(|name| name.to_str()).unwrap_or(path);
                    (crate::utils::derive_package_name(file_name), path.to_string())
                },
            );

            let type_name_literal = input.ident.to_string();
            impls.push(quote! {
                #[cfg(feature = "build-schemas")]
                impl ::proto_rs::schemas::ProtoIdentifiable for #type_with_concrete_args {
                    const PROTO_IDENT: ::proto_rs::schemas::ProtoIdent = ::proto_rs::schemas::ProtoIdent {
                        module_path: ::core::module_path!(),
                        name: #type_name_literal,
                        proto_package_name: #proto_package,
                        proto_file_path: #proto_file_path,
                        proto_type: #message_name,
                    };
                }
            });
        }
        quote! { #(#impls)* }
    } else {
        assoc_proto_ident_const(&config, &input.ident, &input.generics, &proto_names)
    };

    let proto_imports = config.imports_mat;
    quote! {
        #proto_imports
        #tokens
        #proto_ident_const
    }
    .into()
}
