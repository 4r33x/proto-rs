#![allow(deprecated)]

use core::cell::Cell;

use solana_instruction_error::InstructionError;
use solana_transaction_error::TransactionError;

use crate::DecodeError;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::proto_message;

extern crate self as proto_rs;

#[allow(clippy::enum_variant_names)]
#[proto_message(proto_path = "protos/solana.proto")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InstructionErrorProto {
    #[proto(tag = 1)]
    GenericError,
    #[proto(tag = 2)]
    InvalidArgument,
    #[proto(tag = 3)]
    InvalidInstructionData,
    #[proto(tag = 4)]
    InvalidAccountData,
    #[proto(tag = 5)]
    AccountDataTooSmall,
    #[proto(tag = 6)]
    InsufficientFunds,
    #[proto(tag = 7)]
    IncorrectProgramId,
    #[proto(tag = 8)]
    MissingRequiredSignature,
    #[proto(tag = 9)]
    AccountAlreadyInitialized,
    #[proto(tag = 10)]
    UninitializedAccount,
    #[proto(tag = 11)]
    UnbalancedInstruction,
    #[proto(tag = 12)]
    ModifiedProgramId,
    #[proto(tag = 13)]
    ExternalAccountLamportSpend,
    #[proto(tag = 14)]
    ExternalAccountDataModified,
    #[proto(tag = 15)]
    ReadonlyLamportChange,
    #[proto(tag = 16)]
    ReadonlyDataModified,
    #[proto(tag = 17)]
    DuplicateAccountIndex,
    #[proto(tag = 18)]
    ExecutableModified,
    #[proto(tag = 19)]
    RentEpochModified,
    #[proto(tag = 20)]
    NotEnoughAccountKeys,
    #[proto(tag = 21)]
    AccountDataSizeChanged,
    #[proto(tag = 22)]
    AccountNotExecutable,
    #[proto(tag = 23)]
    AccountBorrowFailed,
    #[proto(tag = 24)]
    AccountBorrowOutstanding,
    #[proto(tag = 25)]
    DuplicateAccountOutOfSync,
    #[proto(tag = 26)]
    Custom(#[proto(tag = 1)] u32),
    #[proto(tag = 27)]
    InvalidError,
    #[proto(tag = 28)]
    ExecutableDataModified,
    #[proto(tag = 29)]
    ExecutableLamportChange,
    #[proto(tag = 30)]
    ExecutableAccountNotRentExempt,
    #[proto(tag = 31)]
    UnsupportedProgramId,
    #[proto(tag = 32)]
    CallDepth,
    #[proto(tag = 33)]
    MissingAccount,
    #[proto(tag = 34)]
    ReentrancyNotAllowed,
    #[proto(tag = 35)]
    MaxSeedLengthExceeded,
    #[proto(tag = 36)]
    InvalidSeeds,
    #[proto(tag = 37)]
    InvalidRealloc,
    #[proto(tag = 38)]
    ComputationalBudgetExceeded,
    #[proto(tag = 39)]
    PrivilegeEscalation,
    #[proto(tag = 40)]
    ProgramEnvironmentSetupFailure,
    #[proto(tag = 41)]
    ProgramFailedToComplete,
    #[proto(tag = 42)]
    ProgramFailedToCompile,
    #[proto(tag = 43)]
    Immutable,
    #[proto(tag = 44)]
    IncorrectAuthority,
    #[proto(tag = 45)]
    BorshIoError,
    #[proto(tag = 46)]
    AccountNotRentExempt,
    #[proto(tag = 47)]
    InvalidAccountOwner,
    #[proto(tag = 48)]
    ArithmeticOverflow,
    #[proto(tag = 49)]
    UnsupportedSysvar,
    #[proto(tag = 50)]
    IllegalOwner,
    #[proto(tag = 51)]
    MaxAccountsDataAllocationsExceeded,
    #[proto(tag = 52)]
    MaxAccountsExceeded,
    #[proto(tag = 53)]
    MaxInstructionTraceLengthExceeded,
    #[proto(tag = 54)]
    BuiltinProgramsMustConsumeComputeUnits,
}

