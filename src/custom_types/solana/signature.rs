use prosto_derive::proto_dump;
pub use solana_signature::SIGNATURE_BYTES as BYTES;
use solana_signature::Signature as ByteSeq;

use crate::impl_protoext_for_byte_array;

extern crate self as proto_rs;

#[proto_dump(proto_path = "protos/solana.proto")]
pub struct SignatureProto {
    pub inner: [u8; BYTES],
}

impl_protoext_for_byte_array!(ByteSeq, BYTES);
