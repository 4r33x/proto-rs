use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

use super::Code;
use super::GrpcTransport;
use super::Request;
use super::Response;
use super::Status;
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

impl<T> From<T> for TonicTransport<T> {
    fn from(inner: T) -> Self {
        Self::new(inner)
    }
}

pub struct TonicResponseStream<T> {
    inner: Pin<Box<tonic::Streaming<T>>>,
}

impl<T> TonicResponseStream<T> {
    fn new(inner: tonic::Streaming<T>) -> Self {
        Self { inner: Box::pin(inner) }
    }
}

impl<T> Stream for TonicResponseStream<T> {
    type Item = Result<T, Status>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(context).map(|item| item.map(|result| result.map_err(|status| status_from_tonic(&status))))
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
    type Error = Status;
    type ResponseStream<R>
        = TonicResponseStream<R>
    where
        R: Send + 'static;

    async fn unary<Req, Res>(&mut self, route: &'static str, request: Request<Req>) -> Result<Response<Res>, Self::Error>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static,
    {
        self.ready().await?;
        let path = tonic::codegen::http::uri::PathAndQuery::from_static(route);
        let codec = crate::ProtoCodec::<Req, Res, crate::SunByVal>::default();
        self.inner
            .unary(request_into_tonic(request), path, codec)
            .await
            .map(response_from_tonic)
            .map_err(|status| status_from_tonic(&status))
    }

    async fn client_streaming<Req, Res, S>(&mut self, route: &'static str, request: Request<S>) -> Result<Response<Res>, Self::Error>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static,
        S: Stream<Item = Req> + Send + 'static,
    {
        self.ready().await?;
        let path = tonic::codegen::http::uri::PathAndQuery::from_static(route);
        let codec = crate::ProtoCodec::<Req, Res, crate::SunByVal>::default();
        self.inner
            .client_streaming(request_into_tonic(request), path, codec)
            .await
            .map(response_from_tonic)
            .map_err(|status| status_from_tonic(&status))
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
        self.ready().await?;
        let path = tonic::codegen::http::uri::PathAndQuery::from_static(route);
        let codec = crate::ProtoCodec::<Req, Res, crate::SunByVal>::default();
        self.inner
            .server_streaming(request_into_tonic(request), path, codec)
            .await
            .map(|response| response_from_tonic(response).map(TonicResponseStream::new))
            .map_err(|status| status_from_tonic(&status))
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
        self.ready().await?;
        let path = tonic::codegen::http::uri::PathAndQuery::from_static(route);
        let codec = crate::ProtoCodec::<Req, Res, crate::SunByVal>::default();
        self.inner
            .streaming(request_into_tonic(request), path, codec)
            .await
            .map(|response| response_from_tonic(response).map(TonicResponseStream::new))
            .map_err(|status| status_from_tonic(&status))
    }
}

impl<T> TonicTransport<T>
where
    T: tonic::client::GrpcService<tonic::body::Body>,
    T::Error: Into<tonic::codegen::StdError>,
{
    async fn ready(&mut self) -> Result<(), Status> {
        self.inner.ready().await.map_err(|error| Status::unknown(format!("service was not ready: {}", error.into())))
    }
}

#[doc(hidden)]
pub fn request_from_tonic<T>(request: tonic::Request<T>) -> Request<T> {
    let (metadata, extensions, message) = request.into_parts();
    Request::from_parts(metadata.into_headers(), extensions, message)
}

#[doc(hidden)]
pub fn request_into_tonic<T>(request: Request<T>) -> tonic::Request<T> {
    let (metadata, extensions, message) = request.into_parts();
    tonic::Request::from_parts(tonic::metadata::MetadataMap::from_headers(metadata), extensions, message)
}

#[doc(hidden)]
pub fn response_from_tonic<T>(response: tonic::Response<T>) -> Response<T> {
    let (metadata, message, extensions) = response.into_parts();
    Response::from_parts(metadata.into_headers(), message, extensions)
}

#[doc(hidden)]
pub fn response_into_tonic<T>(response: Response<T>) -> tonic::Response<T> {
    let (metadata, message, extensions) = response.into_parts();
    tonic::Response::from_parts(tonic::metadata::MetadataMap::from_headers(metadata), message, extensions)
}

#[doc(hidden)]
pub fn status_from_tonic(status: &tonic::Status) -> Status {
    Status::with_details_and_metadata(
        code_from_tonic(status.code()),
        status.message(),
        bytes::Bytes::copy_from_slice(status.details()),
        status.metadata().clone().into_headers(),
    )
}

#[doc(hidden)]
pub fn status_into_tonic(status: Status) -> tonic::Status {
    let (code, message, details, metadata) = status.into_parts();
    tonic::Status::with_details_and_metadata(
        code_into_tonic(code),
        message,
        details,
        tonic::metadata::MetadataMap::from_headers(metadata),
    )
}

#[doc(hidden)]
pub fn stream_status_into_tonic<T>(result: Result<T, Status>) -> Result<T, tonic::Status> {
    result.map_err(status_into_tonic)
}

const fn code_from_tonic(code: tonic::Code) -> Code {
    Code::from_i32(code as i32)
}

const fn code_into_tonic(code: Code) -> tonic::Code {
    match code {
        Code::Ok => tonic::Code::Ok,
        Code::Cancelled => tonic::Code::Cancelled,
        Code::Unknown => tonic::Code::Unknown,
        Code::InvalidArgument => tonic::Code::InvalidArgument,
        Code::DeadlineExceeded => tonic::Code::DeadlineExceeded,
        Code::NotFound => tonic::Code::NotFound,
        Code::AlreadyExists => tonic::Code::AlreadyExists,
        Code::PermissionDenied => tonic::Code::PermissionDenied,
        Code::ResourceExhausted => tonic::Code::ResourceExhausted,
        Code::FailedPrecondition => tonic::Code::FailedPrecondition,
        Code::Aborted => tonic::Code::Aborted,
        Code::OutOfRange => tonic::Code::OutOfRange,
        Code::Unimplemented => tonic::Code::Unimplemented,
        Code::Internal => tonic::Code::Internal,
        Code::Unavailable => tonic::Code::Unavailable,
        Code::DataLoss => tonic::Code::DataLoss,
        Code::Unauthenticated => tonic::Code::Unauthenticated,
    }
}