#[allow(deprecated)]
fn instruction_error_from_native(value: &InstructionError) -> InstructionErrorProto {
    match value {
        InstructionError::GenericError => InstructionErrorProto::GenericError,
        InstructionError::InvalidArgument => InstructionErrorProto::InvalidArgument,
        InstructionError::InvalidInstructionData => InstructionErrorProto::InvalidInstructionData,
        InstructionError::InvalidAccountData => InstructionErrorProto::InvalidAccountData,
        InstructionError::AccountDataTooSmall => InstructionErrorProto::AccountDataTooSmall,
        InstructionError::InsufficientFunds => InstructionErrorProto::InsufficientFunds,
        InstructionError::IncorrectProgramId => InstructionErrorProto::IncorrectProgramId,
        InstructionError::MissingRequiredSignature => InstructionErrorProto::MissingRequiredSignature,
        InstructionError::AccountAlreadyInitialized => InstructionErrorProto::AccountAlreadyInitialized,
        InstructionError::UninitializedAccount => InstructionErrorProto::UninitializedAccount,
        InstructionError::UnbalancedInstruction => InstructionErrorProto::UnbalancedInstruction,
        InstructionError::ModifiedProgramId => InstructionErrorProto::ModifiedProgramId,
        InstructionError::ExternalAccountLamportSpend => InstructionErrorProto::ExternalAccountLamportSpend,
        InstructionError::ExternalAccountDataModified => InstructionErrorProto::ExternalAccountDataModified,
        InstructionError::ReadonlyLamportChange => InstructionErrorProto::ReadonlyLamportChange,
        InstructionError::ReadonlyDataModified => InstructionErrorProto::ReadonlyDataModified,
        InstructionError::DuplicateAccountIndex => InstructionErrorProto::DuplicateAccountIndex,
        InstructionError::ExecutableModified => InstructionErrorProto::ExecutableModified,
        InstructionError::RentEpochModified => InstructionErrorProto::RentEpochModified,
        InstructionError::NotEnoughAccountKeys => InstructionErrorProto::NotEnoughAccountKeys,
        InstructionError::AccountDataSizeChanged => InstructionErrorProto::AccountDataSizeChanged,
        InstructionError::AccountNotExecutable => InstructionErrorProto::AccountNotExecutable,
        InstructionError::AccountBorrowFailed => InstructionErrorProto::AccountBorrowFailed,
        InstructionError::AccountBorrowOutstanding => InstructionErrorProto::AccountBorrowOutstanding,
        InstructionError::DuplicateAccountOutOfSync => InstructionErrorProto::DuplicateAccountOutOfSync,
        InstructionError::Custom(value) => InstructionErrorProto::Custom(*value),
        InstructionError::InvalidError => InstructionErrorProto::InvalidError,
        InstructionError::ExecutableDataModified => InstructionErrorProto::ExecutableDataModified,
        InstructionError::ExecutableLamportChange => InstructionErrorProto::ExecutableLamportChange,
        InstructionError::ExecutableAccountNotRentExempt => InstructionErrorProto::ExecutableAccountNotRentExempt,
        InstructionError::UnsupportedProgramId => InstructionErrorProto::UnsupportedProgramId,
        InstructionError::CallDepth => InstructionErrorProto::CallDepth,
        InstructionError::MissingAccount => InstructionErrorProto::MissingAccount,
        InstructionError::ReentrancyNotAllowed => InstructionErrorProto::ReentrancyNotAllowed,
        InstructionError::MaxSeedLengthExceeded => InstructionErrorProto::MaxSeedLengthExceeded,
        InstructionError::InvalidSeeds => InstructionErrorProto::InvalidSeeds,
        InstructionError::InvalidRealloc => InstructionErrorProto::InvalidRealloc,
        InstructionError::ComputationalBudgetExceeded => InstructionErrorProto::ComputationalBudgetExceeded,
        InstructionError::PrivilegeEscalation => InstructionErrorProto::PrivilegeEscalation,
        InstructionError::ProgramEnvironmentSetupFailure => InstructionErrorProto::ProgramEnvironmentSetupFailure,
        InstructionError::ProgramFailedToComplete => InstructionErrorProto::ProgramFailedToComplete,
        InstructionError::ProgramFailedToCompile => InstructionErrorProto::ProgramFailedToCompile,
        InstructionError::Immutable => InstructionErrorProto::Immutable,
        InstructionError::IncorrectAuthority => InstructionErrorProto::IncorrectAuthority,
        InstructionError::BorshIoError => InstructionErrorProto::BorshIoError,
        InstructionError::AccountNotRentExempt => InstructionErrorProto::AccountNotRentExempt,
        InstructionError::InvalidAccountOwner => InstructionErrorProto::InvalidAccountOwner,
        InstructionError::ArithmeticOverflow => InstructionErrorProto::ArithmeticOverflow,
        InstructionError::UnsupportedSysvar => InstructionErrorProto::UnsupportedSysvar,
        InstructionError::IllegalOwner => InstructionErrorProto::IllegalOwner,
        InstructionError::MaxAccountsDataAllocationsExceeded => InstructionErrorProto::MaxAccountsDataAllocationsExceeded,
        InstructionError::MaxAccountsExceeded => InstructionErrorProto::MaxAccountsExceeded,
        InstructionError::MaxInstructionTraceLengthExceeded => InstructionErrorProto::MaxInstructionTraceLengthExceeded,
        InstructionError::BuiltinProgramsMustConsumeComputeUnits => InstructionErrorProto::BuiltinProgramsMustConsumeComputeUnits,
    }
}

