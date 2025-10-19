use core::any::TypeId;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;

use tonic::Request;

use bytes::Bytes;

use crate::BytesMode;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::SunByRef;
use crate::tonic::ToZeroCopyRequest;

/// A wrapper around [`tonic::Request<Vec<u8>>`] that remembers the protobuf
/// message type that produced the encoded bytes.
#[derive(Debug)]
pub struct ZeroCopyRequest<T> {
    inner: Request<Vec<u8>>,
    _marker: PhantomData<T>,
}

impl<T> ZeroCopyRequest<T> {
    #[inline]
    pub fn from_request(request: Request<Vec<u8>>) -> Self {
        Self { inner: request, _marker: PhantomData }
    }

    #[inline]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from_request(Request::new(bytes))
    }

    #[inline]
    pub fn into_request(self) -> Request<Vec<u8>> {
        self.inner
    }

    #[inline]
    pub fn as_request(&self) -> &Request<Vec<u8>> {
        &self.inner
    }

    #[inline]
    pub fn as_request_mut(&mut self) -> &mut Request<Vec<u8>> {
        &mut self.inner
    }
}

impl<T> From<ZeroCopyRequest<T>> for Request<Vec<u8>> {
    fn from(request: ZeroCopyRequest<T>) -> Self {
        request.into_request()
    }
}

impl<T> From<Request<T>> for ZeroCopyRequest<T>
where
    T: ProtoExt + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, OwnedSun = T>,
{
    fn from(request: Request<T>) -> Self {
        let (metadata, extensions, message) = request.into_parts();
        let mut message = ManuallyDrop::new(message);
        let type_id = TypeId::of::<T>();

        let encoded = if type_id == TypeId::of::<Vec<u8>>() {
            unsafe { core::ptr::read((&mut *message) as *mut T as *mut Vec<u8>) }
        } else if type_id == TypeId::of::<Bytes>() {
            let bytes = unsafe { core::ptr::read((&mut *message) as *mut T as *mut Bytes) };
            bytes.to_vec()
        } else {
            let encoded = T::encode_to_vec(&*message);
            unsafe { ManuallyDrop::drop(&mut message); }
            encoded
        };

        ZeroCopyRequest::from_request(Request::from_parts(metadata, extensions, encoded))
    }
}

impl<'a, T> From<Request<&'a T>> for ZeroCopyRequest<T>
where
    T: ProtoExt + 'static,
    for<'b> T::Shadow<'b>: ProtoShadow<Sun<'b> = &'b T, OwnedSun = T>,
{
    fn from(request: Request<&'a T>) -> Self {
        let (metadata, extensions, message) = request.into_parts();
        let type_id = TypeId::of::<T>();
        let encoded = if type_id == TypeId::of::<Vec<u8>>() {
            unsafe { (&*(message as *const T as *const Vec<u8>)).clone() }
        } else if type_id == TypeId::of::<Bytes>() {
            unsafe { (&*(message as *const T as *const Bytes)).to_vec() }
        } else {
            T::encode_to_vec(message)
        };
        ZeroCopyRequest::from_request(Request::from_parts(metadata, extensions, encoded))
    }
}

impl<T> ZeroCopyRequest<T>
where
    T: ProtoExt + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, OwnedSun = T>,
{
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
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, OwnedSun = T>,
{
    type Encode = T;
    type Mode = SunByRef;

    fn into_request(self) -> Request<Self::Encode> {
        self
    }
}

impl<T> ProtoRequest<T> for T
where
    T: ProtoExt + Send + Sync + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, OwnedSun = T>,
{
    type Encode = T;
    type Mode = SunByRef;

    fn into_request(self) -> Request<Self::Encode> {
        Request::new(self)
    }
}

impl<T> ProtoRequest<T> for ZeroCopyRequest<T> {
    type Encode = Vec<u8>;
    type Mode = BytesMode;

    fn into_request(self) -> Request<Self::Encode> {
        self.inner
    }
}
impl<T> ToZeroCopyRequest<T> for &T
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<Sun<'b> = &'b T, OwnedSun = T>,
{
    fn to_zero_copy(self) -> ZeroCopyRequest<T> {
        let encoded = T::encode_to_vec(self);
        ZeroCopyRequest::from_bytes(encoded)
    }
}

impl<T> ToZeroCopyRequest<T> for Request<&T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<Sun<'b> = &'b T, OwnedSun = T>,
{
    fn to_zero_copy(self) -> ZeroCopyRequest<T> {
        let (meta, ext, t) = self.into_parts();
        let encoded = T::encode_to_vec(t);
        ZeroCopyRequest::from_request(Request::from_parts(meta, ext, encoded))
    }
}
