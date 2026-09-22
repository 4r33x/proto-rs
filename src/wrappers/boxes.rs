use alloc::boxed::Box;

use bytes::Buf;

use crate::DecodeError;
use crate::ProtoDecoder;
use crate::ProtoDefault;
use crate::ProtoEncode;
use crate::ProtoFieldMerge;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::traits::buffer::RevWriter;

impl<T: ProtoExt> ProtoExt for Box<T> {
    const KIND: ProtoKind = T::KIND;
    const WRAP_ROOT: bool = true;
    const ENCODED_SIZE_HINT: crate::EncodeSizeHint = T::ENCODED_SIZE_HINT;
}

impl<T: ProtoFieldMerge + ProtoDefault> ProtoDecoder for Box<T> {
    #[inline]
    fn finish(&mut self, state: &crate::DecodeState<'_>) -> Result<(), DecodeError> {
        T::finish_value(self.as_mut(), state)
    }

    #[inline]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            T::merge_value(value.as_mut(), wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let state = crate::DecodeState::default();
        self.merge_with_state(wire_type, buf, ctx, &state)?;
        self.finish(&state)
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
        T::merge_value_with_state(self.as_mut(), wire_type, buf, ctx, state)
    }
}

impl<T: ProtoDefault> ProtoDefault for Box<T> {
    #[inline]
    fn proto_default() -> Self {
        Box::new(<T as ProtoDefault>::proto_default())
    }
}

impl<T: ProtoDecode> ProtoDecode for Box<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = Box<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<Box<U>> for Box<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<Box<U>, DecodeError> {
        Ok(Box::write(Box::new_uninit(), (*self).to_sun()?))
    }
}

impl<T> ProtoArchive for Box<T>
where
    T: ProtoArchive + ProtoExt,
{
    #[inline]
    fn is_default(&self) -> bool {
        T::is_default(self.as_ref())
    }

    #[inline]
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint
    where
        Self: ProtoExt,
    {
        <T as ProtoArchive>::encoded_size_hint::<TAG>(self.as_ref())
    }

    #[inline]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        <T as ProtoArchive>::archive::<TAG>(self.as_ref(), w);
    }
}

impl<T: ProtoEncode> ProtoEncode for Box<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
{
    type Shadow<'a> = T::Shadow<'a>;
    #[inline]
    fn size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        T::size_hint::<TAG>(self.as_ref())
    }
}

impl<'a, T, S> ProtoShadowEncode<'a, Box<T>> for S
where
    S: ProtoShadowEncode<'a, T>,
{
    #[inline]
    fn from_sun(value: &'a Box<T>) -> Self {
        S::from_sun(value.as_ref())
    }
}

impl<T> ProtoArchive for &Box<T>
where
    T: ProtoArchive + ProtoExt,
{
    #[inline]
    fn is_default(&self) -> bool {
        T::is_default(self.as_ref())
    }

    #[inline]
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint
    where
        Self: ProtoExt,
    {
        <T as ProtoArchive>::encoded_size_hint::<TAG>(self.as_ref())
    }

    #[inline]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        <T as ProtoArchive>::archive::<TAG>(self.as_ref(), w);
    }
}
