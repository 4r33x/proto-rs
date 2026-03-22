//CODEGEN BELOW - DO NOT TOUCH ME
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
