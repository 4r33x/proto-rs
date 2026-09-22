use core::marker::PhantomData;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

use bytes::Bytes;

use super::ProtoResponse;
use super::Request;
use super::Response;
use super::Status;
use super::Stream;
use crate::ProtoDecode;
use crate::ProtoEncode;
use crate::ProtoExt;
use crate::ZeroCopy;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RpcKind {
    Unary,
    ClientStreaming,
    ServerStreaming,
    BidirectionalStreaming,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MethodDescriptor {
    pub path: &'static str,
    pub kind: RpcKind,
}

pub struct MessageStream {
    inner: MessageStreamInner,
}

enum MessageStreamInner {
    Once(Option<Bytes>),
    Streaming(Pin<Box<dyn Stream<Item = Result<Bytes, Status>> + Send + 'static>>),
}

impl MessageStream {
    pub fn new(stream: impl Stream<Item = Result<Bytes, Status>> + Send + 'static) -> Self {
        Self {
            inner: MessageStreamInner::Streaming(Box::pin(stream)),
        }
    }

    pub const fn once(message: Bytes) -> Self {
        Self {
            inner: MessageStreamInner::Once(Some(message)),
        }
    }

    pub const fn empty() -> Self {
        Self {
            inner: MessageStreamInner::Once(None),
        }
    }

    pub async fn next(&mut self) -> Option<Result<Bytes, Status>> {
        core::future::poll_fn(|context| Pin::new(&mut *self).poll_next(context)).await
    }
}

impl Stream for MessageStream {
    type Item = Result<Bytes, Status>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &mut self.inner {
            MessageStreamInner::Once(message) => Poll::Ready(message.take().map(Ok)),
            MessageStreamInner::Streaming(stream) => stream.as_mut().poll_next(context),
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait GrpcService: Send + Sync + 'static {
    fn methods(&self) -> &'static [MethodDescriptor];

    async fn call(&self, path: &str, request: Request<MessageStream>) -> Result<Response<MessageStream>, Status>;
}

pub async fn decode_unary_request<T>(request: Request<MessageStream>) -> Result<Request<T>, Status>
where
    T: ProtoDecode + Send + Sync + 'static,
{
    let (metadata, extensions, mut messages) = request.into_parts();
    let bytes = messages.next().await.transpose()?.ok_or_else(|| Status::invalid_argument("missing request message"))?;
    if messages.next().await.transpose()?.is_some() {
        return Err(Status::invalid_argument("received multiple messages for a unary request"));
    }
    let mut message = T::decode(bytes, crate::DecodeContext::default()).map_err(|error| Status::invalid_argument(error.to_string()))?;
    T::validate_with_ext(&mut message, &extensions).map_err(|error| Status::invalid_argument(error.to_string()))?;
    Ok(Request::from_parts(metadata, extensions, message))
}

pub trait GrpcEncode<Mode = crate::SunByRef> {
    fn encode_grpc(self) -> Result<Bytes, Status>;
}

impl<T> GrpcEncode for T
where
    T: ProtoEncode + ProtoExt,
{
    fn encode_grpc(self) -> Result<Bytes, Status> {
        Ok(self.to_zero_copy().into_bytes())
    }
}

impl<T> GrpcEncode<crate::BytesMode> for ZeroCopy<T>
where
    T: ProtoEncode + ProtoExt,
{
    fn encode_grpc(self) -> Result<Bytes, Status> {
        Ok(self.into_bytes())
    }
}

// Response<Box<T>> and Response<Arc<T>> expose T in the RPC schema. Match
// the tonic codec's SunByRefDeref mode rather than encoding a root wrapper.
impl<T, P> GrpcEncode<crate::coders::SunByRefDeref> for P
where
    T: ProtoEncode + ProtoExt,
    P: core::ops::Deref<Target = T>,
{
    fn encode_grpc(self) -> Result<Bytes, Status> {
        Ok(self.deref().to_zero_copy().into_bytes())
    }
}

pub fn encode_unary_response<R, P>(response: R) -> Result<Response<MessageStream>, Status>
where
    R: ProtoResponse<P>,
    R::Encode: GrpcEncode<R::Mode>,
{
    let (metadata, message, extensions) = response.into_response().into_parts();
    Ok(Response::from_parts(
        metadata,
        MessageStream::once(message.encode_grpc()?),
        extensions,
    ))
}

pub fn encode_streaming_response<R, P, S>(response: Response<S>) -> Response<MessageStream>
where
    R: ProtoResponse<P> + 'static,
    P: 'static,
    R::Encode: GrpcEncode<R::Mode>,
    S: Stream<Item = Result<R, Status>> + Send + 'static,
{
    let (metadata, stream, extensions) = response.into_parts();
    let stream = EncodedResponseStream::<S, R, P> {
        inner: stream,
        marker: PhantomData,
    };
    Response::from_parts(metadata, MessageStream::new(stream), extensions)
}

pin_project_lite::pin_project! {
    struct EncodedResponseStream<S, R, P> {
        #[pin]
        inner: S,
        marker: PhantomData<fn() -> (R, P)>,
    }
}

impl<S, R, P> Stream for EncodedResponseStream<S, R, P>
where
    R: ProtoResponse<P>,
    R::Encode: GrpcEncode<R::Mode>,
    S: Stream<Item = Result<R, Status>>,
{
    type Item = Result<Bytes, Status>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project()
            .inner
            .poll_next(context)
            .map(|item| item.map(|result| result.and_then(|response| response.into_response().into_inner().encode_grpc())))
    }
}
