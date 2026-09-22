//! Transport-neutral gRPC primitives and client contract.

pub use futures_core::Stream;

mod types;
pub use types::*;
mod service;
pub use service::*;
mod response;
pub use response::*;

#[cfg(feature = "tonic")]
mod tonic;
#[cfg(feature = "tonic")]
pub use tonic::*;

#[cfg(feature = "tonic-transport")]
mod channel;
#[cfg(feature = "tonic-transport")]
pub use channel::AutoChannel;
#[cfg(feature = "tonic-transport")]
pub use channel::ChannelOptions;
#[cfg(feature = "tonic-transport")]
mod encoded_sender;
#[cfg(feature = "tonic-transport")]
pub use encoded_sender::EncodedSender;
#[cfg(feature = "tonic-transport")]
pub use encoded_sender::EncodedStream;

#[cfg(all(feature = "linux-zerocopy", target_os = "linux"))]
pub mod zerocopy;

/// Minimal client-side contract needed by generated gRPC clients.
#[allow(async_fn_in_trait)]
pub trait GrpcTransport {
    type Error;
    type ResponseStream<T>: Stream<Item = Result<T, Self::Error>> + Send + 'static
    where
        T: Send + 'static;

    async fn unary<Req, Res>(&mut self, route: &'static str, request: Request<Req>) -> Result<Response<Res>, Self::Error>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static;

    async fn client_streaming<Req, Res, S>(&mut self, route: &'static str, request: Request<S>) -> Result<Response<Res>, Self::Error>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static,
        S: Stream<Item = Req> + Send + 'static;

    async fn server_streaming<Req, Res>(
        &mut self,
        route: &'static str,
        request: Request<Req>,
    ) -> Result<Response<Self::ResponseStream<Res>>, Self::Error>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static;

    async fn bidirectional_streaming<Req, Res, S>(
        &mut self,
        route: &'static str,
        request: Request<S>,
    ) -> Result<Response<Self::ResponseStream<Res>>, Self::Error>
    where
        Req: crate::ProtoEncode + crate::ProtoExt + Send + Sync + 'static,
        Res: crate::ProtoDecode + Send + Sync + 'static,
        S: Stream<Item = Req> + Send + 'static;
}
