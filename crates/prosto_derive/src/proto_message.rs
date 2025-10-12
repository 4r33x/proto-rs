use proc_macro::TokenStream;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;
use syn::parse_macro_input;

use crate::emit_proto::generate_complex_enum_proto;
use crate::emit_proto::generate_simple_enum_proto;
use crate::emit_proto::generate_struct_proto;
use crate::parse::UnifiedProtoConfig;

mod complex_enum_handler;
mod enum_handler;
mod struct_handler;
mod unified_field_handler;

pub fn proto_message_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let type_ident = input.ident.to_string();
    let mut config = UnifiedProtoConfig::from_attributes(attr, &type_ident, &input.attrs, &input.data);

    let (proto, rust_code) = match input.data.clone() {
        Data::Struct(data) => {
            let proto = generate_struct_proto(&type_ident, &data.fields);

            let rust_code = struct_handler::handle_struct(input, &data);
            (proto, rust_code)
        }
        Data::Enum(data) => {
            let is_simple_enum = data.variants.iter().all(|v| matches!(v.fields, Fields::Unit));
            if is_simple_enum {
                let proto = generate_simple_enum_proto(&type_ident, &data);

                let rust_code = enum_handler::handle_enum(input, &data);
                (proto, rust_code)
            } else {
                let proto = generate_complex_enum_proto(&type_ident, &data);

                let rust_code = complex_enum_handler::handle_complex_enum(input, &data);
                (proto, rust_code)
            }
        }
        _ => panic!("proto_message can only be used on structs and enums"),
    };

    config.register_and_emit_proto(&type_ident, &proto);
    let proto = config.imports_mat;
    quote! {
        #proto
        #rust_code
    }
    .into()
}
