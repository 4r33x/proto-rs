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

#[derive(Debug, Clone)]
pub struct ProtoCodec<Encode = (), Decode = ()> {
    _marker: PhantomData<(Encode, Decode)>,
}

impl<Encode, Decode> Default for ProtoCodec<Encode, Decode> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<Encode, Decode> ProtoCodec<Encode, Decode> {
    pub fn new() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<Encode, Decode> Codec for ProtoCodec<Encode, Decode>
where
    Encode: AsBytes + Send + 'static,
    Decode: ProtoExt + Send + 'static,
{
    type Encode = Encode;
    type Decode = Decode;
    type Encoder = ProtoEncoderBytes<Encode>;
    type Decoder = ProtoDecoder<Decode>;

    fn encoder(&mut self) -> Self::Encoder {
        ProtoEncoderBytes::default()
    }

    fn decoder(&mut self) -> Self::Decoder {
        ProtoDecoder::default()
    }
}

#[derive(Debug, Clone)]
pub struct ProtoEncoder<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for ProtoEncoder<T> {
    fn default() -> Self {
        Self { _marker: PhantomData }
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

#[derive(Debug, Clone)]
pub struct ProtoEncoderBytes<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for ProtoEncoderBytes<T> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<T> Encoder for ProtoEncoderBytes<T>
where
    T: AsBytes,
{
    type Item = T;
    type Error = tonic::Status;

    fn encode(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        dst.put_slice(item.as_bytes());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProtoEncoderMsg<T, Mode> {
    _marker: core::marker::PhantomData<(T, Mode)>,
}

impl<T, N> Default for ProtoEncoderMsg<T, N> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

pub struct SunByVal {} // Sun<'a> = T
pub struct SunByRef {} // Sun<'a> = &'a T

pub trait EncoderExt<T, Mode> {
    fn encode_sun(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status>;
}

// ----- Specialization via helper trait (disjoint impls) -----

// Case 1: Sun<'a> = T  (owned)
impl<T> EncoderExt<T, SunByVal> for ProtoEncoderMsg<T, SunByVal>
where
    T: ProtoExt + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<'a, Sun<'a> = T, OwnedSun = T>,
{
    fn encode_sun(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        T::encode(item, dst).map_err(|e| Status::internal(format!("encode failed: {e}")))
    }
}

// Case 2: Sun<'a> = &'a T (borrowed)
impl<T> EncoderExt<T, SunByRef> for ProtoEncoderMsg<T, SunByRef>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<'a, Sun<'a> = &'a T, OwnedSun = T>,
{
    fn encode_sun(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        T::encode(&item, dst).map_err(|e| Status::internal(format!("encode failed: {e}")))
    }
}

// ----- Single blanket Encoder impl that delegates to the helper -----

impl<T, Mode> Encoder for ProtoEncoderMsg<T, Mode>
where
    T: ProtoExt,
    // Pick the unique Mode whose helper impl is satisfied for this T.
    ProtoEncoderMsg<T, Mode>: EncoderExt<T, Mode>,
{
    type Item = T; // take owned input from tonic; borrow or move internally per Mode
    type Error = Status;

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
