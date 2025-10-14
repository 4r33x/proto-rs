use fastnum::UD128;

use crate::ProtoShadow;

use super::common::{combine_words, fractional_digits_from_i16, raw_split_digits};

extern crate self as proto_rs;

#[crate::proto_message(proto_path = "protos/fastnum.proto", convert = fastnum::UD128)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UD128Proto {
    #[proto(tag = 1)]
    /// Lower 64 bits of the digits
    pub lo: u64,
    #[proto(tag = 2)]
    /// Upper 64 bits of the digits
    pub hi: u64,
    #[proto(tag = 3)]
    /// Fractional digits count (can be negative for scientific notation)
    pub fractional_digits_count: i32,
}

impl ProtoShadow for UD128Proto {
    type Sun = UD128;

    fn to_sun(self) -> Self::Sun {
        let digits = combine_words(self.lo, self.hi);
        let mut value = UD128::from_u128(digits).expect("invalid decimal digits");

        match self.fractional_digits_count.cmp(&0) {
            core::cmp::Ordering::Greater => {
                value = value / UD128::TEN.powi(self.fractional_digits_count);
            }
            core::cmp::Ordering::Less => {
                value = value * UD128::TEN.powi(-self.fractional_digits_count);
            }
            core::cmp::Ordering::Equal => {}
        }

        value
    }

    fn cast_shadow(value: &Self::Sun) -> Self {
        let (lo, hi) = raw_split_digits(value.digits());
        let fractional_digits_count = fractional_digits_from_i16(value.fractional_digits_count());

        Self { lo, hi, fractional_digits_count }
    }
}

#[cfg(test)]
mod tests {
    use fastnum::udec128;

    use super::*;
    use crate::ProtoExt;

    #[test]
    fn test_roundtrip() {
        let original = udec128!(123456789.987654321);
        let encoded = original.encode_to_vec();
        let restored = UD128::decode(encoded.as_slice()).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_positive_value() {
        let val = udec128!(123.45);
        let encoded = val.encode_to_vec();
        let restored = UD128::decode(encoded.as_slice()).unwrap();
        assert_eq!(restored.fractional_digits_count(), val.fractional_digits_count());
        assert_eq!(val, restored);
    }

    #[test]
    fn test_scientific_notation() {
        let val = udec128!(5e9);
        let encoded = val.encode_to_vec();
        let restored = UD128::decode(encoded.as_slice()).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_large_value() {
        let val = udec128!(123456789123456789.123456789123456789);
        let encoded = val.encode_to_vec();
        let restored = UD128::decode(encoded.as_slice()).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_zero() {
        let zero = UD128::ZERO;
        let encoded = zero.encode_to_vec();
        let restored = UD128::decode(encoded.as_slice()).unwrap();
        assert_eq!(zero, restored);
    }

    #[test]
    fn test_small_fractional() {
        let val = udec128!(0.0000012345);
        let encoded = val.encode_to_vec();
        let restored = UD128::decode(encoded.as_slice()).unwrap();
        assert_eq!(restored.fractional_digits_count(), val.fractional_digits_count());
        assert_eq!(val, restored);
    }

    #[test]
    fn test_max_value() {
        let max_val = UD128::MAX;
        let encoded = max_val.encode_to_vec();
        let restored = UD128::decode(encoded.as_slice()).unwrap();
        assert_eq!(max_val, restored);
    }

    #[test]
    fn test_encoded_len() {
        let val = udec128!(42.42);
        let encoded = val.encode_to_vec();
        assert_eq!(encoded.len(), val.encoded_len());
    }
}
