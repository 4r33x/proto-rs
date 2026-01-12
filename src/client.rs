//CODEGEN BELOW - DO NOT TOUCH ME
pub mod custom_types {
    #[allow(unused_imports)]
    use proto_rs::{proto_message, proto_rpc};

    #[proto_message]
    pub struct CustomEx {
        pub mutex: MEx,
        pub mutex_copy: u64,
        pub mutex_custom: MEx,
        pub mutex_copy_custom: u64,
        pub arc: MEx,
        pub arc_copy: u64,
        pub arc_custom: MEx,
        pub arc_copy_custom: u64,
        pub boxed: MEx,
        pub box_copy: u64,
        pub boxed_custom: MEx,
        pub box_copy_custom: u64,
        pub custom_map: ::proto_rs::alloc::collections::BTreeMap<u32, MEx>,
        pub custom_option: ::core::option::Option<MEx>,
        pub custom_option_copy: ::core::option::Option<u64>,
        pub custom_vec_bytes: ::proto_rs::alloc::vec::Vec<u32>,
        pub custom_vec_deque_bytes: ::proto_rs::alloc::vec::Vec<u32>,
        pub custom_vec_copy: ::proto_rs::alloc::vec::Vec<u64>,
        pub custom_vec_deque_copy: ::proto_rs::alloc::vec::Vec<u64>,
        pub custom_vec: ::proto_rs::alloc::vec::Vec<MEx>,
        pub custom_vec_deque: ::proto_rs::alloc::vec::Vec<MEx>,
    }

    #[proto_message]
    pub struct MEx {
        pub id: u64,
    }

}
#[allow(clippy::upper_case_acronyms)]
pub mod extra_types {
    #[allow(unused_imports)]
    use proto_rs::{proto_message, proto_rpc};
    use crate::goon_types::GoonPong;
    use crate::goon_types::Id;
    use crate::goon_types::RizzPing;
    use crate::goon_types::ServiceStatus;

    const MY_CONST: usize = 1337;

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
        pub status: ::core::primitive::u32,
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
    use crate::custom_types::CustomEx;
    use crate::custom_types::MEx;
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

    #[allow(dead_code)]
    #[proto_rpc(rpc_package = "sigma_rpc", rpc_server = false, rpc_client = true)]
    pub trait SigmaRpc {
        type RizzUniStream: ::tonic::codegen::tokio_stream::Stream<Item = ::core::result::Result<FooResponse, ::tonic::Status>> + ::core::marker::Send;
        type RizzUni2Stream: ::tonic::codegen::tokio_stream::Stream<Item = ::core::result::Result<FooResponse, ::tonic::Status>> + ::core::marker::Send;

        async fn rizz_ping(
            &self,
            request: ::tonic::Request<RizzPing>,
        ) -> ::core::result::Result<::tonic::Response<GoonPong>, ::tonic::Status>;

        async fn rizz_uni(
            &self,
            request: ::tonic::Request<BarSub>,
        ) -> ::core::result::Result<::tonic::Response<Self::RizzUniStream>, ::tonic::Status>;

        async fn rizz_uni2(
            &self,
            request: ::tonic::Request<BarSub>,
        ) -> ::core::result::Result<::tonic::Response<Self::RizzUni2Stream>, ::tonic::Status>;

        #[allow(dead_code)]
        async fn build(
            &self,
            request: ::tonic::Request<Envelope<BuildRequest>>,
        ) -> ::core::result::Result<::tonic::Response<::core::primitive::u32>, ::tonic::Status>;

        async fn build2(
            &self,
            request: ::tonic::Request<Envelope<BuildRequest>>,
        ) -> ::core::result::Result<::tonic::Response<Envelope<BuildResponse>>, ::tonic::Status>;

        async fn owner_lookup(
            &self,
            request: ::tonic::Request<::core::primitive::u64>,
        ) -> ::core::result::Result<::tonic::Response<BuildResponse>, ::tonic::Status>;

        async fn custom_ex_echo(
            &self,
            request: ::tonic::Request<CustomEx>,
        ) -> ::core::result::Result<::tonic::Response<CustomEx>, ::tonic::Status>;

        async fn mutex_echo(
            &self,
            request: ::tonic::Request<::std::sync::Mutex<MEx>>,
        ) -> ::core::result::Result<::tonic::Response<::std::sync::Mutex<MEx>>, ::tonic::Status>;

        async fn arc_echo(
            &self,
            request: ::tonic::Request<::std::sync::Arc<MEx>>,
        ) -> ::core::result::Result<::tonic::Response<::std::sync::Arc<MEx>>, ::tonic::Status>;

        async fn box_echo(
            &self,
            request: ::tonic::Request<::std::boxed::Box<MEx>>,
        ) -> ::core::result::Result<::tonic::Response<::std::boxed::Box<MEx>>, ::tonic::Status>;

        async fn option_echo(
            &self,
            request: ::tonic::Request<::core::option::Option<MEx>>,
        ) -> ::core::result::Result<::tonic::Response<::core::option::Option<MEx>>, ::tonic::Status>;

        async fn vec_echo(
            &self,
            request: ::tonic::Request<::proto_rs::alloc::vec::Vec<MEx>>,
        ) -> ::core::result::Result<::tonic::Response<::proto_rs::alloc::vec::Vec<MEx>>, ::tonic::Status>;

        async fn vec_deque_echo(
            &self,
            request: ::tonic::Request<::proto_rs::alloc::vec::Vec<MEx>>,
        ) -> ::core::result::Result<::tonic::Response<::proto_rs::alloc::vec::Vec<MEx>>, ::tonic::Status>;

        async fn hash_map_echo(
            &self,
            request: ::tonic::Request<::proto_rs::alloc::collections::BTreeMap<u32, MEx>>,
        ) -> ::core::result::Result<::tonic::Response<::proto_rs::alloc::collections::BTreeMap<u32, MEx>>, ::tonic::Status>;

        async fn btree_map_echo(
            &self,
            request: ::tonic::Request<::proto_rs::alloc::collections::BTreeMap<u32, MEx>>,
        ) -> ::core::result::Result<::tonic::Response<::proto_rs::alloc::collections::BTreeMap<u32, MEx>>, ::tonic::Status>;

        async fn hash_set_echo(
            &self,
            request: ::tonic::Request<::proto_rs::alloc::vec::Vec<MEx>>,
        ) -> ::core::result::Result<::tonic::Response<::proto_rs::alloc::vec::Vec<MEx>>, ::tonic::Status>;

        async fn btree_set_echo(
            &self,
            request: ::tonic::Request<::proto_rs::alloc::vec::Vec<MEx>>,
        ) -> ::core::result::Result<::tonic::Response<::proto_rs::alloc::vec::Vec<MEx>>, ::tonic::Status>;

        async fn mex_echo(
            &self,
            request: ::tonic::Request<MEx>,
        ) -> ::core::result::Result<::tonic::Response<MEx>, ::tonic::Status>;

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
