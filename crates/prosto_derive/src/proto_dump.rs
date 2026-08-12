use proc_macro::TokenStream;
use quote::quote;
use syn::Attribute;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;
use syn::ItemTrait;

use crate::generic_substitutions::apply_generic_substitutions_enum;
use crate::generic_substitutions::apply_generic_substitutions_fields;
use crate::parse::UnifiedProtoConfig;
use crate::proto_rpc::utils::extract_methods_and_types;
use crate::schema::SchemaTokens;
use crate::schema::schema_tokens_for_complex_enum;
use crate::schema::schema_tokens_for_service;
use crate::schema::schema_tokens_for_simple_enum;
use crate::schema::schema_tokens_for_struct;

pub fn proto_dump_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(input) = syn::parse::<DeriveInput>(item.clone()) {
        let type_ident = input.ident.to_string();
        let config = UnifiedProtoConfig::from_attributes(attr, &type_ident, &input.attrs, &input.data, input.generics.clone());
        return struct_or_enum(input, config);
    }
    if let Ok(input) = syn::parse::<syn::ItemTrait>(item) {
        let type_ident = input.ident.to_string();
        let config = UnifiedProtoConfig::from_attributes(attr, &type_ident, &input.attrs, &input, input.generics.clone());
        return trait_service(input, config);
    }

    panic!("proto_dump can only be used on structs, enums, or traits (services)");
}

fn struct_or_enum(mut input: DeriveInput, config: UnifiedProtoConfig) -> TokenStream {
    let proto_name = input.ident.to_string();
    let clean_name = proto_name.strip_suffix("Proto").unwrap_or(&proto_name);

    let generic_variants = match config.generic_type_variants(&input.generics) {
        Ok(variants) => variants,
        Err(err) => return err.to_compile_error().into(),
    };

    let schema = match &input.data {
        Data::Struct(data) => {
            let mut schema_tokens_col = quote! {};
            let mut inventory_tokens_col = quote! {};
            for variant in &generic_variants {
                let message_name = if variant.suffix.is_empty() {
                    clean_name.to_string()
                } else {
                    format!("{clean_name}{}", variant.suffix)
                };
                let fields = apply_generic_substitutions_fields(&data.fields, &variant.substitutions);
                // Use _concrete version if we have substitutions
                let SchemaTokens { schema, inventory_submit } = if variant.substitutions.is_empty() {
                    schema_tokens_for_struct(&input.ident, &message_name, &fields, &config, &message_name)
                } else {
                    crate::schema::schema_tokens_for_struct_concrete(&input.ident, &message_name, &fields, &config, &message_name)
                };
                schema_tokens_col = quote! { #schema #schema_tokens_col};
                inventory_tokens_col = quote! { #inventory_submit #inventory_tokens_col};
            }
            SchemaTokens {
                schema: schema_tokens_col,
                inventory_submit: inventory_tokens_col,
            }
        }
        Data::Enum(data) => {
            let is_simple_enum = data.variants.iter().all(|v| matches!(v.fields, Fields::Unit));
            let mut schema_tokens_col = quote! {};
            let mut inventory_tokens_col = quote! {};
            for variant in &generic_variants {
                let message_name = if variant.suffix.is_empty() {
                    clean_name.to_string()
                } else {
                    format!("{clean_name}{}", variant.suffix)
                };
                let data = apply_generic_substitutions_enum(data, &variant.substitutions);
                // Use _concrete version if we have substitutions
                let schema_tokens = if variant.substitutions.is_empty() {
                    if is_simple_enum {
                        schema_tokens_for_simple_enum(&input.ident, &message_name, &data, &config, &message_name)
                    } else {
                        schema_tokens_for_complex_enum(&input.ident, &message_name, &data, &config, &message_name)
                    }
                } else {
                    if is_simple_enum {
                        crate::schema::schema_tokens_for_simple_enum_concrete(&input.ident, &message_name, &data, &config, &message_name)
                    } else {
                        crate::schema::schema_tokens_for_complex_enum_concrete(&input.ident, &message_name, &data, &config, &message_name)
                    }
                };
                let SchemaTokens { schema, inventory_submit } = schema_tokens;
                schema_tokens_col = quote! { #schema #schema_tokens_col};
                inventory_tokens_col = quote! { #inventory_submit #inventory_tokens_col};
            }
            SchemaTokens {
                schema: schema_tokens_col,
                inventory_submit: inventory_tokens_col,
            }
        }
        Data::Union(_) => {
            panic!("proto_dump can only be used on structs and enums, make PR/issue if you want unions")
        }
    };
    strip_proto_attributes(&mut input.data);
    let proto = config.imports_mat;
    let SchemaTokens { schema, inventory_submit } = schema;
    quote! {
        #input
        #proto
        #schema
        #inventory_submit
    }
    .into()
}

fn trait_service(mut input: ItemTrait, config: UnifiedProtoConfig) -> TokenStream {
    let proto_name = input.ident.to_string();
    let clean_name = proto_name.strip_suffix("Proto").unwrap_or(&proto_name);
    let (methods, _) = extract_methods_and_types(&input);
    let rpc_package = config.get_rpc_package();
    let schema_tokens = schema_tokens_for_service(&input.ident, clean_name, &methods, rpc_package, &config, clean_name);
    strip_proto_attributes_from_trait(&mut input);
    let proto = config.imports_mat;
    let SchemaTokens { schema, inventory_submit } = schema_tokens;
    quote! {
        #input
        #proto
        #schema
        #inventory_submit
    }
    .into()
}

fn strip_proto_attributes(data: &mut Data) {
    match data {
        Data::Struct(data) => strip_proto_from_fields(&mut data.fields),
        Data::Enum(data) => {
            for variant in &mut data.variants {
                strip_proto_from_attrs(&mut variant.attrs);
                strip_proto_from_fields(&mut variant.fields);
            }
        }
        Data::Union(_) => {}
    }
}

fn strip_proto_from_fields(fields: &mut Fields) {
    match fields {
        Fields::Named(fields) => {
            for field in &mut fields.named {
                strip_proto_from_attrs(&mut field.attrs);
            }
        }
        Fields::Unnamed(fields) => {
            for field in &mut fields.unnamed {
                strip_proto_from_attrs(&mut field.attrs);
            }
        }
        Fields::Unit => {}
    }
}

fn strip_proto_from_attrs(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| !attr.path().is_ident("proto"));
}

fn strip_proto_attributes_from_trait(item: &mut ItemTrait) {
    strip_proto_from_attrs(&mut item.attrs);
    for trait_item in &mut item.items {
        match trait_item {
            syn::TraitItem::Const(const_item) => {
                strip_proto_from_attrs(&mut const_item.attrs);
            }
            syn::TraitItem::Fn(fn_item) => {
                strip_proto_from_attrs(&mut fn_item.attrs);
                for input in &mut fn_item.sig.inputs {
                    if let syn::FnArg::Typed(pat_type) = input {
                        strip_proto_from_attrs(&mut pat_type.attrs);
                    }
                }
            }
            syn::TraitItem::Type(type_item) => {
                strip_proto_from_attrs(&mut type_item.attrs);
            }
            syn::TraitItem::Macro(macro_item) => {
                strip_proto_from_attrs(&mut macro_item.attrs);
            }

            _ => {}
        }
    }
}
