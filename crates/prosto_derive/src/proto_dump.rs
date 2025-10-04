use proc_macro::TokenStream;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Expr;
use syn::ExprLit;
use syn::Lit;
use syn::MetaNameValue;
use syn::parse_macro_input;

use crate::utils::ProtoConfig;
use crate::utils::generate_enum_proto;
use crate::utils::generate_struct_proto;
use crate::write_file::write_proto_file;

pub fn proto_dump_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let mut config = ProtoConfig::default();

    // Parse file name from attribute
    if !attr.is_empty() {
        let attr_input = parse_macro_input!(attr as MetaNameValue);
        if attr_input.path.is_ident("file") {
            if let Expr::Lit(ExprLit { lit: Lit::Str(litstr), .. }) = &attr_input.value {
                config.file_name = litstr.value();
            } else {
                panic!("Expected string literal for file attribute");
            }
        } else {
            panic!("Expected file = \"...\" attribute");
        }
    }

    let proto_name = input.ident.to_string();
    let clean_name = proto_name.strip_suffix("Proto").unwrap_or(&proto_name);
    // Generate proto definition based on type
    let proto_def = match &input.data {
        Data::Struct(data) => generate_struct_proto(clean_name, &data.fields),
        Data::Enum(data) => generate_enum_proto(clean_name, &data.variants),
        _ => panic!("proto_dump can only be used on structs and enums"),
    };

    // Write to proto file
    write_proto_file(&config.file_name, &proto_def);

    // Return original item unchanged
    quote! {
        #input
    }
    .into()
}
