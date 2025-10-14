use proc_macro::TokenStream;
use quote::quote;
use syn::Attribute;
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
    strip_proto_attributes(&mut input.data);
    let proto = config.imports_mat;
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
    strip_proto_attributes_from_trait(&mut input);
    let proto = config.imports_mat;
    quote! {
        #input
        #proto
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
            syn::TraitItem::Verbatim(_) => {}
            _ => {}
        }
    }
}
