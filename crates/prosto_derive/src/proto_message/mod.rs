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
use crate::generic_substitutions::apply_generic_substitutions_enum;
use crate::generic_substitutions::apply_generic_substitutions_fields;
use crate::parse::UnifiedProtoConfig;
use crate::schema::assoc_proto_ident_const;
use crate::schema::schema_tokens_for_complex_enum;
use crate::schema::schema_tokens_for_simple_enum;
use crate::schema::schema_tokens_for_struct;

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
    let generic_variants = match config.generic_type_variants(&input.generics) {
        Ok(variants) => variants,
        Err(err) => return err.to_compile_error().into(),
    };

    if config.transparent && config.proto_path().is_some() {
        return Error::new_spanned(&input.ident, "transparent proto_message types must not be written to .proto files")
            .to_compile_error()
            .into();
    }

    let tokens = match input.data {
        Data::Struct(ref data) => {
            for proto_name in &proto_names {
                for variant in &generic_variants {
                    let message_name = if variant.suffix.is_empty() {
                        proto_name.clone()
                    } else {
                        format!("{proto_name}{}", variant.suffix)
                    };
                    let substituted_fields = apply_generic_substitutions_fields(&data.fields, &variant.substitutions);
                    let proto = generate_struct_proto(&message_name, &substituted_fields);
                    let schema_tokens = schema_tokens_for_struct(&input.ident, &message_name, &substituted_fields, &config, &message_name);
                    config.register_and_emit_proto(&proto, schema_tokens);
                }
            }

            let item_struct: ItemStruct = syn::parse2(item_ts).expect("failed to parse struct");
            generate_struct_impl(&input, &item_struct, data, &config)
        }
        Data::Enum(ref data) => {
            let is_simple_enum = data.variants.iter().all(|variant| matches!(variant.fields, Fields::Unit));
            for proto_name in &proto_names {
                for variant in &generic_variants {
                    let message_name = if variant.suffix.is_empty() {
                        proto_name.clone()
                    } else {
                        format!("{proto_name}{}", variant.suffix)
                    };
                    let data = apply_generic_substitutions_enum(data, &variant.substitutions);
                    let proto = if is_simple_enum {
                        generate_simple_enum_proto(&message_name, &data)
                    } else {
                        generate_complex_enum_proto(&message_name, &data)
                    };
                    let schema_tokens = if is_simple_enum {
                        schema_tokens_for_simple_enum(&input.ident, &message_name, &data, &config, &message_name)
                    } else {
                        schema_tokens_for_complex_enum(&input.ident, &message_name, &data, &config, &message_name)
                    };
                    config.register_and_emit_proto(&proto, schema_tokens);
                }
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

    let proto_ident_const = assoc_proto_ident_const(&config, &input.ident, &input.generics, &proto_names, &generic_variants);
    let proto_imports = config.imports_mat;
    quote! {
        #proto_imports
        #tokens
        #proto_ident_const
    }
    .into()
}
