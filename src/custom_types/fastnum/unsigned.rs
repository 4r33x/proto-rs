use core::convert::TryInto;

use fastnum::UD128;

use crate::DecodeError;
use crate::ProtoShadowDecode;
use crate::proto_message;

#[inline(always)]
fn ud128_lo(value: &UD128) -> u64 {
    let digits: u128 = value
        .digits()
        .try_into()
        .expect("UD128 digits should fit in u128");
    digits as u64
}

#[inline(always)]
fn ud128_hi(value: &UD128) -> u64 {
    let digits: u128 = value
        .digits()
        .try_into()
        .expect("UD128 digits should fit in u128");
    (digits >> 64) as u64
}

#[inline(always)]
fn ud128_fractional_digits_count(value: &UD128) -> i32 {
    i32::from(value.fractional_digits_count())
}

#[proto_message(proto_path = "protos/fastnum.proto", sun = UD128)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct UD128Proto {
    #[proto(tag = 1, getter = "ud128_lo($)")]
    /// Lower 64 bits of the digits
    pub lo: u64,
    #[proto(tag = 2, getter = "ud128_hi($)")]
    /// Upper 64 bits of the digits
    pub hi: u64,
    #[proto(tag = 3, getter = "ud128_fractional_digits_count($)")]
    /// Fractional digits count (can be negative for scientific notation)
    pub fractional_digits_count: i32,
}

impl ProtoShadowDecode<UD128> for UD128Proto {
    fn to_sun(self) -> Result<UD128, DecodeError> {
        let digits = ((self.hi as u128) << 64) | (self.lo as u128);

        let mut result = UD128::from_u128(digits).map_err(|err| DecodeError::new(err.to_string()))?;

        if self.fractional_digits_count > 0 {
            result /= UD128::TEN.powi(self.fractional_digits_count);
        } else if self.fractional_digits_count < 0 {
            result *= UD128::TEN.powi(-self.fractional_digits_count);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {

    use fastnum::udec128;

    use super::*;
    use crate::ProtoShadowEncode;
    #[allow(dead_code)]
    #[proto_message(proto_path = "protos/fastnum_test.proto")]
    struct UD128Wrapper {
        inner: UD128,
    }

    fn encode(value: &UD128) -> UD128Proto {
        <UD128Proto as ProtoShadowEncode<'_, UD128>>::from_sun(value)
    }

    fn decode(proto: UD128Proto) -> UD128 {
        ProtoShadowDecode::<UD128>::to_sun(proto).unwrap()
    }

    #[test]
    fn test_roundtrip() {
        let original = udec128!(123456789.987654321);
        let proto = encode(&original);
        let restored = decode(proto);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_fractional_digits() {
        // Test case from docs: 123.45 has 2 fractional digits
        let val = udec128!(123.45);
        let proto = encode(&val);
        assert_eq!(proto.fractional_digits_count, 2);

        let restored = decode(proto);
        assert_eq!(val, restored);
    }

    #[test]
    fn test_scientific_notation() {
        // Test case: 5e9 has -9 fractional digits
        let val = udec128!(5e9);
        let proto = encode(&val);
        let restored = decode(proto);
        assert_eq!(val, restored);
    }

    #[test]
    fn test_no_fractional_part() {
        let val = udec128!(12345);
        let proto = encode(&val);
        assert_eq!(proto.fractional_digits_count, 0);

        let restored = decode(proto);
        assert_eq!(val, restored);
    }

    #[test]
    fn test_small_fractional() {
        // Test case: 0.0000012345 has 10 fractional digits
        let val = udec128!(0.0000012345);
        let proto = encode(&val);
        assert_eq!(proto.fractional_digits_count, 10);

        let restored = decode(proto);
        assert_eq!(val, restored);
    }

    #[test]
    fn test_max_value() {
        let max_val = UD128::MAX;
        let proto = encode(&max_val);
        let restored = decode(proto);
        assert_eq!(max_val, restored);
    }

    #[test]
    fn test_zero() {
        let zero = UD128::ZERO;
        let proto = encode(&zero);
        let restored = decode(proto);
        assert_eq!(zero, restored);
    }

    #[test]
    fn test_proto_fields() {
        // Verify proto structure for 123.45
        let val = udec128!(123.45);
        let proto = encode(&val);

        // digits = 12345, fractional_count = 2
        // Reconstruct: (hi << 64) | lo = digits
        let digits = ((proto.hi as u128) << 64) | (proto.lo as u128);
        assert_eq!(digits, 12345);
        assert_eq!(proto.fractional_digits_count, 2);
    }
}
