use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;
use core::ops::DerefMut;

use bytes::Buf;
use bytes::BufMut;

use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::check_wire_type;
use crate::encoding::key_len;
use crate::error::DecodeError;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;

pub type ZeroCopyBufferInner = Vec<u8>;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ZeroCopyBuffer {
    inner: ZeroCopyBufferInner,
}

impl Deref for ZeroCopyBuffer {
    type Target = ZeroCopyBufferInner;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for ZeroCopyBuffer {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl ZeroCopyBuffer {
    #[inline(always)]
    pub fn with_capacity(len: usize) -> Self {
        Self {
            inner: ZeroCopyBufferInner::with_capacity(len),
        }
    }

    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            inner: ZeroCopyBufferInner::new(),
        }
    }

    #[inline(always)]
    pub const fn inner_mut(&mut self) -> &mut ZeroCopyBufferInner {
        &mut self.inner
    }
}

unsafe impl BufMut for ZeroCopyBuffer {
    #[inline(always)]
    fn remaining_mut(&self) -> usize {
        BufMut::remaining_mut(&self.inner)
    }

    #[inline(always)]
    unsafe fn advance_mut(&mut self, cnt: usize) {
        unsafe {
            BufMut::advance_mut(&mut self.inner, cnt);
        }
    }

    #[inline(always)]
    fn chunk_mut(&mut self) -> &mut bytes::buf::UninitSlice {
        BufMut::chunk_mut(&mut self.inner)
    }
}

#[derive(PartialEq, Eq)]
pub struct ZeroCopy<T> {
    inner: ZeroCopyBuffer,
    _marker: PhantomData<T>,
}

impl<T> Clone for ZeroCopy<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> Default for ZeroCopy<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Debug for ZeroCopy<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ZeroCopy").field("len", &self.inner.len()).finish()
    }
}

impl<T> ZeroCopy<T> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            inner: ZeroCopyBuffer::new(),
            _marker: PhantomData,
        }
    }
    #[inline(always)]
    pub fn into_buffer(self) -> ZeroCopyBuffer {
        self.inner
    }
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    #[inline(always)]
    pub fn decode(&self) -> Result<T, DecodeError>
    where
        T: ProtoDecode + ProtoExt,
        T::ShadowDecoded: ProtoDecoder + ProtoExt + ProtoShadowDecode<T>,
    {
        decode_zero_copy_bytes::<T>(self.as_bytes())
    }
}

fn decode_zero_copy_bytes<T>(mut buf: impl Buf) -> Result<T, DecodeError>
where
    T: ProtoDecode + ProtoExt,
    T::ShadowDecoded: ProtoDecoder + ProtoExt + ProtoShadowDecode<T>,
{
    if !buf.has_remaining() {
        let shadow = <T::ShadowDecoded as ProtoDecoder>::proto_default();
        return <T::ShadowDecoded as ProtoShadowDecode<T>>::to_sun(shadow);
    }

    // Messages and SimpleEnums are encoded with field tags and need full decode
    if matches!(T::KIND, ProtoKind::Message | ProtoKind::SimpleEnum) {
        T::decode(buf, DecodeContext::default())
    } else {
        // Primitives and other types are stored as raw payloads
        let mut shadow = <T::ShadowDecoded as ProtoDecoder>::proto_default();
        <T::ShadowDecoded as ProtoDecoder>::merge(&mut shadow, T::WIRE_TYPE, &mut buf, DecodeContext::default())?;
        <T::ShadowDecoded as ProtoShadowDecode<T>>::to_sun(shadow)
    }
}

impl<T> From<&T> for ZeroCopy<T>
where
    T: ProtoEncode + ProtoExt,
{
    #[inline(always)]
    fn from(value: &T) -> Self {
        let bytes = value.encode_to_zerocopy();
        Self {
            inner: bytes,
            _marker: PhantomData,
        }
    }
}

impl<T> From<T> for ZeroCopy<T>
where
    T: ProtoEncode + ProtoExt,
{
    #[inline(always)]
    fn from(value: T) -> Self {
        let bytes = value.encode_to_zerocopy();
        Self {
            inner: bytes,
            _marker: PhantomData,
        }
    }
}

