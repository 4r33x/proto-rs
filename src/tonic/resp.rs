use crate::ProtoEncode;
use crate::grpc::ProtoResponse;

#[inline]
pub fn map_proto_stream_result<R, P>(result: Result<R, tonic::Status>) -> Result<<R as ProtoResponse<P>>::Encode, tonic::Status>
where
    R: ProtoResponse<P>,
    P: ProtoEncode,
{
    result.map(crate::grpc::map_proto_response::<R, P>)
}
