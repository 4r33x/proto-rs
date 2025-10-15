// ---------- imports (adjust for no_std) ----------
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::MessageField;
use crate::ProtoExt;
use crate::SingularField;
use crate::encoding::DecodeContext;
use crate::encoding::wire_type::WireType;
use crate::traits::ViewOf;

// ---------------- Blanket impls for MessageField ----------------

impl<T> SingularField for T
where
    T: MessageField,
{
    #[inline]
    fn encode_singular_field(tag: u32, value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
        let len = <Self as ProtoExt>::encoded_len(&value);
        if len != 0 {
            crate::encoding::message::encode::<Self>(tag, value, buf);
        }
    }

    #[inline]
    fn merge_singular_field(wire_type: WireType, value: &mut Self::Shadow<'_>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        crate::encoding::message::merge::<Self, _>(wire_type, value, buf, ctx)
    }

    #[inline]
    fn encoded_len_singular_field(tag: u32, value: &ViewOf<'_, Self>) -> usize {
        let len = <Self as ProtoExt>::encoded_len(value);
        if len == 0 { 0 } else { crate::encoding::message::encoded_len::<Self>(tag, value) }
    }
}
