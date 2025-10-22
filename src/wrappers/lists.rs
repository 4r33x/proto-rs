use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::{self};
use crate::traits::ProtoKind;

// -----------------------------------------------------------------------------
// Vec<T>: ProtoShadow
// -----------------------------------------------------------------------------
impl<T> ProtoShadow for Vec<T>
where
    T: ProtoShadow,
{
    type Sun<'a> = &'a Vec<T>;
    type OwnedSun = Vec<T>;
    type View<'a> = &'a Vec<T>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value
    }
}

// -----------------------------------------------------------------------------
// Vec<T>: ProtoWire
// -----------------------------------------------------------------------------
impl<T> ProtoWire for Vec<T>
where
    for<'a> T: ProtoWire<EncodeInput<'a> = &'a T> + 'a,
{
    type EncodeInput<'a> = &'a Vec<T>;
    const KIND: ProtoKind = T::KIND;

    #[inline]
    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
        match T::KIND {
            // Primitive or simple enums => support packed encoding
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => encoding::encoded_len_packed(value),
            // Message / Bytes / String => length-delimited repeated
            ProtoKind::Message | ProtoKind::Bytes | ProtoKind::String => encoding::encoded_len_repeated(value),
        }
    }

    #[inline]
    fn encode_raw(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                encoding::encode_packed(value, buf);
            }
            ProtoKind::Message | ProtoKind::Bytes | ProtoKind::String => {
                encoding::encode_repeated(value, buf);
            }
        }
    }

    #[inline]
    fn decode_into(wire_type: WireType, values: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => encoding::merge_repeated_packed(wire_type, values, buf, ctx),
            ProtoKind::Message | ProtoKind::Bytes | ProtoKind::String => encoding::merge_repeated_unpacked(wire_type, values, buf, ctx),
        }
    }

    #[inline]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn proto_default() -> Self {
        Vec::new()
    }

    #[inline]
    fn clear(&mut self) {
        self.clear()
    }
}

// -----------------------------------------------------------------------------
// Vec<T>: ProtoExt
// -----------------------------------------------------------------------------
impl<T> ProtoExt for Vec<T>
where
    T: ProtoExt,
{
    type Shadow<'a> = Vec<T>;

    #[inline(always)]
    fn merge_field(values: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match T::Shadow::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => encoding::merge_repeated_packed_field(tag, wire, value, buf, ctx),
            ProtoKind::Message | ProtoKind::Bytes => {}
            ProtoKind::String => encoding::string::merge_repeated(wire_type, values, buf, ctx),
        }
    }
}

trait MergeRepeated {}

impl<T> MergeRepeated for T {}

impl MergeRepeated for String {}
