// ---------- imports (adjust for no_std) ----------
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use std::mem::MaybeUninit;
use std::sync::Arc;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::MessageField;
use crate::ProtoExt;
use crate::RepeatedField;
use crate::SingularField;
use crate::encoding::DecodeContext;
use crate::encoding::wire_type::WireType;
use crate::traits::ProtoShadow;
use crate::traits::ViewOf;

// ---------------- Blanket impls for MessageField ----------------

impl<T> SingularField for T
where
    T: MessageField,
{
    #[inline]
    fn encode_singular_field(tag: u32, value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
        let len = <Self as ProtoExt>::encoded_len(&value);
        if len != 0 {
            crate::encoding::message::encode::<Self>(tag, value, buf);
        }
    }

    #[inline]
    fn merge_singular_field(wire_type: WireType, value: &mut Self::Shadow<'_>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        crate::encoding::message::merge::<Self, _>(wire_type, value, buf, ctx)
    }

    #[inline]
    fn encoded_len_singular_field(tag: u32, value: &ViewOf<'_, Self>) -> usize {
        let len = <Self as ProtoExt>::encoded_len(value);
        if len == 0 { 0 } else { crate::encoding::message::encoded_len::<Self>(tag, value) }
    }
}

impl<T> RepeatedField for T
where
    T: MessageField,
    for<'a> <T as ProtoExt>::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, View<'a> = &'a T, OwnedSun = T>,
{
    #[inline]
    fn encode_repeated_field<'a, I>(tag: u32, values: I, buf: &mut impl BufMut)
    where
        Self: 'a,
        I: IntoIterator<Item = ViewOf<'a, Self>>,
    {
        crate::encoding::message::encode_repeated::<Self, _>(tag, values, buf);
    }
    #[inline]
    fn encoded_len_repeated_field(tag: u32, values: &[ViewOf<'_, Self>]) -> usize {
        crate::encoding::message::encoded_len_repeated::<Self>(tag, values)
    }

    #[inline]
    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self::Shadow<'_>>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        crate::encoding::message::merge_repeated::<Self>(wire_type, values, buf, ctx)
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

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        let inner = *self.0;
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
        BoxedShadow(Box::new(T::proto_default()))
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

impl<T: MessageField> MessageField for Box<T> {}

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
        ArcedShadow(Box::new(T::proto_default()))
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
        T::clear(Arc::get_mut(self).unwrap());
    }
}

// ---------- MessageField passthrough ----------
impl<T: MessageField> MessageField for Arc<T> {}

impl<S> ProtoShadow for Vec<S>
where
    S: ProtoShadow,
{
    type Sun<'a>
        = &'a [S::Sun<'a>]
    where
        Self: 'a;

    type OwnedSun = Vec<S::OwnedSun>;

    // Use a slice view to avoid allocating a Vec of views during encode paths.
    type View<'a>
        = Vec<S::View<'a>>
    where
        Self: 'a;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        self.into_iter().map(|x| x.to_sun()).collect::<Result<Vec<_>, DecodeError>>()
    }

    #[inline]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value.into_iter().map(|&x| S::from_sun(x)).collect()
    }
}

// ---------- Vec<T>: ProtoExt ----------
impl<T> ProtoExt for Vec<T>
where
    T: ProtoExt + RepeatedField,
    // Ensure element shadows are the by-ref identity so the cast above is valid:
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, View<'a> = &'a T, OwnedSun = T>,
{
    type Shadow<'a>
        = Vec<T::Shadow<'a>>
    where
        T: 'a;

    #[inline]
    fn proto_default<'a>() -> Self::Shadow<'a> {
        Vec::new()
    }

    #[inline]
    fn encoded_len(value: &ViewOf<'_, Self>) -> usize {
        // ViewOf<Vec<T>> = &[&T]
        if value.is_empty() { 0 } else { T::encoded_len_repeated_field(1, value) }
    }

    #[inline]
    fn encode_raw(value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
        // value: &[&T] → iterator of &T without extra alloc/copies
        if !value.is_empty() {
            T::encode_repeated_field(1, value.iter().copied(), buf);
        }
    }

    #[inline]
    fn merge_field(
        value: &mut Self::Shadow<'_>, // Vec<T::Shadow<'_>>
        _tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        T::merge_repeated_field(wire_type, value, buf, ctx)
    }

    #[inline]
    fn post_decode(value: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        // Vec<T::Shadow<'_>> → Vec<T>
        value.to_sun()
    }

    #[inline]
    fn clear(&mut self) {
        // Avoid recursive call to the trait method:
        Vec::<T>::clear(self);
    }
}
