use alloc::sync::Arc;
use core::mem::MaybeUninit;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;

impl<T: ProtoExt> ProtoExt for Arc<T> {
    const KIND: ProtoKind = T::KIND;
}

impl<T: ProtoDecoder + ProtoExt + Clone> ProtoDecoder for Arc<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        Arc::new(T::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        T::clear(Arc::make_mut(self));
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            T::merge(Arc::make_mut(value), wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        T::merge(Arc::make_mut(self), wire_type, buf, ctx)
    }
}

impl<T: ProtoDecode + Clone> ProtoDecode for Arc<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt + Clone,
{
    type ShadowDecoded = Box<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<Arc<U>> for Box<T>
where
    T: ProtoShadowDecode<U> + Clone,
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
    type Archived<'a> = T::Archived<'a>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        T::is_default(self.as_ref())
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        T::len(archived)
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        unsafe { T::encode(archived, buf) };
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        T::archive(self.as_ref())
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
    type Archived<'x> = T::Archived<'x>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        T::is_default(self.as_ref())
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        T::len(archived)
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        unsafe { T::encode(archived, buf) };
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        T::archive(self.as_ref())
    }
}
