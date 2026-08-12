#![allow(dead_code)]

use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

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

static COUNTED_MESSAGE_VALIDATOR_CALLS: AtomicUsize = AtomicUsize::new(0);
static COUNTED_FIELD_VALIDATOR_CALLS: AtomicUsize = AtomicUsize::new(0);
static COUNTED_SIMPLE_ENUM_VALIDATOR_CALLS: AtomicUsize = AtomicUsize::new(0);
static COUNTED_TRANSPARENT_VALIDATOR_CALLS: AtomicUsize = AtomicUsize::new(0);
static VALIDATION_COUNTER_LOCK: Mutex<()> = Mutex::new(());

fn validate_counted_message(msg: &mut CountedMessage) -> Result<(), DecodeError> {
    COUNTED_MESSAGE_VALIDATOR_CALLS.fetch_add(1, Ordering::SeqCst);
    if msg.value == 0 {
        return Err(DecodeError::new("counted message value must be non-zero"));
    }
    Ok(())
}

fn validate_counted_field(value: &mut u32) -> Result<(), DecodeError> {
    COUNTED_FIELD_VALIDATOR_CALLS.fetch_add(1, Ordering::SeqCst);
    if *value == 0 {
        return Err(DecodeError::new("counted field value must be non-zero"));
    }
    Ok(())
}

fn validate_counted_simple_enum(value: &mut CountedSimpleEnum) -> Result<(), DecodeError> {
    COUNTED_SIMPLE_ENUM_VALIDATOR_CALLS.fetch_add(1, Ordering::SeqCst);
    if matches!(value, CountedSimpleEnum::Invalid) {
        return Err(DecodeError::new("counted simple enum cannot be invalid"));
    }
    Ok(())
}

fn validate_counted_transparent(value: &mut CountedTransparent) -> Result<(), DecodeError> {
    COUNTED_TRANSPARENT_VALIDATOR_CALLS.fetch_add(1, Ordering::SeqCst);
    if value.0 == 0 {
        return Err(DecodeError::new("counted transparent value must be non-zero"));
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
#[proto(validator = validate_swap_amount)]
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Amount {
    One(u64),
    Two(u32),
    #[default]
    Three,
}

fn validate_swap_amount(amount: &Amount) -> Result<(), DecodeError> {
    match amount {
        Amount::One(v) if *v == 0 => Err(DecodeError::new("Amount is zero")),
        Amount::Two(v) if *v == 0 => Err(DecodeError::new("Amount is zero")),
        _ => Ok(()),
    }
}

#[proto_message(proto_path = "protos/tests/validation.proto")]
#[proto(validator = validate_message_with_both)]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MessageWithBothValidators {
    #[proto(validator = validate_id)]
    pub id: Id,
    pub scores: Vec<i32>,
}

#[proto_message]
#[proto(validator = validate_counted_message)]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CountedMessage {
    pub value: u32,
}

#[proto_message]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CountedMessageHolder {
    pub inner: CountedMessage,
}

#[proto_message]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CountedFieldMessage {
    #[proto(validator = validate_counted_field)]
    pub value: u32,
}

#[proto_message]
#[derive(Clone, Debug, PartialEq, Default)]
pub enum CountedComplexFieldEnum {
    #[default]
    Empty,
    Value(#[proto(validator = validate_counted_field)] u32),
}

#[proto_message]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CountedComplexFieldEnumHolder {
    pub value: CountedComplexFieldEnum,
}

#[proto_message]
#[proto(validator = validate_counted_simple_enum)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum CountedSimpleEnum {
    #[default]
    Valid,
    AlsoValid,
    Invalid,
}

#[proto_message]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CountedSimpleEnumHolder {
    pub value: CountedSimpleEnum,
}

#[proto_message(transparent)]
#[proto(validator = validate_counted_transparent)]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CountedTransparent(pub u32);

