use core::mem::MaybeUninit;
use std::sync::Arc;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::traits::ProtoKind;

impl<T> ProtoShadow for Arc<T>
where
    T: ProtoShadow<OwnedSun = T>,
{
    type Sun<'a> = T::Sun<'a>;
    type OwnedSun = Arc<T>;
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

impl<T> ProtoWire for Arc<T>
where
    for<'a> T: ProtoWire<EncodeInput<'a> = &'a T> + 'a,
{
    type EncodeInput<'a> = &'a Arc<T>;
    const KIND: ProtoKind = T::KIND;

    #[inline(always)]
    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
        T::encoded_len_impl(&value.as_ref())
    }

    #[inline(always)]
    fn encode_raw(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        T::encode_raw(value.as_ref(), buf);
    }

    #[inline(always)]
    fn decode_into(w: WireType, v: &mut Self, b: &mut impl Buf, c: DecodeContext) -> Result<(), DecodeError> {
        let v = Arc::get_mut(v).ok_or(DecodeError::new("Decoded Arc should be unique"))?;
        T::decode_into(w, v, b, c)
    }

    #[inline(always)]
    fn is_default(&self) -> bool {
        T::is_default(self.as_ref())
    }

    #[inline(always)]
    fn proto_default() -> Self {
        Arc::new(T::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        if let Some(v) = Arc::get_mut(self) {
            T::clear(v)
        };
    }
}

pub struct ArcedShadow<S>(pub Box<S>);

impl<T> ProtoExt for Arc<T>
where
    T: ProtoExt,
    for<'a> T: 'a,
{
    type Shadow<'a>
        = ArcedShadow<<T as ProtoExt>::Shadow<'a>>
    where
        T: 'a;

    #[inline(always)]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        T::merge_field(value.0.as_mut(), tag, wire, buf, ctx)
    }
}

impl<SHD> ProtoWire for ArcedShadow<SHD>
where
    SHD: ProtoWire,
{
    type EncodeInput<'b> = <SHD as ProtoWire>::EncodeInput<'b>;
    const KIND: ProtoKind = SHD::KIND;

    #[inline(always)]
    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
        SHD::encoded_len_impl(value)
    }

    #[inline(always)]
    fn encode_raw(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        SHD::encode_raw(value, buf);
    }

    #[inline(always)]
    fn decode_into(wt: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        SHD::decode_into(wt, &mut value.0, buf, ctx)
    }

    #[inline(always)]
    fn is_default(&self) -> bool {
        SHD::is_default(&self.0)
    }

    #[inline(always)]
    fn proto_default() -> Self {
        ArcedShadow(Box::write(Box::new_uninit(), SHD::proto_default()))
    }

    #[inline(always)]
    fn clear(&mut self) {
        SHD::clear(&mut self.0);
    }
}

impl<SHD, T> ProtoShadow for ArcedShadow<SHD>
where
    SHD: ProtoShadow<OwnedSun = T>,
{
    type Sun<'a> = SHD::Sun<'a>;
    type View<'a> = SHD::View<'a>;
    type OwnedSun = Arc<T>;

    #[inline(always)]
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

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        SHD::from_sun(value)
    }
}
