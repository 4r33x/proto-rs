use core::future::Future;
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
    max_encode_preallocation: usize,
}

impl<T> TonicTransport<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: tonic::client::Grpc::new(inner),
            max_encode_preallocation: crate::DEFAULT_MAX_ENCODE_PREALLOCATION,
        }
    }

    pub fn into_inner(self) -> tonic::client::Grpc<T> {
        self.inner
    }

    /// See [`crate::ProtoCodec::with_max_encode_preallocation`].
    #[must_use]
    pub const fn with_max_encode_preallocation(mut self, limit: usize) -> Self {
        self.max_encode_preallocation = limit;
        self
    }
}

impl<T> From<T> for TonicTransport<T> {
    fn from(inner: T) -> Self {
        Self::new(inner)
    }
}

pin_project_lite::pin_project! {
    pub struct TonicUnaryFuture<F> { #[pin] inner: F }
}
impl<F, R> Future for TonicUnaryFuture<F>
where
    F: Future<Output = Result<tonic::Response<R>, tonic::Status>>,
{
    type Output = Result<Response<R>, Status>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().inner.poll(cx).map(|result| result.map(response_from_tonic).map_err(status_from_tonic_owned))
    }
}

pin_project_lite::pin_project! {
    pub struct TonicStreamingFuture<F> { #[pin] inner: F }
}
impl<F, R> Future for TonicStreamingFuture<F>
where
    F: Future<Output = Result<tonic::Response<tonic::Streaming<R>>, tonic::Status>>,
{
    type Output = Result<Response<TonicResponseStream<R>>, Status>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().inner.poll(cx).map(|result| {
            result.map(|response| response_from_tonic(response).map(TonicResponseStream::new)).map_err(status_from_tonic_owned)
        })
    }
}

impl<T> TonicTransport<T>
where
    T: tonic::client::GrpcService<tonic::body::Body> + Send,
    T::Future: Send,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = bytes::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    /// Prepare before returning: the resulting future borrows only this client.
    pub fn unary_ref<'a, Req, Res>(
        &'a mut self,
        route: &'static str,
        request: Request<&Req>,
    ) -> TonicUnaryFuture<<tonic::client::Grpc<T> as crate::PreparedRpc<Res>>::Unary<'a>>
    where
        Req: crate::ProtoEncode + crate::ProtoExt,
        Res: crate::ProtoDecode + Send + Sync + 'static,
    {
        let request = <tonic::Request<&Req> as crate::PrepareRequest<Req>>::prepare_request(
            request_into_tonic(request),
            self.max_encode_preallocation,
        );
        TonicUnaryFuture {
            inner: crate::PreparedRpc::prepared_unary(
                &mut self.inner,
                request,
                tonic::codegen::http::uri::PathAndQuery::from_static(route),
            ),
        }
    }

    pub fn server_streaming_ref<'a, Req, Res>(
        &'a mut self,
        route: &'static str,
        request: Request<&Req>,
    ) -> TonicStreamingFuture<<tonic::client::Grpc<T> as crate::PreparedRpc<Res>>::Streaming<'a>>
    where
        Req: crate::ProtoEncode + crate::ProtoExt,
        Res: crate::ProtoDecode + Send + Sync + 'static,
    {
        let request = <tonic::Request<&Req> as crate::PrepareRequest<Req>>::prepare_request(
            request_into_tonic(request),
            self.max_encode_preallocation,
        );
        TonicStreamingFuture {
            inner: crate::PreparedRpc::prepared_streaming(
                &mut self.inner,
                request,
                tonic::codegen::http::uri::PathAndQuery::from_static(route),
            ),
        }
    }

    #[cfg(feature = "tonic-transport")]
    pub async fn client_streaming_encoded<Res>(
        &mut self,
        route: &'static str,
        request: Request<super::EncodedStream>,
    ) -> Result<Response<Res>, Status>
    where
        Res: crate::ProtoDecode + Send + Sync + 'static,
    {
        self.ready().await?;
        self.inner
            .client_streaming(
                request_into_tonic(request),
                tonic::codegen::http::uri::PathAndQuery::from_static(route),
                crate::ProtoCodec::<crate::PreparedMessage, Res, crate::BytesMode>::default(),
            )
            .await
            .map(response_from_tonic)
            .map_err(status_from_tonic_owned)
    }

    #[cfg(feature = "tonic-transport")]
    pub async fn bidirectional_encoded<Res>(
        &mut self,
        route: &'static str,
        request: Request<super::EncodedStream>,
    ) -> Result<Response<TonicResponseStream<Res>>, Status>
    where
        Res: crate::ProtoDecode + Send + Sync + 'static,
    {
        self.ready().await?;
        self.inner
            .streaming(
                request_into_tonic(request),
                tonic::codegen::http::uri::PathAndQuery::from_static(route),
                crate::ProtoCodec::<crate::PreparedMessage, Res, crate::BytesMode>::default(),
            )
            .await
            .map(|response| response_from_tonic(response).map(TonicResponseStream::new))
            .map_err(status_from_tonic_owned)
    }
}

pub struct TonicResponseStream<T> {
    inner: tonic::Streaming<T>,
}

impl<T> TonicResponseStream<T> {
    const fn new(inner: tonic::Streaming<T>) -> Self {
        Self { inner }
    }
}

impl<T> Stream for TonicResponseStream<T> {
    type Item = Result<T, Status>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(context).map(|item| item.map(|result| result.map_err(status_from_tonic_owned)))
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

    fn unary<Req, Res>(&mut self, route: &'static str, request: Request<Req>) -> impl Future<Output = Result<Response<Res>, Self::Error>>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static,
    {
        let (metadata, extensions, value) = request.into_parts();
        self.unary_ref(route, Request::from_parts(metadata, extensions, &value))
    }

    async fn client_streaming<Req, Res, S>(&mut self, route: &'static str, request: Request<S>) -> Result<Response<Res>, Self::Error>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static,
        S: Stream<Item = Req> + Send + 'static,
    {
        self.ready().await?;
        let path = tonic::codegen::http::uri::PathAndQuery::from_static(route);
        let codec = crate::ProtoCodec::<Req, Res, crate::SunByRef>::default().with_max_encode_preallocation(self.max_encode_preallocation);
        self.inner
            .client_streaming(request_into_tonic(request), path, codec)
            .await
            .map(response_from_tonic)
            .map_err(status_from_tonic_owned)
    }

    fn server_streaming<Req, Res>(
        &mut self,
        route: &'static str,
        request: Request<Req>,
    ) -> impl Future<Output = Result<Response<Self::ResponseStream<Res>>, Self::Error>>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static,
    {
        let (metadata, extensions, value) = request.into_parts();
        self.server_streaming_ref(route, Request::from_parts(metadata, extensions, &value))
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
        let codec = crate::ProtoCodec::<Req, Res, crate::SunByRef>::default().with_max_encode_preallocation(self.max_encode_preallocation);
        self.inner
            .streaming(request_into_tonic(request), path, codec)
            .await
            .map(|response| response_from_tonic(response).map(TonicResponseStream::new))
            .map_err(status_from_tonic_owned)
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
pub fn status_from_tonic_owned(mut status: tonic::Status) -> Status {
    let metadata = core::mem::take(status.metadata_mut()).into_headers();
    Status::with_details_and_metadata(
        code_from_tonic(status.code()),
        status.message(),
        bytes::Bytes::copy_from_slice(status.details()),
        metadata,
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
