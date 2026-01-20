#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type, maybe_uninit_array_assume_init))]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unused_self)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::inline_always)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate self as proto_rs;

pub use prosto_derive::impl_proto_ident;
pub use prosto_derive::inject_proto_import;
pub use prosto_derive::proto_dump;
pub use prosto_derive::proto_message;
pub use prosto_derive::proto_rpc;
pub use traits::const_test_validate_with_ext;

#[cfg(not(feature = "no-recursion-limit"))]
const RECURSION_LIMIT: u32 = 100;

#[doc(hidden)]
pub extern crate alloc;

// Re-export the bytes crate for use within derived code.
pub use bytes;

mod coders;
// mod custom_types;
#[cfg(feature = "tonic")]
mod tonic;
mod types;
mod wrappers;
mod zero_copy;

#[doc(hidden)]
pub mod encoding;
mod error;
mod name;
mod traits;

/// Build-time proto schema registry
/// Only available when "build-schemas" feature is enabled
#[cfg(all(feature = "build-schemas", feature = "std"))]
pub mod schemas;

pub use crate::coders::BytesMode;
pub use crate::coders::ProtoCodec;
pub use crate::coders::ProtoEncoder;
pub use crate::coders::SunByRef;
pub use crate::coders::SunByVal;
pub use crate::encoding::length_delimiter::decode_length_delimiter;
pub use crate::encoding::length_delimiter::encode_length_delimiter;
pub use crate::encoding::length_delimiter::length_delimiter_len;
pub use crate::error::DecodeError;
pub use crate::error::EncodeError;
pub use crate::error::UnknownEnumValue;
pub use crate::name::Name;
#[cfg(feature = "tonic")]
pub use crate::tonic::EncoderExt;
#[cfg(feature = "tonic")]
pub use crate::tonic::ProtoRequest;
#[cfg(feature = "tonic")]
pub use crate::tonic::ProtoResponse;
#[cfg(feature = "tonic")]
pub use crate::tonic::ToZeroCopyRequest;
#[cfg(feature = "tonic")]
pub use crate::tonic::ToZeroCopyResponse;
#[cfg(feature = "tonic")]
pub use crate::tonic::ZeroCopyRequest;
#[cfg(feature = "tonic")]
pub use crate::tonic::ZeroCopyResponse;
#[cfg(feature = "tonic")]
pub use crate::tonic::map_proto_response;
#[cfg(feature = "tonic")]
pub use crate::tonic::map_proto_stream_result;
pub use crate::traits::ProtoArchive;
pub use crate::traits::ProtoDecode;
pub use crate::traits::ProtoDecoder;
pub use crate::traits::ProtoEncode;
pub use crate::traits::ProtoExt;
pub use crate::traits::ProtoKind;
pub use crate::traits::ProtoShadowDecode;
pub use crate::traits::ProtoShadowEncode;
pub use crate::traits::ArchivedProtoInner;
// #[cfg(feature = "papaya")]
// pub use crate::wrappers::conc_map::papaya_map_encode_input;
// #[cfg(feature = "papaya")]
// pub use crate::wrappers::conc_set::papaya_set_encode_input;
pub use crate::zero_copy::ToZeroCopy;
pub use crate::zero_copy::ZeroCopy;

// Example build.rs that users can copy:
#[cfg(all(feature = "build-schemas", feature = "std", doc))]
/// Example build.rs for consuming projects
///
/// ```no_run
/// // build.rs
/// fn main() {
///     // Only generate protos when explicitly requested
///     if std::env::var("GENERATE_PROTOS").is_ok() {
///         match proto_rs::schemas::write_all("protos", &proto_rs::schemas::RustClientCtx::disabled()) {
///             Ok(count) => println!("Generated {} proto files", count),
///             Err(e) => panic!("Failed to generate protos: {}", e),
///         }
///     }
/// }
/// ```
mod _build_example {}
