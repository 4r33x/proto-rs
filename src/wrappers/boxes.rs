use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::EncodeInputFromRef;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::traits::ProtoKind;

impl<T> ProtoShadow<Self> for Box<T>
where
    T: ProtoShadow<T, OwnedSun = T>,
    for<'a> <T as ProtoShadow<T>>::Sun<'a>: crate::traits::SunFromRefValue<'a, T, Output = <T as ProtoShadow<T>>::Sun<'a>>,
{
    type Sun<'a> = &'a Box<T>;
    type OwnedSun = Box<T>;
    type View<'a> = &'a Box<T>;
    type ProtoArchive = Box<T::ProtoArchive>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value
    }

    #[inline(always)]
    fn to_archive(value: Self::View<'_>) -> Self::ProtoArchive {
        let inner_sun = <<T as ProtoShadow<T>>::Sun<'_> as crate::traits::SunFromRefValue<'_, T>>::sun_from_ref(value.as_ref());
        let inner_view = T::from_sun(inner_sun);
        Box::new(T::to_archive(inner_view))
    }
}

impl<T> ProtoWire for Box<T>
where
    for<'a> T: ProtoWire + EncodeInputFromRef<'a> + 'a,
{
    type EncodeInput<'a> = &'a Box<T>;
    const KIND: ProtoKind = T::KIND;

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        let input = T::encode_input_from_ref((*value).as_ref());
        unsafe { T::encoded_len_impl_raw(&input) }
    }
    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        let input = T::encode_input_from_ref(value.as_ref());
        T::encode_raw_unchecked(input, buf);
    }
    #[inline(always)]
    fn decode_into(w: WireType, v: &mut Self, b: &mut impl Buf, c: DecodeContext) -> Result<(), DecodeError> {
        T::decode_into(w, v.as_mut(), b, c)
    }
    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        let input = T::encode_input_from_ref((*value).as_ref());
        T::is_default_impl(&input)
    }
    #[inline(always)]
    fn proto_default() -> Self {
        Box::new(T::proto_default())
    }
    #[inline(always)]
    fn clear(&mut self) {
        T::clear(self.as_mut());
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

pub struct BoxedShadow<S>(pub Box<S>);

impl<SHD> ProtoWire for BoxedShadow<SHD>
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
        <SHD as ProtoWire>::encode_raw_unchecked(value, buf);
    }

    #[inline(always)]
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        SHD::decode_into(wire_type, &mut value.0, buf, ctx)
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        SHD::is_default_impl(value)
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

impl<SHD, T> ProtoShadow<Box<T>> for BoxedShadow<SHD>
where
    SHD: ProtoShadow<T, OwnedSun = T>,
{
    type Sun<'a> = SHD::Sun<'a>;
    type View<'a> = SHD::View<'a>;
    type OwnedSun = Box<T>;
    type ProtoArchive = Box<SHD::ProtoArchive>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(Box::write(Box::new_uninit(), self.0.to_sun()?))
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        SHD::from_sun(value)
    }

    #[inline(always)]
    fn to_archive(value: Self::View<'_>) -> Self::ProtoArchive {
        Box::new(SHD::to_archive(value))
    }
}
