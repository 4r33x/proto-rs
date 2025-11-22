use chrono::DateTime;
use chrono::Utc;

use crate::DecodeError;
use crate::ProtoShadow;
use crate::proto_message;

#[proto_message(proto_path = "protos/chrono.proto", sun = DateTime<Utc>)]
struct DateTimeProto {
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

    #[proto_message(proto_path = "protos/chrono_test.proto")]
    struct ChronoWrapper {
        inner: DateTime<Utc>,
    }
}
