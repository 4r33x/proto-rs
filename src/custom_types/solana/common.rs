use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::EncodeError;
use crate::ProtoKind;
use crate::ProtoWire;
use crate::encoding;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FixedBytes<const N: usize> {
    bytes: [u8; N],
}

impl<const N: usize> FixedBytes<N> {
    pub const fn new(bytes: [u8; N]) -> Self {
        Self { bytes }
    }

    pub fn into_array(self) -> [u8; N] {
        self.bytes
    }

    pub fn as_array(&self) -> &[u8; N] {
        &self.bytes
    }
}

impl<const N: usize> Default for FixedBytes<N> {
    fn default() -> Self {
        Self { bytes: [0u8; N] }
    }
}

impl<const N: usize> From<[u8; N]> for FixedBytes<N> {
    fn from(bytes: [u8; N]) -> Self {
        Self::new(bytes)
    }
}

impl<const N: usize> AsRef<[u8; N]> for FixedBytes<N> {
    fn as_ref(&self) -> &[u8; N] {
        self.as_array()
    }
}

impl<const N: usize> ProtoWire for FixedBytes<N> {
    type EncodeInput<'a> = &'a Self;
    const KIND: ProtoKind = ProtoKind::Bytes;

    #[inline(always)]
    fn proto_default() -> Self {
        Self::default()
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.bytes.fill(0);
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        let _ = value;
        false
    }

    #[inline(always)]
    fn encoded_len_impl(_value: &Self::EncodeInput<'_>) -> usize {
        encoding::encoded_len_varint(N as u64) + N
    }

    #[inline(always)]
    fn encoded_len_tagged_impl(_value: &Self::EncodeInput<'_>, tag: u32) -> usize {
        encoding::key_len(tag) + encoding::encoded_len_varint(N as u64) + N
    }

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(_value: &Self::EncodeInput<'_>) -> usize {
        encoding::encoded_len_varint(N as u64) + N
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        encoding::encode_varint(N as u64, buf);
        buf.put_slice(&value.bytes);
    }

    #[inline(always)]
    fn encode_entrypoint(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        encoding::encode_varint(N as u64, buf);
        buf.put_slice(&value.bytes);
        Ok(())
    }

    #[inline(always)]
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, _ctx: DecodeContext) -> Result<(), DecodeError> {
        if wire_type != WireType::LengthDelimited {
            return Err(DecodeError::new("invalid wire type for Solana byte array"));
        }

        let len = encoding::decode_varint(buf)? as usize;
        if len != N {
            return Err(DecodeError::new("invalid length for Solana byte array"));
        }

        if buf.remaining() < N {
            return Err(DecodeError::new("buffer underflow"));
        }

        buf.copy_to_slice(&mut value.bytes);
        Ok(())
    }
}
