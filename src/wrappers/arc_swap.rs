use std::sync::Arc;

use arc_swap::ArcSwap;
use arc_swap::ArcSwapOption;
use bytes::Buf;
use bytes::BufMut;

use super::arcs::ArcedShadow;
use crate::DecodeError;
use crate::EncodeInputFromRef;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::traits::ProtoKind;

impl<T> ProtoShadow<Self> for ArcSwap<T>
where
    T: ProtoShadow<T, OwnedSun = T>,
{
    type Sun<'a> = T::Sun<'a>;
    type OwnedSun = ArcSwap<T>;
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

impl<T> ProtoWire for ArcSwap<T>
where
    for<'a> T: ProtoWire + EncodeInputFromRef<'a> + 'a,
{
    type EncodeInput<'a> = &'a ArcSwap<T>;
    const KIND: ProtoKind = <T as ProtoWire>::KIND;

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        let guard = value.load();
        let input = <T as EncodeInputFromRef<'_>>::encode_input_from_ref(&guard);
        unsafe { <T as ProtoWire>::encoded_len_impl_raw(&input) }
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        let guard = value.load();
        let input = <T as EncodeInputFromRef<'_>>::encode_input_from_ref(&guard);
        <T as ProtoWire>::encode_raw_unchecked(input, buf);
    }

    #[inline(always)]
    fn decode_into(wire: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let mut inner = <Arc<T> as ProtoWire>::proto_default();
        <Arc<T> as ProtoWire>::decode_into(wire, &mut inner, buf, ctx)?;
        value.store(inner);
        Ok(())
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        let guard = value.load();
        let input = <T as EncodeInputFromRef<'_>>::encode_input_from_ref(&guard);
        <T as ProtoWire>::is_default_impl(&input)
    }

    #[inline(always)]
    fn proto_default() -> Self {
        ArcSwap::from_pointee(T::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.store(<Arc<T> as ProtoWire>::proto_default());
    }
}

impl<T> ProtoExt for ArcSwap<T>
where
    T: ProtoExt,
    for<'a> T: 'a,
{
    type Shadow<'a>
        = ArcedShadow<<T as ProtoExt>::Shadow<'a>>
    where
        T: 'a;

    #[inline(always)]
    fn merge_field(
        value: &mut Self::Shadow<'_>,
        tag: u32,
        wire: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        T::merge_field(value.0.as_mut(), tag, wire, buf, ctx)
    }
}

impl<SHD, T> ProtoShadow<ArcSwap<T>> for ArcedShadow<SHD>
where
    SHD: ProtoShadow<T, OwnedSun = T>,
{
    type Sun<'a> = SHD::Sun<'a>;
    type View<'a> = SHD::View<'a>;
    type OwnedSun = ArcSwap<T>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        let arc_value = <ArcedShadow<SHD> as ProtoShadow<Arc<T>>>::to_sun(self)?;
        Ok(ArcSwap::new(arc_value))
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        SHD::from_sun(value)
    }
}

impl<T> ProtoShadow<Self> for ArcSwapOption<T>
where
    T: ProtoShadow<T, OwnedSun = T>,
{
    type Sun<'a> = Option<T::Sun<'a>>;
    type OwnedSun = ArcSwapOption<T>;
    type View<'a> = Option<T::View<'a>>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value.map(T::from_sun)
    }
}

impl<T> ProtoWire for ArcSwapOption<T>
where
    for<'a> T: ProtoWire + EncodeInputFromRef<'a> + 'a,
{
    type EncodeInput<'a> = &'a ArcSwapOption<T>;
    const KIND: ProtoKind = <Arc<T> as ProtoWire>::KIND;

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        let guard = value.load();
        match guard.as_ref() {
            Some(inner) => {
                let input = <T as EncodeInputFromRef<'_>>::encode_input_from_ref(inner.as_ref());
                unsafe { <T as ProtoWire>::encoded_len_impl_raw(&input) }
            }
            None => unreachable!(),
        }
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        let guard = value.load();
        if let Some(inner) = guard.as_ref() {
            let input = <T as EncodeInputFromRef<'_>>::encode_input_from_ref(inner.as_ref());
            <T as ProtoWire>::encode_raw_unchecked(input, buf);
        }
    }

    #[inline(always)]
    fn decode_into(wire: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let mut inner = None;
        Option::<Arc<T>>::decode_into(wire, &mut inner, buf, ctx)?;
        value.store(inner);
        Ok(())
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        let guard = value.load();
        guard.as_ref().is_none_or(|inner| {
            let input = <T as EncodeInputFromRef<'_>>::encode_input_from_ref(inner.as_ref());
            <T as ProtoWire>::is_default_impl(&input)
        })
    }

    #[inline(always)]
    fn proto_default() -> Self {
        ArcSwapOption::new(None)
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.store(None);
    }
}

impl<T> ProtoExt for ArcSwapOption<T>
where
    T: ProtoExt,
    for<'a> T: 'a,
{
    type Shadow<'a>
        = Option<ArcedShadow<<T as ProtoExt>::Shadow<'a>>>
    where
        T: 'a;

    #[inline(always)]
    fn merge_field(
        value: &mut Self::Shadow<'_>,
        tag: u32,
        wire: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        let inner = value.get_or_insert_with(ArcedShadow::proto_default);
        T::merge_field(inner.0.as_mut(), tag, wire, buf, ctx)
    }
}

impl<SHD, T> ProtoShadow<ArcSwapOption<T>> for Option<ArcedShadow<SHD>>
where
    SHD: ProtoShadow<T, OwnedSun = T>,
{
    type Sun<'a> = Option<SHD::Sun<'a>>;
    type View<'a> = Option<SHD::View<'a>>;
    type OwnedSun = ArcSwapOption<T>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        let value = self.map(<ArcedShadow<SHD> as ProtoShadow<Arc<T>>>::to_sun).transpose()?;
        Ok(ArcSwapOption::new(value))
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value.map(SHD::from_sun)
    }
}
