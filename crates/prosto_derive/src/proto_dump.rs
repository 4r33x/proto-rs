use proc_macro::TokenStream;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;
use syn::ItemTrait;

use crate::emit_proto::generate_complex_enum_proto;
use crate::emit_proto::generate_service_content;
use crate::emit_proto::generate_simple_enum_proto;
use crate::emit_proto::generate_struct_proto;
use crate::parse::UnifiedProtoConfig;
use crate::proto_rpc::utils::extract_methods_and_types;

pub fn proto_dump_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(input) = syn::parse::<DeriveInput>(item.clone()) {
        let type_ident = input.ident.to_string();
        let config = UnifiedProtoConfig::from_attributes(attr, &type_ident, &input.attrs, &input.data);
        return struct_or_enum(input, config);
    }
    if let Ok(input) = syn::parse::<syn::ItemTrait>(item) {
        let type_ident = input.ident.to_string();
        let config = UnifiedProtoConfig::from_attributes(attr, &type_ident, &input.attrs, &input);
        return trait_service(input, config);
    }

    panic!("proto_dump can only be used on structs, enums, or traits (services)");
}

fn struct_or_enum(mut input: DeriveInput, mut config: UnifiedProtoConfig) -> TokenStream {
    let proto_name = input.ident.to_string();
    let clean_name = proto_name.strip_suffix("Proto").unwrap_or(&proto_name);

    let proto_def = match &input.data {
        Data::Struct(data) => generate_struct_proto(clean_name, &data.fields),
        Data::Enum(data) => {
            let is_simple_enum = data.variants.iter().all(|v| matches!(v.fields, Fields::Unit));
            if is_simple_enum {
                generate_simple_enum_proto(clean_name, data)
            } else {
                generate_complex_enum_proto(clean_name, data)
            }
        }
        Data::Union(_) => panic!("proto_dump can only be used on structs and enums, make PR/issue if you want unions"),
    };

    config.register_and_emit_proto(clean_name, &proto_def);
    let proto = config.imports_mat;
    strip_proto_attributes(&mut input);
    quote! {
        #input
        #proto
    }
    .into()
}

fn trait_service(mut input: ItemTrait, mut config: UnifiedProtoConfig) -> TokenStream {
    let proto_name = input.ident.to_string();
    let clean_name = proto_name.strip_suffix("Proto").unwrap_or(&proto_name);
    let (methods, _) = extract_methods_and_types(&input);
    let proto_def = generate_service_content(&input.ident, &methods, &config.type_imports);
    config.register_and_emit_proto(clean_name, &proto_def);
    let proto = config.imports_mat;
    strip_trait_proto_attributes(&mut input);
    quote! {
        #input
        #proto
    }
    .into()
}

fn strip_proto_attributes(input: &mut DeriveInput) {
    input.attrs.retain(|attr| !attr.path().is_ident("proto"));

    match &mut input.data {
        Data::Struct(data) => strip_fields(&mut data.fields),
        Data::Enum(data) => {
            for variant in &mut data.variants {
                variant.attrs.retain(|attr| !attr.path().is_ident("proto"));
                strip_fields(&mut variant.fields);
            }
        }
        Data::Union(_) => {}
    }
}

fn strip_fields(fields: &mut Fields) {
    match fields {
        Fields::Named(named) => {
            for field in &mut named.named {
                field.attrs.retain(|attr| !attr.path().is_ident("proto"));
            }
        }
        Fields::Unnamed(unnamed) => {
            for field in &mut unnamed.unnamed {
                field.attrs.retain(|attr| !attr.path().is_ident("proto"));
            }
        }
        Fields::Unit => {}
    }
}

fn strip_trait_proto_attributes(item: &mut ItemTrait) {
    item.attrs.retain(|attr| !attr.path().is_ident("proto"));
    for trait_item in &mut item.items {
        if let syn::TraitItem::Fn(fun) = trait_item {
            fun.attrs.retain(|attr| !attr.path().is_ident("proto"));
        }
    }
}
