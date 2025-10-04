use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemTrait;
use syn::LitStr;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;

mod client;
mod proto;
mod server;
mod utils;

use client::generate_client_module;
use proto::generate_proto_file;
use server::generate_server_module;
use utils::extract_methods_and_types;

/// Attribute arguments for proto_rpc
struct ProtoRpcArgs {
    package: syn::Ident,
    options: GenerationOptions,
}

#[derive(Default)]
struct GenerationOptions {
    generate_client: bool,
    generate_server: bool,
    generate_proto: bool,
    proto_path: Option<String>,
}

impl Parse for ProtoRpcArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse package name (required)
        let package: syn::Ident = input.parse()?;

        let mut options = GenerationOptions {
            generate_client: false,
            generate_server: false,
            generate_proto: false,
            proto_path: None,
        };

        // Parse optional arguments
        while !input.is_empty() {
            input.parse::<Token![,]>()?;

            if input.is_empty() {
                break;
            }

            let key: syn::Ident = input.parse()?;
            let key_str = key.to_string();

            match key_str.as_str() {
                "client" => {
                    input.parse::<Token![=]>()?;
                    let value: syn::LitBool = input.parse()?;
                    options.generate_client = value.value;
                }
                "server" => {
                    input.parse::<Token![=]>()?;
                    let value: syn::LitBool = input.parse()?;
                    options.generate_server = value.value;
                }
                "proto" => {
                    input.parse::<Token![=]>()?;
                    let value: syn::LitBool = input.parse()?;
                    options.generate_proto = value.value;
                }
                "proto_path" => {
                    input.parse::<Token![=]>()?;
                    let value: LitStr = input.parse()?;
                    options.proto_path = Some(value.value());
                }
                _ => {
                    return Err(syn::Error::new(key.span(), format!("Unknown option: {}", key_str)));
                }
            }
        }
        if !options.generate_client && options.generate_server && options.generate_proto {
            panic!("No options for codegen selected")
        }

        Ok(ProtoRpcArgs { package, options })
    }
}

pub fn proto_rpc_impl(args: TokenStream, item: TokenStream) -> TokenStream {
    let args: ProtoRpcArgs = syn::parse2(args).expect("Failed to parse macro arguments");
    let input: ItemTrait = syn::parse2(item).expect("Failed to parse trait");

    let trait_name = &input.ident;
    let vis = &input.vis;
    let package_name = args.package.to_string();

    // Extract methods, types, and imports
    let (methods, user_associated_types, proto_imports) = extract_methods_and_types(&input);

    // Generate .proto file if requested
    if args.options.generate_proto {
        let proto_path = args.options.proto_path.as_deref().unwrap_or("generated.proto");

        generate_proto_file(
            trait_name,
            &methods,
            proto_path,
            &proto_imports, // Pass imports
        );
    }

    // Generate user-facing trait
    let user_methods: Vec<_> = methods.iter().map(|m| &m.user_method_signature).collect();

    // Generate client module if requested
    let client_module = if args.options.generate_client {
        generate_client_module(trait_name, vis, &package_name, &methods)
    } else {
        quote! {}
    };

    // Generate server module if requested
    let server_module = if args.options.generate_server {
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
    }
}
