use core::marker::PhantomData;

use bytes::BufMut;
use tonic::Request;
use tonic::Status;
use tonic::codec::Codec;
use tonic::codec::DecodeBuf;
use tonic::codec::Decoder;
use tonic::codec::EncodeBuf;
use tonic::codec::Encoder;

use crate::ProtoExt;
use crate::alloc::vec::Vec;
use crate::traits::ProtoShadow;

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
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, OwnedSun = T>,
{
    fn from(request: Request<T>) -> Self {
        let (metadata, extensions, message) = request.into_parts();
        let encoded = T::encode_to_vec(&message);
        ZeroCopyRequest::from_request(Request::from_parts(metadata, extensions, encoded))
    }
}

impl<'a, T> From<Request<&'a T>> for ZeroCopyRequest<T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<Sun<'b> = &'b T, OwnedSun = T>,
{
    fn from(request: Request<&'a T>) -> Self {
        let (metadata, extensions, message) = request.into_parts();
        let encoded = T::encode_to_vec(message);
        ZeroCopyRequest::from_request(Request::from_parts(metadata, extensions, encoded))
    }
}

impl<T> ZeroCopyRequest<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, OwnedSun = T>,
{
    pub fn from_message(message: T) -> Self {
        Request::new(message).into()
    }
}

impl<T> Clone for ZeroCopyRequest<T> {
    fn clone(&self) -> Self {
        let metadata = self.as_request().metadata().clone();
        let extensions = self.as_request().extensions().clone();
        let payload = self.as_request().get_ref().clone();
        ZeroCopyRequest::from_request(Request::from_parts(metadata, extensions, payload))
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

impl<T> ProtoRequest<T> for ZeroCopyRequest<T> {
    type Encode = Vec<u8>;
    type Mode = BytesMode;

    fn into_request(self) -> Request<Self::Encode> {
        self.into_request()
    }
}

pub trait ToZeroCopy<T> {
    fn to_zero_copy(&self) -> ZeroCopyRequest<T>;
}

impl<'a, T> ToZeroCopy<T> for &'a T
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<Sun<'b> = &'b T, OwnedSun = T>,
{
    fn to_zero_copy(&self) -> ZeroCopyRequest<T> {
        let encoded = T::encode_to_vec(*self);
        ZeroCopyRequest::from_request(Request::new(encoded))
    }
}

impl<'a, T> ToZeroCopy<T> for Request<&'a T>
where
    T: ProtoExt,
    for<'b> T::Shadow<'b>: ProtoShadow<Sun<'b> = &'b T, OwnedSun = T>,
{
    fn to_zero_copy(&self) -> ZeroCopyRequest<T> {
        let metadata = self.metadata().clone();
        let extensions = self.extensions().clone();
        let encoded = T::encode_to_vec(self.get_ref());
        ZeroCopyRequest::from_request(Request::from_parts(metadata, extensions, encoded))
    }
}

pub trait AsBytes {
    fn as_bytes(&self) -> &[u8];
}

impl AsBytes for Vec<u8> {
    fn as_bytes(&self) -> &[u8] {
        self
    }
}
impl<const N: usize> AsBytes for [u8; N] {
    fn as_bytes(&self) -> &[u8] {
        self
    }
}
#[derive(Clone, Copy, Default)]
pub struct BytesMode;
#[derive(Clone, Copy, Default)]
pub struct SunByVal {} // Sun<'a> = T
#[derive(Clone, Copy, Default)]
pub struct SunByRef {} // Sun<'a> = &'a T

unsafe impl Send for BytesMode {}
unsafe impl Sync for BytesMode {}
unsafe impl Send for SunByVal {}
unsafe impl Sync for SunByVal {}
unsafe impl Send for SunByRef {}
unsafe impl Sync for SunByRef {}

#[derive(Debug, Clone)]
pub struct ProtoCodec<Encode = (), Decode = (), Mode = SunByRef> {
    _marker: PhantomData<(Encode, Decode, Mode)>,
}

impl<Encode, Decode, Mode> Default for ProtoCodec<Encode, Decode, Mode> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<Encode, Decode, Mode> ProtoCodec<Encode, Decode, Mode> {
    pub fn new() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<Encode, Decode, Mode> Codec for ProtoCodec<Encode, Decode, Mode>
where
    Encode: Send + 'static,
    Decode: ProtoExt + Send + 'static,
    Mode: Send + Sync + 'static,
    ProtoEncoder<Encode, Mode>: EncoderExt<Encode, Mode>,
{
    type Encode = Encode;
    type Decode = Decode;
    type Encoder = ProtoEncoder<Encode, Mode>;
    type Decoder = ProtoDecoder<Decode>;

    fn encoder(&mut self) -> Self::Encoder {
        ProtoEncoder::default()
    }

    fn decoder(&mut self) -> Self::Decoder {
        ProtoDecoder::default()
    }
}

#[derive(Debug, Clone)]
pub struct ProtoEncoder<T, Mode> {
    _marker: core::marker::PhantomData<(T, Mode)>,
}

impl<T, Mode> Default for ProtoEncoder<T, Mode> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

pub trait EncoderExt<T, Mode> {
    fn encode_sun(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status>;
}

impl<T, Mode> EncoderExt<T, Mode> for ProtoEncoder<T, BytesMode>
where
    T: AsBytes,
{
    fn encode_sun(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        dst.put_slice(item.as_bytes());
        Ok(())
    }
}

// ----- Specialization via helper trait (disjoint impls) -----

// Case 1: Sun<'a> = T  (owned)
impl<T> EncoderExt<T, SunByVal> for ProtoEncoder<T, SunByVal>
where
    T: ProtoExt + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = T, OwnedSun = T>,
{
    fn encode_sun(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        T::encode(item, dst).map_err(|e| Status::internal(format!("encode failed: {e}")))
    }
}

// Case 2: Sun<'a> = &'a T (borrowed)
impl<T> EncoderExt<T, SunByRef> for ProtoEncoder<T, SunByRef>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, OwnedSun = T>,
{
    fn encode_sun(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        T::encode(&item, dst).map_err(|e| Status::internal(format!("encode failed: {e}")))
    }
}

// ----- Single blanket Encoder impl that delegates to the helper -----

impl<T, Mode> Encoder for ProtoEncoder<T, Mode>
where
    ProtoEncoder<T, Mode>: EncoderExt<T, Mode>,
{
    type Item = T;
    type Error = Status;

    #[inline]
    fn encode(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        <Self as EncoderExt<T, Mode>>::encode_sun(self, item, dst)
    }
}
#[derive(Debug, Clone)]
pub struct ProtoDecoder<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for ProtoDecoder<T> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<T> Decoder for ProtoDecoder<T>
where
    T: ProtoExt,
{
    type Item = T;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        // Always attempt to decode: tonic gives full frames, even empty ones
        match T::decode(src) {
            Ok(msg) => Ok(Some(msg)),
            Err(err) => Err(Status::data_loss(format!("failed to decode message: {err}"))),
        }
    }
}