impl ProtoShadow<InstructionError> for InstructionErrorProto {
    type Sun<'a> = &'a InstructionError;
    type OwnedSun = InstructionError;
    type View<'a> = &'a InstructionErrorProto;

    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        let value = match self {
            Self::GenericError => InstructionError::GenericError,
            Self::InvalidArgument => InstructionError::InvalidArgument,
            Self::InvalidInstructionData => InstructionError::InvalidInstructionData,
            Self::InvalidAccountData => InstructionError::InvalidAccountData,
            Self::AccountDataTooSmall => InstructionError::AccountDataTooSmall,
            Self::InsufficientFunds => InstructionError::InsufficientFunds,
            Self::IncorrectProgramId => InstructionError::IncorrectProgramId,
            Self::MissingRequiredSignature => InstructionError::MissingRequiredSignature,
            Self::AccountAlreadyInitialized => InstructionError::AccountAlreadyInitialized,
            Self::UninitializedAccount => InstructionError::UninitializedAccount,
            Self::UnbalancedInstruction => InstructionError::UnbalancedInstruction,
            Self::ModifiedProgramId => InstructionError::ModifiedProgramId,
            Self::ExternalAccountLamportSpend => InstructionError::ExternalAccountLamportSpend,
            Self::ExternalAccountDataModified => InstructionError::ExternalAccountDataModified,
            Self::ReadonlyLamportChange => InstructionError::ReadonlyLamportChange,
            Self::ReadonlyDataModified => InstructionError::ReadonlyDataModified,
            Self::DuplicateAccountIndex => InstructionError::DuplicateAccountIndex,
            Self::ExecutableModified => InstructionError::ExecutableModified,
            Self::RentEpochModified => InstructionError::RentEpochModified,
            Self::NotEnoughAccountKeys => InstructionError::NotEnoughAccountKeys,
            Self::AccountDataSizeChanged => InstructionError::AccountDataSizeChanged,
            Self::AccountNotExecutable => InstructionError::AccountNotExecutable,
            Self::AccountBorrowFailed => InstructionError::AccountBorrowFailed,
            Self::AccountBorrowOutstanding => InstructionError::AccountBorrowOutstanding,
            Self::DuplicateAccountOutOfSync => InstructionError::DuplicateAccountOutOfSync,
            Self::Custom(value) => InstructionError::Custom(value),
            Self::InvalidError => InstructionError::InvalidError,
            Self::ExecutableDataModified => InstructionError::ExecutableDataModified,
            Self::ExecutableLamportChange => InstructionError::ExecutableLamportChange,
            Self::ExecutableAccountNotRentExempt => InstructionError::ExecutableAccountNotRentExempt,
            Self::UnsupportedProgramId => InstructionError::UnsupportedProgramId,
            Self::CallDepth => InstructionError::CallDepth,
            Self::MissingAccount => InstructionError::MissingAccount,
            Self::ReentrancyNotAllowed => InstructionError::ReentrancyNotAllowed,
            Self::MaxSeedLengthExceeded => InstructionError::MaxSeedLengthExceeded,
            Self::InvalidSeeds => InstructionError::InvalidSeeds,
            Self::InvalidRealloc => InstructionError::InvalidRealloc,
            Self::ComputationalBudgetExceeded => InstructionError::ComputationalBudgetExceeded,
            Self::PrivilegeEscalation => InstructionError::PrivilegeEscalation,
            Self::ProgramEnvironmentSetupFailure => InstructionError::ProgramEnvironmentSetupFailure,
            Self::ProgramFailedToComplete => InstructionError::ProgramFailedToComplete,
            Self::ProgramFailedToCompile => InstructionError::ProgramFailedToCompile,
            Self::Immutable => InstructionError::Immutable,
            Self::IncorrectAuthority => InstructionError::IncorrectAuthority,
            Self::BorshIoError => InstructionError::BorshIoError,
            Self::AccountNotRentExempt => InstructionError::AccountNotRentExempt,
            Self::InvalidAccountOwner => InstructionError::InvalidAccountOwner,
            Self::ArithmeticOverflow => InstructionError::ArithmeticOverflow,
            Self::UnsupportedSysvar => InstructionError::UnsupportedSysvar,
            Self::IllegalOwner => InstructionError::IllegalOwner,
            Self::MaxAccountsDataAllocationsExceeded => InstructionError::MaxAccountsDataAllocationsExceeded,
            Self::MaxAccountsExceeded => InstructionError::MaxAccountsExceeded,
            Self::MaxInstructionTraceLengthExceeded => InstructionError::MaxInstructionTraceLengthExceeded,
            Self::BuiltinProgramsMustConsumeComputeUnits => InstructionError::BuiltinProgramsMustConsumeComputeUnits,
        };

