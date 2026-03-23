//CODEGEN BELOW - DO NOT TOUCH ME
pub mod atomic_types {
    use proto_rs::proto_message;

    #[proto_message]
    pub struct AtomicPrimitives {
        pub flag: bool,
        pub count: u64,
        pub small: u8,
        pub smaller: u16,
        pub signed_small: i8,
        pub signed_smaller: i16,
        pub signed: i32,
        pub sized: u64,
        pub signed_sized: i64,
    }

    #[proto_message]
    pub struct NonZeroPrimitives {
        pub nz_u8: ::core::num::NonZeroU8,
        pub nz_u16: ::core::num::NonZeroU16,
        pub nz_u32: ::core::num::NonZeroU32,
        pub nz_u64: ::core::num::NonZeroU64,
        pub nz_usize: ::core::num::NonZeroU64,
        pub nz_i8: ::core::num::NonZeroI8,
        pub nz_i16: ::core::num::NonZeroI16,
        pub nz_i32: ::core::num::NonZeroI32,
        pub nz_i64: ::core::num::NonZeroI64,
        pub nz_isize: ::core::num::NonZeroI64,
    }

}
pub mod goon_types {
    use proto_rs::proto_message;
    use chrono::DateTime;
    use chrono::Utc;

    #[proto_message]
    pub struct GoonPong {
        pub id: Id,
        pub status: ServiceStatus,
        pub expire_at: ::core::option::Option<DateTime<Utc>>,
        pub expire_at2: DateTime<Utc>,
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
        Active,
        Pending,
        Inactive,
        Completed,
    }

}
