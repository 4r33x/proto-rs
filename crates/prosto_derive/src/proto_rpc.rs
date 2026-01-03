use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::ItemTrait;

mod client;
pub mod rpc_common;
mod server;
pub mod utils; // Add this

use client::generate_client_module;
use server::generate_server_module;
use utils::extract_methods_and_types; // Add this import

use crate::emit_proto::generate_service_content;
use crate::parse::UnifiedProtoConfig;
use crate::write_file::ProtoEntry;

pub fn proto_rpc_impl(args: TokenStream, item: TokenStream) -> TokenStream2 {
    let input: ItemTrait = syn::parse(item).expect("Failed to parse trait");
    let trait_name = &input.ident;
    let ty_ident = trait_name.to_string();
    let mut config = UnifiedProtoConfig::from_attributes(args, &ty_ident, &input.attrs, &input, input.ident.span());
    let vis = &input.vis;
    let package_name = config.get_rpc_package().to_owned();

    // Extract methods, types, and imports
    let (methods, user_associated_types) = extract_methods_and_types(&input);

    // Generate .proto file if requested
    let service_content = generate_service_content(
        trait_name,
        &methods,
        &config.type_imports,
        config.import_all_from.as_deref(),
    );
    config.register_and_emit_proto(&ty_ident, ProtoEntry::definition(service_content, Vec::new()));
    let proto = config.imports_mat.clone();

    // Generate user-facing trait
    let user_methods: Vec<_> = methods.iter().map(|m| &m.user_method_signature).collect();

    // Generate client module if requested
    let client_module = if config.rpc_client {
        generate_client_module(trait_name, vis, &package_name, &methods)
    } else {
        quote! {}
    };

    // Generate server module if requested
    let server_module = if config.rpc_server {
        generate_server_module(trait_name, vis, &package_name, &methods)
    } else {
        quote! {}
    };

    quote! {
        #vis trait #trait_name {
            #(#user_associated_types)*
            #(#user_methods)*
        }

        #client_module
        #server_module
        #proto
    }
}
