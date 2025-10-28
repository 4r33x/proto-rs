use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::traits::ProtoKind;

impl<T> ProtoShadow for Option<T>
where
    T: ProtoShadow,
{
    type Sun<'a> = Option<T::Sun<'a>>;
    type OwnedSun = Option<T::OwnedSun>;
    type View<'a> = Option<T::View<'a>>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        match self {
            Some(inner) => Ok(Some(inner.to_sun()?)),
            None => Ok(None),
        }
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value.map(T::from_sun)
    }
}

// -----------------------------------------------------------------------------
// Option<T>: ProtoWire
// -----------------------------------------------------------------------------
impl<T: ProtoWire> ProtoWire for Option<T> {
    type EncodeInput<'a> = Option<T::EncodeInput<'a>>;
    const KIND: ProtoKind = T::KIND;

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        value.as_ref().is_none_or(T::is_default_impl)
    }

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        match value {
            Some(inner) => T::encoded_len_impl(inner),
            None => unreachable!(),
        }
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        if let Some(inner) = value {
            T::encode_raw_unchecked(inner, buf);
        }
    }

    #[inline(always)]
    fn decode_into(wire: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let mut tmp = value.take().unwrap_or_else(T::proto_default);
        T::decode_into(wire, &mut tmp, buf, ctx)?;
        *value = Some(tmp);
        Ok(())
    }

    #[inline(always)]
    fn proto_default() -> Self {
        None
    }

    #[inline(always)]
    fn clear(&mut self) {
        *self = None;
    }
}

impl<T> ProtoExt for Option<T>
where
    T: ProtoExt,
{
    type Shadow<'a> = Option<<T as ProtoExt>::Shadow<'a>>;

    #[inline(always)]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = value.get_or_insert_with(T::Shadow::proto_default);
        T::merge_field(inner, tag, wire, buf, ctx)
    }
}
