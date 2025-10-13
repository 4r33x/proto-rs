use fastnum::UD128;

use crate::proto_dump;
extern crate self as proto_rs;

//DO NOT USE IT FOR ENCODE\DECODE
#[proto_dump(proto_path = "protos/fastnum.proto")]
#[derive(prost::Message, Clone, PartialEq, Copy)]
pub struct UD128Proto {
    #[prost(uint64, tag = 1)]
    /// Lower 64 bits of the digits
    pub lo: u64,
    #[prost(uint64, tag = 2)]
    /// Upper 64 bits of the digits
    pub hi: u64,
    #[prost(int32, tag = 3)]
    /// Fractional digits count (can be negative for scientific notation)
    pub fractional_digits_count: i32,
}

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
        let parts = UD128Parts::from(self);
        crate::encoding::uint64::encode(1, &parts.lo, buf);
        crate::encoding::uint64::encode(2, &parts.hi, buf);
        crate::encoding::int32::encode(3, &parts.fractional_digits_count, buf);
    }

    fn merge_field(&mut self, tag: u32, wire_type: crate::encoding::WireType, buf: &mut impl bytes::Buf, ctx: crate::encoding::DecodeContext) -> Result<(), crate::DecodeError>
    where
        Self: Sized,
    {
        let mut parts = UD128Parts::from(&*self);
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
        let parts = UD128Parts::from(self);
        crate::encoding::uint64::encoded_len(1, &parts.lo) + crate::encoding::uint64::encoded_len(2, &parts.hi) + crate::encoding::int32::encoded_len(3, &parts.fractional_digits_count)
    }

    fn clear(&mut self) {
        *self = UD128::ZERO;
    }
}

fn ud128_from_proto(proto: UD128Proto) -> Result<UD128, crate::DecodeError> {
    UD128Parts::from(proto).into_value()
}

#[cfg(test)]
mod tests {

    use fastnum::udec128;

    use super::*;

    #[test]
    fn test_roundtrip() {
        let original = udec128!(123456789.987654321);
        let proto = original.to_proto();
        let restored = UD128::from_proto(proto).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn test_fractional_digits() {
        // Test case from docs: 123.45 has 2 fractional digits
        let val = udec128!(123.45);
        let proto = val.to_proto();
        assert_eq!(proto.fractional_digits_count, 2);

        let restored = UD128::from_proto(proto).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_scientific_notation() {
        // Test case: 5e9 has -9 fractional digits
        let val = udec128!(5e9);
        let proto = val.to_proto();
        let restored = UD128::from_proto(proto).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_no_fractional_part() {
        let val = udec128!(12345);
        let proto = val.to_proto();
        assert_eq!(proto.fractional_digits_count, 0);

        let restored = UD128::from_proto(proto).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_small_fractional() {
        // Test case: 0.0000012345 has 10 fractional digits
        let val = udec128!(0.0000012345);
        let proto = val.to_proto();
        assert_eq!(proto.fractional_digits_count, 10);

        let restored = UD128::from_proto(proto).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_max_value() {
        let max_val = UD128::MAX;
        let proto = max_val.to_proto();
        let restored = UD128::from_proto(proto).unwrap();
        assert_eq!(max_val, restored);
    }

    #[test]
    fn test_zero() {
        let zero = UD128::ZERO;
        let proto = zero.to_proto();
        let restored = UD128::from_proto(proto).unwrap();
        assert_eq!(zero, restored);
    }

    #[test]
    fn test_proto_fields() {
        // Verify proto structure for 123.45
        let val = udec128!(123.45);
        let proto = val.to_proto();

        // digits = 12345, fractional_count = 2
        // Reconstruct: (hi << 64) | lo = digits
        let digits = ((proto.hi as u128) << 64) | (proto.lo as u128);
        assert_eq!(digits, 12345);
        assert_eq!(proto.fractional_digits_count, 2);
    }
}
