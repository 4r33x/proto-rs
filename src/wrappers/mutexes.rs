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

pub struct MutexShadow<'a, M> {
    mutex: &'a M,
}

pub trait MutexSource {
    type Value: ProtoArchive + ProtoExt;
    fn with_value<R>(&self, f: impl FnOnce(&Self::Value) -> R) -> R;
}

impl<T: ProtoArchive + ProtoExt> MutexSource for std::sync::Mutex<T> {
    type Value = T;
    fn with_value<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.lock().expect("Mutex lock poisoned"))
    }
}

#[cfg(feature = "parking_lot")]
impl<T: ProtoArchive + ProtoExt> MutexSource for parking_lot::Mutex<T> {
    type Value = T;
    fn with_value<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.lock())
    }
}
impl<M: MutexSource> ProtoExt for MutexShadow<'_, M> {
    const KIND: ProtoKind = M::Value::KIND;
    const ENCODED_SIZE_HINT: crate::EncodeSizeHint = M::Value::ENCODED_SIZE_HINT;
}
impl<M: MutexSource> ProtoArchive for MutexShadow<'_, M> {
    fn is_default(&self) -> bool {
        self.mutex.with_value(ProtoArchive::is_default)
    }
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        self.mutex.with_value(ProtoArchive::encoded_size_hint::<TAG>)
    }
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        self.mutex.with_value(|value| value.archive::<TAG>(w));
    }
}
impl<T: ProtoExt> ProtoExt for std::sync::Mutex<T> {
    const KIND: ProtoKind = T::KIND;
    const WRAP_ROOT: bool = true;
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
        self.merge_with_state(wire_type, buf, ctx, &crate::DecodeState::default())
    }

    fn merge_field_with_state(
        value: &mut Self,
        tag: u32,
        wire: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
        state: &crate::DecodeState<'_>,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            value.merge_with_state(wire, buf, ctx, state)
        } else {
            skip_field(wire, tag, buf, ctx)
        }
    }

    fn merge_with_state(
        &mut self,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
        state: &crate::DecodeState<'_>,
    ) -> Result<(), DecodeError> {
        let inner = self.get_mut().map_err(|_| DecodeError::new("Mutex lock poisoned"))?;
        T::merge_value_with_state(inner, wire_type, buf, ctx, state)
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

#[cfg(feature = "parking_lot")]
impl<T: ProtoExt> ProtoExt for parking_lot::Mutex<T> {
    const KIND: ProtoKind = T::KIND;
    const WRAP_ROOT: bool = true;
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
        self.merge_with_state(wire_type, buf, ctx, &crate::DecodeState::default())
    }

    fn merge_field_with_state(
        value: &mut Self,
        tag: u32,
        wire: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
        state: &crate::DecodeState<'_>,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            value.merge_with_state(wire, buf, ctx, state)
        } else {
            skip_field(wire, tag, buf, ctx)
        }
    }

    fn merge_with_state(
        &mut self,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
        state: &crate::DecodeState<'_>,
    ) -> Result<(), DecodeError> {
        let inner = self.get_mut();
        T::merge_value_with_state(inner, wire_type, buf, ctx, state)
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

impl<T: ProtoArchive + ProtoExt + 'static> ProtoEncode for std::sync::Mutex<T> {
    type Shadow<'a> = MutexShadow<'a, Self>;
}
impl<'a, T: ProtoArchive + ProtoExt> ProtoShadowEncode<'a, std::sync::Mutex<T>> for MutexShadow<'a, std::sync::Mutex<T>> {
    fn from_sun(value: &'a std::sync::Mutex<T>) -> Self {
        Self { mutex: value }
    }
}
#[cfg(feature = "parking_lot")]
impl<T: ProtoArchive + ProtoExt + 'static> ProtoEncode for parking_lot::Mutex<T> {
    type Shadow<'a> = MutexShadow<'a, Self>;
}
#[cfg(feature = "parking_lot")]
impl<'a, T: ProtoArchive + ProtoExt> ProtoShadowEncode<'a, parking_lot::Mutex<T>> for MutexShadow<'a, parking_lot::Mutex<T>> {
    fn from_sun(value: &'a parking_lot::Mutex<T>) -> Self {
        Self { mutex: value }
    }
}
