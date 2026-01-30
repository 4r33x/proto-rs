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
use crate::schema::SchemaTokens;
use crate::schema::schema_tokens_for_service;
use crate::schema::type_references_generic_params;

pub fn proto_rpc_impl(args: TokenStream, item: TokenStream) -> TokenStream2 {
    let input: ItemTrait = syn::parse(item).expect("Failed to parse trait");
    let trait_name = &input.ident;
    let ty_ident = trait_name.to_string();
    let mut config = UnifiedProtoConfig::from_attributes(args, &ty_ident, &input.attrs, &input, input.generics.clone());
    let vis = &input.vis;
    let package_name = config.get_rpc_package().to_owned();

    // Extract methods, types, and imports
    let (methods, user_associated_types) = extract_methods_and_types(&input);

    // Generate .proto file if requested
    let service_content = generate_service_content(trait_name, &methods, &config.type_imports, config.import_all_from.as_deref());
    let SchemaTokens { schema, inventory_submit } =
        schema_tokens_for_service(&input.ident, &ty_ident, &methods, &package_name, &config, &ty_ident);
    config.register_and_emit_proto(&service_content);
    let proto = config.imports_mat.clone();

    let mut validator_consts = Vec::new();
    for method in &methods {
        if !type_references_generic_params(&method.request_type, &input.generics) {
            validator_consts.push(build_validator_const(&method.request_type));
        }
        let response_ty = method.inner_response_type.as_ref().unwrap_or(&method.response_type);
        if !type_references_generic_params(response_ty, &input.generics) {
            validator_consts.push(build_validator_const(response_ty));
        }
    }

    // Generate user-facing trait
    let user_methods: Vec<_> = methods.iter().map(|m| &m.user_method_signature).collect();

    let interceptor_trait = if let Some(interceptor_config) = config.rpc_client_ctx.as_ref() {
        let trait_ident = &interceptor_config.trait_ident;
        let ctx_ident = &interceptor_config.ctx_ident;
        quote! {
            #vis trait #trait_ident<#ctx_ident>: Send + Sync + 'static {
                fn intercept<T>(&self, ctx: #ctx_ident, req: &mut tonic::Request<T>);
            }
        }
    } else {
        quote! {}
    };

    // Generate client module if requested
    let client_module = if config.rpc_client {
        generate_client_module(trait_name, vis, &package_name, &methods, config.rpc_client_ctx.as_ref())
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
        #schema
        #inventory_submit
        #proto
        #(#validator_consts)*
        //#interceptor_trait
        #vis trait #trait_name {
            #(#user_associated_types)*
            #(#user_methods)*

        }

        #client_module
        #server_module
    }
}

fn build_validator_const(ty: &syn::Type) -> TokenStream2 {
    quote! {
        #[cfg(feature = "build-schemas")]
        const _: () = <#ty as ::proto_rs::schemas::ProtoIdentifiable>::_VALIDATOR;
    }
}
