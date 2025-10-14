//! `ProtoExt` implementations for arrays
use core::array;

use ::bytes::Buf;
use ::bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;

impl<T: ProtoExt, const N: usize> ProtoExt for [T; N] {
    type Shadow = [T::Shadow; N];

    #[inline]
    fn proto_default() -> Self::Shadow {
        // Uses array::from_fn (stable since Rust 1.63.0)
        // Works for both Copy and non-Copy types
        array::from_fn(|_| T::proto_default())
    }

    fn encode_shadow(shadow: &Self::Shadow, _buf: &mut impl BufMut) {
        let _ = shadow; // Arrays are encoded by the parent struct's codegen
    }

    fn merge_field(_shadow: &mut Self::Shadow, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        // Arrays are decoded by the parent struct's codegen
        skip_field(wire_type, tag, buf, ctx)
    }

    fn encoded_len_shadow(_shadow: &Self::Shadow) -> usize {
        // Array length is calculated by the parent struct's codegen
        0
    }

    fn clear_shadow(shadow: &mut Self::Shadow) {
        *shadow = array::from_fn(|_| T::proto_default());
    }

    fn post_decode(shadow: Self::Shadow) -> Self {
        shadow.map(T::post_decode)
    }

    fn cast_shadow(value: &Self) -> Self::Shadow {
        array::from_fn(|idx| T::cast_shadow(&value[idx]))
    }
}
