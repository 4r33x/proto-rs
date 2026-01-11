//CODEGEN BELOW - DO NOT TOUCH ME
pub mod extra_types {
    #[allow(unused_imports)]
    use proto_rs::{proto_message, proto_rpc};
    use crate::goon_types::GoonPong;
    use crate::goon_types::Id;
    use crate::goon_types::RizzPing;
    use crate::goon_types::ServiceStatus;

    #[proto_message]
    pub struct BuildConfig {
        #[proto(tag = 1, into = "i64")]
        pub timeout: i64,
        #[proto(tag = 3)]
        pub owner: Id,
    }

    #[allow(dead_code)]
    #[proto_message]
    pub struct BuildRequest {
        pub config: BuildConfig,
        pub ping: RizzPing,
        pub owner: Id,
    }

    #[proto_message]
    pub struct BuildResponse {
        #[allow(dead_code)]
        pub status: ServiceStatus,
        pub envelope: Envelope<GoonPong>,
    }

    #[proto_message]
    pub struct Envelope<T> {
        pub payload: T,
        pub trace_id: ::proto_rs::alloc::string::String,
    }

}
pub mod fastnum {
    #[allow(unused_imports)]
    use proto_rs::{proto_message, proto_rpc};

    #[proto_message]
    pub struct D128 {
        pub lo: u64,
        pub hi: u64,
        pub fractional_digits_count: i32,
        pub is_negative: bool,
    }

}
pub mod goon_types {
    #[allow(unused_imports)]
    use proto_rs::{proto_message, proto_rpc};

    #[proto_message]
    pub struct GoonPong {
        pub id: Id,
        pub status: ServiceStatus,
    }

    #[proto_message]
    pub struct Id {
        pub id: u64,
    }

    #[proto_message]
    pub struct RizzPing {
        pub id: Id,
        pub status: ServiceStatus,
    }

    #[proto_message]
    pub enum ServiceStatus {
        ACTIVE = 0,
        PENDING = 1,
        INACTIVE = 2,
        COMPLETED = 3,
    }

}
pub mod rizz_types {
    #[allow(unused_imports)]
    use proto_rs::{proto_message, proto_rpc};

    #[proto_message]
    pub struct BarSub;

    #[proto_message]
    pub struct FooResponse;

}
pub mod sigma_rpc_simple {
    #[allow(unused_imports)]
    use proto_rs::{proto_message, proto_rpc};
    use crate::extra_types::BuildRequest;
    use crate::extra_types::BuildResponse;
    use crate::extra_types::Envelope;
    use crate::fastnum::D128;
    use crate::goon_types::GoonPong;
    use crate::goon_types::Id;
    use crate::goon_types::RizzPing;
    use crate::rizz_types::BarSub;
    use crate::rizz_types::FooResponse;
    use fastnum::UD128;

    #[proto_rpc(rpc_package = "sigma_rpc", rpc_server = false, rpc_client = true)]
    pub trait SigmaRpc {
        type RizzUniStream: ::tonic::codegen::tokio_stream::Stream<Item = ::core::result::Result<FooResponse, ::tonic::Status>> + ::core::marker::Send;

        async fn rizz_ping(
            &self,
            request: ::tonic::Request<RizzPing>,
        ) -> ::core::result::Result<::tonic::Response<GoonPong>, ::tonic::Status>;

        async fn rizz_uni(
            &self,
            request: ::tonic::Request<BarSub>,
        ) -> ::core::result::Result<::tonic::Response<Self::RizzUniStream>, ::tonic::Status>;

        #[allow(dead_code)]
        async fn build(
            &self,
            request: ::tonic::Request<Envelope<BuildRequest>>,
        ) -> ::core::result::Result<::tonic::Response<Envelope<BuildResponse>>, ::tonic::Status>;

        async fn owner_lookup(
            &self,
            request: ::tonic::Request<Id>,
        ) -> ::core::result::Result<::tonic::Response<BuildResponse>, ::tonic::Status>;