pub trait ToZeroCopy<T> {
    fn to_zero_copy(self) -> ZeroCopy<T>;
}

impl<T> ToZeroCopy<T> for &T
where
    T: ProtoEncode + ProtoExt,
{
    fn to_zero_copy(self) -> ZeroCopy<T> {
        ZeroCopy::from(self)
    }
}

impl<T> ToZeroCopy<T> for T
where
    T: ProtoEncode + ProtoExt,
{
    fn to_zero_copy(self) -> ZeroCopy<T> {
        ZeroCopy::from(self)
    }
}

impl<T> ProtoExt for ZeroCopy<T>
where
    T: ProtoExt,
{
    const KIND: ProtoKind = T::KIND;
    // ZeroCopy always stores pre-encoded bytes that need length-delimited encoding,
    // regardless of the underlying type's wire type
    const WIRE_TYPE: crate::encoding::WireType = crate::encoding::WireType::LengthDelimited;
}

impl<T> ProtoDecoder for ZeroCopy<T>
where
    T: ProtoExt,
{
    #[inline(always)]
    fn proto_default() -> Self {
        Self::new()
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.inner.clear();
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if let ProtoKind::Message = T::KIND {
            append_field(tag, wire_type, buf, &mut value.inner, ctx)
        } else if tag == 1 {
            copy_value_payload(wire_type, buf, &mut value.inner, ctx)
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if let ProtoKind::Message = T::KIND {
            check_wire_type(WireType::LengthDelimited, wire_type)?;
        }
        copy_value_payload(wire_type, buf, &mut self.inner, ctx)
    }
}

impl<T> ProtoDecode for ZeroCopy<T>
where
    T: ProtoExt,
{
    type ShadowDecoded = ZeroCopy<T>;
}

impl<T> ProtoShadowDecode<ZeroCopy<T>> for ZeroCopy<T> {
    #[inline(always)]
    fn to_sun(self) -> Result<ZeroCopy<T>, DecodeError> {
        Ok(self)
    }
}

impl<'a, T> ProtoShadowEncode<'a, ZeroCopy<T>> for ZeroCopy<T> {
    #[inline(always)]
    fn from_sun(value: &'a ZeroCopy<T>) -> Self {
        value.clone()
    }
}

impl<T> ProtoArchive for ZeroCopy<T> {
    type Archived<'a> = &'a [u8];

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        archived.len()
    }

    #[inline(always)]
    unsafe fn encode<const TAG: u32>(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        buf.put_slice(archived);
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self) -> Self::Archived<'_> {
        self.inner.as_slice()
    }
}

impl<T> ProtoEncode for ZeroCopy<T>
where
    T: ProtoExt,
{
    type Shadow<'a> = ZeroCopy<T>;
}

/// Decodes varint from a contiguous slice. Returns (value, bytes_consumed).
#[inline]
fn decode_varint_slice(bytes: &[u8]) -> Result<(u64, usize), DecodeError> {
    let mut value: u64 = 0;
    let mut shift = 0;

    for (i, &b) in bytes.iter().enumerate().take(10) {
        value |= ((b & 0x7F) as u64) << shift;
        if b < 0x80 {
            return Ok((value, i + 1));
        }
        shift += 7;
    }

    Err(DecodeError::new("invalid or unterminated varint"))
}

#[inline]
fn push_key(out: &mut ZeroCopyBuffer, tag: u32, wire_type: WireType) {
    let mut key = ((tag as u64) << 3) | u64::from(wire_type as u32);
    loop {
        let mut byte = (key & 0x7F) as u8;
        key >>= 7;
        if key != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if key == 0 {
            break;
        }
    }
}

/// Peek varint for length prefix ONLY. Since Buf is contiguous, this is safe.
#[inline]
fn peek_varint_prefix(buf: &impl Buf) -> Result<(u64, usize), DecodeError> {
    let bytes = buf.chunk();

    if bytes.is_empty() {
        return Err(DecodeError::new("buffer exhausted"));
    }

    decode_varint_slice(bytes)
}

