use alloc::vec::Vec;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;

pub struct MutexShadow<T> {
    bytes: Vec<u8>,
    is_default: bool,
    _marker: core::marker::PhantomData<T>,
}

impl<T: ProtoExt> ProtoExt for std::sync::Mutex<T> {
    const KIND: ProtoKind = T::KIND;
}

impl<T: ProtoDecoder + ProtoExt> ProtoDecoder for std::sync::Mutex<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        std::sync::Mutex::new(T::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        if let Ok(inner) = self.get_mut() {
            T::clear(inner);
        }
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = self.get_mut().map_err(|_| DecodeError::new("Mutex lock poisoned"))?;
        T::merge(inner, wire_type, buf, ctx)
    }
}

impl<T: ProtoDecode> ProtoDecode for std::sync::Mutex<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = std::sync::Mutex<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<std::sync::Mutex<U>> for std::sync::Mutex<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<std::sync::Mutex<U>, DecodeError> {
        let inner = self.into_inner().map_err(|_| DecodeError::new("Mutex lock poisoned"))?;
        Ok(std::sync::Mutex::new(inner.to_sun()?))
    }
}

impl<T: ProtoEncode + ProtoArchive + ProtoExt> ProtoEncode for std::sync::Mutex<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt + ProtoShadowEncode<'a, T>,
{
    type Shadow<'a> = MutexShadow<T>;
}

impl<'a, T> ProtoShadowEncode<'a, std::sync::Mutex<T>> for MutexShadow<T>
where
    T: ProtoEncode + ProtoArchive + ProtoExt,
{
    #[inline]
    fn from_sun(value: &'a std::sync::Mutex<T>) -> Self {
        let guard = value.lock().expect("Mutex lock poisoned");
        let is_default = T::is_default(&*guard);
        let bytes = if is_default { Vec::new() } else { guard.encode_to_vec() };
        Self {
            bytes,
            is_default,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<T> ProtoExt for MutexShadow<T>
where
    T: ProtoExt,
{
    const KIND: ProtoKind = T::KIND;
}

impl<T> ProtoArchive for MutexShadow<T> {
    type Archived<'a> = &'a [u8];

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_default
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        archived.len()
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        buf.put_slice(archived);
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        self.bytes.as_slice()
    }
}

#[cfg(feature = "parking_lot")]
impl<T: ProtoExt> ProtoExt for parking_lot::Mutex<T> {
    const KIND: ProtoKind = T::KIND;
}

#[cfg(feature = "parking_lot")]
impl<T: ProtoDecoder + ProtoExt> ProtoDecoder for parking_lot::Mutex<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        parking_lot::Mutex::new(T::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        T::clear(self.get_mut());
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = self.get_mut();
        T::merge(inner, wire_type, buf, ctx)
    }
}

#[cfg(feature = "parking_lot")]
impl<T: ProtoDecode> ProtoDecode for parking_lot::Mutex<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = parking_lot::Mutex<T::ShadowDecoded>;
}

#[cfg(feature = "parking_lot")]
impl<T, U> ProtoShadowDecode<parking_lot::Mutex<U>> for parking_lot::Mutex<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<parking_lot::Mutex<U>, DecodeError> {
        let inner = self.into_inner();
        Ok(parking_lot::Mutex::new(inner.to_sun()?))
    }
}

#[cfg(feature = "parking_lot")]
impl<T: ProtoEncode + ProtoArchive + ProtoExt> ProtoEncode for parking_lot::Mutex<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt + ProtoShadowEncode<'a, T>,
{
    type Shadow<'a> = MutexShadow<T>;
}

#[cfg(feature = "parking_lot")]
impl<'a, T> ProtoShadowEncode<'a, parking_lot::Mutex<T>> for MutexShadow<T>
where
    T: ProtoEncode + ProtoArchive + ProtoExt,
{
    #[inline]
    fn from_sun(value: &'a parking_lot::Mutex<T>) -> Self {
        let guard = value.lock();
        let is_default = T::is_default(&*guard);
        let bytes = if is_default { Vec::new() } else { guard.encode_to_vec() };
        Self {
            bytes,
            is_default,
            _marker: core::marker::PhantomData,
        }
    }
}
