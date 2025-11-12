use solana_keypair::Keypair;
const BYTES: usize = Keypair::SECRET_KEY_LENGTH;

use crate::DecodeError;
use crate::ProtoShadow;
use crate::proto_message;

extern crate self as proto_rs;

#[allow(dead_code)]
#[proto_message(proto_path = "protos/solana.proto", sun = Keypair)]
pub struct KeypairProto {
    #[proto(tag = 1)]
    inner: [u8; BYTES],
}

impl ProtoShadow<Keypair> for KeypairProto {
    type Sun<'a> = &'a Keypair;
    type OwnedSun = Keypair;
    type View<'a> = Self;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(Keypair::new_from_array(self.inner))
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        Self { inner: *value.secret_bytes() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProtoExt;
    use crate::encoding::WireType;
    use crate::encoding::encode_key;
    use crate::encoding::encode_varint;

    fn sample_keypair_bytes() -> [u8; BYTES] {
        let mut data = [0u8; BYTES];
        for (idx, byte) in data.iter_mut().enumerate() {
            *byte = (idx as u8).wrapping_mul(3).wrapping_add(7);
        }
        data
    }

    #[test]
    fn roundtrip_proto_ext() {
        let original = Keypair::new_from_array(sample_keypair_bytes());
        let encoded = <Keypair as ProtoExt>::encode_to_vec(&original);
        let decoded = <Keypair as ProtoExt>::decode(encoded.as_slice()).expect("decode");
        assert_eq!(decoded.to_bytes(), original.to_bytes());
    }

    #[test]
    fn rejects_incorrect_length() {
        let mut buf = Vec::new();
        encode_key(1, WireType::LengthDelimited, &mut buf);
        encode_varint((BYTES - 1) as u64, &mut buf);
        buf.extend(core::iter::repeat_n(0u8, BYTES - 1));

        match <Keypair as ProtoExt>::decode(buf.as_slice()) {
            Ok(_) => panic!("invalid length should fail"),
            Err(err) => {
                let message = err.to_string();
                assert!(message.contains("invalid length for fixed byte array"), "unexpected error message: {message}");
                assert!(message.contains(&BYTES.to_string()), "missing expected length in error message: {message}");
                assert!(message.contains(&(BYTES - 1).to_string()), "missing actual length in error message: {message}");
            }
        }
    }
}
