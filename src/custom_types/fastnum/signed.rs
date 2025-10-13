use core::cmp::Ordering;

use fastnum::D128;

use crate::ProtoExt;
use crate::proto_dump;
extern crate self as proto_rs;

//DO NOT USE IT FOR ENCODE\DECODE
#[proto_dump(proto_path = "protos/fastnum.proto")]
pub struct D128Proto {
    #[proto(tag = 1)]
    /// Lower 64 bits of the digits
    pub lo: u64,
    #[proto(tag = 2)]
    /// Upper 64 bits of the digits
    pub hi: u64,
    #[proto(tag = 3)]
    /// Fractional digits count (can be negative for scientific notation)
    pub fractional_digits_count: i32,
    #[proto(tag = 4)]
    /// Sign bit: true for negative, false for positive/zero
    pub is_negative: bool,
}

impl ProtoExt for D128 {
    fn proto_default() -> Self
    where
        Self: Sized,
    {
        D128::ZERO
    }

    fn encode_raw(&self, buf: &mut impl bytes::BufMut)
    where
        Self: Sized,
    {
        let (lo, hi) = split_digits(self);
        let fractional_digits_count = fractional_digits(self);
        let is_negative = self.is_sign_negative();

        crate::encoding::uint64::encode(1, &lo, buf);
        crate::encoding::uint64::encode(2, &hi, buf);
        crate::encoding::int32::encode(3, &fractional_digits_count, buf);
        crate::encoding::bool::encode(4, &is_negative, buf);
    }

    fn merge_field(&mut self, tag: u32, wire_type: crate::encoding::WireType, buf: &mut impl bytes::Buf, ctx: crate::encoding::DecodeContext) -> Result<(), crate::DecodeError>
    where
        Self: Sized,
    {
        let (mut lo, mut hi) = split_digits(self);
        let mut fractional_digits_count = fractional_digits(self);
        let mut is_negative = self.is_sign_negative();
        let handled = match tag {
            1 => {
                crate::encoding::uint64::merge(wire_type, &mut lo, buf, ctx)?;
                true
            }
            2 => {
                crate::encoding::uint64::merge(wire_type, &mut hi, buf, ctx)?;
                true
            }
            3 => {
                crate::encoding::int32::merge(wire_type, &mut fractional_digits_count, buf, ctx)?;
                true
            }
            4 => {
                crate::encoding::bool::merge(wire_type, &mut is_negative, buf, ctx)?;
                true
            }
            _ => false,
        };

        if handled {
            *self = decode_decimal(lo, hi, fractional_digits_count, is_negative)?;
            Ok(())
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        let (lo, hi) = split_digits(self);
        let fractional_digits_count = fractional_digits(self);
        let is_negative = self.is_sign_negative();

        crate::encoding::uint64::encoded_len(1, &lo)
            + crate::encoding::uint64::encoded_len(2, &hi)
            + crate::encoding::int32::encoded_len(3, &fractional_digits_count)
            + crate::encoding::bool::encoded_len(4, &is_negative)
    }

    fn clear(&mut self) {
        *self = D128::ZERO;
    }
}

impl crate::MessageField for D128 {}

fn split_digits(value: &D128) -> (u64, u64) {
    let digits = value.digits();
    let limbs = digits.digits();
    let lo = limbs.get(0).copied().unwrap_or(0);
    let hi = limbs.get(1).copied().unwrap_or(0);
    debug_assert!(limbs.iter().skip(2).all(|&digit| digit == 0));
    (lo, hi)
}

fn fractional_digits(value: &D128) -> i32 {
    i32::from(value.fractional_digits_count())
}

#[inline]
fn combine_words(lo: u64, hi: u64) -> u128 {
    ((hi as u128) << 64) | (lo as u128)
}

fn decode_decimal(lo: u64, hi: u64, fractional_digits_count: i32, is_negative: bool) -> Result<D128, crate::DecodeError> {
    let digits = combine_words(lo, hi);
    let mut value = D128::from_u128(digits).map_err(|err| crate::DecodeError::new(err.to_string()))?;

    match fractional_digits_count.cmp(&0) {
        Ordering::Greater => {
            value = value / D128::TEN.powi(fractional_digits_count);
        }
        Ordering::Less => {
            value = value * D128::TEN.powi(-fractional_digits_count);
        }
        Ordering::Equal => {}
    }

    if is_negative {
        value = value.neg();
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use fastnum::dec128;

    use super::*;
    use crate::ProtoExt;

    #[test]
    fn test_roundtrip() {
        let original = dec128!(123456789.987654321);
        let encoded = original.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_negative_value() {
        let val = dec128!(-123.45);
        let encoded = val.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert!(restored.is_sign_negative());
        assert_eq!(restored.fractional_digits_count(), val.fractional_digits_count());
        assert_eq!(val, restored);
    }

    #[test]
    fn test_positive_value() {
        let val = dec128!(123.45);
        let encoded = val.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert!(!restored.is_sign_negative());
        assert_eq!(restored.fractional_digits_count(), val.fractional_digits_count());
        assert_eq!(val, restored);
    }

    #[test]
    fn test_fractional_digits() {
        // Test case from docs: 123.45 has 2 fractional digits
        let val = dec128!(123.45);
        let encoded = val.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert_eq!(restored.fractional_digits_count(), val.fractional_digits_count());
        assert_eq!(val, restored);
    }

    #[test]
    fn test_scientific_notation() {
        // Test case: 5e9 has -9 fractional digits
        let val = dec128!(5e9);
        let encoded = val.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_negative_scientific() {
        let val = dec128!(-5e9);
        let encoded = val.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert!(restored.is_sign_negative());
        assert_eq!(val, restored);
    }

    #[test]
    fn test_no_fractional_part() {
        let val = dec128!(12345);
        let encoded = val.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert_eq!(restored.fractional_digits_count(), val.fractional_digits_count());
        assert_eq!(val, restored);
    }

    #[test]
    fn test_small_fractional() {
        // Test case: 0.0000012345 has 10 fractional digits
        let val = dec128!(0.0000012345);
        let encoded = val.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert_eq!(restored.fractional_digits_count(), val.fractional_digits_count());
        assert_eq!(val, restored);
    }

    #[test]
    fn test_max_value() {
        let max_val = D128::MAX;
        let encoded = max_val.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert_eq!(max_val, restored);
    }

    #[test]
    fn test_min_value() {
        let min_val = D128::MIN;
        let encoded = min_val.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert!(restored.is_sign_negative());
        assert_eq!(min_val, restored);
    }

    #[test]
    fn test_zero() {
        let zero = D128::ZERO;
        let encoded = zero.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert!(!restored.is_sign_negative());
        assert_eq!(zero, restored);
    }

    #[test]
    fn test_negative_zero() {
        let neg_zero = dec128!(-0.0);
        let encoded = neg_zero.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert_eq!(neg_zero, restored);
    }

    #[test]
    fn test_proto_fields() {
        // Verify proto structure for -123.45
        let val = dec128!(-123.45);
        let (lo, hi) = split_digits(&val);
        let digits = combine_words(lo, hi);
        assert_eq!(digits, 12345);
        assert_eq!(fractional_digits(&val), 2);
        assert!(val.is_sign_negative());
    }
}
