use prosto_derive::proto_dump;
pub use solana_address::ADDRESS_BYTES as BYTES;
use solana_address::Address as ByteSeq;

use crate::impl_protoext_for_byte_array;

extern crate self as proto_rs;

#[allow(dead_code)]
#[proto_dump(proto_path = "protos/solana.proto")]
struct AddressProto {
    #[proto(tag = 1)]
    inner: [u8; BYTES],
}

impl_protoext_for_byte_array!(ByteSeq, BYTES);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProtoExt;
    use crate::encoding::WireType;
    use crate::encoding::encode_key;
    use crate::encoding::encode_varint;

    fn sample_address_bytes() -> [u8; BYTES] {
        let mut data = [0u8; BYTES];
        for (idx, byte) in data.iter_mut().enumerate() {
            *byte = (idx as u8).wrapping_mul(3).wrapping_add(7);
        }
        data
    }

    #[test]
    fn roundtrip_proto_ext() {
        let original = ByteSeq::from(sample_address_bytes());
        let encoded = <ByteSeq as ProtoExt>::encode_to_vec(&original);
        let decoded = <ByteSeq as ProtoExt>::decode(encoded.as_slice()).expect("decode");
        assert_eq!(decoded.as_ref(), original.as_ref());
    }

    #[test]
    fn rejects_incorrect_length() {
        let mut buf = Vec::new();
        encode_key(1, WireType::LengthDelimited, &mut buf);
        encode_varint((BYTES - 1) as u64, &mut buf);
        buf.extend(std::iter::repeat_n(0u8, BYTES - 1));

        match <ByteSeq as ProtoExt>::decode(buf.as_slice()) {
            Ok(_) => panic!("invalid length should fail"),
            Err(err) => {
                assert!(err.to_string().contains("invalid length for Solana byte array"));
            }
        }
    }
}