#[proto_message]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CountedTransparentHolder {
    pub value: CountedTransparent,
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

        let encoded = MessageWithFieldValidator::encode_to_vec(&msg);
        let decoded = <MessageWithFieldValidator as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_field_validation_bad_input() {
        let msg = MessageWithFieldValidator {
            id: Id { id: 999 }, // Invalid: id cannot be 999
            scores: vec![1, 2, 3],
        };

        let encoded = MessageWithFieldValidator::encode_to_vec(&msg);
        let result = <MessageWithFieldValidator as ProtoDecode>::decode(&encoded[..], DecodeContext::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Bad id"));
    }

    #[test]
    fn test_message_validation_good_input() {
        let msg = PositiveCount { count: 42 };

        let encoded = PositiveCount::encode_to_vec(&msg);
        let decoded = <PositiveCount as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_message_validation_bad_input_negative() {
        let msg = PositiveCount { count: -5 }; // Invalid: count must be positive

        let encoded = PositiveCount::encode_to_vec(&msg);
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

        let encoded = User::encode_to_vec(&user);
        let decoded = <User as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, user);
    }

    #[test]
    fn test_user_validation_both_fields_set() {
        let user = User {
            name: "Bob".to_string(),
            age: -1, // Invalid: age cannot be negative
        };

        let encoded = User::encode_to_vec(&user);
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

        let encoded = MessageWithBothValidators::encode_to_vec(&msg);
        let decoded = <MessageWithBothValidators as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_both_validators_bad_field() {
        let msg = MessageWithBothValidators {
            id: Id { id: 999 }, // Invalid: id cannot be 999
            scores: vec![10, 20, 30],
        };

        let encoded = MessageWithBothValidators::encode_to_vec(&msg);
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

        let encoded = MessageWithBothValidators::encode_to_vec(&msg);
        let result = <MessageWithBothValidators as ProtoDecode>::decode(&encoded[..], DecodeContext::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("sum of scores"));
    }

    #[test]
    fn test_complex_enum_message_validator_good_input() {
        let msg = Amount::One(10);

        let encoded = Amount::encode_to_vec(&msg);
        let decoded = <Amount as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_complex_enum_message_validator_bad_input() {
        let msg = Amount::Two(0);

        let encoded = Amount::encode_to_vec(&msg);
        let result = <Amount as ProtoDecode>::decode(&encoded[..], DecodeContext::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Amount is zero"));
    }

    #[test]
    fn test_both_validators_both_bad() {
        let msg = MessageWithBothValidators {
            id: Id { id: 999 },     // Invalid: id cannot be 999
            scores: vec![500, 500], // Invalid: sum >= 1000
        };

        let encoded = MessageWithBothValidators::encode_to_vec(&msg);
        let result = <MessageWithBothValidators as ProtoDecode>::decode(&encoded[..], DecodeContext::default());
        // Should fail on field validation first
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Bad id"));
    }

    #[test]
    fn test_message_validator_runs_once_for_top_level_decode() {
        let _guard = VALIDATION_COUNTER_LOCK.lock().unwrap();
        COUNTED_MESSAGE_VALIDATOR_CALLS.store(0, Ordering::SeqCst);

        let msg = CountedMessage { value: 7 };
        let encoded = CountedMessage::encode_to_vec(&msg);
        let decoded = <CountedMessage as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();

        assert_eq!(decoded, msg);
        assert_eq!(COUNTED_MESSAGE_VALIDATOR_CALLS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_top_level_validator_runs_once_for_empty_payload() {
        let _guard = VALIDATION_COUNTER_LOCK.lock().unwrap();
        COUNTED_MESSAGE_VALIDATOR_CALLS.store(0, Ordering::SeqCst);

        let result = <CountedMessage as ProtoDecode>::decode(&[][..], DecodeContext::default());

        assert!(result.is_err());
        assert_eq!(COUNTED_MESSAGE_VALIDATOR_CALLS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_absent_default_nested_field_does_not_run_nested_validator() {
        let _guard = VALIDATION_COUNTER_LOCK.lock().unwrap();
        COUNTED_MESSAGE_VALIDATOR_CALLS.store(0, Ordering::SeqCst);

        let decoded = <CountedMessageHolder as ProtoDecode>::decode(&[][..], DecodeContext::default()).unwrap();

        assert_eq!(decoded, CountedMessageHolder::default());
        assert_eq!(COUNTED_MESSAGE_VALIDATOR_CALLS.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_message_validator_runs_once_for_nested_field_decode() {
        let _guard = VALIDATION_COUNTER_LOCK.lock().unwrap();
        COUNTED_MESSAGE_VALIDATOR_CALLS.store(0, Ordering::SeqCst);

        let msg = CountedMessageHolder {
            inner: CountedMessage { value: 9 },
        };
        let encoded = CountedMessageHolder::encode_to_vec(&msg);
        let decoded = <CountedMessageHolder as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();

        assert_eq!(decoded, msg);
        assert_eq!(COUNTED_MESSAGE_VALIDATOR_CALLS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_field_validator_runs_once_after_full_message_merge() {
        let _guard = VALIDATION_COUNTER_LOCK.lock().unwrap();
        COUNTED_FIELD_VALIDATOR_CALLS.store(0, Ordering::SeqCst);

        // Two occurrences of field 1 merge into the final value 2. The validator
        // must run once after decode, not once per field occurrence.
        let encoded = [0x08, 0x01, 0x08, 0x02];
        let decoded = <CountedFieldMessage as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();

        assert_eq!(decoded.value, 2);
        assert_eq!(COUNTED_FIELD_VALIDATOR_CALLS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_complex_enum_field_validator_runs_for_top_level_and_nested_decode() {
        let _guard = VALIDATION_COUNTER_LOCK.lock().unwrap();
        COUNTED_FIELD_VALIDATOR_CALLS.store(0, Ordering::SeqCst);

        let value = CountedComplexFieldEnum::Value(3);
        let encoded = CountedComplexFieldEnum::encode_to_vec(&value);
        let decoded = <CountedComplexFieldEnum as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(COUNTED_FIELD_VALIDATOR_CALLS.load(Ordering::SeqCst), 1);

        COUNTED_FIELD_VALIDATOR_CALLS.store(0, Ordering::SeqCst);
        let holder = CountedComplexFieldEnumHolder {
            value: CountedComplexFieldEnum::Value(4),
        };
        let encoded = CountedComplexFieldEnumHolder::encode_to_vec(&holder);
        let decoded = <CountedComplexFieldEnumHolder as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, holder);
        assert_eq!(COUNTED_FIELD_VALIDATOR_CALLS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_simple_enum_validator_runs_for_top_level_and_nested_decode() {
        let _guard = VALIDATION_COUNTER_LOCK.lock().unwrap();
        COUNTED_SIMPLE_ENUM_VALIDATOR_CALLS.store(0, Ordering::SeqCst);

        let encoded = CountedSimpleEnum::encode_to_vec(&CountedSimpleEnum::Valid);
        let decoded = <CountedSimpleEnum as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, CountedSimpleEnum::Valid);
        assert_eq!(COUNTED_SIMPLE_ENUM_VALIDATOR_CALLS.load(Ordering::SeqCst), 1);

        COUNTED_SIMPLE_ENUM_VALIDATOR_CALLS.store(0, Ordering::SeqCst);
        let holder = CountedSimpleEnumHolder {
            value: CountedSimpleEnum::AlsoValid,
        };
        let encoded = CountedSimpleEnumHolder::encode_to_vec(&holder);
        let decoded = <CountedSimpleEnumHolder as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, holder);
        assert_eq!(COUNTED_SIMPLE_ENUM_VALIDATOR_CALLS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_transparent_validator_runs_for_top_level_and_nested_decode() {
        let _guard = VALIDATION_COUNTER_LOCK.lock().unwrap();
        COUNTED_TRANSPARENT_VALIDATOR_CALLS.store(0, Ordering::SeqCst);

        let value = CountedTransparent(11);
        let encoded = CountedTransparent::encode_to_vec(&value);
        let decoded = <CountedTransparent as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(COUNTED_TRANSPARENT_VALIDATOR_CALLS.load(Ordering::SeqCst), 1);

        COUNTED_TRANSPARENT_VALIDATOR_CALLS.store(0, Ordering::SeqCst);
        let holder = CountedTransparentHolder {
            value: CountedTransparent(13),
        };
        let encoded = CountedTransparentHolder::encode_to_vec(&holder);
        let decoded = <CountedTransparentHolder as ProtoDecode>::decode(&encoded[..], DecodeContext::default()).unwrap();
        assert_eq!(decoded, holder);
        assert_eq!(COUNTED_TRANSPARENT_VALIDATOR_CALLS.load(Ordering::SeqCst), 1);
    }
}
