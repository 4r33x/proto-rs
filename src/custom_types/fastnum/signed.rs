use core::convert::TryInto;

use fastnum::D64;
use fastnum::D128;

use crate::DecodeError;
use crate::ProtoShadowDecode;
use crate::proto_message;

trait FastnumSignedParts {
    fn digits_u128(&self) -> u128;
    fn fractional_digits_count_i32(&self) -> i32;
    fn is_negative(&self) -> bool;
}

impl FastnumSignedParts for D128 {
    #[inline(always)]
    fn digits_u128(&self) -> u128 {
        self.digits()
            .try_into()
            .expect("D128 digits should fit in u128")
    }

    #[inline(always)]
    fn fractional_digits_count_i32(&self) -> i32 {
        i32::from(self.fractional_digits_count())
    }

    #[inline(always)]
    fn is_negative(&self) -> bool {
        self.is_sign_negative()
    }
}

impl FastnumSignedParts for D64 {
    #[inline(always)]
    fn digits_u128(&self) -> u128 {
        self.digits()
            .try_into()
            .expect("D64 digits should fit in u128")
    }

    #[inline(always)]
    fn fractional_digits_count_i32(&self) -> i32 {
        i32::from(self.fractional_digits_count())
    }

    #[inline(always)]
    fn is_negative(&self) -> bool {
        self.is_sign_negative()
    }
}

#[inline(always)]
fn fastnum_lo<T: FastnumSignedParts>(value: &T) -> u64 {
    value.digits_u128() as u64
}

#[inline(always)]
fn fastnum_hi<T: FastnumSignedParts>(value: &T) -> u64 {
    (value.digits_u128() >> 64) as u64
}

#[inline(always)]
fn fastnum_fractional_digits_count<T: FastnumSignedParts>(value: &T) -> i32 {
    value.fractional_digits_count_i32()
}

#[inline(always)]
fn fastnum_is_negative<T: FastnumSignedParts>(value: &T) -> bool {
    value.is_negative()
}

#[proto_message(proto_path = "protos/fastnum.proto", sun = [D128, D64])]
pub struct D128Proto {
    #[proto(tag = 1, getter = "fastnum_lo($)")]
    /// Lower 64 bits of the digits
    pub lo: u64,
    #[proto(tag = 2, getter = "fastnum_hi($)")]
    /// Upper 64 bits of the digits
    pub hi: u64,
    #[proto(tag = 3, getter = "fastnum_fractional_digits_count($)")]
    /// Fractional digits count (can be negative for scientific notation)
    pub fractional_digits_count: i32,
    #[proto(tag = 4, getter = "fastnum_is_negative($)")]
    /// Sign bit: true for negative, false for positive/zero
    pub is_negative: bool,
}

impl ProtoShadowDecode<D128> for D128Proto {
    fn to_sun(self) -> Result<D128, DecodeError> {
        let digits = ((self.hi as u128) << 64) | (self.lo as u128);

        let mut result = D128::from_u128(digits).map_err(|err| DecodeError::new(err.to_string()))?;

        if self.fractional_digits_count > 0 {
            result /= D128::TEN.powi(self.fractional_digits_count);
        } else if self.fractional_digits_count < 0 {
            result *= D128::TEN.powi(-self.fractional_digits_count);
        }

        if self.is_negative {
            result = -result;
        }

        Ok(result)
    }
}

impl ProtoShadowDecode<D64> for D128Proto {
    fn to_sun(self) -> Result<D64, DecodeError> {
        let mut result = D64::from_u64(self.lo);

        if self.fractional_digits_count > 0 {
            result /= D64::TEN.powi(self.fractional_digits_count);
        } else if self.fractional_digits_count < 0 {
            result *= D64::TEN.powi(-self.fractional_digits_count);
        }
        if self.is_negative {
            result = -result;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use fastnum::dec128;

    use super::*;
    use crate::ProtoShadowEncode;

    #[allow(dead_code)]
    #[proto_message(proto_path = "protos/fastnum_test.proto")]
    struct D128Wrapper {
        inner: D128,
    }

    fn encode(value: &D128) -> D128Proto {
        <D128Proto as ProtoShadowEncode<'_, D128>>::from_sun(value)
    }

    fn decode(proto: D128Proto) -> D128 {
        ProtoShadowDecode::<D128>::to_sun(proto).unwrap()
    }

    #[test]
    fn test_roundtrip() {
        let original = dec128!(123456789.987654321);
        let proto = encode(&original);
        let restored = decode(proto);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_negative_value() {
        let val = dec128!(-123.45);
        let proto = encode(&val);
        assert!(proto.is_negative);
        assert_eq!(proto.fractional_digits_count, 2);

        let restored = decode(proto);
        assert_eq!(val, restored);
    }

    #[test]
    fn test_positive_value() {
        let val = dec128!(123.45);
        let proto = encode(&val);
        assert!(!proto.is_negative);

        let restored = decode(proto);
        assert_eq!(val, restored);
    }

    #[test]
    fn test_fractional_digits() {
        // Test case from docs: 123.45 has 2 fractional digits
        let val = dec128!(123.45);
        let proto = encode(&val);
        assert_eq!(proto.fractional_digits_count, 2);

        let restored = decode(proto);
        assert_eq!(val, restored);
    }

    #[test]
    fn test_scientific_notation() {
        // Test case: 5e9 has -9 fractional digits
        let val = dec128!(5e9);
        let proto = encode(&val);
        let restored = decode(proto);
        assert_eq!(val, restored);
    }

    #[test]
    fn test_negative_scientific() {
        let val = dec128!(-5e9);
        let proto = encode(&val);
        assert!(proto.is_negative);

        let restored = decode(proto);
        assert_eq!(val, restored);
    }

    #[test]
    fn test_no_fractional_part() {
        let val = dec128!(12345);
        let proto = encode(&val);
        assert_eq!(proto.fractional_digits_count, 0);

        let restored = decode(proto);
        assert_eq!(val, restored);
    }

    #[test]
    fn test_small_fractional() {
        // Test case: 0.0000012345 has 10 fractional digits
        let val = dec128!(0.0000012345);
        let proto = encode(&val);
        assert_eq!(proto.fractional_digits_count, 10);

        let restored = decode(proto);
        assert_eq!(val, restored);
    }

    #[test]
    fn test_max_value() {
        let max_val = D128::MAX;
        let proto = encode(&max_val);
        let restored = decode(proto);
        assert_eq!(max_val, restored);
    }

    #[test]
    fn test_min_value() {
        let min_val = D128::MIN;
        let proto = encode(&min_val);
        assert!(proto.is_negative);

        let restored = decode(proto);
        assert_eq!(min_val, restored);
    }

    #[test]
    fn test_zero() {
        let zero = D128::ZERO;
        let proto = encode(&zero);
        assert!(!proto.is_negative);

        let restored = decode(proto);
        assert_eq!(zero, restored);
    }

    #[test]
    fn test_negative_zero() {
        let neg_zero = dec128!(-0.0);
        let proto = encode(&neg_zero);

        let restored = decode(proto);
        assert_eq!(neg_zero, restored);
    }

    #[test]
    fn test_proto_fields() {
        // Verify proto structure for -123.45
        let val = dec128!(-123.45);
        let proto = encode(&val);

        // digits = 12345 (absolute value), fractional_count = 2, negative = true
        let digits = ((proto.hi as u128) << 64) | (proto.lo as u128);
        assert_eq!(digits, 12345);
        assert_eq!(proto.fractional_digits_count, 2);
        assert!(proto.is_negative);
    }
}
