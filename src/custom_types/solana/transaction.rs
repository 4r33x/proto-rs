//! Structured protobuf adapters, not Solana's native transaction wire format.
//! Encoding borrows payloads; callers still validate/sanitize before execution.
use solana_address::Address;
use solana_hash::Hash;
use solana_message::Message as LegacyMessage;
use solana_message::MessageHeader;
use solana_message::VersionedMessage as NativeVersionedMessage;
use solana_message::compiled_instruction::CompiledInstruction;
use solana_message::v0::Message as MessageV0;
use solana_message::v0::MessageAddressTableLookup;
use solana_message::v1::Message as MessageV1;
use solana_message::v1::TransactionConfig;
use solana_signature::Signature;
use solana_transaction::Transaction;
use solana_transaction::versioned::VersionedTransaction;

use crate::DecodeError;
use crate::DecodeIrBuilder;
use crate::DecodeState;
use crate::EncodeSizeHint;
use crate::ProtoArchive;
use crate::ProtoDecode;
use crate::ProtoDecoder;
use crate::ProtoDefault;
use crate::ProtoEncode;
use crate::ProtoExt;
use crate::ProtoKind;
use crate::ProtoShadowDecode;
use crate::ProtoShadowEncode;
use crate::RevWriter;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::proto_message;

extern crate self as proto_rs;

// All native structs have a matching owned decode shadow and a borrowed encode
// view. Keep tags explicit: these are protobuf tags, not native wire offsets.
macro_rules! solana_struct {
    ($proto:ident, $view:ident, $native:ident { $($field:ident: $ty:ty = $tag:literal),+ $(,)? }) => {
        // Match the native SDK field names, including MessageHeader's num_*.
        #[allow(clippy::struct_field_names)]
        pub struct $view<'a> { $($field: &'a $ty),+ }

        #[allow(dead_code, clippy::struct_field_names)]
        #[proto_message(proto_path = "protos/solana.proto", sun = [$native], sun_ir = $view<'a>)]
        pub struct $proto { $(#[proto(tag = $tag)] pub $field: $ty),+ }

        impl ProtoShadowDecode<$native> for $proto {
            fn to_sun(self) -> Result<$native, DecodeError> {
                Ok($native { $($field: self.$field),+ })
            }
        }

        impl<'a> ProtoShadowEncode<'a, $native> for $view<'a> {
            fn from_sun(value: &'a $native) -> Self {
                Self { $($field: &value.$field),+ }
            }
        }

        impl DecodeIrBuilder<$proto> for $native {
            // Uniform handling of both Copy fields and owned collections.
            #[allow(clippy::clone_on_copy)]
            fn build_ir(&self) -> Result<$proto, DecodeError> {
                Ok($proto { $($field: self.$field.clone()),+ })
            }
        }
    };
}

solana_struct!(
    MessageHeaderProto,
    MessageHeaderIr,
    MessageHeader {
        num_required_signatures: u8 = 1,
        num_readonly_signed_accounts: u8 = 2,
        num_readonly_unsigned_accounts: u8 = 3,
    }
);

solana_struct!(CompiledInstructionProto, CompiledInstructionIr, CompiledInstruction {
    program_id_index: u8 = 1,
    accounts: Vec<u8> = 2,
    data: Vec<u8> = 3,
});

solana_struct!(MessageAddressTableLookupProto, MessageAddressTableLookupIr, MessageAddressTableLookup {
    account_key: Address = 1,
    writable_indexes: Vec<u8> = 2,
    readonly_indexes: Vec<u8> = 3,
});

solana_struct!(LegacyMessageProto, LegacyMessageIr, LegacyMessage {
    header: MessageHeader = 1,
    account_keys: Vec<Address> = 2,
    recent_blockhash: Hash = 3,
    instructions: Vec<CompiledInstruction> = 4,
});

solana_struct!(MessageV0Proto, MessageV0Ir, MessageV0 {
    header: MessageHeader = 1,
    account_keys: Vec<Address> = 2,
    recent_blockhash: Hash = 3,
    instructions: Vec<CompiledInstruction> = 4,
    address_table_lookups: Vec<MessageAddressTableLookup> = 5,
});

solana_struct!(TransactionConfigProto, TransactionConfigIr, TransactionConfig {
    priority_fee: Option<u64> = 1,
    compute_unit_limit: Option<u32> = 2,
    loaded_accounts_data_size_limit: Option<u32> = 3,
    heap_size: Option<u32> = 4,
});

solana_struct!(MessageV1Proto, MessageV1Ir, MessageV1 {
    header: MessageHeader = 1,
    account_keys: Vec<Address> = 2,
    lifetime_specifier: Hash = 3,
    instructions: Vec<CompiledInstruction> = 4,
    config: TransactionConfig = 5,
});

#[proto_message(proto_path = "protos/solana.proto")]
pub enum VersionedMessage {
    #[proto(tag = 1)]
    Legacy(LegacyMessage),
    #[proto(tag = 2)]
    V0(MessageV0),
    #[proto(tag = 3)]
    V1(MessageV1),
}

impl From<NativeVersionedMessage> for VersionedMessage {
    fn from(value: NativeVersionedMessage) -> Self {
        match value {
            NativeVersionedMessage::Legacy(message) => Self::Legacy(message),
            NativeVersionedMessage::V0(message) => Self::V0(message),
            NativeVersionedMessage::V1(message) => Self::V1(message),
        }
    }
}

