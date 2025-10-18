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
use crate::utils::to_pascal_case;

/// Extract methods and associated types from the trait definition
pub fn extract_methods_and_types(input: &ItemTrait) -> (Vec<MethodInfo>, Vec<TokenStream>) {
    let mut methods = Vec::new();
    let mut user_associated_types = Vec::new();

    for item in &input.items {
        match item {
            TraitItem::Fn(method) => {
                let method_name = method.sig.ident.clone();
                let (request_type, response_type) = extract_types(&method.sig);
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

                let response_assoc = if is_streaming {
                    None
                } else {
                    Some(syn::Ident::new(&format!("{}Response", to_pascal_case(&method_name.to_string())), method_name.span()))
                };

                let user_method_signature = generate_user_method_signature(
                    &method.attrs,
                    &method_name,
                    &request_type,
                    &response_type,
                    is_streaming,
                    stream_type_name.as_ref(),
                    response_assoc.as_ref(),
                );

                if let Some(response_assoc) = &response_assoc {
                    user_associated_types.push(quote! {
                        type #response_assoc: ::proto_rs::ProtoResponse<#response_type>;
                    });
                }

                methods.push(MethodInfo {
                    name: method_name,
                    request_type,
                    response_type,
                    is_streaming,
                    stream_type_name,
                    inner_response_type,
                    response_associated_type: response_assoc,
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
    _response_type: &Type,
    is_streaming: bool,
    stream_type_name: Option<&syn::Ident>,
    response_assoc: Option<&syn::Ident>,
) -> TokenStream {
    if is_streaming {
        let stream_name = stream_type_name.unwrap();
        quote! {
            #(#attrs)*
            fn #method_name<'life0, 'async_trait>(
                &'life0 self,
                request: tonic::Request<#request_type>,
            ) -> ::core::pin::Pin<Box<
                dyn ::core::future::Future<
                    Output = Result<tonic::Response<Self::#stream_name>, tonic::Status>
                > + ::core::marker::Send + 'async_trait
            >>
            where
                'life0: 'async_trait,
                Self: 'async_trait;
        }
    } else {
        let response_assoc = response_assoc.expect("missing response associated type for unary method");
        quote! {
            #(#attrs)*
            fn #method_name<'life0, 'async_trait>(
                &'life0 self,
                request: tonic::Request<#request_type>,
            ) -> ::core::pin::Pin<Box<
                dyn ::core::future::Future<
                    Output = Result<Self::#response_assoc, tonic::Status>
                > + ::core::marker::Send + 'async_trait
            >>
            where
                'life0: 'async_trait,
                Self: 'async_trait;
        }
    }
}

/// Extract request and response types from method signature
pub fn extract_types(sig: &syn::Signature) -> (Box<Type>, Box<Type>) {
    let mut request_type = None;
    let mut response_type = None;

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
    if let ReturnType::Type(_, ty) = &sig.output
        && let Type::Path(TypePath { path, .. }) = &**ty
        && let Some(segment) = path.segments.last()
        && segment.ident == "Result"
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(Type::Path(TypePath { path, .. }))) = args.args.first()
        && let Some(response_segment) = path.segments.last()
        && response_segment.ident == "Response"
        && let syn::PathArguments::AngleBracketed(response_args) = &response_segment.arguments
        && let Some(syn::GenericArgument::Type(inner_ty)) = response_args.args.first()
    {
        response_type = Some(Box::new(inner_ty.clone()));
    }

    (request_type.expect("Could not extract request type"), response_type.expect("Could not extract response type"))
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

        let (req_type, resp_type) = extract_types(&sig);
        assert_eq!(quote!(#req_type).to_string(), "MyRequest");
        assert_eq!(quote!(#resp_type).to_string(), "MyResponse");
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
