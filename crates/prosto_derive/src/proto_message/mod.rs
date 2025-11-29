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
use crate::parse::substitute_generic_types;

mod complex_enums;
mod enums;
mod structs;
mod unified_field_handler;

use complex_enums::generate_complex_enum_impl;
use enums::generate_simple_enum_impl;
use structs::generate_struct_impl;

pub fn proto_message_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_ts: TokenStream2 = item.clone().into();
    let input: DeriveInput = syn::parse2(item_ts.clone()).expect("proto_message expects a type definition");

    let type_ident = input.ident.to_string();
    let mut config = UnifiedProtoConfig::from_attributes(attr, &type_ident, &input.attrs, &input.data);

    // Check if we have generic types specified
    if config.has_generic_types() {
        return handle_generic_types(input, item_ts, config);
    }

    let proto_names = config.proto_message_names(&type_ident);

    let tokens = match input.data {
        Data::Struct(ref data) => {
            for proto_name in &proto_names {
                let proto = generate_struct_proto(proto_name, &data.fields);
                config.register_and_emit_proto(proto_name, &proto);
            }

            let item_struct: ItemStruct = syn::parse2(item_ts).expect("failed to parse struct");
            generate_struct_impl(&input, &item_struct, data, &config)
        }
        Data::Enum(ref data) => {
            let is_simple_enum = data.variants.iter().all(|variant| matches!(variant.fields, Fields::Unit));
            for proto_name in &proto_names {
                let proto = if is_simple_enum {
                    generate_simple_enum_proto(proto_name, data)
                } else {
                    generate_complex_enum_proto(proto_name, data)
                };
                config.register_and_emit_proto(proto_name, &proto);
            }

            let item_enum: ItemEnum = syn::parse2(item_ts).expect("failed to parse enum");
            if is_simple_enum {
                generate_simple_enum_impl(&input, &item_enum, data, &config)
            } else {
                match generate_complex_enum_impl(&input, &item_enum, data, &config) {
                    Ok(tokens) => tokens,
                    Err(err) => return err.to_compile_error().into(),
                }
            }
        }
        Data::Union(_) => Error::new_spanned(&input.ident, "proto_message cannot be used on unions").to_compile_error(),
    };

    let proto_imports = config.imports_mat;
    quote! {
        #proto_imports
        #tokens
    }
    .into()
}