        Ok(value)
    }

    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        INSTRUCTION_ERROR_SHADOW.with(|cell| {
            cell.set(instruction_error_from_native(value));
            unsafe { &*cell.as_ptr() }
        })
    }
}

impl ProtoExt for InstructionError {
    type Shadow<'b> = InstructionErrorProto;

    #[inline(always)]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: crate::encoding::WireType, buf: &mut impl bytes::Buf, ctx: crate::encoding::DecodeContext) -> Result<(), DecodeError> {
        <InstructionErrorProto as ProtoExt>::merge_field(value, tag, wire_type, buf, ctx)
    }
}

#[allow(clippy::large_enum_variant)]
#[proto_message(proto_path = "protos/solana.proto")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TransactionErrorProto {
    #[proto(tag = 1)]
    AccountInUse,
    #[proto(tag = 2)]
    AccountLoadedTwice,
    #[proto(tag = 3)]
    AccountNotFound,
    #[proto(tag = 4)]
    ProgramAccountNotFound,
    #[proto(tag = 5)]
    InsufficientFundsForFee,
    #[proto(tag = 6)]
    InvalidAccountForFee,
    #[proto(tag = 7)]
    AlreadyProcessed,
    #[proto(tag = 8)]
    BlockhashNotFound,
    #[proto(tag = 9)]
    InstructionError {
        #[proto(tag = 1)]
        index: u8,
        #[proto(tag = 2, message)]
        error: InstructionErrorProto,
    },
    #[proto(tag = 10)]
    CallChainTooDeep,
    #[proto(tag = 11)]
    MissingSignatureForFee,
    #[proto(tag = 12)]
    InvalidAccountIndex,
    #[proto(tag = 13)]
    SignatureFailure,
    #[proto(tag = 14)]
    InvalidProgramForExecution,
    #[proto(tag = 15)]
    SanitizeFailure,
    #[proto(tag = 16)]
    ClusterMaintenance,
    #[proto(tag = 17)]
    AccountBorrowOutstanding,
    #[proto(tag = 18)]
    WouldExceedMaxBlockCostLimit,
    #[proto(tag = 19)]
    UnsupportedVersion,
    #[proto(tag = 20)]
    InvalidWritableAccount,
    #[proto(tag = 21)]
    WouldExceedMaxAccountCostLimit,
    #[proto(tag = 22)]
    WouldExceedAccountDataBlockLimit,
    #[proto(tag = 23)]
    TooManyAccountLocks,
    #[proto(tag = 24)]
    AddressLookupTableNotFound,
    #[proto(tag = 25)]
    InvalidAddressLookupTableOwner,
    #[proto(tag = 26)]
    InvalidAddressLookupTableData,
    #[proto(tag = 27)]
    InvalidAddressLookupTableIndex,
    #[proto(tag = 28)]
    InvalidRentPayingAccount,
    #[proto(tag = 29)]
    WouldExceedMaxVoteCostLimit,
    #[proto(tag = 30)]
    WouldExceedAccountDataTotalLimit,
    #[proto(tag = 31)]
    DuplicateInstruction(#[proto(tag = 1)] u8),
    #[proto(tag = 32)]
    InsufficientFundsForRent {
        #[proto(tag = 1)]
        account_index: u8,
    },
    #[proto(tag = 33)]
    MaxLoadedAccountsDataSizeExceeded,
    #[proto(tag = 34)]
    InvalidLoadedAccountsDataSizeLimit,
    #[proto(tag = 35)]
    ResanitizationNeeded,
    #[proto(tag = 36)]
    ProgramExecutionTemporarilyRestricted {
        #[proto(tag = 1)]
        account_index: u8,
    },
    #[proto(tag = 37)]
    UnbalancedTransaction,
    #[proto(tag = 38)]
    ProgramCacheHitMaxLimit,
    #[proto(tag = 39)]
    CommitCancelled,
}

