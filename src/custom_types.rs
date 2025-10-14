// #[cfg(feature = "fastnum")]
// pub mod fastnum;

// #[cfg(feature = "solana")]
// pub mod solana;

use crate::proto_dump;

#[proto_dump(proto_path = "protos/fastnum.proto")]
struct D128Proto {
    #[proto(tag = 1)]
    /// Lower 64 bits of the digits
    pub lo: u64,
    #[proto(tag = 2)]
    /// Upper 64 bits of the digits
    pub hi: u64,
    #[proto(tag = 3)]
    /// Fractional digits count (can be negative for scientific notation)
    pub fractional_digits_count: i32,
    #[proto(tag = 4)]
    /// Sign bit: true for negative, false for positive/zero
    pub is_negative: bool,
}
