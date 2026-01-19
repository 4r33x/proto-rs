use alloc::sync::Arc;
use core::mem::MaybeUninit;

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

impl<T: ProtoExt> ProtoExt for Arc<T> {
    const KIND: ProtoKind = T::KIND;
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

impl<'a, T: ProtoExt> ProtoExt for &'a Arc<T> {
    const KIND: ProtoKind = T::KIND;
}

impl<'a, T> ProtoArchive for &'a Arc<T>
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
