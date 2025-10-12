//! ProtoExt implementations for arrays
use core::array;

use ::bytes::Buf;
use ::bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;

impl<T: ProtoExt, const N: usize> ProtoExt for [T; N] {
    #[inline]
    fn proto_default() -> Self {
        // Uses array::from_fn (stable since Rust 1.63.0)
        // Works for both Copy and non-Copy types
        array::from_fn(|_| T::proto_default())
    }

    fn encode_raw(&self, _buf: &mut impl BufMut) {
        // Arrays are encoded by the parent struct's codegen
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        // Arrays are decoded by the parent struct's codegen
        skip_field(wire_type, tag, buf, ctx)
    }

    fn encoded_len(&self) -> usize {
        // Array length is calculated by the parent struct's codegen
        0
    }

    fn clear(&mut self) {
        *self = Self::proto_default();
    }
}
