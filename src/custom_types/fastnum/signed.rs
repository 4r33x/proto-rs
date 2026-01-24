use core::convert::TryInto;

use fastnum::D64;
use fastnum::D128;

use crate::DecodeError;
use crate::ProtoShadowDecode;
use crate::ProtoShadowEncode;
use crate::proto_message;

#[proto_message(proto_path = "protos/fastnum.proto", sun = [D128, D64])]
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

impl<'a> ProtoShadowEncode<'a, D128> for D128Proto {
    fn from_sun(value: &'a D128) -> Self {
        let digits: u128 = value.digits().try_into().expect("Should be safe as D128 should have u128 capacity");
        let lo = digits as u64;
        let hi = (digits >> 64) as u64;

        Self {
            lo,
            hi,
            fractional_digits_count: i32::from(value.fractional_digits_count()),
            is_negative: value.is_sign_negative(),
        }
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

impl<'a> ProtoShadowEncode<'a, D64> for D128Proto {
    fn from_sun(value: &'a D64) -> Self {
        let digits: u128 = value.digits().try_into().expect("Should be safe as D128 should have u128 capacity");
        let lo = digits as u64;

        Self {
            lo,
            hi: 0,
            fractional_digits_count: i32::from(value.fractional_digits_count()),
            is_negative: value.is_sign_negative(),
        }
    }
}

#[cfg(test)]
mod tests {
    use fastnum::dec128;

    use super::*;

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
