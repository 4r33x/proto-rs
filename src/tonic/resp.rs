use core::marker::PhantomData;

use tonic::Response;

use crate::BytesMode;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::SunByRef;
use crate::tonic::ToZeroCopyResponse;

/// A wrapper around [`tonic::Response<Vec<u8>>`] that remembers the protobuf
/// message type that produced the encoded bytes.
#[derive(Debug)]
pub struct ZeroCopyResponse<T> {
    inner: Response<Vec<u8>>,
    _marker: PhantomData<T>,
}

impl<T> ZeroCopyResponse<T> {
    #[inline]
    pub fn from_response(request: Response<Vec<u8>>) -> Self {
        Self { inner: request, _marker: PhantomData }
    }

    #[inline]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from_response(Response::new(bytes))
    }

    #[inline]
    pub fn into_response(self) -> Response<Vec<u8>> {
        self.inner
    }

    #[inline]
    pub fn as_response(&self) -> &Response<Vec<u8>> {
        &self.inner
    }

    #[inline]
    pub fn as_response_mut(&mut self) -> &mut Response<Vec<u8>> {
        &mut self.inner
    }
}

impl<T> From<ZeroCopyResponse<T>> for Response<Vec<u8>> {
    fn from(request: ZeroCopyResponse<T>) -> Self {
        request.into_response()
    }
}

impl<T> From<Response<T>> for ZeroCopyResponse<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, OwnedSun = T>,
{
    fn from(request: Response<T>) -> Self {
        let (metadata, message, extensions) = request.into_parts();
        let encoded = T::encode_to_vec(&message);
        ZeroCopyResponse::from_response(Response::from_parts(metadata, encoded, extensions))
    }
}

impl<'a, T> From<Response<&'a T>> for ZeroCopyResponse<T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<Sun<'b> = &'b T, OwnedSun = T>,
{
    fn from(request: Response<&'a T>) -> Self {
        let (metadata, message, extensions) = request.into_parts();
        let encoded = T::encode_to_vec(message);
        ZeroCopyResponse::from_response(Response::from_parts(metadata, encoded, extensions))
    }
}

impl<T> ZeroCopyResponse<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, OwnedSun = T>,
{
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
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, OwnedSun = T>,
{
    type Encode = T;
    type Mode = SunByRef;

    fn into_response(self) -> Response<Self::Encode> {
        self
    }
}

impl<T> ProtoResponse<T> for T
where
    T: ProtoExt + Send + Sync + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, OwnedSun = T>,
{
    type Encode = T;
    type Mode = SunByRef;

    fn into_response(self) -> Response<Self::Encode> {
        Response::new(self)
    }
}

impl<T> ProtoResponse<T> for ZeroCopyResponse<T> {
    type Encode = Vec<u8>;
    type Mode = BytesMode;

    fn into_response(self) -> Response<Self::Encode> {
        self.inner
    }
}

impl<T> ToZeroCopyResponse<T> for &T
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<Sun<'b> = &'b T, OwnedSun = T>,
{
    fn to_zero_copy(self) -> ZeroCopyResponse<T> {
        let encoded = T::encode_to_vec(self);
        ZeroCopyResponse::from_bytes(encoded)
    }
}

impl<T> ToZeroCopyResponse<T> for Response<&T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<Sun<'b> = &'b T, OwnedSun = T>,
{
    fn to_zero_copy(self) -> ZeroCopyResponse<T> {
        let (meta, t, ext) = self.into_parts();
        let encoded = T::encode_to_vec(t);
        ZeroCopyResponse::from_response(Response::from_parts(meta, encoded, ext))
    }
}
