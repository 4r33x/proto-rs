//! Common RPC utilities to eliminate duplication between client and server

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::utils::MethodInfo;
use crate::utils::to_pascal_case;

// ============================================================================
// CONVERSION HELPERS
// ============================================================================

/// Generate proto-to-native request conversion (used in server)
pub fn generate_proto_to_native_request(request_type: &Type) -> TokenStream {
    quote! {
        let (metadata, extensions, proto_msg) = request.into_parts();
        let native_msg = #request_type::from_proto(proto_msg)
            .map_err(|e| tonic::Status::invalid_argument(
                format!("Failed to convert request: {}", e)
            ))?;
        let native_request = tonic::Request::from_parts(metadata, extensions, native_msg);
    }
}

/// Generate native-to-proto request conversion (used in client - unary)
pub fn generate_native_to_proto_request_unary() -> TokenStream {
    quote! {
        let req = request.into_request();
        let (metadata, extensions, native_msg) = req.into_parts();
        let proto_msg = native_msg.to_proto();
        let mut proto_req = tonic::Request::from_parts(metadata, extensions, proto_msg);
    }
}

/// Generate native-to-proto request conversion (used in client - streaming)
pub fn generate_native_to_proto_request_streaming() -> TokenStream {
    quote! {
        let req = request.into_request();
        let (metadata, extensions, native_msg) = req.into_parts();
        let proto_msg = native_msg.to_proto();
        let proto_req = tonic::Request::from_parts(metadata, extensions, proto_msg);
    }
}

/// Generate proto-to-native response conversion (used in client)
pub fn generate_proto_to_native_response(response_type: &Type) -> TokenStream {
    quote! {
        let (metadata, proto_response, extensions) = response.into_parts();
        let native_response = #response_type::from_proto(proto_response)
            .map_err(|e| tonic::Status::internal(
                format!("Failed to convert response: {}", e)
            ))?;
        Ok(tonic::Response::from_parts(metadata, native_response, extensions))
    }
}

/// Generate native-to-proto response conversion (used in server)
pub fn generate_native_to_proto_response() -> TokenStream {
    quote! {
        let (metadata, native_body, extensions) = native_response.into_parts();
        let proto_msg = native_body.to_proto();
        Ok(tonic::Response::from_parts(metadata, proto_msg, extensions))
    }
}

// ============================================================================
// PROTO TYPE HELPERS
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
// ROUTE AND CODEC
// ============================================================================

/// Generate RPC route path
pub fn generate_route_path(package_name: &str, trait_name: &syn::Ident, method_name: &syn::Ident) -> String {
    format!("/{}.{}/{}", package_name, trait_name, to_pascal_case(&method_name.to_string()))
}

/// Generate codec initialization
pub fn generate_codec_init() -> TokenStream {
    quote! {
        let codec = tonic_prost::ProstCodec::default();
    }
}

// ============================================================================
// STREAMING HELPERS
// ============================================================================

/// Generate stream conversion for streaming responses (client side)
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

/// Check if method is streaming
pub fn is_streaming_method(method: &MethodInfo) -> bool {
    method.is_streaming
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

/// Generate client interceptor method (complex generic bounds)
pub fn generate_client_with_interceptor(client_struct: &syn::Ident) -> TokenStream {
    quote! {
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> #client_struct<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<http::Request<tonic::body::Body>, Response = http::Response<<T as tonic::client::GrpcService<tonic::body::Body>>::ResponseBody>>,
            <T as tonic::codegen::Service<http::Request<tonic::body::Body>>>::Error: Into<StdError> + std::marker::Send + std::marker::Sync,
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

    #[test]
    fn test_proto_type_generation() {
        let ty: Type = parse_quote! { MyRequest };
        let proto_type = generate_request_proto_type(&ty);

        let expected = quote! { <MyRequest as super::HasProto>::Proto };
        assert_eq!(proto_type.to_string(), expected.to_string());
    }
}
