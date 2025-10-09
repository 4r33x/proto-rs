//! Common RPC utilities to eliminate duplication between client and server

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::utils::MethodInfo;
use crate::utils::to_pascal_case;

// ============================================================================
// SHARED RPC GENERATION
// ============================================================================

/// Generate RPC route path
/// Used by both client and server to ensure consistency
pub fn generate_route_path(package_name: &str, trait_name: &syn::Ident, method_name: &syn::Ident) -> String {
    format!("/{}.{}/{}", package_name, trait_name, to_pascal_case(&method_name.to_string()))
}

/// Generate codec initialization (same for client/server)
pub fn generate_codec_init() -> TokenStream {
    quote! {
        let codec = tonic_prost::ProstCodec::default();
    }
}

/// Generate proto request conversion (used in both client and server)
pub fn generate_request_conversion(request_type: &Type) -> TokenStream {
    quote! {
        let (metadata, extensions, native_msg) = request.into_parts();
        let proto_msg = native_msg.to_proto();
        let proto_req = tonic::Request::from_parts(metadata, extensions, proto_msg);
    }
}

/// Generate proto response conversion (used in both client and server)
pub fn generate_response_conversion(response_type: &Type) -> TokenStream {
    quote! {
        let (metadata, proto_response, extensions) = response.into_parts();
        let native_response = #response_type::from_proto(proto_response)
            .map_err(|e| tonic::Status::internal(format!("Failed to convert response: {}", e)))?;
        Ok(tonic::Response::from_parts(metadata, native_response, extensions))
    }
}

/// Generate stream conversion for streaming responses
pub fn generate_stream_conversion(inner_response_type: &Type) -> TokenStream {
    quote! {
        let (metadata, proto_stream, extensions) = response.into_parts();

        use tonic::codegen::tokio_stream::StreamExt;
        let native_stream = proto_stream.map(|result| {
            result.and_then(|proto_item| {
                #inner_response_type::from_proto(proto_item)
                    .map_err(|e| tonic::Status::internal(format!("Failed to convert response: {}", e)))
            })
        });

        Ok(tonic::Response::from_parts(metadata, native_stream, extensions))
    }
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

/// Generate service ready check (used by client)
pub fn generate_ready_check() -> TokenStream {
    quote! {
        self.inner
            .ready()
            .await
            .map_err(|e| tonic::Status::unknown(format!("Service was not ready: {}", e.into())))?;
    }
}

/// Generate conversion error handling
pub fn generate_conversion_error(field_name: &str) -> TokenStream {
    quote! {
        .map_err(|e| tonic::Status::invalid_argument(
            format!("Failed to convert {}: {}", #field_name, e)
        ))?
    }
}

// ============================================================================
// STREAMING HELPERS
// ============================================================================

/// Check if method is streaming
pub fn is_streaming_method(method: &MethodInfo) -> bool {
    method.is_streaming
}

/// Get stream item type for a streaming method
pub fn get_stream_item_type(method: &MethodInfo) -> Option<&Type> {
    method.inner_response_type.as_ref()
}

// ============================================================================
// TYPE HELPERS
// ============================================================================

/// Generate proto type reference for request
pub fn generate_request_proto_type(request_type: &Type) -> TokenStream {
    quote! { <#request_type as super::HasProto>::Proto }
}

/// Generate proto type reference for response
pub fn generate_response_proto_type(response_type: &Type) -> TokenStream {
    quote! { <#response_type as super::HasProto>::Proto }
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
    syn::Ident::new(&format!("{}Client", trait_name), trait_name.span())
}

/// Generate server struct name from trait
pub fn server_struct_name(trait_name: &syn::Ident) -> syn::Ident {
    syn::Ident::new(&format!("{}Server", trait_name), trait_name.span())
}

// ============================================================================
// SERVICE STRUCT GENERATION
// ============================================================================

/// Generate common service struct fields (used by server)
pub fn generate_service_struct_fields() -> TokenStream {
    quote! {
        inner: Arc<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
}

/// Generate service struct constructors
pub fn generate_service_constructors() -> TokenStream {
    quote! {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }

        pub fn from_arc(inner: Arc<T>) -> Self {
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
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

        let client_struct = client_struct_name(&trait_name);
        assert_eq!(client_struct.to_string(), "TestServiceClient");

        let server_struct = server_struct_name(&trait_name);
        assert_eq!(server_struct.to_string(), "TestServiceServer");
    }

    #[test]
    fn test_proto_type_generation() {
        let ty: Type = parse_quote! { MyRequest };
        let proto_type = generate_request_proto_type(&ty);

        let expected = quote! { <MyRequest as super::HasProto>::Proto };
        assert_eq!(proto_type.to_string(), expected.to_string());
    }
}
