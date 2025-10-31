use core::marker::PhantomData;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

use tonic::Status;

use crate::ProtoExt;
use crate::ProtoResponse;

/// A stream adapter that converts [`ProtoResponse`] items into the encoded
/// representation expected by the gRPC transport.
#[derive(Debug)]
pub struct MapResponseStream<S, R, P> {
    inner: S,
    _marker: PhantomData<(R, P)>,
}

impl<S, R, P> MapResponseStream<S, R, P> {
    pub fn new(inner: S) -> Self {
        Self { inner, _marker: PhantomData }
    }
}

impl<S, R, P> tonic::codegen::tokio_stream::Stream for MapResponseStream<S, R, P>
where
    S: tonic::codegen::tokio_stream::Stream<Item = ::core::result::Result<R, Status>>,
    R: ProtoResponse<P>,
    P: ProtoExt,
{
    type Item = ::core::result::Result<<R as ProtoResponse<P>>::Encode, Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let inner = unsafe {
            // Safety: `inner` is never moved out of `MapResponseStream`, so projecting a
            // pinned mutable reference to it preserves the pinning guarantees of `self`.
            self.map_unchecked_mut(|s| &mut s.inner)
        };
        match inner.poll_next(cx) {
            Poll::Ready(Some(Ok(value))) => {
                let encoded = <R as ProtoResponse<P>>::into_response(value).into_inner();
                Poll::Ready(Some(Ok(encoded)))
            }
            Poll::Ready(Some(Err(status))) => Poll::Ready(Some(Err(status))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<S, R, P> MapResponseStream<S, R, P>
where
    S: tonic::codegen::tokio_stream::Stream<Item = ::core::result::Result<R, Status>>,
    R: ProtoResponse<P>,
    P: ProtoExt,
{
    pub fn into_inner(self) -> S {
        self.inner
    }
}
