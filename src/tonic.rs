use tonic::Status;
use tonic::codec::Codec;
use tonic::codec::DecodeBuf;
use tonic::codec::Decoder;
use tonic::codec::EncodeBuf;
use tonic::codec::Encoder;
mod prepared;
mod req;
mod resp;
use core::ops::Deref;

use bytes::BufMut;
pub use prepared::PreparedMessage;
pub use prepared::PreparedRpc;
pub use req::PrepareRequest;
pub use resp::map_proto_stream_result;

use crate::ProtoDecode;
use crate::ProtoEncode;
use crate::ProtoExt;
use crate::coders::AsBytes;
use crate::coders::BytesMode;
use crate::coders::ProtoCodec;
use crate::coders::ProtoDecoder;
use crate::coders::ProtoEncoder;
use crate::coders::SunByRef;
use crate::coders::SunByRefDeref;
use crate::encoding::DecodeContext;

impl<Encode, Decode, Mode> Codec for ProtoCodec<Encode, Decode, Mode>
where
    Encode: Send + 'static,
    Decode: ProtoDecode + Send + 'static,
    Mode: Send + Sync + 'static,
    ProtoEncoder<Encode, Mode>: EncoderExt<Encode, Mode>,
{
    type Encode = Encode;
    type Decode = Decode;
    type Encoder = ProtoEncoder<Encode, Mode>;
    type Decoder = ProtoDecoder<Decode>;

    fn encoder(&mut self) -> Self::Encoder {
        ProtoEncoder::default().with_max_encode_preallocation(self.max_encode_preallocation)
    }

    fn decoder(&mut self) -> Self::Decoder {
        ProtoDecoder::default()
    }
}

