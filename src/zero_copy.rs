use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;
use core::ops::DerefMut;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::ProtoKind;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::check_wire_type;
use crate::encoding::key_len;
//const ZERO_COPY_SIZE: usize = 64;

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
    pub fn new() -> Self {
        Self {
            inner: ZeroCopyBufferInner::new(),
        }
    }

    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut ZeroCopyBufferInner {
        &mut self.inner
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct ZeroCopy<T> {
    inner: ZeroCopyBuffer,
    _marker: PhantomData<T>,
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
    pub fn new() -> Self {
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
        T: ProtoExt,
        for<'a> T::Shadow<'a>: ProtoShadow<T, OwnedSun = T>,
    {
        decode_zero_copy_bytes::<T>(self.as_bytes())
    }
}

fn decode_zero_copy_bytes<T>(mut buf: impl Buf) -> Result<T, DecodeError>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, OwnedSun = T>,
{
    if !buf.has_remaining() {
        let shadow = <T::Shadow<'static> as ProtoWire>::proto_default();
        return <T::Shadow<'static> as ProtoShadow<T>>::to_sun(shadow);
    }

    if let ProtoKind::Message = <T::Shadow<'static> as ProtoWire>::KIND {
        // For messages, the buffer contains raw field data - use T::decode to loop through fields
        T::decode(buf)
    } else {
        // For all other types (SimpleEnum, primitives, strings, bytes),
        // decode the raw value with its wire type
        let mut shadow = <T::Shadow<'static> as ProtoWire>::proto_default();
        <T::Shadow<'static> as ProtoWire>::decode_into(
            <T::Shadow<'static> as ProtoWire>::WIRE_TYPE,
            &mut shadow,
            &mut buf,
            DecodeContext::default(),
        )?;
        <T::Shadow<'static> as ProtoShadow<T>>::to_sun(shadow)
    }
}

impl<T> From<&T> for ZeroCopy<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    #[inline(always)]
    fn from(value: &T) -> Self {
        let bytes = T::with_shadow(value, |shadow| {
            let archive = <T::Shadow<'_> as ProtoShadow<T>>::to_archive(shadow);
            let archive_input =
                <<T::Shadow<'_> as ProtoShadow<T>>::ProtoArchive as crate::EncodeInputFromRef>::encode_input_from_ref(&archive);
            let len = <<T::Shadow<'_> as ProtoShadow<T>>::ProtoArchive as ProtoWire>::encoded_len_impl(&archive_input);
            if len == 0 {
                return ZeroCopyBuffer::new();
            }
            let mut buf = ZeroCopyBuffer::with_capacity(len);
            <<T::Shadow<'_> as ProtoShadow<T>>::ProtoArchive as ProtoWire>::encode_raw_unchecked(archive_input, buf.inner_mut());
            buf
        });
        Self {
            inner: bytes,
            _marker: PhantomData,
        }
    }
}

impl<T> From<T> for ZeroCopy<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = T, OwnedSun = T>,
{
    #[inline(always)]
    fn from(value: T) -> Self {
        let bytes = T::with_shadow(value, |shadow| {
            let archive = <T::Shadow<'_> as ProtoShadow<T>>::to_archive(shadow);
            let archive_input =
                <<T::Shadow<'_> as ProtoShadow<T>>::ProtoArchive as crate::EncodeInputFromRef>::encode_input_from_ref(&archive);
            let len = <<T::Shadow<'_> as ProtoShadow<T>>::ProtoArchive as ProtoWire>::encoded_len_impl(&archive_input);
            if len == 0 {
                return ZeroCopyBuffer::new();
            }
            let mut buf = ZeroCopyBuffer::with_capacity(len);
            <<T::Shadow<'_> as ProtoShadow<T>>::ProtoArchive as ProtoWire>::encode_raw_unchecked(archive_input, buf.inner_mut());
            buf
        });
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
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    fn to_zero_copy(self) -> ZeroCopy<T> {
        ZeroCopy::from(self)
    }
}

impl<T> ToZeroCopy<T> for T
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = T, OwnedSun = T>,
{
    fn to_zero_copy(self) -> ZeroCopy<T> {
        ZeroCopy::from(self)
    }
}

impl<T: 'static> ProtoShadow<ZeroCopy<T>> for ZeroCopy<T> {
    type Sun<'a> = &'a ZeroCopy<T>;
    type OwnedSun = ZeroCopy<T>;
    type View<'a> = &'a ZeroCopy<T>;
    type ProtoArchive = ZeroCopy<T>;

    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value
    }

    fn to_archive(value: Self::View<'_>) -> Self::ProtoArchive {
        value.clone()
    }
}

impl<T> ProtoWire for ZeroCopy<T>
where
    T: ProtoWire + 'static,
{
    type EncodeInput<'a> = &'a ZeroCopy<T>;
    const KIND: ProtoKind = T::KIND;
    const WIRE_TYPE: WireType = T::WIRE_TYPE;

    fn proto_default() -> Self {
        Self::new()
    }

    fn clear(&mut self) {
        self.inner.clear();
    }

    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        value.inner.is_empty()
    }

    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        value.inner.len()
    }

    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
        value.inner.len()
    }

    fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
        if value.inner.is_empty() {
            0
        } else {
            let payload_len = value.inner.len();
            key_len(tag) + payload_len
        }
    }

    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        buf.put_slice(&value.inner);
    }

    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        check_wire_type(Self::WIRE_TYPE, wire_type)?;

        copy_value_payload(wire_type, buf, &mut value.inner, ctx)
    }
}

impl<T> ProtoExt for ZeroCopy<T>
where
    T: ProtoWire + 'static,
{
    type Shadow<'b> = ZeroCopy<T>;

    fn merge_field(
        value: &mut Self::Shadow<'_>,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        append_field(tag, wire_type, buf, &mut value.inner, ctx)
    }
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

    // Empty buffer
    if bytes.is_empty() {
        return Err(DecodeError::new("buffer exhausted"));
    }

    // decode_varint_slice handles all cases correctly
    decode_varint_slice(bytes)
}

/// Copy payload bytes directly without double-scanning
#[inline]
fn copy_value_payload(wire_type: WireType, buf: &mut impl Buf, into: &mut ZeroCopyBuffer, ctx: DecodeContext) -> Result<(), DecodeError> {
    ctx.limit_reached()?;
    into.clear();

    match wire_type {
        WireType::Varint => {
            // Direct slice access - single scan, no loops
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

            // Skip the length prefix, only copy the payload
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

/// Append full field (key + payload) with minimized scanning
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
            // Reserve space for key + max varint size
            let key_len = key_len(tag);
            out.reserve(key_len + 10);
            push_key(out, tag, wire_type);

            // Direct slice access - no byte-by-byte loop
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
            // Peek length prefix
            let (len_value, len_len) = peek_varint_prefix(buf)?;
            let payload_len = len_value as usize;

            if buf.remaining() < len_len + payload_len {
                return Err(DecodeError::new("buffer underflow"));
            }

            // Reserve and copy key + length + payload
            let key_len = key_len(tag);
            out.reserve(key_len + len_len + payload_len);
            push_key(out, tag, wire_type);

            // Copy length prefix and payload
            let total_len = len_len + payload_len;
            let start = out.len();
            out.resize(start + total_len, 0);
            buf.copy_to_slice(&mut out[start..start + total_len]);
            Ok(())
        }
        WireType::StartGroup | WireType::EndGroup => Err(DecodeError::new("groups are not supported for ZeroCopy")),
    }
}
