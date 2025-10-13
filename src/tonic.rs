use core::marker::PhantomData;

use bytes::Buf;
use tonic::Status;
use tonic::codec::Codec;
use tonic::codec::DecodeBuf;
use tonic::codec::Decoder;
use tonic::codec::EncodeBuf;
use tonic::codec::Encoder;

use crate::ProtoExt;

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
    Encode: ProtoExt + Send + 'static,
    Decode: ProtoExt + Send + 'static,
{
    type Encode = Encode;
    type Decode = Decode;
    type Encoder = ProtoEncoder<Encode>;
    type Decoder = ProtoDecoder<Decode>;

    fn encoder(&mut self) -> Self::Encoder {
        ProtoEncoder { _marker: PhantomData }
    }

    fn decoder(&mut self) -> Self::Decoder {
        ProtoDecoder { _marker: PhantomData }
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

impl<T> Encoder for ProtoEncoder<T>
where
    T: ProtoExt,
{
    type Item = T;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        item.encode(dst).map_err(|err| Status::internal(format!("failed to encode message: {err}")))
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
        let bytes = src.copy_to_bytes(src.remaining());
        T::decode(bytes).map(Some).map_err(|err| Status::internal(format!("failed to decode message: {err}")))
    }
}
