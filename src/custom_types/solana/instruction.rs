use solana_address::Address;
use solana_instruction::AccountMeta;
use solana_instruction::Instruction;

use crate::DecodeError;
use crate::DecodeIrBuilder;
use crate::ProtoShadowDecode;
use crate::ProtoShadowEncode;
use crate::proto_message;

extern crate self as proto_rs;

#[allow(dead_code)]
#[proto_message(proto_path = "protos/solana.proto", sun = solana_instruction::AccountMeta)]
pub struct AccountMetaProto {
    #[proto(tag = 1)]
    pub pubkey: Address,
    #[proto(tag = 2)]
    pub is_signer: bool,
    #[proto(tag = 3)]
    pub is_writable: bool,
}

impl ProtoShadowDecode<AccountMeta> for AccountMetaProto {
    #[inline]
    fn to_sun(self) -> Result<AccountMeta, DecodeError> {
        Ok(AccountMeta {
            pubkey: self.pubkey,
            is_signer: self.is_signer,
            is_writable: self.is_writable,
        })
    }
}

impl<'a> ProtoShadowEncode<'a, AccountMeta> for AccountMetaProto {
    #[inline]
    fn from_sun(value: &'a AccountMeta) -> Self {
        Self {
            pubkey: value.pubkey,
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}

pub struct InstructionIr<'a> {
    program_id: &'a Address,
    accounts: &'a Vec<AccountMeta>,
    data: &'a Vec<u8>,
}

#[allow(dead_code)]
#[proto_message(proto_path = "protos/solana.proto", sun = [solana_instruction::Instruction], sun_ir = InstructionIr<'a>)]
pub struct InstructionProto {
    #[proto(tag = 1)]
    pub program_id: Address,
    #[proto(tag = 2)]
    pub accounts: Vec<AccountMeta>,
    #[proto(tag = 3)]
    pub data: Vec<u8>,
}

impl ProtoShadowDecode<Instruction> for InstructionProto {
    #[inline]
    fn to_sun(self) -> Result<Instruction, DecodeError> {
        Ok(Instruction {
            program_id: self.program_id,
            accounts: self.accounts,
            data: self.data,
        })
    }
}

impl<'a> ProtoShadowEncode<'a, Instruction> for InstructionIr<'a> {
    #[inline]
    fn from_sun(value: &'a Instruction) -> Self {
        Self {
            program_id: &value.program_id,
            accounts: &value.accounts,
            data: &value.data,
        }
    }
}

impl DecodeIrBuilder<InstructionProto> for Instruction {
    fn build_ir(&self) -> Result<InstructionProto, DecodeError> {
        Ok(InstructionProto {
            program_id: self.program_id,
            accounts: self.accounts.clone(),
            data: self.data.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProtoDecode;
    use crate::ProtoEncode;
    use crate::encoding::DecodeContext;

    #[allow(dead_code)]
    #[proto_message(proto_path = "protos/solana_test.proto")]
    struct AccountMetaWrapper {
        meta: AccountMeta,
    }

    #[allow(dead_code)]
    #[proto_message(proto_path = "protos/solana_test.proto")]
    struct InstructionWrapper {
        instruction: Instruction,
    }

    fn make_account_meta(seed: u8, is_signer: bool, is_writable: bool) -> AccountMeta {
        let bytes: [u8; 32] = [seed; 32];
        AccountMeta {
            pubkey: bytes.into(),
            is_signer,
            is_writable,
        }
    }

    fn sample_instruction() -> Instruction {
        Instruction {
            program_id: [0xABu8; 32].into(),
            accounts: vec![
                make_account_meta(0x01, true, true),
                make_account_meta(0x02, false, true),
                make_account_meta(0x03, false, false),
            ],
            data: vec![1, 2, 3, 4, 5, 6, 7, 8],
        }
    }

    #[test]
    fn account_meta_roundtrip() {
        let original = make_account_meta(0x11, true, false);
        let encoded = <AccountMeta as ProtoEncode>::encode_to_vec(&original);
        let decoded = <AccountMeta as ProtoDecode>::decode(encoded.as_slice(), DecodeContext::default()).expect("decode");
        assert_eq!(decoded.pubkey.as_array(), original.pubkey.as_array());
        assert_eq!(decoded.is_signer, original.is_signer);
        assert_eq!(decoded.is_writable, original.is_writable);
    }

    #[test]
    fn account_meta_readonly_roundtrip() {
        let original = make_account_meta(0x42, false, false);
        let encoded = <AccountMeta as ProtoEncode>::encode_to_vec(&original);
        let decoded = <AccountMeta as ProtoDecode>::decode(encoded.as_slice(), DecodeContext::default()).expect("decode");
        assert_eq!(decoded.pubkey.as_array(), original.pubkey.as_array());
        assert!(!decoded.is_signer);
        assert!(!decoded.is_writable);
    }

    #[test]
    fn instruction_roundtrip() {
        let original = sample_instruction();
        let encoded = <Instruction as ProtoEncode>::encode_to_vec(&original);
        let decoded = <Instruction as ProtoDecode>::decode(encoded.as_slice(), DecodeContext::default()).expect("decode");
        assert_eq!(decoded.program_id.as_array(), original.program_id.as_array());
        assert_eq!(decoded.accounts.len(), original.accounts.len());
        for (d, o) in decoded.accounts.iter().zip(original.accounts.iter()) {
            assert_eq!(d.pubkey.as_array(), o.pubkey.as_array());
            assert_eq!(d.is_signer, o.is_signer);
            assert_eq!(d.is_writable, o.is_writable);
        }
        assert_eq!(decoded.data, original.data);
    }

    #[test]
    fn instruction_empty_accounts_and_data() {
        let original = Instruction {
            program_id: [0xFFu8; 32].into(),
            accounts: vec![],
            data: vec![],
        };
        let encoded = <Instruction as ProtoEncode>::encode_to_vec(&original);
        let decoded = <Instruction as ProtoDecode>::decode(encoded.as_slice(), DecodeContext::default()).expect("decode");
        assert_eq!(decoded.program_id.as_array(), original.program_id.as_array());
        assert!(decoded.accounts.is_empty());
        assert!(decoded.data.is_empty());
    }

    #[test]
    fn instruction_zero_copy_bytes_match_encode() {
        let original = sample_instruction();
        let via_encode = <Instruction as ProtoEncode>::encode_to_vec(&original);
        let zero_copy = <Instruction as ProtoEncode>::to_zero_copy(&original);
        assert_eq!(zero_copy.as_bytes(), via_encode.as_slice());
    }
}
