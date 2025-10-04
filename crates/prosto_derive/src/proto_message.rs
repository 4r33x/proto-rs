use proc_macro::TokenStream;
use syn::Data;
use syn::DeriveInput;
use syn::parse_macro_input;

use crate::proto_message::utils::ProtoConfig;

mod utils;

mod enum_handler;
mod struct_handler;

pub fn proto_message_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let config = ProtoConfig::parse_proto_config(attr, &input.attrs);

    match input.data.clone() {
        Data::Struct(data) => struct_handler::handle_struct(input, &data, config),
        Data::Enum(data) => enum_handler::handle_enum(input, &data, config),
        _ => panic!("proto_message can only be used on structs and enums"),
    }
}
