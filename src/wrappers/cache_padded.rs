use core::ops::Deref;
use core::ops::DerefMut;

use bytes::Buf;
use bytes::BufMut;
use crossbeam_utils::CachePadded;

use crate::DecodeError;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::traits::ProtoKind;

impl<T> ProtoShadow<Self> for CachePadded<T>
where
    T: ProtoShadow<T, OwnedSun = T>,
{
    type Sun<'a> = T::Sun<'a>;
    type OwnedSun = CachePadded<T>;
    type View<'a> = T::View<'a>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        T::from_sun(value)
    }
}

impl<T> ProtoWire for CachePadded<T>
where
    for<'a> T: ProtoWire<EncodeInput<'a> = &'a T> + 'a,
{
    type EncodeInput<'a> = &'a CachePadded<T>;
    const KIND: ProtoKind = T::KIND;

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        let inner: &T = (*value).deref();
        unsafe { T::encoded_len_impl_raw(&inner) }
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        T::encode_raw_unchecked(value, buf);
    }

    #[inline(always)]
    fn decode_into(w: WireType, v: &mut Self, b: &mut impl Buf, c: DecodeContext) -> Result<(), DecodeError> {
        let inner: &mut T = v.deref_mut();
        T::decode_into(w, inner, b, c)
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        let inner: &T = (*value).deref();
        T::is_default_impl(&inner)
    }

    #[inline(always)]
    fn proto_default() -> Self {
        CachePadded::new(T::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        let inner: &mut T = self.deref_mut();
        T::clear(inner);
    }
}

impl<T> ProtoExt for CachePadded<T>
where
    T: ProtoExt,
    for<'a> T: 'a,
{
    type Shadow<'a>
        = CachePaddedShadow<<T as ProtoExt>::Shadow<'a>>
    where
        T: 'a;

    #[inline(always)]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner: &mut <T as ProtoExt>::Shadow<'_> = value.0.deref_mut();
        T::merge_field(inner, tag, wire, buf, ctx)
    }
}

pub struct CachePaddedShadow<S>(pub CachePadded<S>);

impl<SHD> ProtoWire for CachePaddedShadow<SHD>
where
    SHD: ProtoWire,
{
    type EncodeInput<'b> = <SHD as ProtoWire>::EncodeInput<'b>;
    const KIND: ProtoKind = SHD::KIND;

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        unsafe { SHD::encoded_len_impl_raw(value) }
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        SHD::encode_raw_unchecked(value, buf);
    }

    #[inline(always)]
    fn decode_into(wt: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner: &mut SHD = value.0.deref_mut();
        SHD::decode_into(wt, inner, buf, ctx)
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        SHD::is_default_impl(value)
    }

    #[inline(always)]
    fn proto_default() -> Self {
        CachePaddedShadow(CachePadded::new(SHD::proto_default()))
    }

    #[inline(always)]
    fn clear(&mut self) {
        let inner: &mut SHD = self.0.deref_mut();
        SHD::clear(inner);
    }
}

impl<SHD, T> ProtoShadow<CachePadded<T>> for CachePaddedShadow<SHD>
where
    SHD: ProtoShadow<T, OwnedSun = T>,
{
    type Sun<'a> = SHD::Sun<'a>;
    type View<'a> = SHD::View<'a>;
    type OwnedSun = CachePadded<T>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        let inner_shadow = self.0.into_inner();
        let value = inner_shadow.to_sun()?;
        Ok(CachePadded::new(value))
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        SHD::from_sun(value)
    }
}
