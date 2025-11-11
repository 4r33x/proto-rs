use core::marker::PhantomData;

use bytes::Buf;
use bytes::BufMut;
use tonic::codegen::http::Extensions;
use tonic::metadata::MetadataMap;

use crate::DecodeError;
use crate::EncodeError;
use crate::ProtoExt;
use crate::ProtoKind;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;

/// Zero-copy wrapper around an encoded protobuf message.
#[derive(Debug)]
pub struct ZeroCopy<T> {
    pub(crate) inner: Vec<u8>,
    pub(crate) metadata: MetadataMap,
    pub(crate) extensions: Extensions,
    pub(crate) _marker: PhantomData<fn() -> T>,
}

impl<T> Default for ZeroCopy<T> {
    fn default() -> Self {
        Self {
            inner: Vec::new(),
            metadata: MetadataMap::new(),
            extensions: Extensions::new(),
            _marker: PhantomData,
        }
    }
}

impl<T> ZeroCopy<T> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn from_parts(metadata: MetadataMap, extensions: Extensions, bytes: Vec<u8>) -> Self {
        Self {
            inner: bytes,
            metadata,
            extensions,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from_parts(MetadataMap::new(), Extensions::new(), bytes)
    }

    #[inline]
    pub fn into_parts(self) -> (MetadataMap, Extensions, Vec<u8>) {
        (self.metadata, self.extensions, self.inner)
    }

    #[inline]
    pub fn bytes(&self) -> &[u8] {
        &self.inner
    }

    #[inline]
    pub fn into_bytes(self) -> Vec<u8> {
        self.inner
    }

    #[inline]
    pub fn metadata(&self) -> &MetadataMap {
        &self.metadata
    }

    #[inline]
    pub fn metadata_mut(&mut self) -> &mut MetadataMap {
        &mut self.metadata
    }

    #[inline]
    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    #[inline]
    pub(crate) fn reset_metadata(&mut self) {
        self.metadata = MetadataMap::new();
        self.extensions = Extensions::new();
    }
}

impl<T> ZeroCopy<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    #[inline]
    pub fn from_message(message: T) -> Self {
        Self::from_bytes(T::encode_to_vec(&message))
    }

    #[inline]
    pub fn from_borrowed(message: &T) -> Self {
        Self::from_bytes(T::encode_to_vec(message))
    }
}

impl<T> ProtoShadow<ZeroCopy<T>> for ZeroCopy<T>
where
    T: 'static,
{
    type Sun<'a> = ZeroCopy<T>;
    type OwnedSun = ZeroCopy<T>;
    type View<'a> = ZeroCopy<T>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value
    }
}

impl<T> ProtoWire for ZeroCopy<T>
where
    T: 'static,
{
    type EncodeInput<'b> = ZeroCopy<T>;

    const KIND: ProtoKind = ProtoKind::Message;

    #[inline]
    fn proto_default() -> Self {
        Self::default()
    }

    #[inline]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        value.inner.is_empty()
    }

    #[inline]
    fn clear(&mut self) {
        self.inner.clear();
        self.reset_metadata();
    }

    #[inline]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        value.inner.len()
    }

    #[inline]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        buf.put_slice(&value.inner);
    }

    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        encoding::check_wire_type(WireType::LengthDelimited, wire_type)?;
        ctx.limit_reached()?;
        let len = encoding::decode_varint(buf)?;
        if len > buf.remaining() as u64 {
            return Err(DecodeError::new("buffer underflow"));
        }
        let len = len as usize;
        value.inner.clear();
        value.inner.extend_from_slice(buf.copy_to_bytes(len).as_ref());
        value.reset_metadata();
        Ok(())
    }
}

impl<T> ProtoExt for ZeroCopy<T>
where
    T: 'static,
{
    type Shadow<'b> = ZeroCopy<T>;

    #[inline]
    fn merge_field(_value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        encoding::skip_field(wire_type, tag, buf, ctx)
    }

    #[inline]
    fn encode(value: ZeroCopy<T>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        let len = value.inner.len();
        let remaining = buf.remaining_mut();
        if len > remaining {
            return Err(EncodeError::new(len, remaining));
        }
        buf.put_slice(&value.inner);
        Ok(())
    }

    #[inline]
    fn encode_to_vec(value: ZeroCopy<T>) -> Vec<u8> {
        value.inner
    }

    #[inline]
    fn decode(mut buf: impl Buf) -> Result<Self, DecodeError> {
        let bytes = buf.copy_to_bytes(buf.remaining());
        Ok(Self::from_bytes(bytes.to_vec()))
    }

    #[inline]
    fn decode_length_delimited(mut buf: impl Buf, ctx: DecodeContext) -> Result<Self, DecodeError> {
        ctx.limit_reached()?;
        let bytes = buf.copy_to_bytes(buf.remaining());
        Ok(Self::from_bytes(bytes.to_vec()))
    }
}
