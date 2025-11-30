use core::marker::PhantomData;

use tonic::Response;
use tonic::Status;

use crate::BytesMode;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::SunByRef;
use crate::alloc::boxed::Box;
use crate::alloc::sync::Arc;
use crate::tonic::ToZeroCopyResponse;
use crate::zero_copy::ZeroCopyBuffer;

#[derive(Debug)]
pub struct ZeroCopyResponse<T> {
    inner: Response<ZeroCopyBuffer>,
    _marker: PhantomData<T>,
}

impl<T> ZeroCopyResponse<T> {
    #[inline]
    pub fn from_zerocopy_response(response: Response<ZeroCopyBuffer>) -> Self {
        Self {
            inner: response,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn from_zerocopy(bytes: ZeroCopyBuffer) -> Self {
        Self::from_zerocopy_response(Response::new(bytes))
    }

    #[inline]
    pub fn into_response(self) -> Response<ZeroCopyBuffer> {
        self.inner
    }

    #[inline]
    pub fn as_response(&self) -> &Response<ZeroCopyBuffer> {
        &self.inner
    }

    #[inline]
    pub fn as_response_mut(&mut self) -> &mut Response<ZeroCopyBuffer> {
        &mut self.inner
    }
}

impl<T> From<ZeroCopyResponse<T>> for Response<ZeroCopyBuffer> {
    #[inline]
    fn from(request: ZeroCopyResponse<T>) -> Self {
        request.into_response()
    }
}

impl<T> From<Response<T>> for ZeroCopyResponse<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    #[inline]
    fn from(request: Response<T>) -> Self {
        let (metadata, message, extensions) = request.into_parts();
        let encoded = T::encode_to_zerocopy(&message);
        ZeroCopyResponse::from_zerocopy_response(Response::from_parts(metadata, encoded, extensions))
    }
}

impl<'a, T> From<Response<&'a T>> for ZeroCopyResponse<T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<T, Sun<'b> = &'b T, OwnedSun = T>,
{
    #[inline]
    fn from(request: Response<&'a T>) -> Self {
        let (metadata, message, extensions) = request.into_parts();
        let encoded = T::encode_to_zerocopy(message);
        ZeroCopyResponse::from_zerocopy_response(Response::from_parts(metadata, encoded, extensions))
    }
}

impl<T> ZeroCopyResponse<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    #[inline]
    pub fn from_message(message: T) -> Self {
        Response::new(message).into()
    }
}

pub trait ProtoResponse<T>: Sized {
    type Encode: Send + Sync + 'static;
    type Mode: Send + Sync + 'static;

    fn into_response(self) -> Response<Self::Encode>;
}

impl<T> ProtoResponse<T> for Response<T>
where
    T: ProtoExt + Send + Sync + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
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
    T: ProtoExt + Send + Sync + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
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
    T: ProtoExt + Send + Sync + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
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
    T: ProtoExt + Send + Sync + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    type Encode = T;
    type Mode = SunByRef;
    #[inline]
    fn into_response(self) -> Response<Self::Encode> {
        Response::new(self)
    }
}

impl<T> ProtoResponse<T> for ZeroCopyResponse<T> {
    type Encode = ZeroCopyBuffer;
    type Mode = BytesMode;
    #[inline]
    fn into_response(self) -> Response<Self::Encode> {
        self.inner
    }
}

#[inline]
pub fn map_proto_response<R, P>(value: R) -> <R as ProtoResponse<P>>::Encode
where
    R: ProtoResponse<P>,
    P: ProtoExt,
{
    <R as ProtoResponse<P>>::into_response(value).into_inner()
}

#[inline]
pub fn map_proto_stream_result<R, P>(result: Result<R, Status>) -> Result<<R as ProtoResponse<P>>::Encode, Status>
where
    R: ProtoResponse<P>,
    P: ProtoExt,
{
    result.map(map_proto_response::<R, P>)
}

impl<T> ToZeroCopyResponse<T> for &T
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<T, Sun<'b> = &'b T, OwnedSun = T>,
{
    #[inline]
    fn to_zero_copy(self) -> ZeroCopyResponse<T> {
        let encoded = T::encode_to_zerocopy(self);
        ZeroCopyResponse::from_zerocopy(encoded)
    }
}

impl<T> ToZeroCopyResponse<T> for Response<&T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<T, Sun<'b> = &'b T, OwnedSun = T>,
{
    #[inline]
    fn to_zero_copy(self) -> ZeroCopyResponse<T> {
        let (meta, t, ext) = self.into_parts();
        let encoded = T::encode_to_zerocopy(t);
        ZeroCopyResponse::from_zerocopy_response(Response::from_parts(meta, encoded, ext))
    }
}