pub trait EncoderExt<T, Mode> {
    #[cfg(feature = "tonic-owned")]
    fn supports_owned_batch_sun(&self) -> bool {
        false
    }
    #[cfg(feature = "tonic-owned")]
    fn encode_owned_batch_sun(
        &mut self,
        _first: T,
        _next: &mut dyn FnMut() -> Option<T>,
        _threshold: usize,
    ) -> Result<bytes::Bytes, Status> {
        Err(Status::internal("owned batching is not supported"))
    }
    #[inline]
    #[cfg(feature = "tonic-owned")]
    fn supports_owned_sun(&self) -> bool {
        false
    }
    fn encode_sun(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status>;

    #[cfg(feature = "tonic-owned")]
    fn encode_owned_sun(&mut self, item: T) -> Result<tonic::codec::EncodeResult<T>, Status> {
        Ok(tonic::codec::EncodeResult::Buffered(item))
    }
}

impl<T> EncoderExt<T, BytesMode> for ProtoEncoder<T, BytesMode>
where
    T: AsBytes,
{
    #[inline]
    #[cfg(feature = "tonic-owned")]
    fn supports_owned_sun(&self) -> bool {
        true
    }
    fn encode_sun(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        dst.put_slice(item.as_bytes());
        Ok(())
    }

    #[cfg(feature = "tonic-owned")]
    fn encode_owned_sun(&mut self, item: T) -> Result<tonic::codec::EncodeResult<T>, Status> {
        item.into_owned_message()
    }
}

impl<T> EncoderExt<T, SunByRef> for ProtoEncoder<T, SunByRef>
where
    T: ProtoEncode + ProtoExt,
{
    #[cfg(feature = "tonic-owned")]
    fn supports_owned_batch_sun(&self) -> bool {
        true
    }
    #[cfg(feature = "tonic-owned")]
    fn encode_owned_batch_sun(&mut self, first: T, next: &mut dyn FnMut() -> Option<T>, threshold: usize) -> Result<bytes::Bytes, Status> {
        self.prepare_batch(&first, next, threshold, |item| item)
    }
    #[cfg(feature = "tonic-owned")]
    fn supports_owned_sun(&self) -> bool {
        true
    }
    #[cfg(feature = "tonic-owned")]
    fn encode_owned_sun(&mut self, item: T) -> Result<tonic::codec::EncodeResult<T>, Status> {
        self.prepare(&item).map(tonic::codec::EncodeResult::Owned)
    }
    fn encode_sun(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        crate::traits::encode_into(&item, self.max_encode_preallocation, dst)
    }
}

impl<T, P> EncoderExt<P, SunByRefDeref> for ProtoEncoder<P, SunByRefDeref>
where
    T: ProtoEncode + ProtoExt,
    P: Deref<Target = T>,
{
    #[cfg(feature = "tonic-owned")]
    fn supports_owned_batch_sun(&self) -> bool {
        true
    }
    #[cfg(feature = "tonic-owned")]
    fn encode_owned_batch_sun(&mut self, first: P, next: &mut dyn FnMut() -> Option<P>, threshold: usize) -> Result<bytes::Bytes, Status> {
        self.prepare_batch(&first, next, threshold, Deref::deref)
    }
    #[cfg(feature = "tonic-owned")]
    fn supports_owned_sun(&self) -> bool {
        true
    }
    #[cfg(feature = "tonic-owned")]
    fn encode_owned_sun(&mut self, item: P) -> Result<tonic::codec::EncodeResult<P>, Status> {
        self.prepare(&*item).map(tonic::codec::EncodeResult::Owned)
    }
    fn encode_sun(&mut self, item: P, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        crate::traits::encode_into(&*item, self.max_encode_preallocation, dst)
    }
}

#[cfg(feature = "tonic-owned")]
impl<T, Mode> ProtoEncoder<T, Mode> {
    fn prepare_batch<P: ProtoEncode + ProtoExt>(
        &mut self,
        first: &T,
        next: &mut dyn FnMut() -> Option<T>,
        threshold: usize,
        value: impl Fn(&T) -> &P,
    ) -> Result<bytes::Bytes, Status> {
        crate::traits::encode_batch(first, next, &mut self.batch_items, value, threshold, self.max_encode_preallocation)
    }
    fn prepare<P: ProtoEncode + ProtoExt>(&mut self, value: &P) -> Result<crate::PreparedMessage, Status> {
        crate::traits::prepare_owned(value, self.max_encode_preallocation)
    }
}

impl AsBytes for crate::PreparedMessage {
    fn as_bytes(&self) -> &[u8] {
        self.payload()
    }
    #[cfg(feature = "tonic-owned")]
    fn into_owned_message(self) -> Result<tonic::codec::EncodeResult<Self>, Status> {
        Ok(tonic::codec::EncodeResult::Owned(self))
    }
}

impl<T, Mode> Encoder for ProtoEncoder<T, Mode>
where
    ProtoEncoder<T, Mode>: EncoderExt<T, Mode>,
{
    #[cfg(feature = "tonic-owned")]
    fn supports_owned_batch(&self) -> bool {
        <Self as EncoderExt<T, Mode>>::supports_owned_batch_sun(self)
    }
    #[cfg(feature = "tonic-owned")]
    fn encode_owned_batch(&mut self, first: T, next: &mut dyn FnMut() -> Option<T>, threshold: usize) -> Result<bytes::Bytes, Status> {
        <Self as EncoderExt<T, Mode>>::encode_owned_batch_sun(self, first, next, threshold)
    }
    #[inline]
    #[cfg(feature = "tonic-owned")]
    fn supports_owned(&self) -> bool {
        <Self as EncoderExt<T, Mode>>::supports_owned_sun(self)
    }
    type Item = T;
    type Error = Status;

    #[inline]
    fn encode(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        <Self as EncoderExt<T, Mode>>::encode_sun(self, item, dst)
    }

    #[inline]
    #[cfg(feature = "tonic-owned")]
    fn encode_owned(&mut self, item: T) -> Result<tonic::codec::EncodeResult<T>, Status> {
        <Self as EncoderExt<T, Mode>>::encode_owned_sun(self, item)
    }
}

impl<T> Decoder for ProtoDecoder<T>
where
    T: ProtoDecode,
{
    type Item = T;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        match T::decode(src, DecodeContext::default()) {
            Ok(msg) => Ok(Some(msg)),
            Err(err) => Err(Status::data_loss(format!("failed to decode message: {err}"))),
        }
    }
}
