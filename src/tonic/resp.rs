use tonic::Response;
use tonic::Status;

use crate::ProtoExt;
use crate::ProtoShadow;
use crate::SunByRef;
use crate::ToZeroCopyResponse;
use crate::ZeroCopy;
use crate::coders::SunByVal;

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

impl<T> ProtoResponse<T> for ZeroCopy<T>
where
    T: ProtoExt + Send + Sync + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    type Encode = ZeroCopy<T>;
    type Mode = SunByVal;

    #[inline]
    fn into_response(self) -> Response<Self::Encode> {
        self.into_tonic_response()
    }
}

impl<T> ZeroCopy<T> {
    #[inline]
    pub fn from_tonic_response(response: Response<Vec<u8>>) -> Self {
        let (metadata, bytes, extensions) = response.into_parts();
        Self::from_parts(metadata, extensions, bytes)
    }

    #[inline]
    pub fn into_tonic_response(self) -> Response<ZeroCopy<T>> {
        let (metadata, extensions, inner) = self.into_parts();
        let body = ZeroCopy::from_parts(Default::default(), Default::default(), inner);
        Response::from_parts(metadata, body, extensions)
    }

    #[inline]
    pub fn into_bytes_response(self) -> Response<Vec<u8>> {
        let (metadata, extensions, inner) = self.into_parts();
        Response::from_parts(metadata, inner, extensions)
    }
}

impl<T> From<ZeroCopy<T>> for Response<Vec<u8>> {
    #[inline]
    fn from(value: ZeroCopy<T>) -> Self {
        value.into_bytes_response()
    }
}

impl<T> From<Response<T>> for ZeroCopy<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    #[inline]
    fn from(response: Response<T>) -> Self {
        let (metadata, message, extensions) = response.into_parts();
        let encoded = T::encode_to_vec(&message);
        ZeroCopy::from_parts(metadata, extensions, encoded)
    }
}

impl<'a, T> From<Response<&'a T>> for ZeroCopy<T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<T, Sun<'b> = &'b T, OwnedSun = T>,
{
    #[inline]
    fn from(response: Response<&'a T>) -> Self {
        let (metadata, message, extensions) = response.into_parts();
        let encoded = T::encode_to_vec(message);
        ZeroCopy::from_parts(metadata, extensions, encoded)
    }
}

impl<T> From<Response<ZeroCopy<T>>> for ZeroCopy<T> {
    #[inline]
    fn from(response: Response<ZeroCopy<T>>) -> Self {
        let (metadata, body, extensions) = response.into_parts();
        let (_, _, bytes) = body.into_parts();
        ZeroCopy::from_parts(metadata, extensions, bytes)
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
    fn to_zero_copy(self) -> ZeroCopy<T> {
        ZeroCopy::from_borrowed(self)
    }
}

impl<T> ToZeroCopyResponse<T> for Response<&T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<T, Sun<'b> = &'b T, OwnedSun = T>,
{
    #[inline]
    fn to_zero_copy(self) -> ZeroCopy<T> {
        let (metadata, value, extensions) = self.into_parts();
        let encoded = T::encode_to_vec(value);
        ZeroCopy::from_parts(metadata, extensions, encoded)
    }
}
