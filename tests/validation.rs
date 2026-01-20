#![allow(dead_code)]

use proto_rs::DecodeError;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::encoding::DecodeContext;
use proto_rs::proto_message;

// Helper validation functions
fn validate_id(id: &Id) -> Result<(), DecodeError> {
    // Validate that id is not 999 (a non-default sentinel value)
    if id.id == 999 {
        return Err(DecodeError::new("Bad id: id cannot be 999"));
    }
    Ok(())
}

fn validate_positive_count(msg: &PositiveCount) -> Result<(), DecodeError> {
    if msg.count <= 0 {
        return Err(DecodeError::new("Bad count: count must be positive"));
    }
    Ok(())
}

fn validate_user(user: &User) -> Result<(), DecodeError> {
    if user.name.is_empty() {
        return Err(DecodeError::new("Bad user: name cannot be empty"));
    }
    if user.age < 0 {
        return Err(DecodeError::new("Bad user: age cannot be negative"));
    }
    Ok(())
}

fn validate_message_with_both(msg: &MessageWithBothValidators) -> Result<(), DecodeError> {
    // Message-level validation: sum of scores must be less than 1000
    let sum: i32 = msg.scores.iter().sum();
    if sum >= 1000 {
        return Err(DecodeError::new("Bad message: sum of scores must be less than 1000"));
    }
    Ok(())
}

// Test types
#[proto_message(proto_path = "protos/tests/validation.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Id {
    pub id: u32,
}

#[proto_message(proto_path = "protos/tests/validation.proto")]
#[proto(validator = validate_positive_count)]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PositiveCount {
    pub count: i32,
}

#[proto_message(proto_path = "protos/tests/validation.proto")]
#[proto(validator = validate_user)]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct User {
    pub name: String,
    pub age: i32,
}

#[proto_message(proto_path = "protos/tests/validation.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MessageWithFieldValidator {
    #[proto(validator = validate_id)]
    pub id: Id,
    pub scores: Vec<i32>,
}

#[proto_message(proto_path = "protos/tests/validation.proto")]
#[proto(validator = validate_message_with_both)]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MessageWithBothValidators {
    #[proto(validator = validate_id)]
    pub id: Id,
    pub scores: Vec<i32>,
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_validation_good_input() {
        let msg = MessageWithFieldValidator {
            id: Id { id: 42 },
            scores: vec![1, 2, 3],
        };

        let encoded = msg.encode_to_vec();
        let decoded = <MessageWithFieldValidator as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_field_validation_bad_input() {
        let msg = MessageWithFieldValidator {
            id: Id { id: 999 }, // Invalid: id cannot be 999
            scores: vec![1, 2, 3],
        };

        let encoded = msg.encode_to_vec();
        let result = <MessageWithFieldValidator as ProtoDecode>::decode(&encoded[..], DecodeContext::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Bad id"));
    }

    #[test]
    fn test_message_validation_good_input() {
        let msg = PositiveCount { count: 42 };

        let encoded = msg.encode_to_vec();
        let decoded = <PositiveCount as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_message_validation_bad_input_negative() {
        let msg = PositiveCount { count: -5 }; // Invalid: count must be positive

        let encoded = msg.encode_to_vec();
        let result = <PositiveCount as ProtoDecode>::decode(&encoded[..], DecodeContext::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Bad count"));
    }

    #[test]
    fn test_user_validation_good_input() {
        let user = User {
            name: "Alice".to_string(),
            age: 25,
        };

        let encoded = user.encode_to_vec();
        let decoded = <User as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, user);
    }

    #[test]
    fn test_user_validation_both_fields_set() {
        let user = User {
            name: "Bob".to_string(),
            age: -1, // Invalid: age cannot be negative
        };

        let encoded = user.encode_to_vec();
        let result = <User as ProtoDecode>::decode(&encoded[..], DecodeContext::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("age cannot be negative"));
    }

    #[test]
    fn test_both_validators_good_input() {
        let msg = MessageWithBothValidators {
            id: Id { id: 42 },
            scores: vec![10, 20, 30],
        };

        let encoded = msg.encode_to_vec();
        let decoded = <MessageWithBothValidators as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_both_validators_bad_field() {
        let msg = MessageWithBothValidators {
            id: Id { id: 999 }, // Invalid: id cannot be 999
            scores: vec![10, 20, 30],
        };

        let encoded = msg.encode_to_vec();
        let result = <MessageWithBothValidators as ProtoDecode>::decode(&encoded[..], DecodeContext::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Bad id"));
    }

    #[test]
    fn test_both_validators_bad_message() {
        let msg = MessageWithBothValidators {
            id: Id { id: 42 },
            scores: vec![500, 500], // Invalid: sum >= 1000
        };

        let encoded = msg.encode_to_vec();
        let result = <MessageWithBothValidators as ProtoDecode>::decode(&encoded[..], DecodeContext::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("sum of scores"));
    }

    #[test]
    fn test_both_validators_both_bad() {
        let msg = MessageWithBothValidators {
            id: Id { id: 999 },     // Invalid: id cannot be 999
            scores: vec![500, 500], // Invalid: sum >= 1000
        };

        let encoded = msg.encode_to_vec();
        let result = <MessageWithBothValidators as ProtoDecode>::decode(&encoded[..], DecodeContext::default());
        // Should fail on field validation first
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Bad id"));
    }
}
