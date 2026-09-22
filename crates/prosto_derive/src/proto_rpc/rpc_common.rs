//! Common RPC utilities to eliminate duplication between client and server

use proc_macro2::TokenStream;
use quote::quote;

use crate::utils::to_pascal_case;

// ============================================================================
// ROUTE AND CODEC
// ============================================================================

/// Generate RPC route path
pub fn generate_route_path(package_name: &str, trait_name: &syn::Ident, method_name: &syn::Ident) -> String {
    format!("/{}.{}/{}", package_name, trait_name, to_pascal_case(&method_name.to_string()))
}

/// Generate codec initialization
pub fn generate_codec_init(encode: TokenStream, decode: TokenStream, mode: Option<TokenStream>) -> TokenStream {
    if let Some(mode) = mode {
        quote! { let codec = ::proto_rs::ProtoCodec::<#encode, #decode, #mode>::default()
        .with_max_encode_preallocation(max_encode_preallocation); }
    } else {
        quote! { let codec = ::proto_rs::ProtoCodec::<#encode, #decode>::default()
        .with_max_encode_preallocation(max_encode_preallocation); }
    }
}

// ============================================================================
// MODULE NAMING
// ============================================================================

/// Generate client module name from trait
pub fn client_module_name(trait_name: &syn::Ident) -> syn::Ident {
    use crate::utils::to_snake_case;
    syn::Ident::new(&format!("{}_client", to_snake_case(&trait_name.to_string())), trait_name.span())
}

/// Generate server module name from trait
pub fn server_module_name(trait_name: &syn::Ident) -> syn::Ident {
    use crate::utils::to_snake_case;
    syn::Ident::new(&format!("{}_server", to_snake_case(&trait_name.to_string())), trait_name.span())
}

/// Generate client struct name from trait
pub fn client_struct_name(trait_name: &syn::Ident) -> syn::Ident {
    syn::Ident::new(&format!("{trait_name}Client"), trait_name.span())
}

/// Generate server struct name from trait
pub fn server_struct_name(trait_name: &syn::Ident) -> syn::Ident {
    syn::Ident::new(&format!("{trait_name}Server"), trait_name.span())
}

// ============================================================================
// SERVICE STRUCT GENERATION
// ============================================================================

/// Generate common service struct fields (used by server)
pub fn generate_service_struct_fields() -> TokenStream {
    quote! {
        inner: ::proto_rs::alloc::sync::Arc<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
        max_encode_preallocation: usize,
    }
}

/// Generate service struct constructors
pub fn generate_service_constructors() -> TokenStream {
    quote! {
        pub fn new(inner: T) -> Self {
            Self::from_arc(::proto_rs::alloc::sync::Arc::new(inner))
        }

        pub fn from_arc(inner: ::proto_rs::alloc::sync::Arc<T>) -> Self {
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
                max_encode_preallocation: ::proto_rs::DEFAULT_MAX_ENCODE_PREALLOCATION,
            }
        }
    }
}

/// Generate client interceptor method (complex generic bounds)
pub fn generate_client_with_interceptor(client_struct: &syn::Ident, has_ctx: bool) -> TokenStream {
    let return_ty = if has_ctx {
        quote! { #client_struct<InterceptedService<T, F>, Ctx> }
    } else {
        quote! { #client_struct<InterceptedService<T, F>> }
    };

    quote! {
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> #return_ty
        where
            F: tonic::service::Interceptor + Send,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<http::Request<tonic::body::Body>, Response = http::Response<<T as tonic::client::GrpcService<tonic::body::Body>>::ResponseBody>>,
            <T as tonic::codegen::Service<http::Request<tonic::body::Body>>>::Error: Into<StdError> + ::core::marker::Send + ::core::marker::Sync,
            <T as tonic::codegen::Service<http::Request<tonic::body::Body>>>::Future: Send,
        {
            #client_struct::new(InterceptedService::new(inner, interceptor))
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_generate_route_path() {
        let trait_name: syn::Ident = parse_quote! { TestService };
        let method_name: syn::Ident = parse_quote! { test_method };

        let path = generate_route_path("my_package", &trait_name, &method_name);
        assert_eq!(path, "/my_package.TestService/TestMethod");
    }

    #[test]
    fn test_module_naming() {
        let trait_name: syn::Ident = parse_quote! { TestService };

        let client_mod = client_module_name(&trait_name);
        assert_eq!(client_mod.to_string(), "test_service_client");

        let server_mod = server_module_name(&trait_name);
        assert_eq!(server_mod.to_string(), "test_service_server");
    }
}