impl ProtoShadowDecode<NativeVersionedMessage> for VersionedMessage {
    fn to_sun(self) -> Result<NativeVersionedMessage, DecodeError> {
        Ok(match self {
            Self::Legacy(message) => NativeVersionedMessage::Legacy(message),
            Self::V0(message) => NativeVersionedMessage::V0(message),
            Self::V1(message) => NativeVersionedMessage::V1(message),
        })
    }
}

impl ProtoExt for NativeVersionedMessage {
    const KIND: ProtoKind = ProtoKind::Message;
}

#[cfg(feature = "build-schemas")]
impl crate::schemas::ProtoIdentifiable for NativeVersionedMessage {
    const PROTO_IDENT: crate::schemas::ProtoIdent = VersionedMessage::PROTO_IDENT;
    const PROTO_TYPE: crate::schemas::ProtoType = VersionedMessage::PROTO_TYPE;
}

impl ProtoDecode for NativeVersionedMessage {
    type ShadowDecoded = VersionedMessage;
}

impl ProtoDefault for NativeVersionedMessage {
    fn proto_default() -> Self {
        Self::Legacy(LegacyMessage::default())
    }
}

// Move, rather than clone, the current variant when merging fragments.
fn with_shadow(
    value: &mut NativeVersionedMessage,
    merge: impl FnOnce(&mut VersionedMessage) -> Result<(), DecodeError>,
) -> Result<(), DecodeError> {
    let mut shadow = VersionedMessage::from(core::mem::replace(value, NativeVersionedMessage::proto_default()));
    let result = merge(&mut shadow);
    *value = shadow.to_sun()?;
    result
}

impl ProtoDecoder for NativeVersionedMessage {
    fn merge_field(value: &mut Self, tag: u32, wire: WireType, buf: &mut impl bytes::Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        Self::merge_field_with_state(value, tag, wire, buf, ctx, &DecodeState::default())
    }

    fn merge_field_with_state(
        value: &mut Self,
        tag: u32,
        wire: WireType,
        buf: &mut impl bytes::Buf,
        ctx: DecodeContext,
        state: &DecodeState<'_>,
    ) -> Result<(), DecodeError> {
        with_shadow(value, |shadow| {
            VersionedMessage::merge_field_with_state(shadow, tag, wire, buf, ctx, state)
        })
    }

    fn merge_with_state(
        &mut self,
        wire: WireType,
        buf: &mut impl bytes::Buf,
        ctx: DecodeContext,
        state: &DecodeState<'_>,
    ) -> Result<(), DecodeError> {
        self.merge_message_fields(wire, buf, ctx, state)
    }

    fn finish(&mut self, state: &DecodeState<'_>) -> Result<(), DecodeError> {
        with_shadow(self, |shadow| shadow.finish(state))
    }
}

// The generic owned enum shadow would clone transaction payloads on encode.
// Borrow the native enum instead, writing the same explicit oneof tags.
impl ProtoEncode for NativeVersionedMessage {
    type Shadow<'a> = &'a Self;
}

impl<'a> ProtoShadowEncode<'a, NativeVersionedMessage> for &'a NativeVersionedMessage {
    fn from_sun(value: &'a NativeVersionedMessage) -> Self {
        value
    }
}

impl ProtoArchive for &NativeVersionedMessage {
    fn is_default(&self) -> bool {
        matches!(self, NativeVersionedMessage::Legacy(message) if message.is_default())
    }

    fn encoded_size_hint<const TAG: u32>(&self) -> EncodeSizeHint {
        if TAG == 0 && self.is_default() {
            return EncodeSizeHint::EMPTY;
        }
        let hint = match self {
            NativeVersionedMessage::Legacy(message) => message.size_hint::<1>(),
            NativeVersionedMessage::V0(message) => message.size_hint::<2>(),
            NativeVersionedMessage::V1(message) => message.size_hint::<3>(),
        };
        hint.for_field::<TAG>(Self::WIRE_TYPE)
    }

    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        if TAG == 0 && self.is_default() {
            return;
        }
        let mark = w.mark();
        match self {
            NativeVersionedMessage::Legacy(message) => message.archive::<1>(w),
            NativeVersionedMessage::V0(message) => message.archive::<2>(w),
            NativeVersionedMessage::V1(message) => message.archive::<3>(w),
        }
        if TAG != 0 {
            w.put_varint(w.written_since(mark) as u64);
            crate::ArchivedProtoField::<TAG, Self>::put_key(w);
        }
    }
}

impl ProtoArchive for NativeVersionedMessage {
    fn is_default(&self) -> bool {
        <&Self as ProtoArchive>::is_default(&self)
    }

    fn encoded_size_hint<const TAG: u32>(&self) -> EncodeSizeHint {
        <&Self as ProtoArchive>::encoded_size_hint::<TAG>(&self)
    }

    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        <&Self as ProtoArchive>::archive::<TAG>(&self, w);
    }
}

solana_struct!(TransactionProto, TransactionIr, Transaction {
    signatures: Vec<Signature> = 1,
    message: LegacyMessage = 2,
});

solana_struct!(VersionedTransactionProto, VersionedTransactionIr, VersionedTransaction {
    signatures: Vec<Signature> = 1,
    message: NativeVersionedMessage = 2,
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrowed_versioned_encoding_matches_the_generated_oneof() {
        for value in [
            NativeVersionedMessage::Legacy(LegacyMessage::default()),
            NativeVersionedMessage::V0(MessageV0::default()),
            NativeVersionedMessage::V1(MessageV1::default()),
        ] {
            assert_eq!(value.encode_to_vec(), VersionedMessage::from(value.clone()).encode_to_vec());
        }
    }
}
