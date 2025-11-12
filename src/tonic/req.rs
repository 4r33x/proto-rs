use core::marker::PhantomData;

use tonic::Request;

use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ToZeroCopyRequest;
use crate::coders::BytesMode;
use crate::coders::SunByRef;
use crate::zero_copy::ZeroCopyBuffer;

/// A wrapper around [`tonic::Request<ZeroCopyBuffer>`] that remembers the protobuf
/// message type that produced the encoded bytes.
#[derive(Debug)]
pub struct ZeroCopyRequest<T> {
    inner: Request<ZeroCopyBuffer>,
    _marker: PhantomData<T>,
}

impl<T> ZeroCopyRequest<T> {
    #[inline]
    pub fn from_zerocopy_request(request: Request<ZeroCopyBuffer>) -> Self {
        Self { inner: request, _marker: PhantomData }
    }

    #[inline]
    pub fn from_zerocopy(bytes: ZeroCopyBuffer) -> Self {
        Self::from_zerocopy_request(Request::new(bytes))
    }

    #[inline]
    pub fn into_request(self) -> Request<ZeroCopyBuffer> {
        self.inner
    }

    #[inline]
    pub fn as_request(&self) -> &Request<ZeroCopyBuffer> {
        &self.inner
    }

    #[inline]
    pub fn as_request_mut(&mut self) -> &mut Request<ZeroCopyBuffer> {
        &mut self.inner
    }
}

impl<T> From<ZeroCopyRequest<T>> for Request<ZeroCopyBuffer> {
    #[inline]
    fn from(request: ZeroCopyRequest<T>) -> Self {
        request.into_request()
    }
}

impl<T> From<Request<T>> for ZeroCopyRequest<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    #[inline]
    fn from(request: Request<T>) -> Self {
        let (metadata, extensions, message) = request.into_parts();
        let encoded = T::encode_to_zerocopy(&message);
        ZeroCopyRequest::from_zerocopy_request(Request::from_parts(metadata, extensions, encoded))
    }
}

impl<'a, T> From<Request<&'a T>> for ZeroCopyRequest<T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<T, Sun<'b> = &'b T, OwnedSun = T>,
{
    #[inline]
    fn from(request: Request<&'a T>) -> Self {
        let (metadata, extensions, message) = request.into_parts();
        let encoded = T::encode_to_zerocopy(message);
        ZeroCopyRequest::from_zerocopy_request(Request::from_parts(metadata, extensions, encoded))
    }
}

impl<T> ZeroCopyRequest<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    #[inline]
    pub fn from_message(message: T) -> Self {
        Request::new(message).into()
    }
}

pub trait ProtoRequest<T>: Sized {
    type Encode: Send + Sync + 'static;
    type Mode: Send + Sync + 'static;
    fn into_request(self) -> Request<Self::Encode>;
}

impl<T> ProtoRequest<T> for Request<T>
where
    T: ProtoExt + Send + Sync + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    type Encode = T;
    type Mode = SunByRef;
    #[inline]
    fn into_request(self) -> Request<Self::Encode> {
        self
    }
}

impl<T> ProtoRequest<T> for T
where
    T: ProtoExt + Send + Sync + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    type Encode = T;
    type Mode = SunByRef;
    #[inline]
    fn into_request(self) -> Request<Self::Encode> {
        Request::new(self)
    }
}

impl<T> ProtoRequest<T> for ZeroCopyRequest<T> {
    type Encode = ZeroCopyBuffer;
    type Mode = BytesMode;
    #[inline]
    fn into_request(self) -> Request<Self::Encode> {
        self.inner
    }
}
impl<T> ToZeroCopyRequest<T> for &T
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<T, Sun<'b> = &'b T, OwnedSun = T>,
{
    #[inline]
    fn to_zero_copy(self) -> ZeroCopyRequest<T> {
        let encoded = T::encode_to_zerocopy(self);
        ZeroCopyRequest::from_zerocopy(encoded)
    }
}

impl<T> ToZeroCopyRequest<T> for Request<&T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<T, Sun<'b> = &'b T, OwnedSun = T>,
{
    #[inline]
    fn to_zero_copy(self) -> ZeroCopyRequest<T> {
        let (meta, ext, t) = self.into_parts();
        let encoded = T::encode_to_zerocopy(t);
        ZeroCopyRequest::from_zerocopy_request(Request::from_parts(meta, ext, encoded))
    }
}
