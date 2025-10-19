//! Utilities for extracting method information from trait definitions

use proc_macro2::TokenStream;
use quote::quote;
use syn::FnArg;
use syn::ItemTrait;
use syn::PatType;
use syn::ReturnType;
use syn::TraitItem;
use syn::Type;
use syn::TypePath;

use crate::utils::MethodInfo;

/// Extract methods and associated types from the trait definition
pub fn extract_methods_and_types(input: &ItemTrait) -> (Vec<MethodInfo>, Vec<TokenStream>) {
    let mut methods = Vec::new();
    let mut user_associated_types = Vec::new();

    for item in &input.items {
        match item {
            TraitItem::Fn(method) => {
                let method_name = method.sig.ident.clone();
                let (request_type, response_type, response_return_type, response_is_result) = extract_types(&method.sig);
                let is_streaming = is_stream_response(&method.sig);

                let (stream_type_name, inner_response_type) = if is_streaming {
                    let stream_name = extract_stream_type_name(&response_type);
                    let inner_type = input
                        .items
                        .iter()
                        .find_map(|item| {
                            if let TraitItem::Type(type_item) = item
                                && type_item.ident == stream_name
                            {
                                return Some(extract_inner_type_from_bounds(&type_item.bounds));
                            }
                            None
                        })
                        .unwrap_or_else(|| panic!("Could not find associated type definition for {stream_name}"));
                    (Some(stream_name), Some(inner_type))
                } else {
                    (None, None)
                };

                let user_method_signature = generate_user_method_signature(
                    &method.attrs,
                    &method_name,
                    &request_type,
                    &response_return_type,
                    response_is_result,
                    is_streaming,
                    stream_type_name.as_ref(),
                );

                methods.push(MethodInfo {
                    name: method_name,
                    request_type,
                    response_type,
                    response_return_type,
                    response_is_result,
                    is_streaming,
                    stream_type_name,
                    inner_response_type,
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
fn generate_user_method_signature(
    attrs: &[syn::Attribute],
    method_name: &syn::Ident,
    request_type: &Type,
    response_return_type: &Type,
    response_is_result: bool,
    is_streaming: bool,
    stream_type_name: Option<&syn::Ident>,
) -> TokenStream {
    if is_streaming {
        let stream_name = stream_type_name.unwrap();
        quote! {
            #(#attrs)*
            fn #method_name(
                &self,
                request: tonic::Request<#request_type>,
            ) -> impl std::future::Future<
                Output = std::result::Result<tonic::Response<Self::#stream_name>, tonic::Status>
            > + ::core::marker::Send
            where
                Self: std::marker::Send + std::marker::Sync;
        }
    } else {
        if response_is_result {
            quote! {
                #(#attrs)*
                fn #method_name(
                    &self,
                    request: tonic::Request<#request_type>,
                ) -> impl std::future::Future<
                    Output = std::result::Result<#response_return_type, tonic::Status>
                > + ::core::marker::Send
                where
                    Self: std::marker::Send + std::marker::Sync;
            }
        } else {
            quote! {
                #(#attrs)*
                fn #method_name(
                    &self,
                    request: tonic::Request<#request_type>,
                ) -> impl std::future::Future<
                    Output = #response_return_type
                > + ::core::marker::Send
                where
                    Self: std::marker::Send + std::marker::Sync;
            }
        }
    }
}

/// Extract request and response types from method signature
pub fn extract_types(sig: &syn::Signature) -> (Box<Type>, Box<Type>, Box<Type>, bool) {
    let mut request_type = None;
    let mut response_type = None;
    let mut response_return_type = None;
    let mut response_is_result = false;

    // Extract request type from arguments
    for arg in &sig.inputs {
        if let FnArg::Typed(PatType { ty, .. }) = arg
            && let Type::Path(TypePath { path, .. }) = &**ty
            && let Some(segment) = path.segments.last()
            && segment.ident == "Request"
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
        {
            request_type = Some(Box::new(inner_ty.clone()));
        }
    }

    // Extract response type from return type
    if let ReturnType::Type(_, ty) = &sig.output {
        if let Type::Path(TypePath { path, .. }) = &**ty {
            if let Some(segment) = path.segments.last() {
                if segment.ident == "Result" {
                    response_is_result = true;
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(success_ty)) = args.args.first() {
                            response_return_type = Some(Box::new(success_ty.clone()));
                        }
                    }
                } else {
                    response_return_type = Some(Box::new((**ty).clone()));
                }
            }
        } else {
            response_return_type = Some(Box::new((**ty).clone()));
        }
    }

    if let Some(success_ty) = &response_return_type {
        response_type = Some(extract_proto_type(success_ty));
    }

    (
        request_type.expect("Could not extract request type"),
        response_type.expect("Could not extract response type"),
        response_return_type.expect("Could not extract response return type"),
        response_is_result,
    )
}

fn extract_proto_type(success_ty: &Type) -> Box<Type> {
    if let Type::Path(TypePath { path, .. }) = success_ty {
        if let Some(segment) = path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments
                && (segment.ident == "Response" || segment.ident == "ZeroCopyResponse")
                && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
            {
                return Box::new(inner_ty.clone());
            }
        }
    }

    Box::new(success_ty.clone())
}

