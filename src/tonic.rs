use tonic::Status;
use tonic::codec::Codec;
use tonic::codec::DecodeBuf;
use tonic::codec::Decoder;
use tonic::codec::EncodeBuf;
use tonic::codec::Encoder;
mod req;
mod resp;
use bytes::BufMut;
pub use req::ProtoRequest;
pub use req::ZeroCopyRequest;
pub use resp::ProtoResponse;
pub use resp::ZeroCopyResponse;

use crate::ProtoExt;
use crate::ProtoShadow;
use crate::coders::AsBytes;
use crate::coders::BytesMode;
use crate::coders::ProtoCodec;
use crate::coders::ProtoDecoder;
use crate::coders::ProtoEncoder;
use crate::coders::SunByRef;
use crate::coders::SunByVal;

pub trait ToZeroCopyResponse<T> {
    fn to_zero_copy(self) -> ZeroCopyResponse<T>;
}
pub trait ToZeroCopyRequest<T> {
    fn to_zero_copy(self) -> ZeroCopyRequest<T>;
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
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = T, OwnedSun = T>,
{
    fn encode_sun(&mut self, item: T, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        T::encode(item, dst).map_err(|e| Status::internal(format!("encode failed: {e}")))
    }
}

// Case 2: Sun<'a> = &'a T (borrowed)
impl<T> EncoderExt<T, SunByRef> for ProtoEncoder<T, SunByRef>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
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
