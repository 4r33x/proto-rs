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
use crate::emit_proto::transparent_proto_type;
use crate::parse::UnifiedProtoConfig;
use crate::write_file::ProtoTypeId;
use crate::write_file::TypeInfo;
use crate::write_file::module_path_from_call_site;
use crate::write_file::register_type_info;

pub(crate) fn build_validate_with_ext_impl(config: &UnifiedProtoConfig) -> TokenStream2 {
    let Some(validator_fn) = &config.validator_with_ext else {
        return quote! {};
    };
    let validator_path: syn::Path = syn::parse_str(validator_fn).expect("invalid validator_with_ext function path");
    quote! {
        #[cfg(feature = "tonic")]
        const VALIDATE_WITH_EXT: bool = true;

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
mod structs;
mod unified_field_handler;

use complex_enums::generate_complex_enum_impl;
use enums::generate_simple_enum_impl;
use structs::generate_struct_impl;

pub fn proto_message_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_ts: TokenStream2 = item.clone().into();
    let input: DeriveInput = syn::parse2(item_ts.clone()).expect("proto_message expects a type definition");

    let type_ident = input.ident.to_string();
    let module_path = module_path_from_call_site();
    let mut config = UnifiedProtoConfig::from_attributes(attr, &type_ident, module_path, &input.attrs, &input.data);
    let proto_names = config.proto_message_names(&type_ident);

    if config.transparent {
        let transparent = match &input.data {
            Data::Struct(data) => match &data.fields {
                Fields::Named(fields) => fields.named.first(),
                Fields::Unnamed(fields) => fields.unnamed.first(),
                Fields::Unit => None,
            }
            .map(transparent_proto_type),
            _ => None,
        };

        if let Some(transparent) = transparent {
            let type_id = ProtoTypeId::for_type(&config.module_path, &config.type_name);
            register_type_info(
                type_id,
                TypeInfo {
                    type_name: config.type_name.clone(),
                    transparent: Some(transparent),
                },
            );
        }
    }

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