        async fn test_decimals(
            &self,
            request: ::tonic::Request<UD128>,
        ) -> ::core::result::Result<::tonic::Response<D128>, ::tonic::Status>;

    }

}
pub mod solana {
    #[allow(unused_imports)]
    use proto_rs::{proto_message, proto_rpc};

    #[proto_message]
    pub struct Address {
        pub inner: [u8; BYTES],
    }

    #[proto_message]
    pub enum InstructionError {
        GenericError,
        InvalidArgument,
        InvalidInstructionData,
        InvalidAccountData,
        AccountDataTooSmall,
        InsufficientFunds,
        IncorrectProgramId,
        MissingRequiredSignature,
        AccountAlreadyInitialized,
        UninitializedAccount,
        UnbalancedInstruction,
        ModifiedProgramId,
        ExternalAccountLamportSpend,
        ExternalAccountDataModified,
        ReadonlyLamportChange,
        ReadonlyDataModified,
        DuplicateAccountIndex,
        ExecutableModified,
        RentEpochModified,
        NotEnoughAccountKeys,
        AccountDataSizeChanged,
        AccountNotExecutable,
        AccountBorrowFailed,
        AccountBorrowOutstanding,
        DuplicateAccountOutOfSync,
        Custom(
            u32,
        ),
        InvalidError,
        ExecutableDataModified,
        ExecutableLamportChange,
        ExecutableAccountNotRentExempt,
        UnsupportedProgramId,
        CallDepth,
        MissingAccount,
        ReentrancyNotAllowed,
        MaxSeedLengthExceeded,
        InvalidSeeds,
        InvalidRealloc,
        ComputationalBudgetExceeded,
        PrivilegeEscalation,
        ProgramEnvironmentSetupFailure,
        ProgramFailedToComplete,
        ProgramFailedToCompile,
        Immutable,
        IncorrectAuthority,
        BorshIoError,
        AccountNotRentExempt,
        InvalidAccountOwner,
        ArithmeticOverflow,
        UnsupportedSysvar,
        IllegalOwner,
        MaxAccountsDataAllocationsExceeded,
        MaxAccountsExceeded,
        MaxInstructionTraceLengthExceeded,
        BuiltinProgramsMustConsumeComputeUnits,
    }

    #[proto_message]
    pub struct Keypair {
        pub inner: [u8; BYTES],
    }

    #[proto_message]
    pub struct Signature {
        pub inner: [u8; BYTES],
    }

    #[proto_message]
    pub enum TransactionError {
        AccountInUse,
        AccountLoadedTwice,
        AccountNotFound,
        ProgramAccountNotFound,
        InsufficientFundsForFee,
        InvalidAccountForFee,
        AlreadyProcessed,
        BlockhashNotFound,
        InstructionError {
            index: u32,
            error: InstructionError,
        },
        CallChainTooDeep,
        MissingSignatureForFee,
        InvalidAccountIndex,
        SignatureFailure,
        InvalidProgramForExecution,
        SanitizeFailure,
        ClusterMaintenance,
        AccountBorrowOutstanding,
        WouldExceedMaxBlockCostLimit,
        UnsupportedVersion,
        InvalidWritableAccount,
        WouldExceedMaxAccountCostLimit,
        WouldExceedAccountDataBlockLimit,
        TooManyAccountLocks,
        AddressLookupTableNotFound,
        InvalidAddressLookupTableOwner,
        InvalidAddressLookupTableData,
        InvalidAddressLookupTableIndex,
        InvalidRentPayingAccount,
        WouldExceedMaxVoteCostLimit,
        WouldExceedAccountDataTotalLimit,
        DuplicateInstruction(
            u32,
        ),
        InsufficientFundsForRent {
            account_index: u32,
        },
        MaxLoadedAccountsDataSizeExceeded,
        InvalidLoadedAccountsDataSizeLimit,
        ResanitizationNeeded,
        ProgramExecutionTemporarilyRestricted {
            account_index: u32,
        },
        UnbalancedTransaction,
        ProgramCacheHitMaxLimit,
        CommitCancelled,
    }

}
