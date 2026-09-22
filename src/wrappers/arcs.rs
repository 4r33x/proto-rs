use alloc::sync::Arc;
use core::mem::MaybeUninit;

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

impl<T: ProtoExt> ProtoExt for Arc<T> {
    const KIND: ProtoKind = T::KIND;
    const WRAP_ROOT: bool = true;
    const ENCODED_SIZE_HINT: crate::EncodeSizeHint = T::ENCODED_SIZE_HINT;
}

impl<T: ProtoDecode> ProtoDecode for Arc<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = Box<T::ShadowDecoded>;
}

impl<T: ProtoFieldMerge + ProtoDefault> ProtoDecoder for Arc<T> {
    #[inline]
    fn finish(&mut self, state: &crate::DecodeState<'_>) -> Result<(), DecodeError> {
        if !state.has_data() {
            return Ok(());
        }
        let inner = Arc::get_mut(self).ok_or_else(|| DecodeError::new("cannot decode into a shared Arc"))?;
        T::finish_value(inner, state)
    }

    #[inline]
    fn merge_field(
        value: &mut Self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl bytes::Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            value.merge(wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl bytes::Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let state = crate::DecodeState::default();
        self.merge_with_state(wire_type, buf, ctx, &state)?;
        self.finish(&state)
    }

    fn merge_field_with_state(
        value: &mut Self,
        tag: u32,
        wire: WireType,
        buf: &mut impl bytes::Buf,
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
        buf: &mut impl bytes::Buf,
        ctx: DecodeContext,
        state: &crate::DecodeState<'_>,
    ) -> Result<(), DecodeError> {
        if let Some(inner) = Arc::get_mut(self) {
            T::merge_value_with_state(inner, wire_type, buf, ctx, state)
        } else {
            Err(DecodeError::new("cannot decode into a shared Arc"))
        }
    }
}

impl<T: ProtoDefault> ProtoDefault for Arc<T> {
    #[inline]
    fn proto_default() -> Self {
        Arc::new(<T as ProtoDefault>::proto_default())
    }
}

impl<T, U> ProtoShadowDecode<Arc<U>> for Box<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<Arc<U>, DecodeError> {
        // allocate Arc<MaybeUninit<T>>
        let mut u: Arc<MaybeUninit<U>> = Arc::new_uninit();

        // just allocated -> unique; write T directly into the slot
        let slot = Arc::get_mut(&mut u).expect("new Arc is uniquely owned");
        slot.write((*self).to_sun()?);

        // disambiguate: assume_init for Arc<MaybeUninit<T>>
        let arc_t: Arc<U> = unsafe { Arc::<MaybeUninit<U>>::assume_init(u) };
        Ok(arc_t)
    }
}

impl<T> ProtoArchive for Arc<T>
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

impl<T: ProtoEncode> ProtoEncode for Arc<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
{
    type Shadow<'a> = T::Shadow<'a>;
    #[inline]
    fn size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        T::size_hint::<TAG>(self.as_ref())
    }
}

impl<'a, T, S> ProtoShadowEncode<'a, Arc<T>> for S
where
    S: ProtoShadowEncode<'a, T>,
{
    #[inline]
    fn from_sun(value: &'a Arc<T>) -> Self {
        S::from_sun(value.as_ref())
    }
}

impl<T> ProtoArchive for &Arc<T>
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
