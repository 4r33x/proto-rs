pub use tonic::Code;
pub use tonic::Extensions;
pub use tonic::Request;
pub use tonic::Response;
pub use tonic::Status;

use super::GrpcTransport;
use super::Stream;

pub struct TonicTransport<T> {
    inner: tonic::client::Grpc<T>,
}

impl<T> TonicTransport<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: tonic::client::Grpc::new(inner),
        }
    }

    pub fn into_inner(self) -> tonic::client::Grpc<T> {
        self.inner
    }
}

impl<T> GrpcTransport for TonicTransport<T>
where
    T: tonic::client::GrpcService<tonic::body::Body> + Send,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = crate::bytes::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
    T::Future: Send,
{
    type Error = tonic::Status;
    type ResponseStream<R>
        = tonic::Streaming<R>
    where
        R: Send + 'static;

    async fn unary<Req, Res>(&mut self, route: &'static str, request: Request<Req>) -> Result<Response<Res>, Self::Error>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static,
    {
        self.inner.ready().await.map_err(|err| tonic::Status::unknown(format!("service was not ready: {}", err.into())))?;
        let path = tonic::codegen::http::uri::PathAndQuery::from_static(route);
        let codec = crate::ProtoCodec::<Req, Res, crate::SunByVal>::default();
        self.inner.unary(request, path, codec).await
    }

    async fn client_streaming<Req, Res, S>(&mut self, route: &'static str, request: Request<S>) -> Result<Response<Res>, Self::Error>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static,
        S: Stream<Item = Req> + Send + 'static,
    {
        self.inner.ready().await.map_err(|err| tonic::Status::unknown(format!("service was not ready: {}", err.into())))?;
        let path = tonic::codegen::http::uri::PathAndQuery::from_static(route);
        let codec = crate::ProtoCodec::<Req, Res, crate::SunByVal>::default();
        self.inner.client_streaming(request, path, codec).await
    }

    async fn server_streaming<Req, Res>(
        &mut self,
        route: &'static str,
        request: Request<Req>,
    ) -> Result<Response<Self::ResponseStream<Res>>, Self::Error>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static,
    {
        self.inner.ready().await.map_err(|err| tonic::Status::unknown(format!("service was not ready: {}", err.into())))?;
        let path = tonic::codegen::http::uri::PathAndQuery::from_static(route);
        let codec = crate::ProtoCodec::<Req, Res, crate::SunByVal>::default();
        self.inner.server_streaming(request, path, codec).await
    }

    async fn bidirectional_streaming<Req, Res, S>(
        &mut self,
        route: &'static str,
        request: Request<S>,
    ) -> Result<Response<Self::ResponseStream<Res>>, Self::Error>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static,
        S: Stream<Item = Req> + Send + 'static,
    {
        self.inner.ready().await.map_err(|err| tonic::Status::unknown(format!("service was not ready: {}", err.into())))?;
        let path = tonic::codegen::http::uri::PathAndQuery::from_static(route);
        let codec = crate::ProtoCodec::<Req, Res, crate::SunByVal>::default();
        self.inner.streaming(request, path, codec).await
    }
}
