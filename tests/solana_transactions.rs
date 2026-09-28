#![cfg(feature = "solana")]

use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::ProtoExt;
use proto_rs::encoding::DecodeContext;
use proto_rs::proto_message;
use solana_address::Address;
use solana_hash::Hash;
use solana_message::Message;
use solana_message::MessageHeader;
use solana_message::VersionedMessage;
use solana_message::compiled_instruction::CompiledInstruction;
use solana_message::v0;
use solana_message::v1;
use solana_signature::Signature;
use solana_transaction::Transaction;
use solana_transaction::versioned::VersionedTransaction;

fn roundtrip<T: ProtoEncode + ProtoDecode + ProtoExt + PartialEq + core::fmt::Debug>(value: &T) {
    let bytes = value.encode_to_vec();
    assert_eq!(&T::decode(bytes.as_slice(), DecodeContext::default()).unwrap(), value);
    assert_eq!(value.to_encoded_snapshot().as_bytes(), bytes);
}

fn messages() -> [VersionedMessage; 3] {
    let header = MessageHeader {
        num_required_signatures: 1,
        num_readonly_signed_accounts: 0,
        num_readonly_unsigned_accounts: 1,
    };
    let keys = vec![Address::new_from_array([1; 32]), Address::new_from_array([2; 32])];
    let hash = Hash::new_from_array([3; 32]);
    let instructions = vec![CompiledInstruction {
        program_id_index: 1,
        accounts: vec![0],
        data: vec![42; 2048],
    }];
    [
        VersionedMessage::Legacy(Message {
            header,
            account_keys: keys.clone(),
            recent_blockhash: Hash::new_from_array([3; 32]),
            instructions: instructions.clone(),
        }),
        VersionedMessage::V0(v0::Message {
            header,
            account_keys: keys.clone(),
            recent_blockhash: Hash::new_from_array([3; 32]),
            instructions: instructions.clone(),
            address_table_lookups: vec![v0::MessageAddressTableLookup {
                account_key: Address::new_from_array([4; 32]),
                writable_indexes: vec![0, 2],
                readonly_indexes: vec![1],
            }],
        }),
        VersionedMessage::V1(v1::Message {
            header,
            account_keys: keys,
            lifetime_specifier: hash,
            instructions,
            config: v1::TransactionConfig {
                priority_fee: Some(u64::MAX),
                compute_unit_limit: Some(1_400_000),
                loaded_accounts_data_size_limit: Some(64 * 1024 * 1024),
                heap_size: Some(64 * 1024),
            },
        }),
    ]
}

#[proto_message]
#[derive(Debug, PartialEq)]
struct Envelope {
    transaction: VersionedTransaction,
    messages: Vec<VersionedMessage>,
    optional_v1: Option<v1::Message>,
}

#[test]
fn native_messages_transactions_and_nested_fields_roundtrip_all_versions() {
    for message in messages() {
        roundtrip(&message);
        match &message {
            VersionedMessage::Legacy(value) => {
                roundtrip(value);
                roundtrip(&Transaction {
                    signatures: vec![Signature::from([7; 64])],
                    message: value.clone(),
                });
            }
            VersionedMessage::V0(value) => roundtrip(value),
            VersionedMessage::V1(value) => roundtrip(value),
        }
        let transaction = VersionedTransaction {
            signatures: vec![Signature::from([7; 64])],
            message: message.clone(),
        };
        roundtrip(&transaction);
        let mut with_unknown = transaction.encode_to_vec();
        with_unknown.extend_from_slice(&[0xa0, 0x06, 1]); // unknown field 100
        assert_eq!(
            VersionedTransaction::decode(with_unknown.as_slice(), DecodeContext::default()).unwrap(),
            transaction
        );
        let optional_v1 = match &message {
            VersionedMessage::V1(m) => Some(m.clone()),
            _ => None,
        };
        roundtrip(&Envelope {
            transaction,
            messages: vec![message],
            optional_v1,
        });
    }
}

