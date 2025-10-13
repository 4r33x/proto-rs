use fastnum::D128;

use crate::ProtoExt;
use crate::proto_dump;
extern crate self as proto_rs;

//DO NOT USE IT FOR ENCODE\DECODE
#[proto_dump(proto_path = "protos/fastnum.proto")]
struct D128Proto {
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
        let parts = D128Parts::from(self);
        crate::encoding::uint64::encode(1, &parts.lo, buf);
        crate::encoding::uint64::encode(2, &parts.hi, buf);
        crate::encoding::int32::encode(3, &parts.fractional_digits_count, buf);
        crate::encoding::bool::encode(4, &parts.is_negative, buf);
    }

    fn merge_field(&mut self, tag: u32, wire_type: crate::encoding::WireType, buf: &mut impl bytes::Buf, ctx: crate::encoding::DecodeContext) -> Result<(), crate::DecodeError>
    where
        Self: Sized,
    {
        let mut parts = D128Parts::from(&*self);
        let handled = match tag {
            1 => {
                crate::encoding::uint64::merge(wire_type, &mut parts.lo, buf, ctx)?;
                true
            }
            2 => {
                crate::encoding::uint64::merge(wire_type, &mut parts.hi, buf, ctx)?;
                true
            }
            3 => {
                crate::encoding::int32::merge(wire_type, &mut parts.fractional_digits_count, buf, ctx)?;
                true
            }
            4 => {
                crate::encoding::bool::merge(wire_type, &mut parts.is_negative, buf, ctx)?;
                true
            }
            _ => false,
        };

        if handled {
            *self = parts.into_value()?;
            Ok(())
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        let parts = D128Parts::from(self);
        crate::encoding::uint64::encoded_len(1, &parts.lo)
            + crate::encoding::uint64::encoded_len(2, &parts.hi)
            + crate::encoding::int32::encoded_len(3, &parts.fractional_digits_count)
            + crate::encoding::bool::encoded_len(4, &parts.is_negative)
    }

    fn clear(&mut self) {
        *self = D128::ZERO;
    }
}

fn d128_from_proto(proto: D128Proto) -> Result<D128, crate::DecodeError> {
    D128::try_from(proto)
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
