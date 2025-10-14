use fastnum::D128;

use crate::ProtoShadow;

use super::common::{combine_words, fractional_digits_from_i16, raw_split_digits};

extern crate self as proto_rs;

#[crate::proto_message(proto_path = "protos/fastnum.proto", convert = fastnum::D128)]
#[derive(Clone, Debug, Default, PartialEq)]
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

impl ProtoShadow for D128Proto {
    type Sun = D128;

    fn to_sun(self) -> Self::Sun {
        let digits = combine_words(self.lo, self.hi);
        let mut value = D128::from_u128(digits).expect("invalid decimal digits");

        match self.fractional_digits_count.cmp(&0) {
            core::cmp::Ordering::Greater => {
                value = value / D128::TEN.powi(self.fractional_digits_count);
            }
            core::cmp::Ordering::Less => {
                value = value * D128::TEN.powi(-self.fractional_digits_count);
            }
            core::cmp::Ordering::Equal => {}
        }

        if self.is_negative {
            value = -value;
        }

        value
    }

    fn cast_shadow(value: &Self::Sun) -> Self {
        let (lo, hi) = raw_split_digits(value.digits());
        let fractional_digits_count = fractional_digits_from_i16(value.fractional_digits_count());

        Self {
            lo,
            hi,
            fractional_digits_count,
            is_negative: value.is_sign_negative(),
        }
    }
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
    fn test_scientific_notation() {
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
    fn test_large_value() {
        let val = dec128!(123456789123456789.123456789123456789);
        let encoded = val.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_zero() {
        let zero = D128::ZERO;
        let encoded = zero.encode_to_vec();
        let restored = D128::decode(encoded.as_slice()).unwrap();
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
    fn test_extreme_values() {
        let max_val = D128::MAX;
        let min_val = D128::MIN;

        let encoded_max = max_val.encode_to_vec();
        let encoded_min = min_val.encode_to_vec();

        let restored_max = D128::decode(encoded_max.as_slice()).unwrap();
        let restored_min = D128::decode(encoded_min.as_slice()).unwrap();

        assert_eq!(max_val, restored_max);
        assert_eq!(min_val, restored_min);
        assert!(restored_min.is_sign_negative());
    }

    #[test]
    fn test_encoded_len() {
        let val = dec128!(42.42);
        let encoded = val.encode_to_vec();
        assert_eq!(encoded.len(), val.encoded_len());
    }
}
