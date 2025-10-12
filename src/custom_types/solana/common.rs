#[macro_export]
macro_rules! impl_protoext_for_byte_array {
    ($ty:ident, $bytes:ident) => {
        impl $crate::ProtoExt for $ty {
            fn encode_raw(&self, buf: &mut impl bytes::BufMut)
            where
                Self: Sized,
            {
                $crate::encoding::encode_key(1, $crate::encoding::WireType::LengthDelimited, buf);
                $crate::encoding::encode_varint($bytes as u64, buf);
                buf.put_slice(&self.inner);
            }

            fn merge_field(&mut self, tag: u32, wire_type: $crate::encoding::WireType, buf: &mut impl bytes::Buf, ctx: $crate::encoding::DecodeContext) -> Result<(), $crate::DecodeError>
            where
                Self: Sized,
            {
                if tag == 1 {
                    if wire_type != $crate::encoding::WireType::LengthDelimited {
                        return Err($crate::DecodeError::new("invalid wire type"));
                    }

                    let len = $crate::encoding::decode_varint(buf)?;
                    if buf.remaining() < len as usize {
                        return Err($crate::DecodeError::new("buffer underflow"));
                    }
                    if len as usize != $bytes {
                        return Err($crate::DecodeError::new(format!("expected {} bytes, got {}", $bytes, len)));
                    }
                    buf.copy_to_slice(&mut self.inner);
                    Ok(())
                } else {
                    $crate::encoding::skip_field(wire_type, tag, buf, ctx)
                }
            }

            fn encoded_len(&self) -> usize {
                let tag_len = 1;
                let len_varint_len = $crate::encoding::encoded_len_varint($bytes as u64);
                let data_len = $bytes;
                tag_len + len_varint_len + data_len
            }

            fn clear(&mut self) {
                self.inner = [0u8; $bytes];
            }
        }
    };
}
