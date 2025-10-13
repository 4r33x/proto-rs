use fastnum::D128;

use crate::ProtoExt;
use crate::proto_dump;
extern crate self as proto_rs;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct D128Parts {
    lo: u64,
    hi: u64,
    fractional_digits_count: i32,
    is_negative: bool,
}

impl From<&D128> for D128Parts {
    fn from(value: &D128) -> Self {
        let digits: u128 = value
            .digits()
            .try_into()
            .expect("D128 should have at most u128 digits");
        Self {
            lo: digits as u64,
            hi: (digits >> 64) as u64,
            fractional_digits_count: value.fractional_digits_count() as i32,
            is_negative: value.is_sign_negative(),
        }
    }
}

impl From<D128> for D128Parts {
    fn from(value: D128) -> Self {
        Self::from(&value)
    }
}

impl From<D128Parts> for D128Proto {
    fn from(parts: D128Parts) -> Self {
        Self {
            lo: parts.lo,
            hi: parts.hi,
            fractional_digits_count: parts.fractional_digits_count,
            is_negative: parts.is_negative,
        }
    }
}

impl From<D128Proto> for D128Parts {
    fn from(proto: D128Proto) -> Self {
        Self {
            lo: proto.lo,
            hi: proto.hi,
            fractional_digits_count: proto.fractional_digits_count,
            is_negative: proto.is_negative,
        }
    }
}

impl D128Parts {
    fn into_value(self) -> Result<D128, crate::DecodeError> {
        let digits = ((self.hi as u128) << 64) | u128::from(self.lo);
        let mut value = D128::from_u128(digits)
            .map_err(|err| crate::DecodeError::new(err.to_string()))?;

        match self.fractional_digits_count.cmp(&0) {
            core::cmp::Ordering::Greater => {
                value /= D128::TEN.powi(self.fractional_digits_count);
            }
            core::cmp::Ordering::Less => {
                value *= D128::TEN.powi(-self.fractional_digits_count);
            }
            core::cmp::Ordering::Equal => {}
        }

        if self.is_negative {
            value = -value;
        }

        Ok(value)
    }
}

impl From<D128> for D128Proto {
    fn from(value: D128) -> Self {
        D128Parts::from(value).into()
    }
}

impl From<&D128> for D128Proto {
    fn from(value: &D128) -> Self {
        D128Parts::from(value).into()
    }
}

// we dont need it, its just reference how we should convert back
impl TryFrom<D128Proto> for D128 {
    type Error = crate::DecodeError;

    fn try_from(proto: D128Proto) -> Result<Self, Self::Error> {
        D128Parts::from(proto).into_value()
    }
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

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: crate::encoding::WireType,
        buf: &mut impl bytes::Buf,
        ctx: crate::encoding::DecodeContext,
    ) -> Result<(), crate::DecodeError>
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
