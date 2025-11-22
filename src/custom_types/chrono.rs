use chrono::DateTime;
use chrono::Utc;

use crate::DecodeError;
use crate::ProtoShadow;
use crate::proto_message;

#[proto_message(proto_path = "protos/chrono.proto", sun = DateTime<Utc>)]
pub struct DateTimeProto {
    #[proto(tag = 1)]
    pub secs: i64,
    #[proto(tag = 2)]
    pub ns: u32,
}

impl ProtoShadow<DateTime<Utc>> for DateTimeProto {
    type Sun<'a> = &'a DateTime<Utc>;
    type OwnedSun = DateTime<Utc>;
    type View<'a> = Self;

    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        DateTime::from_timestamp(self.secs, self.ns).ok_or(DecodeError::new("failed to decode  DateTime"))
    }

    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        Self {
            secs: value.timestamp(),
            ns: value.timestamp_subsec_nanos(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::ProtoExt;

    #[proto_message(proto_path = "protos/chrono_test.proto")]
    struct ChronoWrapper {
        #[proto(tag = 1)]
        inner: DateTime<Utc>,
    }

    #[test]
    fn test_datetime_roundtrip() {
        let dt = DateTime::from_timestamp(1_234_567_890, 123_456_789).expect("valid timestamp");

        // Test encoding and decoding through ProtoExt
        let encoded = chrono::DateTime::encode_to_vec(&dt);
        let decoded = DateTime::<Utc>::decode(encoded.as_slice()).expect("decode");

        assert_eq!(dt, decoded);
    }

    #[test]
    fn test_datetime_in_wrapper() {
        let dt = DateTime::from_timestamp(9_876_543_210, 987_654_321).expect("valid timestamp");
        let wrapper = ChronoWrapper { inner: dt };

        let encoded = ChronoWrapper::encode_to_vec(&wrapper);
        let decoded = ChronoWrapper::decode(encoded.as_slice()).expect("decode");

        assert_eq!(wrapper.inner, decoded.inner);
    }
}
