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

impl<T> ProtoShadow<Self> for std::sync::Mutex<T>
where
    T: ProtoShadow<T, OwnedSun = T>,
    for<'a> T: 'a,
{
    type Sun<'a> = &'a std::sync::Mutex<T>;
    type OwnedSun = std::sync::Mutex<T>;
    type View<'a> = &'a std::sync::Mutex<T>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value
    }
}

impl<T> ProtoWire for std::sync::Mutex<T>
where
    for<'a> T: ProtoWire + EncodeInputFromRef<'a> + 'a,
{
    type EncodeInput<'a> = &'a std::sync::Mutex<T>;
    const KIND: ProtoKind = T::KIND;

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        let guard = value.lock().expect("Mutex lock poisoned");
        let input = T::encode_input_from_ref(&*guard);
        unsafe { T::encoded_len_impl_raw(&input) }
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        let guard = value.lock().expect("Mutex lock poisoned");
        let input = T::encode_input_from_ref(&*guard);
        T::encode_raw_unchecked(input, buf);
    }

    #[inline(always)]
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = value.get_mut().map_err(|_| DecodeError::new("Mutex lock poisoned"))?;
        T::decode_into(wire_type, inner, buf, ctx)
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        let guard = value.lock().expect("Mutex lock poisoned");
        let input = T::encode_input_from_ref(&*guard);
        T::is_default_impl(&input)
    }

    #[inline(always)]
    fn proto_default() -> Self {
        std::sync::Mutex::new(T::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        if let Ok(inner) = self.get_mut() {
            T::clear(inner);
        }
    }
}

impl<T> ProtoExt for std::sync::Mutex<T>
where
    for<'a> T: ProtoShadow<T, OwnedSun = T> + ProtoWire + EncodeInputFromRef<'a> + 'a,
{
    type Shadow<'a> = std::sync::Mutex<T>;

    #[inline(always)]
    fn merge_field(
        value: &mut Self::Shadow<'_>,
        tag: u32,
        wire: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            <std::sync::Mutex<T> as ProtoWire>::decode_into(wire, value, buf, ctx)
        } else {
            crate::encoding::skip_field(wire, tag, buf, ctx)
        }
    }
}

#[cfg(feature = "parking_lot")]
pub struct ParkingLotMutexShadow<S>(pub S);

#[cfg(feature = "parking_lot")]
impl<T> ProtoShadow<Self> for parking_lot::Mutex<T>
where
    T: ProtoShadow<T, OwnedSun = T>,
{
    type Sun<'a> = T::Sun<'a>;
    type OwnedSun = parking_lot::Mutex<T>;
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

#[cfg(feature = "parking_lot")]
impl<T> ProtoWire for parking_lot::Mutex<T>
where
    for<'a> T: ProtoWire + EncodeInputFromRef<'a> + 'a,
{
    type EncodeInput<'a> = &'a parking_lot::Mutex<T>;
    const KIND: ProtoKind = T::KIND;

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        let guard = value.lock();
        let input = T::encode_input_from_ref(&*guard);
        unsafe { T::encoded_len_impl_raw(&input) }
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        let guard = value.lock();
        let input = T::encode_input_from_ref(&*guard);
        T::encode_raw_unchecked(input, buf);
    }

    #[inline(always)]
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = value.get_mut();
        T::decode_into(wire_type, inner, buf, ctx)
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        let guard = value.lock();
        let input = T::encode_input_from_ref(&*guard);
        T::is_default_impl(&input)
    }

    #[inline(always)]
    fn proto_default() -> Self {
        parking_lot::Mutex::new(T::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        T::clear(self.get_mut());
    }
}

#[cfg(feature = "parking_lot")]
impl<T> ProtoExt for parking_lot::Mutex<T>
where
    T: ProtoExt,
    for<'a> T: 'a,
{
    type Shadow<'a>
        = ParkingLotMutexShadow<<T as ProtoExt>::Shadow<'a>>
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
        T::merge_field(&mut value.0, tag, wire, buf, ctx)
    }
}

#[cfg(feature = "parking_lot")]
impl<SHD> ProtoWire for ParkingLotMutexShadow<SHD>
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
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        SHD::decode_into(wire_type, &mut value.0, buf, ctx)
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        SHD::is_default_impl(value)
    }

    #[inline(always)]
    fn proto_default() -> Self {
        ParkingLotMutexShadow(SHD::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        SHD::clear(&mut self.0);
    }
}

#[cfg(feature = "parking_lot")]
impl<SHD, T> ProtoShadow<parking_lot::Mutex<T>> for ParkingLotMutexShadow<SHD>
where
    SHD: ProtoShadow<T, OwnedSun = T>,
{
    type Sun<'a> = SHD::Sun<'a>;
    type View<'a> = SHD::View<'a>;
    type OwnedSun = parking_lot::Mutex<T>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(parking_lot::Mutex::new(self.0.to_sun()?))
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        SHD::from_sun(value)
    }
}
