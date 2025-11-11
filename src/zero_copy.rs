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
use crate::encoding::decode_varint;
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
        T::decode(Bytes::from(self.inner.clone()))
    }

    pub fn into_message(self) -> Result<T, DecodeError>
    where
        T: ProtoExt,
        for<'a> T::Shadow<'a>: ProtoShadow<T, OwnedSun = T>,
    {
        T::decode(Bytes::from(self.inner))
    }
}

impl<T> ZeroCopy<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T, OwnedSun = T>,
{
    pub fn from_ref(value: &T) -> Self {
        Self::from(value)
    }

    pub fn from_owned(value: T) -> Self {
        Self::from(&value)
    }
}

impl<T> ZeroCopy<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = T, OwnedSun = T>,
{
    pub fn from_value(value: T) -> Self {
        Self::from(value)
    }

    pub fn from_copy(value: &T) -> Self
    where
        T: Copy,
    {
        Self::from(*value)
    }
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
        let bytes = T::encode_to_vec(value);
        Self::from_bytes(bytes)
    }
}

impl<T> From<T> for ZeroCopy<T>
where
    T: ProtoExt,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = T, OwnedSun = T>,
{
    fn from(value: T) -> Self {
        let bytes = T::encode_to_vec(value);
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

impl<T> ToZeroCopy<T> for ZeroCopy<T> {
    fn to_zero_copy(self) -> ZeroCopy<T> {
        self
    }
}

impl<T> ToZeroCopy<T> for &ZeroCopy<T> {
    fn to_zero_copy(self) -> ZeroCopy<T> {
        (*self).clone()
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

    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        buf.put_slice(&value.inner);
    }

    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        check_wire_type(Self::WIRE_TYPE, wire_type)?;
        let bytes = copy_value(wire_type, buf, ctx)?;
        value.inner = bytes;
        Ok(())
    }
}

impl<T: 'static> ProtoExt for ZeroCopy<T>
where
    T: ProtoWire,
{
    type Shadow<'b> = ZeroCopy<T>;

    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let data = copy_value(wire_type, buf, ctx)?;
        let mut field = Vec::with_capacity(
            key_len(tag)
                + data.len()
                + match wire_type {
                    WireType::LengthDelimited => encoded_len_varint(data.len() as u64),
                    _ => 0,
                },
        );
        encode_key(tag, wire_type, &mut field);
        if matches!(wire_type, WireType::LengthDelimited) {
            encode_varint(data.len() as u64, &mut field);
        }
        field.extend_from_slice(&data);
        value.inner.extend_from_slice(&field);
        Ok(())
    }
}

fn copy_value(wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<Vec<u8>, DecodeError> {
    ctx.limit_reached()?;
    match wire_type {
        WireType::Varint => {
            let value = decode_varint(buf)?;
            let mut raw = Vec::with_capacity(encoded_len_varint(value));
            encode_varint(value, &mut raw);
            Ok(raw)
        }
        WireType::ThirtyTwoBit => {
            if buf.remaining() < 4 {
                return Err(DecodeError::new("buffer underflow"));
            }
            Ok(buf.copy_to_bytes(4).to_vec())
        }
        WireType::SixtyFourBit => {
            if buf.remaining() < 8 {
                return Err(DecodeError::new("buffer underflow"));
            }
            Ok(buf.copy_to_bytes(8).to_vec())
        }
        WireType::LengthDelimited => {
            let len = decode_varint(buf)?;
            if len > buf.remaining() as u64 {
                return Err(DecodeError::new("buffer underflow"));
            }
            Ok(buf.copy_to_bytes(len as usize).to_vec())
        }
        WireType::StartGroup | WireType::EndGroup => Err(DecodeError::new("groups are not supported for ZeroCopy")),
    }
}
