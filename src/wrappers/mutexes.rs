use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::traits::ProtoKind;

impl<T> ProtoShadow<Self> for std::sync::Mutex<T>
where
    T: ProtoShadow<T, OwnedSun = T>,
{
    type Sun<'a> = T::Sun<'a>;
    type OwnedSun = std::sync::Mutex<T>;
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

impl<T> ProtoWire for std::sync::Mutex<T>
where
    for<'a> T: ProtoWire<EncodeInput<'a> = &'a T> + 'a,
{
    type EncodeInput<'a> = &'a std::sync::Mutex<T>;
    const KIND: ProtoKind = T::KIND;

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        let guard = value.lock().expect("Mutex lock poisoned");
        unsafe { T::encoded_len_impl_raw(&(&*guard)) }
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        let guard = value.lock().expect("Mutex lock poisoned");
        T::encode_raw_unchecked(&*guard, buf);
    }

    #[inline(always)]
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = value.get_mut().map_err(|_| DecodeError::new("Mutex lock poisoned"))?;
        T::decode_into(wire_type, inner, buf, ctx)
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        let guard = value.lock().expect("Mutex lock poisoned");
        T::is_default_impl(&(&*guard))
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

pub struct StdMutexShadow<S>(pub std::sync::Mutex<S>);

impl<T> ProtoExt for std::sync::Mutex<T>
where
    T: ProtoExt,
    for<'a> T: 'a,
{
    type Shadow<'a>
        = StdMutexShadow<<T as ProtoExt>::Shadow<'a>>
    where
        T: 'a;

    #[inline(always)]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = value.0.get_mut().map_err(|_| DecodeError::new("Mutex lock poisoned"))?;
        T::merge_field(inner, tag, wire, buf, ctx)
    }
}

impl<SHD> ProtoWire for StdMutexShadow<SHD>
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
        let inner = value.0.get_mut().map_err(|_| DecodeError::new("Mutex lock poisoned"))?;
        SHD::decode_into(wire_type, inner, buf, ctx)
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        SHD::is_default_impl(value)
    }

    #[inline(always)]
    fn proto_default() -> Self {
        StdMutexShadow(std::sync::Mutex::new(SHD::proto_default()))
    }

    #[inline(always)]
    fn clear(&mut self) {
        if let Ok(inner) = self.0.get_mut() {
            SHD::clear(inner);
        }
    }
}

impl<SHD, T> ProtoShadow<std::sync::Mutex<T>> for StdMutexShadow<SHD>
where
    SHD: ProtoShadow<T, OwnedSun = T>,
{
    type Sun<'a> = SHD::Sun<'a>;
    type View<'a> = SHD::View<'a>;
    type OwnedSun = std::sync::Mutex<T>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        let inner = self.0.into_inner().map_err(|_| DecodeError::new("Mutex lock poisoned"))?;
        Ok(std::sync::Mutex::new(inner.to_sun()?))
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        SHD::from_sun(value)
    }
}

#[cfg(feature = "parking_lot")]
pub struct ParkingLotMutexShadow<S>(pub parking_lot::Mutex<S>);

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
    for<'a> T: ProtoWire<EncodeInput<'a> = &'a T> + 'a,
{
    type EncodeInput<'a> = &'a parking_lot::Mutex<T>;
    const KIND: ProtoKind = T::KIND;

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        let guard = value.lock();
        unsafe { T::encoded_len_impl_raw(&(&*guard)) }
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        let guard = value.lock();
        T::encode_raw_unchecked(&*guard, buf);
    }

    #[inline(always)]
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = value.get_mut();
        T::decode_into(wire_type, inner, buf, ctx)
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        let guard = value.lock();
        T::is_default_impl(&(&*guard))
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
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = value.0.get_mut();
        T::merge_field(inner, tag, wire, buf, ctx)
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
        let inner = value.0.get_mut();
        SHD::decode_into(wire_type, inner, buf, ctx)
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        SHD::is_default_impl(value)
    }

    #[inline(always)]
    fn proto_default() -> Self {
        ParkingLotMutexShadow(parking_lot::Mutex::new(SHD::proto_default()))
    }

    #[inline(always)]
    fn clear(&mut self) {
        SHD::clear(self.0.get_mut());
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
        Ok(parking_lot::Mutex::new(self.0.into_inner().to_sun()?))
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        SHD::from_sun(value)
    }
}
