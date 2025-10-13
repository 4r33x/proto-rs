use fastnum::UD128;

use crate::proto_dump;
extern crate self as proto_rs;

use super::common::{DecimalLike, DecimalProto, FastnumDecimalParts, combine_words, decimal_state, fractional_digits, split_digits};

use bytes::{Buf, BufMut};

use crate::DecodeError;
use crate::encoding::{self, DecodeContext, WireType};

//DO NOT USE IT FOR ENCODE\DECODE
#[proto_dump(proto_path = "protos/fastnum.proto")]
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

impl DecimalProto for UD128Proto {
    type Decimal = UD128;

    fn from_decimal(decimal: &Self::Decimal) -> Self {
        let (lo, hi) = split_digits(decimal);
        let fractional_digits_count = fractional_digits(decimal);
        Self { lo, hi, fractional_digits_count }
    }

    fn try_into_decimal(self) -> Result<Self::Decimal, DecodeError> {
        let digits = combine_words(self.lo, self.hi);
        let mut value = UD128::from_u128(digits).map_err(|err| DecodeError::new(err.to_string()))?;

        match self.fractional_digits_count.cmp(&0) {
            core::cmp::Ordering::Greater => {
                value = value / UD128::TEN.powi(self.fractional_digits_count);
            }
            core::cmp::Ordering::Less => {
                value = value * UD128::TEN.powi(-self.fractional_digits_count);
            }
            core::cmp::Ordering::Equal => {}
        }

        Ok(value)
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<bool, DecodeError> {
        let handled = match tag {
            1 => {
                encoding::uint64::merge(wire_type, &mut self.lo, buf, ctx)?;
                true
            }
            2 => {
                encoding::uint64::merge(wire_type, &mut self.hi, buf, ctx)?;
                true
            }
            3 => {
                encoding::int32::merge(wire_type, &mut self.fractional_digits_count, buf, ctx)?;
                true
            }
            _ => false,
        };

        Ok(handled)
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        encoding::uint64::encode(1, &self.lo, buf);
        encoding::uint64::encode(2, &self.hi, buf);
        encoding::int32::encode(3, &self.fractional_digits_count, buf);
    }

    fn encoded_len(&self) -> usize {
        encoding::uint64::encoded_len(1, &self.lo) + encoding::uint64::encoded_len(2, &self.hi) + encoding::int32::encoded_len(3, &self.fractional_digits_count)
    }
}

impl DecimalLike for UD128 {
    type Proto = UD128Proto;
}

impl FastnumDecimalParts for UD128 {
    fn digits_uint(&self) -> fastnum::bint::UInt<2> {
        self.digits()
    }

    fn fractional_count(&self) -> i16 {
        self.fractional_digits_count()
    }
}

decimal_state!(UNSIGNED_STATE, UD128, with_unsigned_proto, finalize_unsigned, clear_unsigned);

impl crate::ProtoExt for UD128 {
    fn proto_default() -> Self
    where
        Self: Sized,
    {
        UD128::ZERO
    }

    fn encode_raw(&self, buf: &mut impl bytes::BufMut)
    where
        Self: Sized,
    {
        UD128Proto::from_decimal(self).encode_raw(buf);
    }

    fn merge_field(&mut self, tag: u32, wire_type: crate::encoding::WireType, buf: &mut impl bytes::Buf, ctx: crate::encoding::DecodeContext) -> Result<(), crate::DecodeError>
    where
        Self: Sized,
    {
        if with_unsigned_proto(self, |proto| {
            let handled = proto.merge_field(tag, wire_type, buf, ctx)?;
            if handled && matches!(tag, 1 | 2) {
                proto.clone().try_into_decimal()?;
            }
            Ok(handled)
        })? {
            Ok(())
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        UD128Proto::from_decimal(self).encoded_len()
    }

    fn clear(&mut self) {
        clear_unsigned(self);
        *self = UD128::ZERO;
    }

    fn post_decode(&mut self) {
        finalize_unsigned(self).expect("failed to finalize unsigned decimal decode");
    }
}

impl crate::MessageField for UD128 {}

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
    fn test_fractional_digits() {
        // Test case from docs: 123.45 has 2 fractional digits
        let val = udec128!(123.45);
        let encoded = val.encode_to_vec();
        let restored = UD128::decode(encoded.as_slice()).unwrap();
        assert_eq!(restored.fractional_digits_count(), val.fractional_digits_count());
        assert_eq!(val, restored);
    }

    #[test]
    fn test_scientific_notation() {
        // Test case: 5e9 has -9 fractional digits
        let val = udec128!(5e9);
        let encoded = val.encode_to_vec();
        let restored = UD128::decode(encoded.as_slice()).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_no_fractional_part() {
        let val = udec128!(12345);
        let encoded = val.encode_to_vec();
        let restored = UD128::decode(encoded.as_slice()).unwrap();
        assert_eq!(restored.fractional_digits_count(), val.fractional_digits_count());
        assert_eq!(val, restored);
    }

    #[test]
    fn test_small_fractional() {
        // Test case: 0.0000012345 has 10 fractional digits
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
    fn test_zero() {
        let zero = UD128::ZERO;
        let encoded = zero.encode_to_vec();
        let restored = UD128::decode(encoded.as_slice()).unwrap();
        assert_eq!(zero, restored);
    }

    #[test]
    fn test_proto_fields() {
        // Verify proto structure for 123.45
        let val = udec128!(123.45);
        let (lo, hi) = split_digits(&val);
        let digits = combine_words(lo, hi);
        assert_eq!(digits, 12345);
        assert_eq!(fractional_digits(&val), 2);
    }
}
