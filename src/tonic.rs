use core::marker::PhantomData;

use bytes::BufMut;
use tonic::Status;
use tonic::codec::Codec;
use tonic::codec::DecodeBuf;
use tonic::codec::Decoder;
use tonic::codec::EncodeBuf;
use tonic::codec::Encoder;

use crate::ProtoExt;
use crate::traits::ProtoShadow;

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
