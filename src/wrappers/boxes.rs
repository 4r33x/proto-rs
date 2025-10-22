use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::traits::ProtoKind;

impl<T> ProtoShadow for Box<T>
where
    T: ProtoShadow<OwnedSun = T>,
{
    type Sun<'a> = T::Sun<'a>;
    type OwnedSun = Box<T>;
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

impl<T> ProtoWire for Box<T>
where
    for<'a> T: ProtoWire<EncodeInput<'a> = &'a T> + 'a,
{
    type EncodeInput<'a> = &'a Box<T>;
    const KIND: ProtoKind = T::KIND;

    #[inline(always)]
    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
        T::encoded_len_impl(&value.as_ref())
    }
    #[inline(always)]
    fn encode_raw(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        T::encode_raw(value, buf);
    }
    #[inline(always)]
    fn decode_into(w: WireType, v: &mut Self, b: &mut impl Buf, c: DecodeContext) -> Result<(), DecodeError> {
        T::decode_into(w, v.as_mut(), b, c)
    }
    #[inline(always)]
    fn is_default(&self) -> bool {
        T::is_default(self.as_ref())
    }
    #[inline(always)]
    fn proto_default() -> Self {
        Box::new(T::proto_default())
    }
    #[inline(always)]
    fn clear(&mut self) {
        T::clear(self.as_mut())
    }
}

impl<T> ProtoExt for Box<T>
where
    T: ProtoExt,
    for<'a> T: 'a,
{
    type Shadow<'a>
        = BoxedShadow<<T as ProtoExt>::Shadow<'a>>
    where
        T: 'a;

    #[inline(always)]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        T::merge_field(value.0.as_mut(), tag, wire, buf, ctx)
    }
}

pub struct BoxedShadow<S>(pub Box<S>);

impl<SHD> ProtoWire for BoxedShadow<SHD>
where
    SHD: ProtoWire,
{
    type EncodeInput<'b> = <SHD as ProtoWire>::EncodeInput<'b>;

    const KIND: ProtoKind = SHD::KIND;

    #[inline(always)]
    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
        <SHD as ProtoWire>::encoded_len_impl(value)
    }

    #[inline(always)]
    fn encode_raw(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        <SHD as ProtoWire>::encode_raw(value, buf);
    }

    #[inline(always)]
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        SHD::decode_into(wire_type, &mut value.0, buf, ctx)
    }

    #[inline(always)]
    fn is_default(&self) -> bool {
        SHD::is_default(&self.0)
    }

    #[inline(always)]
    fn proto_default() -> Self {
        BoxedShadow(Box::write(Box::new_uninit(), SHD::proto_default()))
    }

    #[inline(always)]
    fn clear(&mut self) {
        SHD::clear(&mut self.0);
    }
}

impl<SHD, T> ProtoShadow for BoxedShadow<SHD>
where
    SHD: ProtoShadow<OwnedSun = T>,
{
    type Sun<'a> = SHD::Sun<'a>;
    type View<'a> = SHD::View<'a>;
    type OwnedSun = Box<T>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(Box::write(Box::new_uninit(), self.0.to_sun()?))
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        SHD::from_sun(value)
    }
}
