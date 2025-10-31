use core::pin::Pin;

use ::tonic::codegen::tokio_stream::StreamExt;
use tonic::Status;

use crate::ProtoExt;
use crate::tonic::ProtoResponse;
use crate::tonic::map_proto_stream_result;

type DynStream<T> = dyn ::futures_core::Stream<Item = Result<T, Status>> + Send + 'static;

pub type BoxEncodeStream<T> = Pin<Box<DynStream<T>>>;

pub fn box_map_proto_stream<S, R, P>(stream: S) -> BoxEncodeStream<<R as ProtoResponse<P>>::Encode>
where
    S: ::futures_core::Stream<Item = Result<R, Status>> + Send + 'static,
    R: ProtoResponse<P> + 'static,
    P: ProtoExt + 'static,
{
    Box::pin(stream.map(map_proto_stream_result::<R, P>))
}
