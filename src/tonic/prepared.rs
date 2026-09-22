use core::future::Future;

use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::Streaming;

use crate::PreparedMessage as OwnedMessage;

/// Prepared payload accepted by generated clients. The owned extension hands
/// its frame to Tonic directly; the upstream adapter copies only the payload.
#[cfg(feature = "tonic-owned")]
pub type PreparedMessage = tonic::codec::OwnedMessage;

#[cfg(not(feature = "tonic-owned"))]
#[derive(Debug)]
pub struct PreparedMessage {
    frame: bytes::Bytes,
}

#[cfg(not(feature = "tonic-owned"))]
impl PreparedMessage {
    pub fn from_uncompressed_frame(frame: bytes::Bytes) -> Result<Self, Status> {
        if frame.len() < 5 || frame[0] != 0 {
            return Err(Status::internal("invalid prepared gRPC frame header"));
        }
        let len = u32::from_be_bytes(frame[1..5].try_into().unwrap()) as usize;
        if len != frame.len() - 5 {
            return Err(Status::internal("invalid prepared gRPC frame length"));
        }
        Ok(Self { frame })
    }

    pub fn payload(&self) -> &[u8] {
        &self.frame[5..]
    }
}

use crate::BytesMode;
use crate::ProtoCodec;
use crate::ProtoDecode;

/// Request-independent future types keep borrowed input out of generated RPC
/// futures. Only the client and prepared owned frame survive preparation.
#[doc(hidden)]
pub trait PreparedRpc<R> {
    type Unary<'a>: Future<Output = Result<Response<R>, Status>> + Send + 'a
    where
        Self: 'a;
    type Streaming<'a>: Future<Output = Result<Response<Streaming<R>>, Status>> + Send + 'a
    where
        Self: 'a;

    fn prepared_unary(
        &mut self,
        request: Result<Request<OwnedMessage>, Status>,
        path: tonic::codegen::http::uri::PathAndQuery,
    ) -> Self::Unary<'_>;
    fn prepared_streaming(
        &mut self,
        request: Result<Request<OwnedMessage>, Status>,
        path: tonic::codegen::http::uri::PathAndQuery,
    ) -> Self::Streaming<'_>;
}

impl<T, R> PreparedRpc<R> for tonic::client::Grpc<T>
where
    T: tonic::client::GrpcService<tonic::body::Body> + Send,
    T::Future: Send,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = bytes::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
    R: ProtoDecode + Send + Sync + 'static,
{
    #[cfg(not(feature = "stable"))]
    type Unary<'a>
        = impl Future<Output = Result<Response<R>, Status>> + Send + 'a
    where
        Self: 'a;
    #[cfg(feature = "stable")]
    type Unary<'a>
        = core::pin::Pin<Box<dyn Future<Output = Result<Response<R>, Status>> + Send + 'a>>
    where
        Self: 'a;
    #[cfg(not(feature = "stable"))]
    type Streaming<'a>
        = impl Future<Output = Result<Response<Streaming<R>>, Status>> + Send + 'a
    where
        Self: 'a;
    #[cfg(feature = "stable")]
    type Streaming<'a>
        = core::pin::Pin<Box<dyn Future<Output = Result<Response<Streaming<R>>, Status>> + Send + 'a>>
    where
        Self: 'a;

    fn prepared_unary(
        &mut self,
        request: Result<Request<OwnedMessage>, Status>,
        path: tonic::codegen::http::uri::PathAndQuery,
    ) -> Self::Unary<'_> {
        let future = async move {
            let request = request?;
            self.ready().await.map_err(|e| Status::unknown(format!("Service was not ready: {}", e.into())))?;
            self.unary(request, path, ProtoCodec::<OwnedMessage, R, BytesMode>::default()).await
        };
        #[cfg(feature = "stable")]
        let future = Box::pin(future);
        future
    }

    fn prepared_streaming(
        &mut self,
        request: Result<Request<OwnedMessage>, Status>,
        path: tonic::codegen::http::uri::PathAndQuery,
    ) -> Self::Streaming<'_> {
        let future = async move {
            let request = request?;
            self.ready().await.map_err(|e| Status::unknown(format!("Service was not ready: {}", e.into())))?;
            self.server_streaming(request, path, ProtoCodec::<OwnedMessage, R, BytesMode>::default()).await
        };
        #[cfg(feature = "stable")]
        let future = Box::pin(future);
        future
    }
}
