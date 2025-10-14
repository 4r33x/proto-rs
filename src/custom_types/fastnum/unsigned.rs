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