fn handle_generic_types(input: DeriveInput, item_ts: TokenStream2, mut config: UnifiedProtoConfig) -> TokenStream {
    let type_ident = input.ident.to_string();
    let instantiations = config.compute_generic_instantiations();

    match input.data {
        Data::Struct(ref data) => {
            for instantiation in &instantiations {
                let concrete_name = format!("{}{}", type_ident, instantiation.name_suffix);

                // Substitute generic types in fields
                let substituted_fields = substitute_fields(&data.fields, &instantiation.substitutions);

                // Generate proto
                let proto = generate_struct_proto(&concrete_name, &substituted_fields);
                config.register_and_emit_proto(&concrete_name, &proto);
            }
        }
        Data::Enum(ref data) => {
            let is_simple_enum = data.variants.iter().all(|variant| matches!(variant.fields, Fields::Unit));

            for instantiation in &instantiations {
                let concrete_name = format!("{}{}", type_ident, instantiation.name_suffix);

                // Generate proto
                let proto = if is_simple_enum {
                    generate_simple_enum_proto(&concrete_name, data)
                } else {
                    // Substitute generic types in enum variants
                    let substituted_data = substitute_enum_variants(data, &instantiation.substitutions);
                    generate_complex_enum_proto(&concrete_name, &substituted_data)
                };
                config.register_and_emit_proto(&concrete_name, &proto);
            }
        }
        Data::Union(_) => {
            return Error::new_spanned(&input.ident, "proto_message cannot be used on unions")
                .to_compile_error()
                .into();
        }
    }

    // Preserve the original item (struct or enum)
    let original_item = match input.data {
        Data::Struct(_) => {
            let item_struct: ItemStruct = syn::parse2(item_ts).expect("failed to parse struct");
            let sanitized = sanitize_struct_for_generics(item_struct);
            quote! { #sanitized }
        }
        Data::Enum(_) => {
            let item_enum: ItemEnum = syn::parse2(item_ts).expect("failed to parse enum");
            let sanitized = sanitize_enum_for_generics(item_enum);
            quote! { #sanitized }
        }
        _ => quote! {},
    };

    // Generate associated const implementations for type identification
    let type_id_impls = generate_type_id_impls(&input.ident, &input.generics, &instantiations);

    let proto_imports = config.imports_mat;

    quote! {
        #original_item
        #type_id_impls
        #proto_imports

        // NOTE: Proto files have been generated for all generic type combinations.
        // The generic types have TYPE_ID and PROTO_TYPE_NAME associated constants
        // that can be used for runtime dispatching in RPC methods.
    }
    .into()
}

fn substitute_fields(fields: &Fields, substitutions: &std::collections::HashMap<String, syn::Type>) -> Fields {
    match fields {
        Fields::Named(named) => {
            let mut new_fields = named.clone();
            for field in &mut new_fields.named {
                field.ty = substitute_generic_types(&field.ty, substitutions);
            }
            Fields::Named(new_fields)
        }
        Fields::Unnamed(unnamed) => {
            let mut new_fields = unnamed.clone();
            for field in &mut new_fields.unnamed {
                field.ty = substitute_generic_types(&field.ty, substitutions);
            }
            Fields::Unnamed(new_fields)
        }
        Fields::Unit => Fields::Unit,
    }
}

fn substitute_enum_variants(
    data: &syn::DataEnum,
    substitutions: &std::collections::HashMap<String, syn::Type>,
) -> syn::DataEnum {
    let mut new_data = data.clone();
    for variant in &mut new_data.variants {
        variant.fields = substitute_fields(&variant.fields, substitutions);
    }
    new_data
}

fn generate_type_id_impls(
    type_name: &syn::Ident,
    generics: &syn::Generics,
    instantiations: &[crate::parse::GenericTypeInstantiation],
) -> TokenStream2 {
    if instantiations.is_empty() {
        return quote! {};
    }

    // Generate enum for TYPE_ID
    let enum_name = quote::format_ident!("{}TypeId", type_name);
    let enum_variants: Vec<_> = instantiations
        .iter()
        .map(|inst| {
            let variant_name = quote::format_ident!("{}", inst.name_suffix);
            quote! { #variant_name }
        })
        .collect();

    let enum_def = quote! {
        /// Type identifier enum for generic instantiations
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #enum_name {
            #(#enum_variants),*
        }
    };

    // Generate impl blocks for each instantiation
    let impls: Vec<_> = instantiations
        .iter()
        .map(|inst| {
            let variant_name = quote::format_ident!("{}", inst.name_suffix);
            let proto_type_name = format!("{}{}", type_name, inst.name_suffix);

            // Build concrete type arguments
            let concrete_args: Vec<_> = generics
                .params
                .iter()
                .filter_map(|param| {
                    if let syn::GenericParam::Type(type_param) = param {
                        let param_name = type_param.ident.to_string();
                        inst.substitutions.get(&param_name).cloned()
                    } else {
                        None
                    }
                })
                .collect();

            if concrete_args.is_empty() {
                return quote! {};
            }

            quote! {
                impl #type_name<#(#concrete_args),*> {
                    /// Type identifier for this generic instantiation
                    pub const TYPE_ID: #enum_name = #enum_name::#variant_name;

                    /// Proto message name for this generic instantiation
                    pub const PROTO_TYPE_NAME: &'static str = #proto_type_name;
                }
            }
        })
        .collect();

    quote! {
        #enum_def
        #(#impls)*
    }
}

fn sanitize_struct_for_generics(mut item: ItemStruct) -> ItemStruct {
    use unified_field_handler::strip_proto_attrs;

    item.attrs = strip_proto_attrs(&item.attrs);
    match &mut item.fields {
        syn::Fields::Named(named) => {
            for field in &mut named.named {
                field.attrs = strip_proto_attrs(&field.attrs);
            }
        }
        syn::Fields::Unnamed(unnamed) => {
            for field in &mut unnamed.unnamed {
                field.attrs = strip_proto_attrs(&field.attrs);
            }
        }
        syn::Fields::Unit => {}
    }
    item
}

fn sanitize_enum_for_generics(mut item: ItemEnum) -> ItemEnum {
    use unified_field_handler::strip_proto_attrs;

    item.attrs = strip_proto_attrs(&item.attrs);
    for variant in &mut item.variants {
        variant.attrs = strip_proto_attrs(&variant.attrs);
        match &mut variant.fields {
            syn::Fields::Named(named) => {
                for field in &mut named.named {
                    field.attrs = strip_proto_attrs(&field.attrs);
                }
            }
            syn::Fields::Unnamed(unnamed) => {
                for field in &mut unnamed.unnamed {
                    field.attrs = strip_proto_attrs(&field.attrs);
                }
            }
            syn::Fields::Unit => {}
        }
    }
    item
}