#[inline]
fn copy_value_payload(wire_type: WireType, buf: &mut impl Buf, into: &mut ZeroCopyBuffer, ctx: DecodeContext) -> Result<(), DecodeError> {
    ctx.limit_reached()?;
    into.clear();

    match wire_type {
        WireType::Varint => {
            let bytes = buf.chunk();
            let (_, len) = decode_varint_slice(bytes)?;

            if buf.remaining() < len {
                return Err(DecodeError::new("buffer underflow"));
            }

            into.resize(len, 0);
            buf.copy_to_slice(&mut into[..]);
            Ok(())
        }
        WireType::ThirtyTwoBit => {
            const SIZE: usize = 4;
            if buf.remaining() < SIZE {
                return Err(DecodeError::new("buffer underflow"));
            }
            into.resize(SIZE, 0);
            buf.copy_to_slice(&mut into[..]);
            Ok(())
        }
        WireType::SixtyFourBit => {
            const SIZE: usize = 8;
            if buf.remaining() < SIZE {
                return Err(DecodeError::new("buffer underflow"));
            }
            into.resize(SIZE, 0);
            buf.copy_to_slice(&mut into[..]);
            Ok(())
        }
        WireType::LengthDelimited => {
            let (len_value, len_len) = peek_varint_prefix(buf)?;
            let payload_len = len_value as usize;

            if buf.remaining() < len_len + payload_len {
                return Err(DecodeError::new("buffer underflow"));
            }

            buf.advance(len_len);
            into.resize(payload_len, 0);
            if payload_len > 0 {
                buf.copy_to_slice(&mut into[..]);
            }
            Ok(())
        }
        WireType::StartGroup | WireType::EndGroup => Err(DecodeError::new("groups are not supported for ZeroCopy")),
    }
}

#[inline]
fn append_field(
    tag: u32,
    wire_type: WireType,
    buf: &mut impl Buf,
    out: &mut ZeroCopyBuffer,
    ctx: DecodeContext,
) -> Result<(), DecodeError> {
    ctx.limit_reached()?;

    match wire_type {
        WireType::Varint => {
            let key_len = key_len(tag);
            out.reserve(key_len + 10);
            push_key(out, tag, wire_type);

            let bytes = buf.chunk();
            let (_, varint_len) = decode_varint_slice(bytes)?;

            if buf.remaining() < varint_len {
                return Err(DecodeError::new("buffer underflow"));
            }

            let start = out.len();
            out.resize(start + varint_len, 0);
            buf.copy_to_slice(&mut out[start..]);
            Ok(())
        }
        WireType::ThirtyTwoBit => {
            const SIZE: usize = 4;
            if buf.remaining() < SIZE {
                return Err(DecodeError::new("buffer underflow"));
            }

            let key_len = key_len(tag);
            out.reserve(key_len + SIZE);
            push_key(out, tag, wire_type);

            let start = out.len();
            out.resize(start + SIZE, 0);
            buf.copy_to_slice(&mut out[start..start + SIZE]);
            Ok(())
        }
        WireType::SixtyFourBit => {
            const SIZE: usize = 8;
            if buf.remaining() < SIZE {
                return Err(DecodeError::new("buffer underflow"));
            }

            let key_len = key_len(tag);
            out.reserve(key_len + SIZE);
            push_key(out, tag, wire_type);

            let start = out.len();
            out.resize(start + SIZE, 0);
            buf.copy_to_slice(&mut out[start..start + SIZE]);
            Ok(())
        }
        WireType::LengthDelimited => {
            let (len_value, len_len) = peek_varint_prefix(buf)?;
            let payload_len = len_value as usize;

            if buf.remaining() < len_len + payload_len {
                return Err(DecodeError::new("buffer underflow"));
            }

            let key_len = key_len(tag);
            out.reserve(key_len + len_len + payload_len);
            push_key(out, tag, wire_type);

            let total_len = len_len + payload_len;
            let start = out.len();
            out.resize(start + total_len, 0);
            buf.copy_to_slice(&mut out[start..start + total_len]);
            Ok(())
        }
        WireType::StartGroup | WireType::EndGroup => Err(DecodeError::new("groups are not supported for ZeroCopy")),
    }
}
