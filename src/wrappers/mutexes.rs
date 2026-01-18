use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::EncodeInputFromRef;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::traits::EncodeInputFromRefValue;
use crate::traits::ProtoKind;

pub struct MutexGuardEncodeInput<G>(G);

impl<G> MutexGuardEncodeInput<G> {
    #[inline(always)]
    fn new(guard: G) -> Self {
        Self(guard)
    }
}

type StdMutexEncodeInput<'a, T> = MutexGuardEncodeInput<std::sync::MutexGuard<'a, T>>;

impl<'a, T> EncodeInputFromRefValue<'a, std::sync::Mutex<T>> for StdMutexEncodeInput<'a, T> {
    type Output = StdMutexEncodeInput<'a, T>;

    #[inline(always)]
    fn encode_input_from_ref(value: &'a std::sync::Mutex<T>) -> Self::Output {
        MutexGuardEncodeInput::new(value.lock().expect("Mutex lock poisoned"))
    }
}

impl<T> ProtoShadow<Self> for std::sync::Mutex<T>
where
    for<'a> T: 'a,
    T: ProtoShadow<T, OwnedSun = T>,
    for<'a> <T as ProtoShadow<T>>::Sun<'a>: crate::traits::SunFromRefValue<'a, T, Output = <T as ProtoShadow<T>>::Sun<'a>>,
{
    type Sun<'a> = StdMutexEncodeInput<'a, T>;
    type OwnedSun = std::sync::Mutex<T>;
    type View<'a> = StdMutexEncodeInput<'a, T>;
    type ProtoArchive = T::ProtoArchive;

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
        let inner: &T = &value.0;
        let inner_sun = <<T as ProtoShadow<T>>::Sun<'_> as crate::traits::SunFromRefValue<'_, T>>::sun_from_ref(inner);
        let inner_view = T::from_sun(inner_sun);
        T::to_archive(inner_view)
    }
}

impl<T> ProtoWire for std::sync::Mutex<T>
where
    for<'a> T: ProtoWire + EncodeInputFromRef<'a> + 'a,
{
    type EncodeInput<'a> = StdMutexEncodeInput<'a, T>;
    const KIND: ProtoKind = T::KIND;

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        let input = T::encode_input_from_ref(&value.0);
        unsafe { T::encoded_len_impl_raw(&input) }
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        let input = T::encode_input_from_ref(&value.0);
        T::encode_raw_unchecked(input, buf);
    }

    #[inline(always)]
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = value.get_mut().map_err(|_| DecodeError::new("Mutex lock poisoned"))?;
        T::decode_into(wire_type, inner, buf, ctx)
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        let input = T::encode_input_from_ref(&value.0);
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

#[cfg(feature = "parking_lot")]
type ParkingLotMutexEncodeInput<'a, T> = MutexGuardEncodeInput<parking_lot::MutexGuard<'a, T>>;

#[cfg(feature = "parking_lot")]
impl<'a, T> EncodeInputFromRefValue<'a, parking_lot::Mutex<T>> for ParkingLotMutexEncodeInput<'a, T> {
    type Output = ParkingLotMutexEncodeInput<'a, T>;

    #[inline(always)]
    fn encode_input_from_ref(value: &'a parking_lot::Mutex<T>) -> Self::Output {
        MutexGuardEncodeInput::new(value.lock())
    }
}

#[cfg(feature = "parking_lot")]
impl<T> ProtoShadow<Self> for parking_lot::Mutex<T>
where
    T: ProtoShadow<T, OwnedSun = T>,
    for<'a> <T as ProtoShadow<T>>::Sun<'a>: crate::traits::SunFromRefValue<'a, T, Output = <T as ProtoShadow<T>>::Sun<'a>>,
{
    type Sun<'a> = ParkingLotMutexEncodeInput<'a, T>;
    type OwnedSun = parking_lot::Mutex<T>;
    type View<'a> = ParkingLotMutexEncodeInput<'a, T>;
    type ProtoArchive = T::ProtoArchive;

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
        let inner: &T = &value.0;
        let inner_sun = <<T as ProtoShadow<T>>::Sun<'_> as crate::traits::SunFromRefValue<'_, T>>::sun_from_ref(inner);
        let inner_view = T::from_sun(inner_sun);
        T::to_archive(inner_view)
    }
}

#[cfg(feature = "parking_lot")]
impl<T> ProtoWire for parking_lot::Mutex<T>
where
    for<'a> T: ProtoWire + EncodeInputFromRef<'a> + 'a,
{
    type EncodeInput<'a> = ParkingLotMutexEncodeInput<'a, T>;
    const KIND: ProtoKind = T::KIND;

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        let input = T::encode_input_from_ref(&value.0);
        unsafe { T::encoded_len_impl_raw(&input) }
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        let input = T::encode_input_from_ref(&value.0);
        T::encode_raw_unchecked(input, buf);
    }

    #[inline(always)]
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = value.get_mut();
        T::decode_into(wire_type, inner, buf, ctx)
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        let input = T::encode_input_from_ref(&value.0);
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
