use proc_macro2::TokenStream;
use quote::quote;

use super::utils::MethodInfo;
use crate::utils::to_pascal_case;
use crate::utils::to_snake_case;

pub fn generate_client_module(trait_name: &syn::Ident, vis: &syn::Visibility, package_name: &str, methods: &[MethodInfo]) -> TokenStream {
    let client_module = syn::Ident::new(&format!("{}_client", to_snake_case(&trait_name.to_string())), trait_name.span());
    let client_struct = syn::Ident::new(&format!("{}Client", trait_name), trait_name.span());

    let client_methods: Vec<_> = methods.iter().map(|method| generate_client_method(method, package_name, trait_name)).collect();

    quote! {
        #vis mod #client_module {
            #![allow(unused_variables, dead_code, missing_docs, clippy::wildcard_imports, clippy::let_unit_value)]
            use tonic::codegen::*;
            use super::*;

            #[derive(Debug, Clone)]
            pub struct #client_struct<T> {
                inner: tonic::client::Grpc<T>,
            }

            impl #client_struct<tonic::transport::Channel> {
                pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
                where
                    D: TryInto<tonic::transport::Endpoint>,
                    D::Error: Into<StdError>,
                {
                    let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
                    Ok(Self::new(conn))
                }
            }

            impl<T> #client_struct<T>
            where
                T: tonic::client::GrpcService<tonic::body::Body>,
                T::Error: Into<StdError>,
                T::ResponseBody: Body<Data = Bytes> + std::marker::Send + 'static,
                <T::ResponseBody as Body>::Error: Into<StdError> + std::marker::Send,
            {
                pub fn new(inner: T) -> Self {
                    let inner = tonic::client::Grpc::new(inner);
                    Self { inner }
                }

                pub fn with_origin(inner: T, origin: http::Uri) -> Self {
                    let inner = tonic::client::Grpc::with_origin(inner, origin);
                    Self { inner }
                }

                pub fn with_interceptor<F>(
                    inner: T,
                    interceptor: F,
                ) -> #client_struct<InterceptedService<T, F>>
                where
                    F: tonic::service::Interceptor,
                    T::ResponseBody: Default,
                    T: tonic::codegen::Service<
                        http::Request<tonic::body::Body>,
                        Response = http::Response<<T as tonic::client::GrpcService<tonic::body::Body>>::ResponseBody>,
                    >,
                    <T as tonic::codegen::Service<http::Request<tonic::body::Body>>>::Error:
                        Into<StdError> + std::marker::Send + std::marker::Sync,
                {
                    #client_struct::new(InterceptedService::new(inner, interceptor))
                }

                #[must_use]
                pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
                    self.inner = self.inner.send_compressed(encoding);
                    self
                }

                #[must_use]
                pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
                    self.inner = self.inner.accept_compressed(encoding);
                    self
                }

                #[must_use]
                pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
                    self.inner = self.inner.max_decoding_message_size(limit);
                    self
                }

                #[must_use]
                pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
                    self.inner = self.inner.max_encoding_message_size(limit);
                    self
                }

                #(#client_methods)*
            }
        }
    }
}

fn generate_client_method(method: &MethodInfo, package_name: &str, trait_name: &syn::Ident) -> TokenStream {
    let method_name = &method.name;
    let request_type = &method.request_type;
    let route_path = format!("/{}.{}/{}", package_name, trait_name, to_pascal_case(&method_name.to_string()));

    if method.is_streaming {
        let inner_response_type = method.inner_response_type.as_ref().unwrap();

        quote! {
            pub async fn #method_name(
                &mut self,
                request: impl tonic::IntoRequest<#request_type>,
            ) -> std::result::Result<
                tonic::Response<impl tonic::codegen::tokio_stream::Stream<Item = Result<#inner_response_type, tonic::Status>>>,
                tonic::Status
            > {
                self.inner
                    .ready()
                    .await
                    .map_err(|e| tonic::Status::unknown(format!("Service was not ready: {}", e.into())))?;

                let codec = tonic_prost::ProstCodec::default();
                let path = http::uri::PathAndQuery::from_static(#route_path);

                let req = request.into_request();
                let (metadata, extensions, native_msg) = req.into_parts();
                let proto_msg = native_msg.to_proto();
                let proto_req = tonic::Request::from_parts(metadata, extensions, proto_msg);

                let response = self.inner.server_streaming(proto_req, path, codec).await?;
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
    } else {
        let response_type = &method.response_type;

        quote! {
            pub async fn #method_name(
                &mut self,
                request: impl tonic::IntoRequest<#request_type>,
            ) -> std::result::Result<tonic::Response<#response_type>, tonic::Status> {
                self.inner
                    .ready()
                    .await
                    .map_err(|e| tonic::Status::unknown(format!("Service was not ready: {}", e.into())))?;

                let codec = tonic_prost::ProstCodec::default();
                let path = http::uri::PathAndQuery::from_static(#route_path);

                let req = request.into_request();
                let (metadata, extensions, native_msg) = req.into_parts();
                let proto_msg = native_msg.to_proto();
                let mut proto_req = tonic::Request::from_parts(metadata, extensions, proto_msg);

                proto_req.extensions_mut().insert(
                    tonic::codegen::GrpcMethod::new(#package_name, stringify!(#method_name))
                );

                let response = self.inner.unary(proto_req, path, codec).await?;
                let (metadata, proto_response, extensions) = response.into_parts();
                let native_response = #response_type::from_proto(proto_response)
                    .map_err(|e| tonic::Status::internal(format!("Failed to convert response: {}", e)))?;

                Ok(tonic::Response::from_parts(metadata, native_response, extensions))
            }
        }
    }
}
