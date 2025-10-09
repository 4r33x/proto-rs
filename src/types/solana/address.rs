use prost::encoding::DecodeContext;
use prost::encoding::WireType;
use prost::encoding::decode_varint;
use prost::encoding::encode_key;
use prost::encoding::encode_varint;
use prosto_derive::proto_dump;
pub use solana_address::ADDRESS_BYTES;
extern crate self as proto_rs;

#[proto_dump(proto_path = "protos/solana.proto")]
#[derive(Clone, PartialEq, Copy)]
pub struct AddressProto {
    pub inner: [u8; ADDRESS_BYTES],
}

impl prost::Message for AddressProto {
    fn encode_raw(&self, buf: &mut impl prost::bytes::BufMut)
    where
        Self: Sized,
    {
        // Encode as field 1 with wire type LengthDelimited (2)
        encode_key(1, WireType::LengthDelimited, buf);
        // Encode the length of the byte array
        encode_varint(ADDRESS_BYTES as u64, buf);
        // Write the actual bytes
        buf.put_slice(&self.inner);
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl prost::bytes::Buf, ctx: DecodeContext) -> Result<(), prost::DecodeError>
    where
        Self: Sized,
    {
        if tag == 1 {
            if wire_type != WireType::LengthDelimited {
                return Err(prost::DecodeError::new("invalid wire type"));
            }

            // Decode the length
            let len = decode_varint(buf)?;

            // Check if we have enough bytes
            if buf.remaining() < len as usize {
                return Err(prost::DecodeError::new("buffer underflow"));
            }

            // Check if the length matches our fixed array size
            if len as usize != ADDRESS_BYTES {
                return Err(prost::DecodeError::new(format!("expected {} bytes, got {}", ADDRESS_BYTES, len)));
            }

            // Read the bytes into our array
            buf.copy_to_slice(&mut self.inner);

            Ok(())
        } else {
            // Skip unknown fields
            prost::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        // Tag (field 1, wire type 2): typically 1 byte
        let tag_len = 1;

        // Length varint: for ADDRESS_BYTES, calculate varint size
        let len_varint_len = prost::encoding::encoded_len_varint(ADDRESS_BYTES as u64);

        // Actual data length
        let data_len = ADDRESS_BYTES;

        tag_len + len_varint_len + data_len
    }

    fn clear(&mut self) {
        self.inner = [0u8; ADDRESS_BYTES];
    }
}