std::thread_local! {
    static TRANSACTION_ERROR_SHADOW: Cell<TransactionErrorProto> =
        Cell::new(TransactionErrorProto::AccountInUse);
}

#[allow(deprecated)]
fn transaction_error_from_native(value: &TransactionError) -> TransactionErrorProto {
    match value {
        TransactionError::AccountInUse => TransactionErrorProto::AccountInUse,
        TransactionError::AccountLoadedTwice => TransactionErrorProto::AccountLoadedTwice,
        TransactionError::AccountNotFound => TransactionErrorProto::AccountNotFound,
        TransactionError::ProgramAccountNotFound => TransactionErrorProto::ProgramAccountNotFound,
        TransactionError::InsufficientFundsForFee => TransactionErrorProto::InsufficientFundsForFee,
        TransactionError::InvalidAccountForFee => TransactionErrorProto::InvalidAccountForFee,
        TransactionError::AlreadyProcessed => TransactionErrorProto::AlreadyProcessed,
        TransactionError::BlockhashNotFound => TransactionErrorProto::BlockhashNotFound,
        TransactionError::InstructionError(index, error) => TransactionErrorProto::InstructionError {
            index: *index,
            error: instruction_error_from_native(error),
        },
        TransactionError::CallChainTooDeep => TransactionErrorProto::CallChainTooDeep,
        TransactionError::MissingSignatureForFee => TransactionErrorProto::MissingSignatureForFee,
        TransactionError::InvalidAccountIndex => TransactionErrorProto::InvalidAccountIndex,
        TransactionError::SignatureFailure => TransactionErrorProto::SignatureFailure,
        TransactionError::InvalidProgramForExecution => TransactionErrorProto::InvalidProgramForExecution,
        TransactionError::SanitizeFailure => TransactionErrorProto::SanitizeFailure,
        TransactionError::ClusterMaintenance => TransactionErrorProto::ClusterMaintenance,
        TransactionError::AccountBorrowOutstanding => TransactionErrorProto::AccountBorrowOutstanding,
        TransactionError::WouldExceedMaxBlockCostLimit => TransactionErrorProto::WouldExceedMaxBlockCostLimit,
        TransactionError::UnsupportedVersion => TransactionErrorProto::UnsupportedVersion,
        TransactionError::InvalidWritableAccount => TransactionErrorProto::InvalidWritableAccount,
        TransactionError::WouldExceedMaxAccountCostLimit => TransactionErrorProto::WouldExceedMaxAccountCostLimit,
        TransactionError::WouldExceedAccountDataBlockLimit => TransactionErrorProto::WouldExceedAccountDataBlockLimit,
        TransactionError::TooManyAccountLocks => TransactionErrorProto::TooManyAccountLocks,
        TransactionError::AddressLookupTableNotFound => TransactionErrorProto::AddressLookupTableNotFound,
        TransactionError::InvalidAddressLookupTableOwner => TransactionErrorProto::InvalidAddressLookupTableOwner,
        TransactionError::InvalidAddressLookupTableData => TransactionErrorProto::InvalidAddressLookupTableData,
        TransactionError::InvalidAddressLookupTableIndex => TransactionErrorProto::InvalidAddressLookupTableIndex,
        TransactionError::InvalidRentPayingAccount => TransactionErrorProto::InvalidRentPayingAccount,
        TransactionError::WouldExceedMaxVoteCostLimit => TransactionErrorProto::WouldExceedMaxVoteCostLimit,
        TransactionError::WouldExceedAccountDataTotalLimit => TransactionErrorProto::WouldExceedAccountDataTotalLimit,
        TransactionError::DuplicateInstruction(index) => TransactionErrorProto::DuplicateInstruction(*index),
        TransactionError::InsufficientFundsForRent { account_index } => TransactionErrorProto::InsufficientFundsForRent { account_index: *account_index },
        TransactionError::MaxLoadedAccountsDataSizeExceeded => TransactionErrorProto::MaxLoadedAccountsDataSizeExceeded,
        TransactionError::InvalidLoadedAccountsDataSizeLimit => TransactionErrorProto::InvalidLoadedAccountsDataSizeLimit,
        TransactionError::ResanitizationNeeded => TransactionErrorProto::ResanitizationNeeded,
        TransactionError::ProgramExecutionTemporarilyRestricted { account_index } => TransactionErrorProto::ProgramExecutionTemporarilyRestricted { account_index: *account_index },
        TransactionError::UnbalancedTransaction => TransactionErrorProto::UnbalancedTransaction,
        TransactionError::ProgramCacheHitMaxLimit => TransactionErrorProto::ProgramCacheHitMaxLimit,
        TransactionError::CommitCancelled => TransactionErrorProto::CommitCancelled,
    }
}

