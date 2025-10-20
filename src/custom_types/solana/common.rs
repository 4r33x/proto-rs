#[macro_export]
macro_rules! impl_protoext_for_byte_array {
    ($ty:ty, $bytes:expr) => {
        impl $crate::traits::ProtoShadow for $ty {
            type Sun<'a> = &'a Self;
            type OwnedSun = Self;
            type View<'a> = &'a Self;

            #[inline]
            fn to_sun(self) -> Result<Self::OwnedSun, $crate::DecodeError> {
                Ok(self)
            }

            #[inline]
            fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
                value
            }
        }

        impl $crate::ProtoExt for $ty {
            type Shadow<'a> = Self;

            #[inline]
            fn proto_default<'a>() -> Self::Shadow<'a> {
                Self::default()
            }

            #[inline]
            fn encoded_len(_value: &$crate::traits::ViewOf<'_, Self>) -> usize {
                const TAG_LEN: usize = 1;
                TAG_LEN + $crate::encoding::encoded_len_varint($bytes as u64) + $bytes
            }

            #[inline]
            fn encode_raw(value: $crate::traits::ViewOf<'_, Self>, buf: &mut impl ::bytes::BufMut) {
                $crate::encoding::encode_key(1, $crate::encoding::WireType::LengthDelimited, buf);
                $crate::encoding::encode_varint($bytes as u64, buf);
                buf.put_slice(value.as_ref());
            }

            #[inline]
            fn merge_field(
                value: &mut Self::Shadow<'_>,
                tag: u32,
                wire_type: $crate::encoding::WireType,
                buf: &mut impl ::bytes::Buf,
                ctx: $crate::encoding::DecodeContext,
            ) -> Result<(), $crate::DecodeError> {
                if tag == 1 {
                    if wire_type != $crate::encoding::WireType::LengthDelimited {
                        return Err($crate::DecodeError::new("invalid wire type for Solana byte array"));
                    }

                    let len = $crate::encoding::decode_varint(buf)? as usize;
                    if len != $bytes {
                        return Err($crate::DecodeError::new("invalid length for Solana byte array"));
                    }

                    if buf.remaining() < $bytes {
                        return Err($crate::DecodeError::new("buffer underflow"));
                    }

                    let mut data = [0u8; $bytes];
                    buf.copy_to_slice(&mut data);
                    *value = <$ty as From<[u8; $bytes]>>::from(data);
                    Ok(())
                } else {
                    $crate::encoding::skip_field(wire_type, tag, buf, ctx)
                }
            }

            #[inline]
            fn clear(&mut self) {
                *self = Self::default();
            }

            fn encode_singular_field(tag: u32, value: ::proto_rs::ViewOf<'_, Self>, buf: &mut impl ::bytes::BufMut) {
                ::proto_rs::encoding::encode_key(tag, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                ::proto_rs::encoding::encode_varint($bytes as u64, buf);
                buf.put_slice(value.as_ref());
            }

            fn merge_singular_field(
                wire_type: ::proto_rs::encoding::WireType,
                value: &mut Self::Shadow<'_>,
                buf: &mut impl ::bytes::Buf,
                ctx: ::proto_rs::encoding::DecodeContext,
            ) -> Result<(), ::proto_rs::DecodeError> {
                let _ = ctx;
                if wire_type != ::proto_rs::encoding::WireType::LengthDelimited {
                    return Err(::proto_rs::DecodeError::new("invalid wire type for Solana byte array"));
                }

                let len = ::proto_rs::encoding::decode_varint(buf)? as usize;
                if len != $bytes {
                    return Err(::proto_rs::DecodeError::new("invalid length for Solana byte array"));
                }

                if buf.remaining() < $bytes {
                    return Err(::proto_rs::DecodeError::new("buffer underflow"));
                }

                let mut data = [0u8; $bytes];
                buf.copy_to_slice(&mut data);
                *value = <$ty as From<[u8; $bytes]>>::from(data);
                Ok(())
            }

            fn encoded_len_singular_field(tag: u32, _value: &::proto_rs::ViewOf<'_, Self>) -> usize {
                ::proto_rs::encoding::key_len(tag) + ::proto_rs::encoding::encoded_len_varint($bytes as u64) + $bytes
            }
        }
    };
}