/// Check if the response type is a stream
pub fn is_stream_response(sig: &syn::Signature) -> bool {
    if let ReturnType::Type(_, ty) = &sig.output {
        let type_string = quote!(#ty).to_string();
        type_string.contains("Self ::") && type_string.contains("Stream")
    } else {
        false
    }
}

/// Extract the stream type name from response type
pub fn extract_stream_type_name(response_type: &Type) -> syn::Ident {
    if let Type::Path(TypePath { path, .. }) = response_type
        && let Some(segment) = path.segments.last()
    {
        return segment.ident.clone();
    }
    panic!("Could not extract stream type name from response type");
}

/// Extract inner type from Stream trait bounds
pub fn extract_inner_type_from_bounds(bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::token::Plus>) -> Type {
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
                            return inner_ty.clone();
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
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_extract_types() {
        let sig: syn::Signature = parse_quote! {
            async fn test_method(
                &self,
                request: tonic::Request<MyRequest>
            ) -> Result<tonic::Response<MyResponse>, tonic::Status>
        };

        let (req_type, resp_type, return_type, is_result) = extract_types(&sig);
        assert_eq!(quote!(#req_type).to_string(), "MyRequest");
        assert_eq!(quote!(#resp_type).to_string(), "MyResponse");
        assert_eq!(quote!(#return_type).to_string(), "tonic :: Response < MyResponse >");
        assert!(is_result);

        let sig_plain: syn::Signature = parse_quote! {
            async fn plain(
                &self,
                request: tonic::Request<MyRequest>
            ) -> MyResponse
        };

        let (_, plain_resp, plain_return, plain_is_result) = extract_types(&sig_plain);
        assert_eq!(quote!(#plain_resp).to_string(), "MyResponse");
        assert_eq!(quote!(#plain_return).to_string(), "MyResponse");
        assert!(!plain_is_result);

        let sig_zero_copy: syn::Signature = parse_quote! {
            async fn zero_copy(
                &self,
                request: tonic::Request<MyRequest>
            ) -> Result<proto_rs::ZeroCopyResponse<MyResponse>, tonic::Status>
        };

        let (_, zero_copy_resp, zero_copy_return, zero_copy_is_result) = extract_types(&sig_zero_copy);
        assert_eq!(quote!(#zero_copy_resp).to_string(), "MyResponse");
        assert_eq!(quote!(#zero_copy_return).to_string(), "proto_rs :: ZeroCopyResponse < MyResponse >");
        assert!(zero_copy_is_result);

        let sig_result_plain: syn::Signature = parse_quote! {
            async fn result_plain(
                &self,
                request: tonic::Request<MyRequest>
            ) -> Result<MyResponse, tonic::Status>
        };

        let (_, result_plain_resp, result_plain_return, result_plain_is_result) = extract_types(&sig_result_plain);
        assert_eq!(quote!(#result_plain_resp).to_string(), "MyResponse");
        assert_eq!(quote!(#result_plain_return).to_string(), "MyResponse");
        assert!(result_plain_is_result);
    }

    #[test]
    fn test_is_stream_response() {
        let streaming_sig: syn::Signature = parse_quote! {
            async fn stream_method(
                &self,
                request: tonic::Request<MyRequest>
            ) -> Result<tonic::Response<Self::MyStream>, tonic::Status>
        };

        assert!(is_stream_response(&streaming_sig));

        let unary_sig: syn::Signature = parse_quote! {
            async fn unary_method(
                &self,
                request: tonic::Request<MyRequest>
            ) -> Result<tonic::Response<MyResponse>, tonic::Status>
        };

        assert!(!is_stream_response(&unary_sig));
    }

    #[test]
    fn test_extract_stream_type_name() {
        let ty: Type = parse_quote! { Self::MyStream };
        let name = extract_stream_type_name(&ty);
        assert_eq!(name.to_string(), "MyStream");
    }
}
