//! Utilities for extracting method information from trait definitions

use std::collections::BTreeMap;

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::FnArg;
use syn::GenericParam;
use syn::ItemTrait;
use syn::PatType;
use syn::ReturnType;
use syn::TraitItem;
use syn::TraitItemType;
use syn::Type;
use syn::TypePath;

use crate::utils::MethodInfo;
use crate::utils::rust_type_path_ident;

struct ParsedMethodSignature {
    request_type: Type,
    response_type: Type,
    response_return_type: Type,
    response_is_result: bool,
    is_streaming: bool,
    stream_type_name: Option<syn::Ident>,
    inner_response_type: Option<Type>,
    stream_item_type: Option<Type>,
}

impl ParsedMethodSignature {
    fn new(sig: &syn::Signature, trait_items: &[TraitItem]) -> Self {
        let request_type = extract_request_type(sig);
        let (response_return_type, response_is_result) = extract_response_return(sig);
        let response_type = extract_proto_type(&response_return_type);
        let (is_streaming, stream_type_name, inner_response_type, stream_item_type) = extract_stream_metadata(&response_type, trait_items);

        Self {
            request_type,
            response_type,
            response_return_type,
            response_is_result,
            is_streaming,
            stream_type_name,
            inner_response_type,
            stream_item_type,
        }
    }
}

/// Extract methods and associated types from the trait definition
pub fn extract_methods_and_types(input: &ItemTrait) -> (Vec<MethodInfo>, Vec<TokenStream>) {
    let mut methods = Vec::with_capacity(input.items.len());
    let mut user_associated_types = Vec::new();

    for item in &input.items {
        match item {
            TraitItem::Fn(method) => {
                let method_name = method.sig.ident.clone();
                let signature = ParsedMethodSignature::new(&method.sig, &input.items);

                let user_method_signature = generate_user_method_signature(&method.attrs, &method_name, &signature);

                methods.push(MethodInfo {
                    name: method_name,
                    request_type: signature.request_type,
                    response_type: signature.response_type,
                    response_return_type: signature.response_return_type,
                    response_is_result: signature.response_is_result,
                    is_streaming: signature.is_streaming,
                    stream_type_name: signature.stream_type_name,
                    inner_response_type: signature.inner_response_type,
                    stream_item_type: signature.stream_item_type,
                    user_method_signature,
                });
            }
            TraitItem::Type(type_item) => {
                let type_name = &type_item.ident;
                let type_attrs = &type_item.attrs;
                let bounds = &type_item.bounds;

                user_associated_types.push(quote! {
                    #(#type_attrs)*
                    type #type_name: #bounds;
                });
            }
            _ => {}
        }
    }

    (methods, user_associated_types)
}

