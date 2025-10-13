use bytes::{Buf, BufMut};

use crate::DecodeError;
use crate::encoding::{DecodeContext, WireType};

use fastnum::bint::UInt;

/// Split the raw coefficient digits of a decimal into their lower and upper
/// 64-bit limbs. Only the first two limbs are relevant for 128-bit decimals.
#[inline]
pub(crate) fn raw_split_digits<const N: usize>(digits: UInt<N>) -> (u64, u64) {
    let limbs = digits.digits();
    let lo = limbs.get(0).copied().unwrap_or(0);
    let hi = limbs.get(1).copied().unwrap_or(0);
    debug_assert!(limbs.iter().skip(2).all(|&digit| digit == 0));
    (lo, hi)
}

/// Cast the fractional digits count from the `fastnum` API into the proto
/// representation used on the wire.
#[inline]
pub(crate) fn fractional_digits_from_i16(count: i16) -> i32 {
    i32::from(count)
}

/// Combine two 64-bit limbs into the original 128-bit coefficient value.
#[inline]
pub(crate) fn combine_words(lo: u64, hi: u64) -> u128 {
    ((hi as u128) << 64) | (lo as u128)
}

/// Shared view over the minimal API required from both signed and unsigned
/// decimal implementations.
pub(crate) trait FastnumDecimalParts {
    fn digits_uint(&self) -> UInt<2>;
    fn fractional_count(&self) -> i16;
}

#[inline]
pub(crate) fn split_digits<T: FastnumDecimalParts>(value: &T) -> (u64, u64) {
    raw_split_digits(value.digits_uint())
}

#[inline]
pub(crate) fn fractional_digits<T: FastnumDecimalParts>(value: &T) -> i32 {
    fractional_digits_from_i16(value.fractional_count())
}

/// Minimal trait used to bridge concrete decimal types with their lightweight
/// proto representation during encode/decode passes.
pub(crate) trait DecimalLike: Sized {
    type Proto: DecimalProto<Decimal = Self> + Default + Clone;
}

/// Proto representation helper trait that knows how to initialise itself from
/// a concrete decimal value and convert back after decoding.
pub(crate) trait DecimalProto: Sized {
    type Decimal: DecimalLike<Proto = Self>;

    fn from_decimal(decimal: &Self::Decimal) -> Self;

    fn try_into_decimal(self) -> Result<Self::Decimal, DecodeError>;

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<bool, DecodeError>;

    fn encode_raw(&self, buf: &mut impl BufMut);

    fn encoded_len(&self) -> usize;
}

macro_rules! decimal_state {
    ($name:ident, $ty:ty, $with_fn:ident, $finalize_fn:ident, $clear_fn:ident) => {
        std::thread_local! {
            static $name: ::core::cell::RefCell<
                ::alloc::collections::BTreeMap<
                    *const $ty,
                    <$ty as $crate::custom_types::fastnum::common::DecimalLike>::Proto,
                >,
            > = ::core::cell::RefCell::new(::alloc::collections::BTreeMap::new());
        }

        pub(crate) fn $with_fn<F, R>(value: &mut $ty, merge: F) -> Result<R, DecodeError>
        where
            F: FnOnce(&mut <$ty as DecimalLike>::Proto) -> Result<R, DecodeError>,
        {
            let key = value as *mut $ty as *const $ty;
            $name.with(|cache| {
                let mut cache = cache.borrow_mut();
                let entry = cache.entry(key).or_insert_with(|| <$ty as DecimalLike>::Proto::from_decimal(value));
                merge(entry)
            })
        }

        pub(crate) fn $finalize_fn(value: &mut $ty) -> Result<(), DecodeError> {
            let key = value as *mut $ty as *const $ty;
            $name.with(|cache| {
                let mut cache = cache.borrow_mut();
                if let Some(proto) = cache.remove(&key) {
                    *value = proto.try_into_decimal()?;
                }
                Ok(())
            })
        }

        pub(crate) fn $clear_fn(value: &mut $ty) {
            let key = value as *mut $ty as *const $ty;
            $name.with(|cache| {
                cache.borrow_mut().remove(&key);
            });
        }
    };
}

pub(crate) use decimal_state;
