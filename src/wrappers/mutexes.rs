use core::ops::Deref;

use bytes::Buf;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoDefault;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoFieldMerge;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::traits::buffer::RevWriter;

pub struct MutexShadow<G> {
    guard: G,
}

impl<T: ProtoExt> ProtoExt for std::sync::Mutex<T> {
    const KIND: ProtoKind = T::KIND;
    const ENCODED_SIZE_HINT: crate::EncodeSizeHint = T::ENCODED_SIZE_HINT;
}

impl<T: ProtoFieldMerge + ProtoDefault> ProtoDecoder for std::sync::Mutex<T> {
    #[inline]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = self.get_mut().map_err(|_| DecodeError::new("Mutex lock poisoned"))?;
        T::merge_value(inner, wire_type, buf, ctx)
    }
}

impl<T: ProtoDefault> ProtoDefault for std::sync::Mutex<T> {
    #[inline]
    fn proto_default() -> Self {
        std::sync::Mutex::new(<T as ProtoDefault>::proto_default())
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

impl<T: ProtoArchive + ProtoExt + 'static> ProtoEncode for std::sync::Mutex<T> {
    type Shadow<'a> = MutexShadow<std::sync::MutexGuard<'a, T>>;
}

impl<'a, T: ProtoArchive + ProtoExt> ProtoShadowEncode<'a, std::sync::Mutex<T>> for MutexShadow<std::sync::MutexGuard<'a, T>> {
    #[inline]
    fn from_sun(value: &'a std::sync::Mutex<T>) -> Self {
        Self {
            guard: value.lock().expect("Mutex lock poisoned"),
        }
    }
}

impl<G> ProtoExt for MutexShadow<G>
where
    G: Deref,
    G::Target: ProtoExt,
{
    const KIND: ProtoKind = G::Target::KIND;
    const ENCODED_SIZE_HINT: crate::EncodeSizeHint = G::Target::ENCODED_SIZE_HINT;
}

impl<G> ProtoArchive for MutexShadow<G>
where
    G: Deref,
    G::Target: ProtoArchive + ProtoExt,
{
    #[inline]
    fn is_default(&self) -> bool {
        self.guard.is_default()
    }

    #[inline]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        self.guard.archive::<TAG>(w);
    }
}

#[cfg(feature = "parking_lot")]
impl<T: ProtoExt> ProtoExt for parking_lot::Mutex<T> {
    const KIND: ProtoKind = T::KIND;
    const ENCODED_SIZE_HINT: crate::EncodeSizeHint = T::ENCODED_SIZE_HINT;
}

#[cfg(feature = "parking_lot")]
impl<T: ProtoFieldMerge + ProtoDefault> ProtoDecoder for parking_lot::Mutex<T> {
    #[inline]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = self.get_mut();
        T::merge_value(inner, wire_type, buf, ctx)
    }
}

#[cfg(feature = "parking_lot")]
impl<T: ProtoDefault> ProtoDefault for parking_lot::Mutex<T> {
    #[inline]
    fn proto_default() -> Self {
        parking_lot::Mutex::new(<T as ProtoDefault>::proto_default())
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
impl<T: ProtoArchive + ProtoExt + 'static> ProtoEncode for parking_lot::Mutex<T> {
    type Shadow<'a> = MutexShadow<parking_lot::MutexGuard<'a, T>>;
}

#[cfg(feature = "parking_lot")]
impl<'a, T: ProtoArchive + ProtoExt> ProtoShadowEncode<'a, parking_lot::Mutex<T>> for MutexShadow<parking_lot::MutexGuard<'a, T>> {
    #[inline]
    fn from_sun(value: &'a parking_lot::Mutex<T>) -> Self {
        Self { guard: value.lock() }
    }
}
