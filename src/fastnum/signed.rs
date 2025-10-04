use fastnum::D128;
use prosto_derive::proto_dump;

use crate::HasProto;

#[proto_dump(file = "protos/fastnum.proto")]
#[derive(prost::Message, Clone, PartialEq, Copy)]
pub struct D128Proto {
    #[prost(uint64, tag = 1)]
    /// Lower 64 bits of the digits
    pub lo: u64,
    #[prost(uint64, tag = 2)]
    /// Upper 64 bits of the digits
    pub hi: u64,
    #[prost(int32, tag = 3)]
    /// Fractional digits count (can be negative for scientific notation)
    pub fractional_digits_count: i32,
    #[prost(bool, tag = 4)]
    /// Sign bit: true for negative, false for positive/zero
    pub is_negative: bool,
}

impl HasProto for D128 {
    type Proto = D128Proto;

    fn to_proto(&self) -> Self::Proto {
        let digits: u128 = self.digits().try_into().expect("Should be safe as D128 should have u128 capacity");
        let lo = digits as u64;
        let hi = (digits >> 64) as u64;
        let fractional_digits_count = self.fractional_digits_count() as i32;
        let is_negative = self.is_sign_negative();

        D128Proto {
            lo,
            hi,
            fractional_digits_count,
            is_negative,
        }
    }

    fn from_proto(proto: Self::Proto) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        // Reconstruct u128 from two u64 parts
        let digits = ((proto.hi as u128) << 64) | (proto.lo as u128);

        // Create D128 from digits and fractional count
        let mut result = D128::from_u128(digits)?;

        if proto.fractional_digits_count > 0 {
            // Use D128 for the power to avoid overflow
            result /= D128::TEN.powi(proto.fractional_digits_count);
        } else if proto.fractional_digits_count < 0 {
            result *= D128::TEN.powi(-proto.fractional_digits_count);
        }

        // Apply sign
        if proto.is_negative {
            result = -result;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use fastnum::dec128;

    use super::*;

    #[test]
    fn test_roundtrip() {
        let original = dec128!(123456789.987654321);
        let proto = original.to_proto();
        let restored = D128::from_proto(proto).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_negative_value() {
        let val = dec128!(-123.45);
        let proto = val.to_proto();
        assert!(proto.is_negative);
        assert_eq!(proto.fractional_digits_count, 2);

        let restored = D128::from_proto(proto).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_positive_value() {
        let val = dec128!(123.45);
        let proto = val.to_proto();
        assert!(!proto.is_negative);

        let restored = D128::from_proto(proto).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_fractional_digits() {
        // Test case from docs: 123.45 has 2 fractional digits
        let val = dec128!(123.45);
        let proto = val.to_proto();
        assert_eq!(proto.fractional_digits_count, 2);

        let restored = D128::from_proto(proto).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_scientific_notation() {
        // Test case: 5e9 has -9 fractional digits
        let val = dec128!(5e9);
        let proto = val.to_proto();
        let restored = D128::from_proto(proto).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_negative_scientific() {
        let val = dec128!(-5e9);
        let proto = val.to_proto();
        assert!(proto.is_negative);

        let restored = D128::from_proto(proto).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_no_fractional_part() {
        let val = dec128!(12345);
        let proto = val.to_proto();
        assert_eq!(proto.fractional_digits_count, 0);

        let restored = D128::from_proto(proto).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_small_fractional() {
        // Test case: 0.0000012345 has 10 fractional digits
        let val = dec128!(0.0000012345);
        let proto = val.to_proto();
        assert_eq!(proto.fractional_digits_count, 10);

        let restored = D128::from_proto(proto).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_max_value() {
        let max_val = D128::MAX;
        let proto = max_val.to_proto();
        let restored = D128::from_proto(proto).unwrap();
        assert_eq!(max_val, restored);
    }

    #[test]
    fn test_min_value() {
        let min_val = D128::MIN;
        let proto = min_val.to_proto();
        assert!(proto.is_negative);

        let restored = D128::from_proto(proto).unwrap();
        assert_eq!(min_val, restored);
    }

    #[test]
    fn test_zero() {
        let zero = D128::ZERO;
        let proto = zero.to_proto();
        assert!(!proto.is_negative);

        let restored = D128::from_proto(proto).unwrap();
        assert_eq!(zero, restored);
    }

    #[test]
    fn test_negative_zero() {
        let neg_zero = dec128!(-0.0);
        let proto = neg_zero.to_proto();

        let restored = D128::from_proto(proto).unwrap();
        assert_eq!(neg_zero, restored);
    }

    #[test]
    fn test_proto_fields() {
        // Verify proto structure for -123.45
        let val = dec128!(-123.45);
        let proto = val.to_proto();

        // digits = 12345 (absolute value), fractional_count = 2, negative = true
        let digits = ((proto.hi as u128) << 64) | (proto.lo as u128);
        assert_eq!(digits, 12345);
        assert_eq!(proto.fractional_digits_count, 2);
        assert!(proto.is_negative);
    }
}
