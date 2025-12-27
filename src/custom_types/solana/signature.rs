pub use solana_signature::SIGNATURE_BYTES as BYTES;
use solana_signature::Signature;

use crate::DecodeError;
use crate::ProtoShadow;
use crate::proto_message;

extern crate self as proto_rs;

#[allow(dead_code)]
#[proto_message(proto_path = "protos/solana.proto", sun = Signature)]
pub struct SignatureProto {
    #[proto(tag = 1)]
    pub inner: [u8; BYTES],
}

impl ProtoShadow<Signature> for SignatureProto {
    type Sun<'a> = &'a Signature;
    type OwnedSun = Signature;
    type View<'a> = Self;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(Signature::from(self.inner))
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        let mut inner = [0u8; BYTES];
        inner.copy_from_slice(value.as_ref());
        Self { inner }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProtoExt;
    use crate::encoding::WireType;
    use crate::encoding::encode_key;
    use crate::encoding::encode_varint;
    #[allow(dead_code)]
    #[proto_message(proto_path = "protos/solana_test.proto")]
    struct SignatureWrapper {
        sig: Signature,
    }

    fn sample_signature_bytes() -> [u8; BYTES] {
        let mut data = [0u8; BYTES];
        for (idx, byte) in data.iter_mut().enumerate() {
            *byte = (idx as u8).wrapping_mul(5).wrapping_add(11);
        }
        data
    }

    #[test]
    fn roundtrip_proto_ext() {
        let original = Signature::from(sample_signature_bytes());
        let encoded = <Signature as ProtoExt>::encode_to_vec(&original);
        let decoded = <Signature as ProtoExt>::decode(encoded.as_slice()).expect("decode");
        assert_eq!(decoded.as_ref(), original.as_ref());
    }

    #[test]
    fn rejects_incorrect_length() {
        let mut buf = Vec::new();
        encode_key(1, WireType::LengthDelimited, &mut buf);
        encode_varint((BYTES - 2) as u64, &mut buf);
        buf.extend(core::iter::repeat_n(0u8, BYTES - 2));

        match <Signature as ProtoExt>::decode(buf.as_slice()) {
            Ok(_) => panic!("invalid length should fail"),
            Err(err) => {
                let message = err.to_string();
                assert!(message.contains("invalid length for fixed byte array"), "unexpected error message: {message}");
                assert!(message.contains(&BYTES.to_string()), "missing expected length in error message: {message}");
                assert!(message.contains(&(BYTES - 2).to_string()), "missing actual length in error message: {message}");
            }
        }
    }
}
