use core::marker::PhantomData;
use std::io::Cursor;

use bytes::Buf;
use bytes::Bytes;
use bytes::BytesMut;
use tonic::Status;
use tonic::codec::Codec;
use tonic::codec::Decoder;
use tonic::codec::Encoder;

use crate::ProtoExt;

/// A [`tonic::codec::Codec`] implementation that works directly with [`ProtoExt`] types.
///
/// This codec avoids any intermediate prost conversions by encoding and decoding
/// the provided Rust types using their `ProtoExt` implementations.
#[derive(Debug, Clone, Default)]
pub struct ProtoCodec<Encode = (), Decode = ()> {
    _marker: PhantomData<(Encode, Decode)>,
}

impl<Encode, Decode> ProtoCodec<Encode, Decode> {
    /// Creates a new codec instance.
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
        ProtoEncoder::default()
    }

    fn decoder(&mut self) -> Self::Decoder {
        ProtoDecoder::default()
    }
}

/// Encoder for `ProtoExt` messages that produces length-delimited frames.
#[derive(Debug, Clone, Default)]
pub struct ProtoEncoder<T> {
    _marker: PhantomData<T>,
}

impl<T> Encoder for ProtoEncoder<T>
where
    T: ProtoExt,
{
    type Item = T;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let len = item.encoded_len();
        let delimiter = crate::encoding::length_delimiter::length_delimiter_len(len);
        dst.reserve(len + delimiter);

        item.encode_length_delimited(dst).map_err(|err| Status::internal(format!("failed to encode message: {err}")))
    }
}

/// Decoder for `ProtoExt` messages that expects length-delimited frames.
#[derive(Debug, Clone, Default)]
pub struct ProtoDecoder<T> {
    _marker: PhantomData<T>,
}

impl<T> Decoder for ProtoDecoder<T>
where
    T: ProtoExt,
{
    type Item = T;
    type Error = Status;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let mut cursor = Cursor::new(&src[..]);
        let len = match crate::encoding::length_delimiter::decode_length_delimiter(&mut cursor) {
            Ok(len) => len,
            Err(err) => {
                if src.len() < 10 {
                    // More data may be required to finish decoding the delimiter.
                    return Ok(None);
                }

                return Err(Status::internal(format!("failed to decode length delimiter: {err}")));
            }
        };

        let delimiter_len = cursor.position() as usize;
        if src.len() < delimiter_len + len {
            return Ok(None);
        }

        src.advance(delimiter_len);
        let data = src.split_to(len);
        let bytes: Bytes = data.freeze();

        T::decode(bytes).map(Some).map_err(|err| Status::internal(format!("failed to decode message: {err}")))
    }
}
