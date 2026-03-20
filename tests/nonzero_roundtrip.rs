#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]

use core::num::NonZeroI8;
use core::num::NonZeroI16;
use core::num::NonZeroI32;
use core::num::NonZeroI64;
use core::num::NonZeroIsize;
use core::num::NonZeroU8;
use core::num::NonZeroU16;
use core::num::NonZeroU32;
use core::num::NonZeroU64;
use core::num::NonZeroUsize;

use bytes::Bytes;
use bytes::BytesMut;
use prost::Message as ProstMessage;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::ProtoExt;
use proto_rs::encoding::DecodeContext;
use proto_rs::proto_message;

/// All `NonZero` primitive types in a single message.
#[proto_message(proto_path = "protos/tests/nonzero_types.proto")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonZeroPrimitives {
    pub nzu8: NonZeroU8,
    pub nzu16: NonZeroU16,
    pub nzu32: NonZeroU32,
    pub nzu64: NonZeroU64,
    pub nzi8: NonZeroI8,
    pub nzi16: NonZeroI16,
    pub nzi32: NonZeroI32,
    pub nzi64: NonZeroI64,
    pub nzusize: NonZeroUsize,
    pub nzisize: NonZeroIsize,
}

/// Real-world example: server configuration where every value must be non-zero.
#[proto_message(proto_path = "protos/tests/nonzero_types.proto")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerConfig {
    /// Listening port — never 0.
    pub port: NonZeroU16,
    /// Maximum concurrent connections — never 0.
    pub max_connections: NonZeroU32,
    /// Thread pool size — never 0.
    pub worker_threads: NonZeroU8,
    /// Request timeout in milliseconds — never 0.
    pub request_timeout_ms: NonZeroU64,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct NonZeroPrimitivesProst {
    #[prost(uint32, tag = "1")]
    pub nzu8: u32,
    #[prost(uint32, tag = "2")]
    pub nzu16: u32,
    #[prost(uint32, tag = "3")]
    pub nzu32: u32,
    #[prost(uint64, tag = "4")]
    pub nzu64: u64,
    #[prost(int32, tag = "5")]
    pub nzi8: i32,
    #[prost(int32, tag = "6")]
    pub nzi16: i32,
    #[prost(int32, tag = "7")]
    pub nzi32: i32,
    #[prost(int64, tag = "8")]
    pub nzi64: i64,
    #[prost(uint64, tag = "9")]
    pub nzusize: u64,
    #[prost(int64, tag = "10")]
    pub nzisize: i64,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct ServerConfigProst {
    #[prost(uint32, tag = "1")]
    pub port: u32,
    #[prost(uint32, tag = "2")]
    pub max_connections: u32,
    #[prost(uint32, tag = "3")]
    pub worker_threads: u32,
    #[prost(uint64, tag = "4")]
    pub request_timeout_ms: u64,
}

fn encode_proto<M: ProtoEncode + ProtoExt>(value: &M) -> Bytes {
    Bytes::from(M::encode_to_vec(value))
}

fn encode_prost<M: ProstMessage>(value: &M) -> Bytes {
    let mut buf = BytesMut::with_capacity(value.encoded_len());
    value.encode(&mut buf).expect("prost encode failed");
    buf.freeze()
}

fn decode_proto<M: ProtoDecode>(bytes: Bytes) -> M {
    M::decode(bytes, DecodeContext::default()).expect("proto decode failed")
}

const fn sample_nonzero_primitives() -> NonZeroPrimitives {
    NonZeroPrimitives {
        nzu8: NonZeroU8::new(200).unwrap(),
        nzu16: NonZeroU16::new(1000).unwrap(),
        nzu32: NonZeroU32::new(100_000).unwrap(),
        nzu64: NonZeroU64::new(10_000_000_000).unwrap(),
        nzi8: NonZeroI8::new(-100).unwrap(),
        nzi16: NonZeroI16::new(-1000).unwrap(),
        nzi32: NonZeroI32::new(-100_000).unwrap(),
        nzi64: NonZeroI64::new(-10_000_000_000).unwrap(),
        nzusize: NonZeroUsize::new(usize::MAX / 2).unwrap(),
        nzisize: NonZeroIsize::new(isize::MIN / 2).unwrap(),
    }
}

const fn sample_server_config() -> ServerConfig {
    ServerConfig {
        port: NonZeroU16::new(8080).unwrap(),
        max_connections: NonZeroU32::new(10_000).unwrap(),
        worker_threads: NonZeroU8::new(8).unwrap(),
        request_timeout_ms: NonZeroU64::new(30_000).unwrap(),
    }
}

#[test]
fn nonzero_primitives_roundtrip() {
    let msg = sample_nonzero_primitives();
    let bytes = encode_proto(&msg);
    let decoded: NonZeroPrimitives = decode_proto(bytes);
    assert_eq!(decoded, msg);
}

#[test]
fn server_config_roundtrip() {
    let msg = sample_server_config();
    let bytes = encode_proto(&msg);
    let decoded: ServerConfig = decode_proto(bytes);
    assert_eq!(decoded, msg);
}

#[test]
fn nonzero_defaults_not_encoded() {
    // A message where every field equals its proto_default (MAX) encodes to nothing.
    let msg = NonZeroPrimitives {
        nzu8: NonZeroU8::MAX,
        nzu16: NonZeroU16::MAX,
        nzu32: NonZeroU32::MAX,
        nzu64: NonZeroU64::MAX,
        nzi8: NonZeroI8::MAX,
        nzi16: NonZeroI16::MAX,
        nzi32: NonZeroI32::MAX,
        nzi64: NonZeroI64::MAX,
        nzusize: NonZeroUsize::new(usize::MAX).unwrap(),
        nzisize: NonZeroIsize::new(isize::MAX).unwrap(),
    };
    let bytes = encode_proto(&msg);
    assert!(bytes.is_empty(), "all-MAX message should encode to empty payload");
}

#[test]
fn nonzero_defaults_roundtrip_from_empty_bytes() {
    // Decoding empty bytes yields all-MAX (proto_default) values.
    let decoded: NonZeroPrimitives = decode_proto(Bytes::new());
    assert_eq!(decoded.nzu8, NonZeroU8::MAX);
    assert_eq!(decoded.nzu16, NonZeroU16::MAX);
    assert_eq!(decoded.nzu32, NonZeroU32::MAX);
    assert_eq!(decoded.nzu64, NonZeroU64::MAX);
    assert_eq!(decoded.nzi8, NonZeroI8::MAX);
    assert_eq!(decoded.nzi16, NonZeroI16::MAX);
    assert_eq!(decoded.nzi32, NonZeroI32::MAX);
    assert_eq!(decoded.nzi64, NonZeroI64::MAX);
    assert_eq!(decoded.nzusize, NonZeroUsize::new(usize::MAX).unwrap());
    assert_eq!(decoded.nzisize, NonZeroIsize::new(isize::MAX).unwrap());
}

#[test]
fn nonzero_cross_library_compatibility() {
    let proto_msg = sample_nonzero_primitives();
    let prost_equivalent = NonZeroPrimitivesProst {
        nzu8: proto_msg.nzu8.get().into(),
        nzu16: proto_msg.nzu16.get().into(),
        nzu32: proto_msg.nzu32.get(),
        nzu64: proto_msg.nzu64.get(),
        nzi8: proto_msg.nzi8.get().into(),
        nzi16: proto_msg.nzi16.get().into(),
        nzi32: proto_msg.nzi32.get(),
        nzi64: proto_msg.nzi64.get(),
        nzusize: proto_msg.nzusize.get() as u64,
        nzisize: proto_msg.nzisize.get() as i64,
    };

    // proto_rs → prost
    let proto_bytes = encode_proto(&proto_msg);
    let prost_decoded = NonZeroPrimitivesProst::decode(proto_bytes.clone()).expect("prost decode from proto bytes");
    assert_eq!(prost_decoded, prost_equivalent);

    // prost → proto_rs
    let prost_bytes = encode_prost(&prost_equivalent);
    let proto_decoded: NonZeroPrimitives = decode_proto(prost_bytes.clone());
    assert_eq!(proto_decoded, proto_msg);
}

#[test]
fn server_config_cross_library_compatibility() {
    let proto_msg = sample_server_config();
    let prost_equivalent = ServerConfigProst {
        port: proto_msg.port.get().into(),
        max_connections: proto_msg.max_connections.get(),
        worker_threads: proto_msg.worker_threads.get().into(),
        request_timeout_ms: proto_msg.request_timeout_ms.get(),
    };

    let proto_bytes = encode_proto(&proto_msg);
    let prost_decoded = ServerConfigProst::decode(proto_bytes.clone()).expect("prost decode server config");
    assert_eq!(prost_decoded, prost_equivalent);

    let prost_bytes = encode_prost(&prost_equivalent);
    let proto_decoded: ServerConfig = decode_proto(prost_bytes);
    assert_eq!(proto_decoded, proto_msg);
}

#[test]
fn nonzero_single_field_value_one() {
    // Encode/decode a message where fields have value 1 (the smallest valid NonZero value).
    let msg = NonZeroPrimitives {
        nzu8: NonZeroU8::new(1).unwrap(),
        nzu16: NonZeroU16::new(1).unwrap(),
        nzu32: NonZeroU32::new(1).unwrap(),
        nzu64: NonZeroU64::new(1).unwrap(),
        nzi8: NonZeroI8::new(1).unwrap(),
        nzi16: NonZeroI16::new(1).unwrap(),
        nzi32: NonZeroI32::new(1).unwrap(),
        nzi64: NonZeroI64::new(1).unwrap(),
        nzusize: NonZeroUsize::new(1).unwrap(),
        nzisize: NonZeroIsize::new(1).unwrap(),
    };
    let bytes = encode_proto(&msg);
    let decoded: NonZeroPrimitives = decode_proto(bytes);
    assert_eq!(decoded, msg);
}

#[test]
fn nonzero_negative_signed_values() {
    let msg = NonZeroPrimitives {
        nzu8: NonZeroU8::new(42).unwrap(),
        nzu16: NonZeroU16::new(42).unwrap(),
        nzu32: NonZeroU32::new(42).unwrap(),
        nzu64: NonZeroU64::new(42).unwrap(),
        nzi8: NonZeroI8::new(-1).unwrap(),
        nzi16: NonZeroI16::new(-1).unwrap(),
        nzi32: NonZeroI32::new(-1).unwrap(),
        nzi64: NonZeroI64::new(-1).unwrap(),
        nzusize: NonZeroUsize::new(42).unwrap(),
        nzisize: NonZeroIsize::new(-1).unwrap(),
    };
    let bytes = encode_proto(&msg);
    let decoded: NonZeroPrimitives = decode_proto(bytes);
    assert_eq!(decoded, msg);
}
