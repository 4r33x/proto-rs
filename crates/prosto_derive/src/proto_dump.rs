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

fn struct_or_enum(input: DeriveInput, mut config: UnifiedProtoConfig) -> TokenStream {
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
        _ => panic!("proto_dump can only be used on structs and enums"),
    };

    config.register_and_emit_proto(clean_name, &proto_def);
    let proto = config.imports_mat;
    quote! {
        #input
        #proto
    }
    .into()
}

fn trait_service(input: ItemTrait, mut config: UnifiedProtoConfig) -> TokenStream {
    let proto_name = input.ident.to_string();
    let clean_name = proto_name.strip_suffix("Proto").unwrap_or(&proto_name);
    let (methods, _) = extract_methods_and_types(&input);
    let proto_def = generate_service_content(&input.ident, &methods, &config.type_imports);
    config.register_and_emit_proto(clean_name, &proto_def);
    let proto = config.imports_mat;
    quote! {
        #input
        #proto
    }
    .into()
}