#[derive(Clone, PartialEq, prost::Message)]
struct ConfigWire {
    #[prost(uint64, optional, tag = "1")]
    priority_fee: Option<u64>,
    #[prost(uint32, optional, tag = "2")]
    compute_unit_limit: Option<u32>,
    #[prost(uint32, optional, tag = "3")]
    loaded_accounts_data_size_limit: Option<u32>,
    #[prost(uint32, optional, tag = "4")]
    heap_size: Option<u32>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct V1Wire {
    #[prost(message, optional, tag = "5")]
    config: Option<ConfigWire>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct VersionWire {
    #[prost(message, optional, tag = "3")]
    v1: Option<V1Wire>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct TransactionWire {
    #[prost(message, optional, tag = "2")]
    message: Option<VersionWire>,
}

#[test]
fn v1_optional_config_distinguishes_absent_from_present_zero_and_matches_prost() {
    for bits in 0..16 {
        let config = v1::TransactionConfig {
            priority_fee: (bits & 1 != 0).then_some(0),
            compute_unit_limit: (bits & 2 != 0).then_some(0),
            loaded_accounts_data_size_limit: (bits & 4 != 0).then_some(0),
            heap_size: (bits & 8 != 0).then_some(0),
        };
        roundtrip(&config);
        let expected = ConfigWire {
            priority_fee: config.priority_fee,
            compute_unit_limit: config.compute_unit_limit,
            loaded_accounts_data_size_limit: config.loaded_accounts_data_size_limit,
            heap_size: config.heap_size,
        };
        assert_eq!(config.encode_to_vec(), prost::Message::encode_to_vec(&expected));
        let transaction = VersionedTransaction {
            signatures: vec![],
            message: VersionedMessage::V1(v1::Message {
                config,
                ..Default::default()
            }),
        };
        let wire = <TransactionWire as prost::Message>::decode(transaction.encode_to_vec().as_slice()).unwrap();
        assert_eq!(wire.message.unwrap().v1.unwrap().config.unwrap_or_default(), expected);
    }
}

#[proto_message]
struct MessageEnvelope {
    message: VersionedMessage,
}

#[test]
fn versioned_message_merges_fragments_and_switches_oneofs() {
    // Two V1 occurrences: header.required_signatures=1, then config.priority_fee=Some(0).
    let first = [0x1a, 4, 0x0a, 2, 8, 1];
    let second = [0x1a, 4, 0x2a, 2, 8, 0];
    let bytes = [first.as_slice(), second.as_slice()].concat();
    let value = VersionedMessage::decode(bytes.as_slice(), DecodeContext::default()).unwrap();
    let VersionedMessage::V1(message) = &value else {
        panic!("wrong version")
    };
    assert_eq!(message.header.num_required_signatures, 1);
    assert_eq!(message.config.priority_fee, Some(0));
    let nested = [&[0x0a, 6][..], &first, &[0x0a, 6], &second].concat();
    assert_eq!(
        MessageEnvelope::decode(nested.as_slice(), DecodeContext::default()).unwrap().message,
        value
    );
    let switched = VersionedMessage::decode(&[0x12, 0, 0x1a, 0][..], DecodeContext::default()).unwrap();
    assert!(matches!(switched, VersionedMessage::V1(_)));
}

#[test]
fn malformed_versioned_payloads_are_rejected() {
    for bytes in [&[0x1a, 4, 8][..], &[0x18, 1], &[0x1a, 2, 0x2a, 5]] {
        assert!(VersionedMessage::decode(bytes, DecodeContext::default()).is_err());
    }
    assert!(MessageHeader::decode(&[8, 0x80, 2][..], DecodeContext::default()).is_err()); // u8 overflow
}

#[test]
fn empty_versioned_messages_preserve_oneof_and_repeated_presence() {
    let values = vec![
        VersionedMessage::Legacy(Message::default()),
        VersionedMessage::V0(v0::Message::default()),
        VersionedMessage::V1(v1::Message::default()),
    ];
    for value in &values {
        roundtrip(value);
    }
    roundtrip(&values);
    roundtrip(&VersionedTransaction {
        signatures: vec![],
        message: VersionedMessage::Legacy(Message::default()),
    });
}

#[cfg(feature = "build-schemas")]
#[test]
fn generated_schema_has_distinct_message_versions_and_optional_config() {
    use proto_rs::schemas::ProtoIdentifiable;
    use proto_rs::schemas::ProtoType;
    use proto_rs::schemas::RustClientCtx;
    use proto_rs::schemas::write_only_these;

    let path = std::env::temp_dir().join(format!("proto-rs-solana-schema-{}.proto", std::process::id()));
    assert_eq!(
        write_only_these(&[("protos/solana.proto", path.to_str().unwrap())], &RustClientCtx::disabled()).unwrap(),
        1
    );
    let schema = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(schema.trim_end(), include_str!("../protos/solana.proto").trim_end());
    for name in [
        "LegacyMessage",
        "MessageV0",
        "MessageV1",
        "VersionedMessage",
        "VersionedTransaction",
        "TransactionConfig",
    ] {
        assert_eq!(schema.matches(&format!("message {name} {{")).count(), 1, "{name}");
    }
    assert!(schema.contains("optional uint64 priority_fee = 1;"));
    assert!(schema.contains("optional uint32 compute_unit_limit = 2;"));
    assert!(schema.contains("optional uint32 loaded_accounts_data_size_limit = 3;"));
    assert!(schema.contains("optional uint32 heap_size = 4;"));
    assert_eq!(Message::PROTO_TYPE, ProtoType::Message("LegacyMessage"));
    assert_eq!(v0::Message::PROTO_TYPE, ProtoType::Message("MessageV0"));
    assert_eq!(v1::Message::PROTO_TYPE, ProtoType::Message("MessageV1"));
    assert_eq!(VersionedMessage::PROTO_TYPE, ProtoType::Message("VersionedMessage"));
}
