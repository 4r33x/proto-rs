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

    #[proto_message]
    pub struct BuildRequest {
        #[proto(tag = 1)]
        pub config: BuildConfig,
        #[proto(tag = 2)]
        pub ping: RizzPing,
        #[proto(tag = 3)]
        pub owner: Id,
    }

    #[proto_message]
    pub struct BuildResponse {
        #[proto(tag = 1)]
        pub status: ServiceStatus,
        #[proto(tag = 2)]
        pub envelope: Envelope,
    }

    #[proto_message]
    pub struct Envelope<T> {
        #[proto(tag = 1)]
        pub payload: BuildRequest,
        #[proto(tag = 2)]
        pub trace_id: ::proto_rs::alloc::string::String,
    }

    #[proto_message]
    pub struct Envelope<T> {
        #[proto(tag = 1)]
        pub payload: BuildResponse,
        #[proto(tag = 2)]
        pub trace_id: ::proto_rs::alloc::string::String,
    }

    #[proto_message]
    pub struct Envelope<T> {
        #[proto(tag = 1)]
        pub payload: GoonPong,
        #[proto(tag = 2)]
        pub trace_id: ::proto_rs::alloc::string::String,
    }

}
pub mod fastnum {
    #[allow(unused_imports)]
    use proto_rs::{proto_message, proto_rpc};

    #[proto_message]
    pub struct D128Proto {
        #[proto(tag = 1)]
        pub lo: u64,
        #[proto(tag = 2)]
        pub hi: u64,
        #[proto(tag = 3)]
        pub fractional_digits_count: i32,
        #[proto(tag = 4)]
        pub is_negative: bool,
    }

    #[proto_message]
    pub struct D128Proto {
        #[proto(tag = 1)]
        pub lo: u64,
        #[proto(tag = 2)]
        pub hi: u64,
        #[proto(tag = 3)]
        pub fractional_digits_count: i32,
        #[proto(tag = 4)]
        pub is_negative: bool,
    }

    #[proto_message]
    pub struct UD128Proto {
        #[proto(tag = 1)]
        pub lo: u64,
        #[proto(tag = 2)]
        pub hi: u64,
        #[proto(tag = 3)]
        pub fractional_digits_count: i32,
    }

}
pub mod goon_types {
    #[allow(unused_imports)]
    use proto_rs::{proto_message, proto_rpc};

    #[proto_message]
    pub enum ServiceStatus {
        ACTIVE = 0,
        PENDING = 1,
        INACTIVE = 2,
        COMPLETED = 3,
    }

    #[proto_message]
    pub struct GoonPong {
        #[proto(tag = 1)]
        pub id: Id,
        #[proto(tag = 2)]
        pub status: ServiceStatus,
    }

    #[proto_message]
    pub struct Id {
        #[proto(tag = 1)]
        pub id: u64,
    }

    #[proto_message]
    pub struct RizzPing {
        #[proto(tag = 1)]
        pub id: Id,
        #[proto(tag = 2)]
        pub status: ServiceStatus,
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
    use crate::extra_types::BuildResponse;
    use crate::extra_types::Envelope;
    use crate::fastnum::D128Proto;
    use crate::fastnum::UD128Proto;
    use crate::goon_types::GoonPong;
    use crate::goon_types::Id;
    use crate::goon_types::RizzPing;
    use crate::rizz_types::BarSub;
    use crate::rizz_types::FooResponse;

    #[proto_rpc(rpc_package = "sigma_rpc", rpc_server = false, rpc_client = true)]
    pub trait SigmaRpc {
        type RizzUniStream: ::tonic::codegen::tokio_stream::Stream<Item = ::core::result::Result<FooResponse, ::tonic::Status>> + ::core::marker::Send + 'static;

        async fn rizz_ping(
            &self,
            request: ::tonic::Request<RizzPing>,
        ) -> ::core::result::Result<::tonic::Response<GoonPong>, ::tonic::Status>
        where
            Self: ::core::marker::Send + ::core::marker::Sync;

        async fn rizz_uni(
            &self,
            request: ::tonic::Request<BarSub>,
        ) -> ::core::result::Result<::tonic::Response<Self::RizzUniStream>, ::tonic::Status>
        where
            Self: ::core::marker::Send + ::core::marker::Sync;

        async fn build(
            &self,
            request: ::tonic::Request<Envelope>,
        ) -> ::core::result::Result<::tonic::Response<Envelope>, ::tonic::Status>
        where
            Self: ::core::marker::Send + ::core::marker::Sync;

        async fn owner_lookup(
            &self,
            request: ::tonic::Request<Id>,
        ) -> ::core::result::Result<::tonic::Response<BuildResponse>, ::tonic::Status>
        where
            Self: ::core::marker::Send + ::core::marker::Sync;

        async fn test_decimals(
            &self,
            request: ::tonic::Request<UD128Proto>,
        ) -> ::core::result::Result<::tonic::Response<D128Proto>, ::tonic::Status>
        where
            Self: ::core::marker::Send + ::core::marker::Sync;

    }

}
pub mod solana {
    #[allow(unused_imports)]
    use proto_rs::{proto_message, proto_rpc};

    #[proto_message]
    pub struct AddressProto {
        #[proto(tag = 1)]
        pub inner: ::proto_rs::alloc::vec::Vec<u8>,
    }

    #[proto_message]
    pub struct KeypairProto {
        #[proto(tag = 1)]
        pub inner: ::proto_rs::alloc::vec::Vec<u8>,
    }

    #[proto_message]
    pub struct SignatureProto {
        #[proto(tag = 1)]
        pub inner: ::proto_rs::alloc::vec::Vec<u8>,
    }

    #[proto_message]
    pub enum InstructionErrorProto {
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
            #[proto(tag = 1)]
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
    pub enum TransactionErrorProto {
        AccountInUse,
        AccountLoadedTwice,
        AccountNotFound,
        ProgramAccountNotFound,
        InsufficientFundsForFee,
        InvalidAccountForFee,
        AlreadyProcessed,
        BlockhashNotFound,
        InstructionError {
            #[proto(tag = 1)]
            index: u32,
            #[proto(tag = 2)]
            error: InstructionErrorProto,
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
            #[proto(tag = 1)]
            u32,
        ),
        InsufficientFundsForRent {
            #[proto(tag = 1)]
            account_index: u32,
        },
        MaxLoadedAccountsDataSizeExceeded,
        InvalidLoadedAccountsDataSizeLimit,
        ResanitizationNeeded,
        ProgramExecutionTemporarilyRestricted {
            #[proto(tag = 1)]
            account_index: u32,
        },
        UnbalancedTransaction,
        ProgramCacheHitMaxLimit,
        CommitCancelled,
    }

}
