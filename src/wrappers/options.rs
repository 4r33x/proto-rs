use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::traits::ProtoKind;

impl<T> ProtoShadow<Option<T>> for Option<T::Shadow<'_>>
where
    T: ProtoExt + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T>,
{
    type Sun<'a> = &'a Option<T>;
    type OwnedSun = Option<T>;
    type View<'a> = Option<<T::Shadow<'a> as ProtoShadow<T>>::View<'a>>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        self.map(ProtoShadow::to_sun).transpose()
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value.as_ref().map(|v| <T::Shadow<'_> as ProtoShadow<T>>::from_sun(v))
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
            Some(inner) => unsafe { T::encoded_len_impl_raw(inner) },
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
    T: ProtoExt + 'static,
    for<'a> T::Shadow<'a>: ProtoShadow<T, Sun<'a> = &'a T>,
{
    type Shadow<'a> = Option<<T as ProtoExt>::Shadow<'a>>;

    #[inline(always)]
    fn merge_field(
        value: &mut Self::Shadow<'_>,
        tag: u32,
        wire: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        let inner = value.get_or_insert_with(T::Shadow::proto_default);
        T::merge_field(inner, tag, wire, buf, ctx)
    }
}
