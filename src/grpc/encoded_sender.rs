use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

use crate::PreparedMessage as OwnedMessage;

/// Borrowed-input producer for client/bidirectional streaming. Encoding is
/// synchronous; only queue backpressure is asynchronous. Each pending send owns
/// its allocation, so creating unlimited pending futures uses unlimited memory.
pub struct EncodedSender<T> {
    sender: tokio::sync::mpsc::Sender<OwnedMessage>,
    preallocation: usize,
    _message: PhantomData<fn() -> T>,
}

impl<T> Clone for EncodedSender<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            preallocation: self.preallocation,
            _message: PhantomData,
        }
    }
}

pub struct EncodedStream(tokio::sync::mpsc::Receiver<OwnedMessage>);

impl super::Stream for EncodedStream {
    type Item = OwnedMessage;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_recv(cx)
    }
}

impl<T: crate::ProtoEncode + crate::ProtoExt> EncodedSender<T> {
    /// Queue size controls queued messages, not cached allocations or pending
    /// send futures. A zero queue size is rejected by Tokio.
    pub fn channel(queue_size: usize) -> (Self, EncodedStream) {
        let (sender, receiver) = tokio::sync::mpsc::channel(queue_size);
        (
            Self {
                sender,
                preallocation: crate::DEFAULT_MAX_ENCODE_PREALLOCATION,
                _message: PhantomData,
            },
            EncodedStream(receiver),
        )
    }

    #[must_use]
    pub const fn with_max_encode_preallocation(mut self, limit: usize) -> Self {
        self.preallocation = limit;
        self
    }

    pub fn send<'a>(&'a self, message: &T) -> impl Future<Output = Result<(), tonic::Status>> + Send + use<'a, T> {
        let prepared = <&T as crate::PrepareRequest<T>>::prepare_request(message, self.preallocation).map(tonic::Request::into_inner);
        async move { self.sender.send(prepared?).await.map_err(|_| tonic::Status::cancelled("encoded request stream closed")) }
    }
}