/// Generate user-facing method signature for the trait
fn generate_user_method_signature(attrs: &[syn::Attribute], method_name: &syn::Ident, signature: &ParsedMethodSignature) -> TokenStream {
    let future_output = if signature.is_streaming {
        let stream_name = signature.stream_type_name.as_ref().expect("streaming method must define stream name");
        if signature.response_is_result {
            quote! { ::core::result::Result<tonic::Response<Self::#stream_name>, tonic::Status> }
        } else {
            quote! { tonic::Response<Self::#stream_name> }
        }
    } else if signature.response_is_result {
        let response_return_type = &signature.response_return_type;
        quote! { ::core::result::Result<#response_return_type, tonic::Status> }
    } else {
        let response_return_type = &signature.response_return_type;
        quote! { #response_return_type }
    };

    let request_type = &signature.request_type;

    let future_type = method_future_return_type(future_output);

    quote! {
        #(#attrs)*
        fn #method_name(
            &self,
            request: tonic::Request<#request_type>,
        ) -> #future_type
        where
            Self: ::core::marker::Send + ::core::marker::Sync;
    }
}

pub(crate) fn method_future_return_type(future_output: TokenStream) -> TokenStream {
    quote! {
        impl ::core::future::Future<Output = #future_output> + ::core::marker::Send + '_
    }
}

pub(crate) fn associated_future_type(future_output: TokenStream, requires_static: bool) -> TokenStream {
    let static_bound = if requires_static {
        quote! { + 'static }
    } else {
        quote! {}
    };

    if cfg!(feature = "stable") {
        quote! {
            ::core::pin::Pin<
                ::proto_rs::alloc::boxed::Box<
                    dyn ::core::future::Future<Output = #future_output>
                        + ::core::marker::Send
                        #static_bound
                >
            >
        }
    } else {
        quote! {
            impl ::core::future::Future<Output = #future_output> + ::core::marker::Send #static_bound
        }
    }
}

pub(crate) fn wrap_async_block(block: TokenStream, boxed: bool) -> TokenStream {
    if boxed && cfg!(feature = "stable") {
        quote! { ::proto_rs::alloc::boxed::Box::pin(#block) }
    } else {
        block
    }
}

pub fn generate_proto_generic_traits(trait_generics: &syn::Generics, generic_types: &BTreeMap<syn::Ident, Vec<Type>>, package: &str, trait_name: &syn::Ident) -> TokenStream {
    let mut tokens = TokenStream::new();
    let route_prefix = format!("/{package}.{}", trait_name);

    for param in &trait_generics.params {
        if let GenericParam::Type(ty_param) = param {
            let ident = &ty_param.ident;
            let enum_ident = format_ident!("ProtoGeneric{}", ident);
            let sealed_ident = format_ident!("Sealed{}", ident);

            let Some(types) = generic_types.get(ident) else {
                continue;
            };

            let variants: Vec<_> = types
                .iter()
                .map(|ty| {
                    let variant_ident = format_ident!("{}", rust_type_path_ident(ty));
                    quote! { #variant_ident }
                })
                .collect();

            let impls: Vec<_> = types
                .iter()
                .map(|ty| {
                    let variant_ident = format_ident!("{}", rust_type_path_ident(ty));
                    quote! {
                        impl #sealed_ident for #ty {
                            const ENUM: #enum_ident = #enum_ident::#variant_ident;
                            const ROUTE_PREFIX: &'static str = #route_prefix;
                        }
                    }
                })
                .collect();

            tokens.extend(quote! {
                #[allow(non_camel_case_types)]
                pub enum #enum_ident {
                    #(#variants),*
                }

                pub trait #sealed_ident {
                    const ENUM: #enum_ident;
                    const ROUTE_PREFIX: &'static str;
                }

                #(#impls)*
            });
        }
    }

    tokens
}

fn extract_request_type(sig: &syn::Signature) -> Type {
    sig.inputs
        .iter()
        .find_map(|arg| {
            if let FnArg::Typed(PatType { ty, .. }) = arg
                && let Type::Path(TypePath { path, .. }) = &**ty
                && let Some(segment) = path.segments.last()
                && segment.ident == "Request"
                && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
            {
                Some(inner_ty.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("Could not extract request type"))
}

fn extract_response_return(sig: &syn::Signature) -> (Type, bool) {
    if let ReturnType::Type(_, ty) = &sig.output {
        if let Type::Path(TypePath { path, .. }) = &**ty
            && let Some(segment) = path.segments.last()
            && segment.ident == "Result"
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(success_ty)) = args.args.first()
        {
            return (success_ty.clone(), true);
        }

        return ((**ty).clone(), false);
    }

    panic!("RPC trait methods must return a type");
}

fn extract_proto_type(success_ty: &Type) -> Type {
    let mut current = success_ty.clone();

    loop {
        if let Type::Path(TypePath { path, .. }) = &current
            && let Some(segment) = path.segments.last()
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
            && matches!(segment.ident.to_string().as_str(), "Response" | "ZeroCopyResponse" | "Box" | "Arc")
        {
            current = inner_ty.clone();
            continue;
        }

        break;
    }

    current
}

fn extract_stream_metadata(response_type: &Type, trait_items: &[TraitItem]) -> (bool, Option<syn::Ident>, Option<Type>, Option<Type>) {
    if let Type::Path(TypePath { qself: None, path }) = response_type {
        let mut segments = path.segments.iter();
        if let (Some(self_segment), Some(stream_segment)) = (segments.next(), segments.next())
            && self_segment.ident == "Self"
        {
            let stream_name = stream_segment.ident.clone();
            let (item_ty, proto_ty) = find_stream_item_types(&stream_name, trait_items).unwrap_or_else(|| panic!("Could not find associated type definition for {stream_name}"));
            return (true, Some(stream_name), Some(proto_ty), Some(item_ty));
        }
    }

    (false, None, None, None)
}

fn find_stream_item_types(stream_name: &syn::Ident, trait_items: &[TraitItem]) -> Option<(Type, Type)> {
    trait_items.iter().find_map(|item| match item {
        TraitItem::Type(TraitItemType { ident, bounds, .. }) if ident == stream_name => Some(extract_stream_item_from_bounds(bounds)),
        _ => None,
    })
}

/// Extract inner type from Stream trait bounds
pub fn extract_stream_item_from_bounds(bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>) -> (Type, Type) {
    for bound in bounds {
        if let syn::TypeParamBound::Trait(trait_bound) = bound {
            for segment in &trait_bound.path.segments {
                if segment.ident == "Stream"
                    && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                {
                    for arg in &args.args {
                        if let syn::GenericArgument::AssocType(assoc) = arg
                            && assoc.ident == "Item"
                            && let Type::Path(result_path) = &assoc.ty
                            && let Some(result_seg) = result_path.path.segments.last()
                            && result_seg.ident == "Result"
                            && let syn::PathArguments::AngleBracketed(result_args) = &result_seg.arguments
                            && let Some(syn::GenericArgument::Type(inner_ty)) = result_args.args.first()
                        {
                            let item_ty = inner_ty.clone();
                            let proto_ty = extract_proto_type(inner_ty);
                            return (item_ty, proto_ty);
                        }
                    }
                }
            }
        }
    }
    panic!("Could not extract inner type from Stream bounds");
}

#[cfg(test)]
mod tests {
    use syn::ItemTrait;
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_parsed_method_signature_variants() {
        let trait_input: ItemTrait = parse_quote! {
            trait TestService {
                type MyStream: tonic::codegen::tokio_stream::Stream<Item = Result<MyResponse, tonic::Status>> + Send + 'static;
                type ZeroCopyStream: tonic::codegen::tokio_stream::Stream<Item = Result<proto_rs::ZeroCopyResponse<MyResponse>, tonic::Status>> + Send + 'static;

                async fn unary(
                    &self,
                    request: tonic::Request<MyRequest>
                ) -> Result<tonic::Response<MyResponse>, tonic::Status>;

                async fn zero_copy(
                    &self,
                    request: tonic::Request<MyRequest>
                ) -> Result<proto_rs::ZeroCopyResponse<MyResponse>, tonic::Status>;

                async fn streaming(
                    &self,
                    request: tonic::Request<MyRequest>
                ) -> Result<tonic::Response<Self::MyStream>, tonic::Status>;

                async fn streaming_zero_copy(
                    &self,
                    request: tonic::Request<MyRequest>
                ) -> Result<tonic::Response<Self::ZeroCopyStream>, tonic::Status>;

                async fn arced(
                    &self,
                    request: tonic::Request<MyRequest>
                ) -> Result<tonic::Response<::std::sync::Arc<MyResponse>>, tonic::Status>;

                async fn boxed(
                    &self,
                    request: tonic::Request<MyRequest>
                ) -> Result<tonic::Response<::std::boxed::Box<MyResponse>>, tonic::Status>;

                async fn plain(
                    &self,
                    request: tonic::Request<MyRequest>
                ) -> MyResponse;
            }
        };

        let signatures: Vec<_> = trait_input
            .items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Fn(fun) => Some(ParsedMethodSignature::new(&fun.sig, &trait_input.items)),
                _ => None,
            })
            .collect();

        let unary = &signatures[0];
        let request_ty = &unary.request_type;
        let response_ty = &unary.response_type;
        assert_eq!(quote!(#request_ty).to_string(), "MyRequest");
        assert_eq!(quote!(#response_ty).to_string(), "MyResponse");
        assert!(unary.response_is_result);
        assert!(!unary.is_streaming);

        let zero_copy = &signatures[1];
        let zero_copy_return = &zero_copy.response_return_type;
        assert_eq!(quote!(#zero_copy_return).to_string(), "proto_rs :: ZeroCopyResponse < MyResponse >");
        assert!(zero_copy.response_is_result);

        let streaming = &signatures[2];
        assert!(streaming.is_streaming);
        assert_eq!(streaming.stream_type_name.as_ref().unwrap().to_string(), "MyStream");
        let stream_proto = streaming.inner_response_type.as_ref().unwrap();
        assert_eq!(quote!(#stream_proto).to_string(), "MyResponse");
        let stream_item = streaming.stream_item_type.as_ref().unwrap();
        assert_eq!(quote!(#stream_item).to_string(), "MyResponse");

        let zero_copy_stream = &signatures[3];
        assert!(zero_copy_stream.is_streaming);
        assert_eq!(zero_copy_stream.stream_type_name.as_ref().unwrap().to_string(), "ZeroCopyStream");
        let zero_proto = zero_copy_stream.inner_response_type.as_ref().unwrap();
        assert_eq!(quote!(#zero_proto).to_string(), "MyResponse");
        let zero_item = zero_copy_stream.stream_item_type.as_ref().unwrap();
        assert_eq!(quote!(#zero_item).to_string(), "proto_rs :: ZeroCopyResponse < MyResponse >");

        let arced = &signatures[4];
        assert!(arced.response_is_result);
        let arced_response = &arced.response_type;
        assert_eq!(quote!(#arced_response).to_string(), "MyResponse");

        let boxed = &signatures[5];
        assert!(boxed.response_is_result);
        let boxed_response = &boxed.response_type;
        assert_eq!(quote!(#boxed_response).to_string(), "MyResponse");

        let plain = &signatures[6];
        assert!(!plain.response_is_result);
        let plain_return = &plain.response_return_type;
        assert_eq!(quote!(#plain_return).to_string(), "MyResponse");
    }
}
