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
use crate::traits::ProtoFieldMerge;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::traits::buffer::RevWriter;

impl<T: ProtoExt> ProtoExt for Arc<T> {
    const KIND: ProtoKind = T::KIND;
}

impl<T: ProtoDecode> ProtoDecode for Arc<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = Box<T::ShadowDecoded>;
}

impl<T: ProtoFieldMerge + ProtoDefault> ProtoDecoder for Arc<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        Arc::new(<T as ProtoDefault>::proto_default_value())
    }

    #[inline(always)]
    fn clear(&mut self) {
        if let Some(inner) = Arc::get_mut(self) {
            *inner = <T as ProtoDefault>::proto_default_value();
        } else {
            *self = Arc::new(<T as ProtoDefault>::proto_default_value());
        }
    }

    #[inline(always)]
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

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl bytes::Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if let Some(inner) = Arc::get_mut(self) {
            T::merge_value(inner, wire_type, buf, ctx)
        } else {
            let mut value = <T as ProtoDefault>::proto_default_value();
            T::merge_value(&mut value, wire_type, buf, ctx)?;
            *self = Arc::new(value);
            Ok(())
        }
    }
}

impl<T, U> ProtoShadowDecode<Arc<U>> for Box<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<Arc<U>, DecodeError> {
        // allocate Arc<MaybeUninit<T>>
        let u: Arc<MaybeUninit<U>> = Arc::new_uninit();

        // just allocated -> unique; write T directly into the slot
        let slot: &mut MaybeUninit<U> = unsafe { &mut *(Arc::as_ptr(&u).cast_mut()) };
        slot.write((*self).to_sun()?);

        // disambiguate: assume_init for Arc<MaybeUninit<T>>
        let arc_t: Arc<U> = unsafe { Arc::<MaybeUninit<U>>::assume_init(u) };
        Ok(arc_t)
    }
}

impl<T> ProtoArchive for Arc<T>
where
    T: ProtoArchive,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        T::is_default(self.as_ref())
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        <T as ProtoArchive>::archive::<TAG>(self.as_ref(), w);
    }
}

impl<T: ProtoEncode> ProtoEncode for Arc<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
{
    type Shadow<'a> = T::Shadow<'a>;
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
    T: ProtoArchive,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        T::is_default(self.as_ref())
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        <T as ProtoArchive>::archive::<TAG>(self.as_ref(), w);
    }
}
