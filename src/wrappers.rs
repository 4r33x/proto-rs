// ---------- imports (adjust for no_std) ----------
extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::mem::MaybeUninit;
use std::collections::BTreeSet;
#[cfg(feature = "std")]
use std::collections::HashSet;
#[cfg(feature = "std")]
use std::hash::Hash;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::OwnedSunOf;
use crate::ProtoExt;
use crate::RepeatedCollection;
use crate::encoding::DecodeContext;
use crate::encoding::check_wire_type;
use crate::encoding::wire_type::WireType;
use crate::traits::ProtoShadow;
use crate::traits::ViewOf;

/// Generic implementation for Option<T>
impl<T: ProtoShadow> ProtoShadow for Option<T> {
    type Sun<'a> = Option<T::Sun<'a>>;

    type OwnedSun = Option<T::OwnedSun>;
    type View<'a> = Option<T::View<'a>>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        // Map Option<T> → Option<T::OwnedSun>
        self.map(T::to_sun).transpose()
    }

    #[inline]
    fn from_sun<'a>(v: Self::Sun<'_>) -> Self::View<'_> {
        v.map(T::from_sun)
    }
}

impl<T> RepeatedCollection<T> for Vec<T> {
    #[inline]
    fn reserve_hint(&mut self, additional: usize) {
        Vec::reserve(self, additional);
    }

    #[inline]
    fn push(&mut self, value: T) {
        Vec::push(self, value);
    }
}

impl<T: Ord> RepeatedCollection<T> for BTreeSet<T> {
    #[inline]
    fn push(&mut self, value: T) {
        let _ = BTreeSet::insert(self, value);
    }
}

#[cfg(feature = "std")]
impl<T: Eq + Hash, S: std::hash::BuildHasher> RepeatedCollection<T> for HashSet<T, S> {
    #[inline]
    fn reserve_hint(&mut self, additional: usize) {
        HashSet::reserve(self, additional);
    }

    #[inline]
    fn push(&mut self, value: T) {
        let _ = HashSet::insert(self, value);
    }
}

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

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        let inner = self.0;
        Ok(Box::write(Box::new_uninit(), inner.to_sun()?))
    }

    #[inline(always)]
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

    #[inline(always)]
    fn repeated_merge<'a, C>(values: &mut C, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>
    where
        C: RepeatedCollection<OwnedSunOf<'a, Self>>,
        Self: 'a,
    {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let mut msg = Self::proto_default();
        crate::encoding::message::merge::<Self, _>(WireType::LengthDelimited, &mut msg, buf, ctx)?;
        values.push(msg.to_sun()?);
        Ok(())
    }

    #[inline(always)]
    fn proto_default<'a>() -> Self::Shadow<'a> {
        BoxedShadow(Box::write(Box::new_uninit(), T::proto_default()))
    }

    #[inline(always)]
    fn encoded_len(value: &ViewOf<'_, Self>) -> usize {
        T::encoded_len(value)
    }

    #[inline(always)]
    fn clear(&mut self) {
        T::clear(self);
    }

    #[inline(always)]
    fn is_default(value: &ViewOf<'_, Self>) -> bool {
        T::is_default(value)
    }

    #[inline(always)]
    fn encode_raw_checked(value: ViewOf<'_, Self>, buf: &mut impl BufMut, remaining: &mut usize) -> Result<(), crate::EncodeError> {
        T::encode_raw_checked(value, buf, remaining)
    }

    #[inline(always)]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        T::merge_field(value.0.as_mut(), tag, wire, buf, ctx)
    }

    #[inline(always)]
    fn post_decode(value: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        value.to_sun()
    }

    #[inline(always)]
    fn repeated_encode_checked<'a, I>(iter: I, buf: &mut impl BufMut, remaining: &mut usize) -> Result<(), crate::EncodeError>
    where
        I: IntoIterator<Item = ViewOf<'a, Self>>,
        Self: 'a,
    {
        T::repeated_encode_checked(iter, buf, remaining)
    }

    #[inline(always)]
    fn repeated_encoded_len<'a, I>(iter: I) -> usize
    where
        I: IntoIterator,
        I::Item: AsRef<ViewOf<'a, Self>>,
        Self: 'a,
    {
        T::repeated_encoded_len(iter)
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

    #[inline]
    fn encode_singular_field(tag: u32, value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
        T::encode_singular_field(tag, value, buf);
    }

    #[inline]
    fn merge_singular_field(wire_type: WireType, value: &mut Self::Shadow<'_>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        T::merge_singular_field(wire_type, value.0.as_mut(), buf, ctx)
    }

    #[inline]
    fn encoded_len_singular_field(tag: u32, value: &ViewOf<'_, Self>) -> usize {
        T::encoded_len_singular_field(tag, value)
    }
}
