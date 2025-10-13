#![cfg(any(feature = "fastnum", feature = "solana"))]

use proto_rs::ProtoExt;
use proto_rs::proto_message;

#[cfg(feature = "fastnum")]
mod fastnum_fields {
    use super::*;
    use fastnum::{D128, UD128, dec128, udec128};
    use proto_rs::custom_types::fastnum::DecimalProtoExt;

    #[proto_message(proto_path = "protos/tests/custom_fastnum.proto")]
    #[derive(Clone, Debug, PartialEq, Default)]
    struct DecimalHolder {
        #[proto(tag = 1)]
        signed: D128,
        #[proto(tag = 2)]
        unsigned: UD128,
        #[proto(tag = 3)]
        history: Vec<D128>,
    }

    #[proto_message(proto_path = "protos/tests/custom_fastnum.proto")]
    #[derive(Clone, Debug, PartialEq, Default)]
    enum DecimalEnvelope {
        #[default]
        Empty,
        #[proto(tag = 2)]
        Signed { value: D128 },
        #[proto(tag = 3)]
        Unsigned { value: UD128 },
    }

    #[test]
    fn decimal_struct_roundtrip() {
        let value = DecimalHolder {
            signed: dec128!(-1234.5678),
            unsigned: udec128!(9876.54321),
            history: vec![dec128!(1.5), dec128!(2.5)],
        };

        let encoded = value.encode_to_vec();
        let decoded = DecimalHolder::decode(encoded.as_slice()).expect("decode decimal struct");
        assert_eq!(decoded, value);
    }

    #[test]
    fn decimal_enum_roundtrip() {
        let variant = DecimalEnvelope::Signed { value: dec128!(-0.00123) };
        let encoded = variant.encode_to_vec();
        let decoded = DecimalEnvelope::decode(encoded.as_slice()).expect("decode decimal enum");
        assert_eq!(decoded, variant);
    }

    #[test]
    fn decimal_proto_helpers_roundtrip() {
        let original = dec128!(-42.001);
        let proto = original.to_proto();
        let restored = D128::from_proto(proto).expect("from_proto");
        assert_eq!(restored, original);
    }
}

#[cfg(feature = "solana")]
mod solana_fields {
    use super::*;
    use solana_address::Address;
    use solana_signature::Signature;

    const ADDRESS_BYTES: usize = solana_address::ADDRESS_BYTES;
    const SIGNATURE_BYTES: usize = solana_signature::SIGNATURE_BYTES;

    #[proto_message(proto_path = "protos/tests/custom_solana.proto")]
    #[derive(Clone, PartialEq, Default)]
    struct SolanaPayload {
        #[proto(tag = 1)]
        address: Address,
        #[proto(tag = 2)]
        signature: Signature,
    }

    #[test]
    fn solana_struct_roundtrip() {
        let address_bytes = core::array::from_fn(|idx| (idx as u8).wrapping_mul(7));
        let signature_bytes = core::array::from_fn(|idx| (idx as u8).wrapping_mul(3).wrapping_add(1));

        let value = SolanaPayload {
            address: Address::from(address_bytes),
            signature: Signature::from(signature_bytes),
        };

        let encoded = value.encode_to_vec();
        let decoded = SolanaPayload::decode(encoded.as_slice()).expect("decode solana payload");
        assert_eq!(decoded.address.as_ref(), value.address.as_ref());
        assert_eq!(decoded.signature.as_ref(), value.signature.as_ref());
        assert_eq!(decoded.address.as_ref().len(), ADDRESS_BYTES);
        assert_eq!(decoded.signature.as_ref().len(), SIGNATURE_BYTES);
    }
}
