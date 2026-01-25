use tonic::Response;
use tonic::Status;

use crate::ProtoEncode;
use crate::ProtoExt;
use crate::SunByRef;
use crate::alloc::boxed::Box;
use crate::alloc::sync::Arc;
use crate::coders::ZeroCopyMode;
use crate::traits::ProtoArchive;
use crate::traits::ZeroCopy;

pub trait ProtoResponse<T>: Sized {
    type Encode: Send + Sync + 'static;
    type Mode: Send + Sync + 'static;

    fn into_response(self) -> Response<Self::Encode>;
}

impl<T> ProtoResponse<T> for Response<T>
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = T;
    type Mode = SunByRef;
    #[inline]
    fn into_response(self) -> Response<Self::Encode> {
        self
    }
}

impl<T> ProtoResponse<T> for Response<Arc<T>>
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = Arc<T>;
    type Mode = crate::coders::SunByRefDeref;

    #[inline]
    fn into_response(self) -> Response<Self::Encode> {
        self
    }
}

impl<T> ProtoResponse<T> for Response<Box<T>>
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = Box<T>;
    type Mode = crate::coders::SunByRefDeref;

    #[inline]
    fn into_response(self) -> Response<Self::Encode> {
        self
    }
}

impl<T> ProtoResponse<T> for T
where
    T: ProtoEncode + Send + Sync + 'static,
{
    type Encode = T;
    type Mode = SunByRef;
    #[inline]
    fn into_response(self) -> Response<Self::Encode> {
        Response::new(self)
    }
}

// ZeroCopy<T> can be used in place of T for responses
impl<T> ProtoResponse<T> for ZeroCopy<T>
where
    T: ProtoEncode + ProtoExt + Send + Sync + 'static,
    for<'s> <T as ProtoEncode>::Shadow<'s>: ProtoArchive,
{
    type Encode = ZeroCopy<T>;
    type Mode = ZeroCopyMode;
    #[inline]
    fn into_response(self) -> Response<Self::Encode> {
        Response::new(self)
    }
}

impl<T> ProtoResponse<T> for Response<ZeroCopy<T>>
where
    T: ProtoEncode + ProtoExt + Send + Sync + 'static,
    for<'s> <T as ProtoEncode>::Shadow<'s>: ProtoArchive,
{
    type Encode = ZeroCopy<T>;
    type Mode = ZeroCopyMode;
    #[inline]
    fn into_response(self) -> Response<Self::Encode> {
        self
    }
}

#[inline]
pub fn map_proto_response<R, P>(value: R) -> <R as ProtoResponse<P>>::Encode
where
    R: ProtoResponse<P>,
    P: ProtoEncode,
{
    <R as ProtoResponse<P>>::into_response(value).into_inner()
}

#[inline]
pub fn map_proto_stream_result<R, P>(result: Result<R, Status>) -> Result<<R as ProtoResponse<P>>::Encode, Status>
where
    R: ProtoResponse<P>,
    P: ProtoEncode,
{
    result.map(map_proto_response::<R, P>)
}
