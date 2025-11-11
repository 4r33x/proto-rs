use core::marker::PhantomData;

use tonic::Request;

use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ToZeroCopyRequest;
use crate::ZeroCopy;
use crate::coders::SunByRef;
use crate::coders::SunByVal;

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

impl<T> ProtoRequest<T> for ZeroCopy<T>
where
    T: ProtoExt + Send + Sync + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    type Encode = ZeroCopy<T>;
    type Mode = SunByVal;

    #[inline]
    fn into_request(self) -> Request<Self::Encode> {
        self.into_tonic_request()
    }
}

impl<T> ZeroCopy<T> {
    #[inline]
    pub fn from_tonic_request(request: Request<Vec<u8>>) -> Self {
        let (metadata, extensions, bytes) = request.into_parts();
        Self::from_parts(metadata, extensions, bytes)
    }

    #[inline]
    pub fn into_tonic_request(self) -> Request<ZeroCopy<T>> {
        let ZeroCopy { inner, metadata, extensions, .. } = self;
        let body = ZeroCopy {
            inner,
            metadata: Default::default(),
            extensions: Default::default(),
            _marker: PhantomData,
        };
        Request::from_parts(metadata, extensions, body)
    }

    #[inline]
    pub fn into_bytes_request(self) -> Request<Vec<u8>> {
        let ZeroCopy { inner, metadata, extensions, .. } = self;
        Request::from_parts(metadata, extensions, inner)
    }
}

impl<T> From<ZeroCopy<T>> for Request<Vec<u8>> {
    #[inline]
    fn from(value: ZeroCopy<T>) -> Self {
        value.into_bytes_request()
    }
}

impl<T> From<Request<T>> for ZeroCopy<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    #[inline]
    fn from(request: Request<T>) -> Self {
        let (metadata, extensions, message) = request.into_parts();
        let encoded = T::encode_to_vec(&message);
        ZeroCopy::from_parts(metadata, extensions, encoded)
    }
}

impl<'a, T> From<Request<&'a T>> for ZeroCopy<T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<T, Sun<'b> = &'b T, OwnedSun = T>,
{
    #[inline]
    fn from(request: Request<&'a T>) -> Self {
        let (metadata, extensions, message) = request.into_parts();
        let encoded = T::encode_to_vec(message);
        ZeroCopy::from_parts(metadata, extensions, encoded)
    }
}

impl<T> From<Request<ZeroCopy<T>>> for ZeroCopy<T> {
    #[inline]
    fn from(request: Request<ZeroCopy<T>>) -> Self {
        let (metadata, extensions, body) = request.into_parts();
        let (_, _, bytes) = body.into_parts();
        ZeroCopy::from_parts(metadata, extensions, bytes)
    }
}

impl<T> ToZeroCopyRequest<T> for &T
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<T, Sun<'b> = &'b T, OwnedSun = T>,
{
    #[inline]
    fn to_zero_copy(self) -> ZeroCopy<T> {
        ZeroCopy::from_borrowed(self)
    }
}

impl<T> ToZeroCopyRequest<T> for Request<&T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<T, Sun<'b> = &'b T, OwnedSun = T>,
{
    #[inline]
    fn to_zero_copy(self) -> ZeroCopy<T> {
        let (metadata, extensions, value) = self.into_parts();
        let encoded = T::encode_to_vec(value);
        ZeroCopy::from_parts(metadata, extensions, encoded)
    }
}
