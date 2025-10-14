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
