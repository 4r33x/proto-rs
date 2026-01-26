use chrono::DateTime;
use chrono::TimeDelta;
use chrono::Utc;

use crate::DecodeError;
use crate::ProtoShadowDecode;
use crate::ProtoShadowEncode;
use crate::impl_proto_ident;
use crate::proto_message;

impl_proto_ident!(Utc);

#[proto_message(proto_path = "protos/chrono.proto", sun = [DateTime<Utc>])]
pub struct DateTimeProto {
    #[proto(tag = 1)]
    pub secs: i64,
    #[proto(tag = 2)]
    pub ns: u32,
}

impl ProtoShadowDecode<DateTime<Utc>> for DateTimeProto {
    fn to_sun(self) -> Result<DateTime<Utc>, DecodeError> {
        DateTime::from_timestamp(self.secs, self.ns).ok_or(DecodeError::new("failed to decode TimeDelta"))
    }
}

impl<'a> ProtoShadowEncode<'a, DateTime<Utc>> for DateTimeProto {
    fn from_sun(value: &'a DateTime<Utc>) -> Self {
        Self {
            secs: value.timestamp(),
            ns: value.timestamp_subsec_nanos(),
        }
    }
}

#[proto_message(proto_path = "protos/chrono.proto", sun = [TimeDelta])]
pub struct TimeDeltaProto {
    #[proto(tag = 1)]
    pub secs: i64,
    #[proto(tag = 2)]
    pub ns: u32,
}

impl ProtoShadowDecode<TimeDelta> for TimeDeltaProto {
    fn to_sun(self) -> Result<TimeDelta, DecodeError> {
        TimeDelta::new(self.secs, self.ns).ok_or(DecodeError::new("failed to decode TimeDelta"))
    }
}

impl<'a> ProtoShadowEncode<'a, TimeDelta> for TimeDeltaProto {
    fn from_sun(value: &'a TimeDelta) -> Self {
        let secs = value.num_seconds();
        let sub = value.subsec_nanos(); // may be negative

        if sub >= 0 {
            // normal case
            Self { secs, ns: sub as u32 }
        } else {
            // negative fractional case
            Self {
                secs: secs - 1,
                ns: (sub + 1_000_000_000) as u32,
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::ProtoDecode;
    use crate::ProtoEncode;
    use crate::encoding::DecodeContext;

    #[proto_message(proto_path = "protos/chrono_test.proto")]
    struct ChronoWrapper {
        #[proto(tag = 1)]
        inner: DateTime<Utc>,
    }

    #[proto_message(proto_path = "protos/chrono_test.proto")]
    struct DeltaWrapper {
        #[proto(tag = 1)]
        inner: TimeDelta,
    }

    fn roundtrip(td: TimeDelta) {
        let encoded = <TimeDelta as ProtoEncode>::encode_to_vec(&td);
        let decoded = <TimeDelta as ProtoDecode>::decode(encoded.as_slice(), DecodeContext::default()).unwrap();
        assert_eq!(td, decoded);
    }

    #[test]
    fn test_small_values() {
        roundtrip(TimeDelta::new(0, 0).unwrap());
        roundtrip(TimeDelta::new(1, 123_456_789).unwrap());
        roundtrip(TimeDelta::new(-1, 123_456_789).unwrap());
        roundtrip(TimeDelta::nanoseconds(1));
        roundtrip(TimeDelta::nanoseconds(-1));
    }

    #[test]
    fn test_large_values() {
        roundtrip(TimeDelta::try_seconds(123_456_789).unwrap());
        roundtrip(TimeDelta::try_milliseconds(i64::MAX / 2).unwrap());
        roundtrip(TimeDelta::try_milliseconds(-i64::MAX / 2).unwrap());
    }

    #[test]
    fn test_edge_values() {
        roundtrip(TimeDelta::MAX);
        roundtrip(TimeDelta::MIN);
        roundtrip(TimeDelta::try_milliseconds(i64::MAX).unwrap());
        roundtrip(TimeDelta::try_milliseconds(-i64::MAX).unwrap());
    }

    #[test]
    fn test_fractional_sign_cases() {
        // These are the pathological cases that break naive implementations
        roundtrip(TimeDelta::new(-1, 1).unwrap());
        roundtrip(TimeDelta::new(-1, 999_999_999).unwrap());
        roundtrip(TimeDelta::new(-100, 500_000_000).unwrap());
    }

    #[test]
    fn test_wrapper_struct() {
        let td = TimeDelta::new(123, 987_654_321).unwrap();
        let wrapper = DeltaWrapper { inner: td };

        let encoded = <DeltaWrapper as ProtoEncode>::encode_to_vec(&wrapper);
        let decoded = <DeltaWrapper as ProtoDecode>::decode(encoded.as_slice(), DecodeContext::default()).unwrap();

        assert_eq!(wrapper.inner, decoded.inner);
    }
    #[test]
    fn test_datetime_roundtrip() {
        let dt = DateTime::from_timestamp(1_234_567_890, 123_456_789).expect("valid timestamp");

        // Test encoding and decoding through ProtoExt
        let encoded = <DateTime<Utc> as ProtoEncode>::encode_to_vec(&dt);
        let decoded = <DateTime<Utc> as ProtoDecode>::decode(encoded.as_slice(), DecodeContext::default()).expect("decode");

        assert_eq!(dt, decoded);
    }

    #[test]
    fn test_datetime_in_wrapper() {
        let dt = DateTime::from_timestamp(9_876_543_210, 987_654_321).expect("valid timestamp");
        let wrapper = ChronoWrapper { inner: dt };

        let encoded = <ChronoWrapper as ProtoEncode>::encode_to_vec(&wrapper);
        let decoded = <ChronoWrapper as ProtoDecode>::decode(encoded.as_slice(), DecodeContext::default()).expect("decode");

        assert_eq!(wrapper.inner, decoded.inner);
    }
}
