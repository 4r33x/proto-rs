// ---------- imports (adjust for no_std) ----------
extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::mem::MaybeUninit;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::encoding::DecodeContext;
use crate::encoding::wire_type::WireType;
use crate::traits::ProtoShadow;
use crate::traits::ViewOf;

impl<T> ProtoShadow for Box<T>
where
    for<'a> T: ProtoShadow<OwnedSun = T> + 'a,
{
    type Sun<'a> = T::Sun<'a>;
    type OwnedSun = Box<T>;
    type View<'a> = T::View<'a>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        T::from_sun(value)
    }
}

pub struct BoxedShadow<S>(pub Box<S>);

impl<S, T> ProtoShadow for BoxedShadow<S>
where
    S: ProtoShadow<OwnedSun = T>,
{
    type Sun<'a> = S::Sun<'a>;
    type View<'a> = S::View<'a>;
    type OwnedSun = Box<T>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        let inner = self.0;
        Ok(Box::write(Box::new_uninit(), inner.to_sun()?))
    }

    #[inline]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        S::from_sun(value)
    }
}

impl<T> ProtoExt for Box<T>
where
    T: ProtoExt,
{
    // “Just a wrapper”: same shadow as T, but adapted to yield Box<T>
    type Shadow<'a>
        = BoxedShadow<T::Shadow<'a>>
    where
        T: 'a;

    #[inline]
    fn proto_default<'a>() -> Self::Shadow<'a> {
        BoxedShadow(Box::write(Box::new_uninit(), T::proto_default()))
    }

    #[inline]
    fn encoded_len(value: &ViewOf<'_, Self>) -> usize {
        T::encoded_len(value)
    }

    #[inline]
    fn encode_raw(value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
        T::encode_raw(value, buf);
    }

    #[inline]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        T::merge_field(value.0.as_mut(), tag, wire, buf, ctx)
    }

    #[inline]
    fn post_decode(value: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        value.to_sun()
    }

    #[inline]
    fn clear(&mut self) {
        T::clear(self);
    }
}

// ---------- Identity shadow for Arc<T> (alloc-free fast path when chosen as a shadow) ----------
impl<T> ProtoShadow for Arc<T>
where
    for<'a> T: ProtoShadow<OwnedSun = Arc<T>> + 'a,
{
    type Sun<'a> = T::Sun<'a>;
    type OwnedSun = T::OwnedSun;
    type View<'a> = T::View<'a>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        T::from_sun(value)
    }
}

// ---------- Generic adapter: any S: ProtoShadow<OwnedSun = T> -> OwnedSun = Arc<T> ----------
pub struct ArcedShadow<S>(pub Box<S>);

impl<S, T> ProtoShadow for ArcedShadow<S>
where
    S: ProtoShadow<OwnedSun = T>,
{
    type Sun<'a> = S::Sun<'a>;
    type View<'a> = S::View<'a>;
    type OwnedSun = Arc<T>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        let inner = *self.0;
        let value = inner.to_sun()?;
        // allocate Arc<MaybeUninit<T>>
        let u: Arc<MaybeUninit<T>> = Arc::new_uninit();

        // just allocated -> unique; write T directly into the slot
        let slot: &mut MaybeUninit<T> = unsafe { &mut *(Arc::as_ptr(&u).cast_mut()) };
        slot.write(value);

        // disambiguate: assume_init for Arc<MaybeUninit<T>>
        let arc_t: Arc<T> = unsafe { Arc::<MaybeUninit<T>>::assume_init(u) };
        Ok(arc_t)
    }

    #[inline]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        <S as ProtoShadow>::from_sun(value)
    }
}

// ---------- ProtoExt for Arc<T>: “just a wrapper” over T’s shadow ----------
impl<T> ProtoExt for Arc<T>
where
    T: ProtoExt,
{
    // Same shadow as T, adapted to produce Arc<T>
    type Shadow<'a>
        = ArcedShadow<T::Shadow<'a>>
    where
        T: 'a;

    #[inline]
    fn proto_default<'a>() -> Self::Shadow<'a> {
        ArcedShadow(Box::write(Box::new_uninit(), T::proto_default()))
    }

    #[inline]
    fn encoded_len(value: &ViewOf<'_, Self>) -> usize {
        T::encoded_len(value)
    }

    #[inline]
    fn encode_raw(value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
        T::encode_raw(value, buf);
    }

    #[inline]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        T::merge_field(value.0.as_mut(), tag, wire_type, buf, ctx)
    }

    #[inline]
    fn post_decode(value: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        // ArcedShadow::to_sun() -> Arc<T>
        value.to_sun()
    }

    #[inline]
    fn clear(&mut self) {
        T::clear(Arc::get_mut(self).expect("Arc should be always unique here"));
    }
}
