use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::FnArg;
use syn::ItemTrait;
use syn::LitStr;
use syn::PatType;
use syn::ReturnType;
use syn::Token;
use syn::TraitItem;
use syn::Type;
use syn::TypePath;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;

/// Information about a method extracted from the trait
pub struct MethodInfo {
    pub name: syn::Ident,
    pub _attrs: Vec<syn::Attribute>,
    pub request_type: Box<Type>,
    pub response_type: Box<Type>,
    pub is_streaming: bool,
    pub stream_type_name: Option<syn::Ident>,
    pub inner_response_type: Option<Type>,
    pub user_method_signature: TokenStream,
}

pub struct ProtoImports {
    pub imports: HashMap<String, Vec<String>>,
}

impl Parse for ProtoImports {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut imports = HashMap::new();

        while !input.is_empty() {
            let package: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            let content;
            syn::bracketed!(content in input);
            let types: Punctuated<LitStr, Token![,]> = content.parse_terminated(|buf| buf.parse::<LitStr>(), Token![,])?;

            imports.insert(package.to_string(), types.iter().map(|s| s.value()).collect());

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(ProtoImports { imports })
    }
}

/// Extract methods and associated types from the trait definition
pub fn extract_methods_and_types(input: &ItemTrait) -> (Vec<MethodInfo>, Vec<TokenStream>, HashMap<String, Vec<String>>) {
    let mut methods = Vec::new();
    let mut user_associated_types = Vec::new();
    let mut proto_imports = HashMap::new();

    // Extract proto_imports from trait attributes
    for attr in &input.attrs {
        if attr.path().is_ident("proto_imports")
            && let Ok(imports) = attr.parse_args::<ProtoImports>()
        {
            proto_imports = imports.imports;
        }
    }

    for item in &input.items {
        match item {
            TraitItem::Fn(method) => {
                let method_name = method.sig.ident.clone();
                let method_attrs = method.attrs.clone();
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
                                Some(extract_inner_type_from_bounds(&type_item.bounds))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| panic!("Could not find associated type definition for {}", stream_name));
                    (Some(stream_name), Some(inner_type))
                } else {
                    (None, None)
                };

                let user_method_signature = generate_user_method_signature(&method_attrs, &method_name, &request_type, &response_type, is_streaming, stream_type_name.as_ref());

                methods.push(MethodInfo {
                    name: method_name,
                    _attrs: method_attrs,
                    request_type,
                    response_type,
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

    (methods, user_associated_types, proto_imports)
}

fn generate_user_method_signature(
    attrs: &[syn::Attribute],
    method_name: &syn::Ident,
    request_type: &Type,
    response_type: &Type,
    is_streaming: bool,
    stream_type_name: Option<&syn::Ident>,
) -> TokenStream {
    if is_streaming {
        let stream_name = stream_type_name.unwrap();
        quote! {
            #(#attrs)*
            fn #method_name<'life0, 'async_trait>(
                &'life0 self,
                request: tonic::Request<#request_type>,
            ) -> ::core::pin::Pin<Box<dyn ::core::future::Future<Output = Result<tonic::Response<Self::#stream_name>, tonic::Status>> + ::core::marker::Send + 'async_trait>>
            where
                'life0: 'async_trait,
                Self: 'async_trait;
        }
    } else {
        quote! {
            #(#attrs)*
            fn #method_name<'life0, 'async_trait>(
                &'life0 self,
                request: tonic::Request<#request_type>,
            ) -> ::core::pin::Pin<Box<dyn ::core::future::Future<Output = Result<tonic::Response<#response_type>, tonic::Status>> + ::core::marker::Send + 'async_trait>>
            where
                'life0: 'async_trait,
                Self: 'async_trait;
        }
    }
}

pub fn extract_types(sig: &syn::Signature) -> (Box<Type>, Box<Type>) {
    let mut request_type = None;
    let mut response_type = None;

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

pub fn is_stream_response(sig: &syn::Signature) -> bool {
    if let ReturnType::Type(_, ty) = &sig.output {
        let type_string = quote!(#ty).to_string();
        type_string.contains("Self ::") && type_string.contains("Stream")
    } else {
        false
    }
}

pub fn extract_stream_type_name(response_type: &Type) -> syn::Ident {
    if let Type::Path(TypePath { path, .. }) = response_type
        && let Some(segment) = path.segments.last()
    {
        return segment.ident.clone();
    }
    panic!("Could not extract stream type name from response type");
}

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
