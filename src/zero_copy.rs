use core::fmt;
use core::marker::PhantomData;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::ProtoKind;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::alloc::vec::Vec;
use crate::bytes::Bytes;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::check_wire_type;
use crate::encoding::encode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
use crate::encoding::key_len;

#[derive(PartialEq, Eq)]
pub struct ZeroCopy<T> {
    inner: Vec<u8>,
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
    pub const fn new() -> Self {
        Self {
            inner: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { inner: bytes, _marker: PhantomData }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.inner
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn decode(&self) -> Result<T, DecodeError>
    where
        T: ProtoExt,
        for<'a> T::Shadow<'a>: ProtoShadow<T, OwnedSun = T>,
    {
        decode_zero_copy_bytes::<T>(Bytes::from(self.inner.clone()))
    }
}

fn decode_zero_copy_bytes<T>(mut buf: Bytes) -> Result<T, DecodeError>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, OwnedSun = T>,
{
    let mut shadow = <T::Shadow<'static> as ProtoWire>::proto_default();

    if !buf.has_remaining() {
        return <T::Shadow<'static> as ProtoShadow<T>>::to_sun(shadow);
    }

    <T::Shadow<'static> as ProtoWire>::decode_into(<T::Shadow<'static> as ProtoWire>::WIRE_TYPE, &mut shadow, &mut buf, DecodeContext::default())?;

    <T::Shadow<'static> as ProtoShadow<T>>::to_sun(shadow)
}

impl<T> From<ZeroCopy<T>> for Vec<u8> {
    fn from(value: ZeroCopy<T>) -> Self {
        value.into_bytes()
    }
}

impl<T> From<&T> for ZeroCopy<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    fn from(value: &T) -> Self {
        let bytes = T::with_shadow(value, |shadow| {
            let len = <T::Shadow<'_> as ProtoWire>::encoded_len_impl(&shadow);
            if len == 0 {
                Vec::new()
            } else if <T::Shadow<'_> as ProtoWire>::WIRE_TYPE == WireType::LengthDelimited {
                let prefix_len = encoded_len_varint(len as u64);
                let mut buf = Vec::with_capacity(prefix_len + len);
                encode_varint(len as u64, &mut buf);
                <T::Shadow<'_> as ProtoWire>::encode_raw_unchecked(shadow, &mut buf);
                buf
            } else {
                let mut buf = Vec::with_capacity(len);
                <T::Shadow<'_> as ProtoWire>::encode_raw_unchecked(shadow, &mut buf);
                buf
            }
        });
        Self::from_bytes(bytes)
    }
}

impl<T> From<T> for ZeroCopy<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = T, OwnedSun = T>,
{
    fn from(value: T) -> Self {
        let bytes = T::with_shadow(value, |shadow| {
            let len = <T::Shadow<'_> as ProtoWire>::encoded_len_impl(&shadow);
            if len == 0 {
                Vec::new()
            } else if <T::Shadow<'_> as ProtoWire>::WIRE_TYPE == WireType::LengthDelimited {
                let prefix_len = encoded_len_varint(len as u64);
                let mut buf = Vec::with_capacity(prefix_len + len);
                encode_varint(len as u64, &mut buf);
                <T::Shadow<'_> as ProtoWire>::encode_raw_unchecked(shadow, &mut buf);
                buf
            } else {
                let mut buf = Vec::with_capacity(len);
                <T::Shadow<'_> as ProtoWire>::encode_raw_unchecked(shadow, &mut buf);
                buf
            }
        });
        Self::from_bytes(bytes)
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

    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value
    }
}

impl<T> ProtoWire for ZeroCopy<T>
where
    T: ProtoWire + 'static,
{
    type EncodeInput<'a> = &'a ZeroCopy<T>;
    const KIND: ProtoKind = T::KIND;
    const WIRE_TYPE: WireType = T::WIRE_TYPE; // ← ADD THIS!

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
        if Self::is_default_impl(value) { 0 } else { key_len(tag) + value.inner.len() }
    }

    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        buf.put_slice(&value.inner);
    }

    fn encode_entrypoint(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
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

    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
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
fn copy_value_payload(wire_type: WireType, buf: &mut impl Buf, into: &mut Vec<u8>, ctx: DecodeContext) -> Result<(), DecodeError> {
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

            into.resize(len_len + payload_len, 0);
            buf.copy_to_slice(&mut into[..]);
            Ok(())
        }
        WireType::StartGroup | WireType::EndGroup => Err(DecodeError::new("groups are not supported for ZeroCopy")),
    }
}

/// Append full field (key + payload) with minimized scanning
#[inline]
fn append_field(tag: u32, wire_type: WireType, buf: &mut impl Buf, out: &mut Vec<u8>, ctx: DecodeContext) -> Result<(), DecodeError> {
    ctx.limit_reached()?;

    match wire_type {
        WireType::Varint => {
            // Reserve space for key + max varint size
            let key_len = key_len(tag);
            out.reserve(key_len + 10);
            encode_key(tag, wire_type, out);

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
            encode_key(tag, wire_type, out);

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
            encode_key(tag, wire_type, out);

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
            encode_key(tag, wire_type, out);

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