#[allow(deprecated)]
impl ProtoShadow<TransactionError> for TransactionErrorProto {
    type Sun<'a> = &'a TransactionError;
    type OwnedSun = TransactionError;
    type View<'a> = &'a TransactionErrorProto;

    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        let value = match self {
            Self::AccountInUse => TransactionError::AccountInUse,
            Self::AccountLoadedTwice => TransactionError::AccountLoadedTwice,
            Self::AccountNotFound => TransactionError::AccountNotFound,
            Self::ProgramAccountNotFound => TransactionError::ProgramAccountNotFound,
            Self::InsufficientFundsForFee => TransactionError::InsufficientFundsForFee,
            Self::InvalidAccountForFee => TransactionError::InvalidAccountForFee,
            Self::AlreadyProcessed => TransactionError::AlreadyProcessed,
            Self::BlockhashNotFound => TransactionError::BlockhashNotFound,
            Self::InstructionError { index, error } => {
                let error = <InstructionErrorProto as ProtoShadow<InstructionError>>::to_sun(error)?;
                TransactionError::InstructionError(index, error)
            }
            Self::CallChainTooDeep => TransactionError::CallChainTooDeep,
            Self::MissingSignatureForFee => TransactionError::MissingSignatureForFee,
            Self::InvalidAccountIndex => TransactionError::InvalidAccountIndex,
            Self::SignatureFailure => TransactionError::SignatureFailure,
            Self::InvalidProgramForExecution => TransactionError::InvalidProgramForExecution,
            Self::SanitizeFailure => TransactionError::SanitizeFailure,
            Self::ClusterMaintenance => TransactionError::ClusterMaintenance,
            Self::AccountBorrowOutstanding => TransactionError::AccountBorrowOutstanding,
            Self::WouldExceedMaxBlockCostLimit => TransactionError::WouldExceedMaxBlockCostLimit,
            Self::UnsupportedVersion => TransactionError::UnsupportedVersion,
            Self::InvalidWritableAccount => TransactionError::InvalidWritableAccount,
            Self::WouldExceedMaxAccountCostLimit => TransactionError::WouldExceedMaxAccountCostLimit,
            Self::WouldExceedAccountDataBlockLimit => TransactionError::WouldExceedAccountDataBlockLimit,
            Self::TooManyAccountLocks => TransactionError::TooManyAccountLocks,
            Self::AddressLookupTableNotFound => TransactionError::AddressLookupTableNotFound,
            Self::InvalidAddressLookupTableOwner => TransactionError::InvalidAddressLookupTableOwner,
            Self::InvalidAddressLookupTableData => TransactionError::InvalidAddressLookupTableData,
            Self::InvalidAddressLookupTableIndex => TransactionError::InvalidAddressLookupTableIndex,
            Self::InvalidRentPayingAccount => TransactionError::InvalidRentPayingAccount,
            Self::WouldExceedMaxVoteCostLimit => TransactionError::WouldExceedMaxVoteCostLimit,
            Self::WouldExceedAccountDataTotalLimit => TransactionError::WouldExceedAccountDataTotalLimit,
            Self::DuplicateInstruction(index) => TransactionError::DuplicateInstruction(index),
            Self::InsufficientFundsForRent { account_index } => TransactionError::InsufficientFundsForRent { account_index },
            Self::MaxLoadedAccountsDataSizeExceeded => TransactionError::MaxLoadedAccountsDataSizeExceeded,
            Self::InvalidLoadedAccountsDataSizeLimit => TransactionError::InvalidLoadedAccountsDataSizeLimit,
            Self::ResanitizationNeeded => TransactionError::ResanitizationNeeded,
            Self::ProgramExecutionTemporarilyRestricted { account_index } => TransactionError::ProgramExecutionTemporarilyRestricted { account_index },
            Self::UnbalancedTransaction => TransactionError::UnbalancedTransaction,
            Self::ProgramCacheHitMaxLimit => TransactionError::ProgramCacheHitMaxLimit,
            Self::CommitCancelled => TransactionError::CommitCancelled,
        };

        Ok(value)
    }

    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        TRANSACTION_ERROR_SHADOW.with(|cell| {
            cell.set(transaction_error_from_native(value));
            unsafe { &*cell.as_ptr() }
        })
    }
}

impl ProtoExt for TransactionError {
    type Shadow<'b> = TransactionErrorProto;

    #[inline(always)]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: crate::encoding::WireType, buf: &mut impl bytes::Buf, ctx: crate::encoding::DecodeContext) -> Result<(), DecodeError> {
        <TransactionErrorProto as ProtoExt>::merge_field(value, tag, wire_type, buf, ctx)
    }
}
