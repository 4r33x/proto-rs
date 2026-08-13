//! Protobuf encoding and decoding errors.

use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;

use crate::encoding::WireType;

/// A Protobuf message decoding error.
///
/// `DecodeError` indicates that the input buffer does not contain a valid
/// Protobuf message. The error details should be considered 'best effort': in
/// general it is not possible to exactly pinpoint why data is malformed.
#[derive(Clone, PartialEq, Eq)]
pub struct DecodeError {
    /// A 'best effort' root cause description.
    description: Cow<'static, str>,
    /// A stack of (message, field) name pairs, which identify the specific
    /// message type and field where decoding failed. The stack contains an
    /// entry per level of nesting.
    stack: Vec<(&'static str, &'static str)>,
}

impl DecodeError {
    /// Creates a new `DecodeError` with a 'best effort' root cause description.
    ///
    /// Meant to be used only by `Message` implementations.
    #[doc(hidden)]
    #[cold]
    pub fn new(description: impl Into<Cow<'static, str>>) -> DecodeError {
        DecodeError {
            description: description.into(),
            stack: Vec::new(),
        }
    }

    #[doc(hidden)]
    #[cold]
    pub fn invalid_key_value(value: u64) -> Self {
        Self::new(alloc::format!("invalid key value: {value}"))
    }

    #[doc(hidden)]
    #[cold]
    pub fn invalid_wire_type_value(value: u64) -> Self {
        let description = match value {
            6 => "invalid wire type value: 6",
            7 => "invalid wire type value: 7",
            _ => "invalid wire type value",
        };
        Self::new(description)
    }

    #[doc(hidden)]
    #[cold]
    pub fn wire_type_mismatch(expected: WireType) -> Self {
        let description = match expected {
            WireType::Varint => "invalid wire type (expected Varint)",
            WireType::SixtyFourBit => "invalid wire type (expected SixtyFourBit)",
            WireType::LengthDelimited => "invalid wire type (expected LengthDelimited)",
            WireType::StartGroup => "invalid wire type (expected StartGroup)",
            WireType::EndGroup => "invalid wire type (expected EndGroup)",
            WireType::ThirtyTwoBit => "invalid wire type (expected ThirtyTwoBit)",
        };
        Self::new(description)
    }

    #[doc(hidden)]
    #[cold]
    pub fn invalid_wire_type_for_kind(kind: &'static str) -> Self {
        let description = match kind {
            "Primitive" => "invalid wire type Primitive",
            "SimpleEnum" => "invalid wire type SimpleEnum",
            "Message" => "invalid wire type Message",
            "Bytes" => "invalid wire type Bytes",
            "String" => "invalid wire type String",
            "Repeated" => "invalid wire type Repeated",
            _ => "invalid wire type",
        };
        Self::new(description)
    }

    /// Pushes a (message, field) name location pair on to the location stack.
    ///
    /// Meant to be used only by `Message` implementations.
    #[doc(hidden)]
    pub fn push(&mut self, message: &'static str, field: &'static str) {
        self.stack.push((message, field));
    }
}

impl fmt::Debug for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecodeError").field("description", &self.description).field("stack", &self.stack).finish()
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("failed to decode Protobuf message: ")?;
        for &(message, field) in &self.stack {
            write!(f, "{message}.{field}: ")?;
        }
        f.write_str(&self.description)
    }
}

impl std::error::Error for DecodeError {}

impl From<DecodeError> for std::io::Error {
    fn from(error: DecodeError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, error)
    }
}

/// A Protobuf message encoding error.
///
/// `EncodeError` always indicates that a message failed to encode because the
/// provided buffer had insufficient capacity. Message encoding is otherwise
/// infallible.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EncodeError {
    required: usize,
    remaining: usize,
}

impl EncodeError {
    /// Creates a new `EncodeError`.
    pub(crate) const fn new(required: usize, remaining: usize) -> EncodeError {
        EncodeError { required, remaining }
    }

    /// Returns the required buffer capacity to encode the message.
    pub const fn required_capacity(&self) -> usize {
        self.required
    }

    /// Returns the remaining length in the provided buffer at the time of encoding.
    pub const fn remaining(&self) -> usize {
        self.remaining
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to encode Protobuf message; insufficient buffer capacity (required: {}, remaining: {})",
            self.required, self.remaining
        )
    }
}

impl core::error::Error for EncodeError {}

impl From<EncodeError> for std::io::Error {
    fn from(error: EncodeError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, error)
    }
}

/// An error indicating that an unknown enumeration value was encountered.
///
/// The Protobuf spec mandates that enumeration value sets are ‘open’, so this
/// error's value represents an integer value unrecognized by the
/// presently used enum definition.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UnknownEnumValue(pub i32);

impl fmt::Display for UnknownEnumValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown enumeration value {}", self.0)
    }
}

impl core::error::Error for UnknownEnumValue {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::alloc::string::ToString;
    #[test]
    fn test_push() {
        let mut decode_error = DecodeError::new("something failed");
        decode_error.push("Foo bad", "bar.foo");
        decode_error.push("Baz bad", "bar.baz");

        assert_eq!(
            decode_error.to_string(),
            "failed to decode Protobuf message: Foo bad.bar.foo: Baz bad.bar.baz: something failed"
        );
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn decode_error_remains_compact() {
        assert_eq!(core::mem::size_of::<DecodeError>(), 48);
    }

    #[test]
    fn typed_wire_errors_preserve_diagnostics_without_owned_strings() {
        let invalid = DecodeError::invalid_wire_type_value(7);
        assert_eq!(invalid.description, Cow::Borrowed("invalid wire type value: 7"));

        let mismatch = DecodeError::wire_type_mismatch(WireType::LengthDelimited);
        assert_eq!(mismatch.description, Cow::Borrowed("invalid wire type (expected LengthDelimited)"));

        let kind = DecodeError::invalid_wire_type_for_kind("Message");
        assert_eq!(kind.description, Cow::Borrowed("invalid wire type Message"));
    }

    #[test]
    fn test_into_std_io_error() {
        let decode_error = DecodeError::new("something failed");
        let std_io_error = std::io::Error::from(decode_error);

        assert_eq!(std_io_error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(std_io_error.to_string(), "failed to decode Protobuf message: something failed");
    }
}
