use bytes::Buf;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoDefault;
use crate::traits::ProtoFieldMerge;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::traits::buffer::RevWriter;

impl<T: ProtoExt> ProtoExt for Option<T> {
    const KIND: ProtoKind = T::KIND;
}

impl<T: ProtoFieldMerge + ProtoDefault> ProtoDecoder for Option<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        None
    }

    #[inline(always)]
    fn clear(&mut self) {
        *self = None;
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            let inner = value.get_or_insert_with(<T as ProtoDefault>::proto_default_value);
            T::merge_value(inner, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let inner = self.get_or_insert_with(<T as ProtoDefault>::proto_default_value);
        T::merge_value(inner, wire_type, buf, ctx)
    }
}

impl<T: ProtoDecode> ProtoDecode for Option<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = Option<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<Option<U>> for Option<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<Option<U>, DecodeError> {
        match self {
            Some(inner) => Ok(Some(inner.to_sun()?)),
            None => Ok(None),
        }
    }
}

impl<T> ProtoArchive for Option<T>
where
    T: ProtoArchive,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_none()
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        if let Some(inner) = self.as_ref() {
            inner.archive::<TAG>(w);
        }
    }
}

impl<T: ProtoEncode> ProtoEncode for Option<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
{
    type Shadow<'a> = Option<T::Shadow<'a>>;
}

impl<'a, T, S> ProtoShadowEncode<'a, Option<T>> for Option<S>
where
    S: ProtoShadowEncode<'a, T>,
{
    #[inline]
    fn from_sun(value: &'a Option<T>) -> Self {
        value.as_ref().map(|v| S::from_sun(v))
    }
}

impl<T> ProtoArchive for &Option<T>
where
    T: ProtoArchive,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_none()
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        if let Some(inner) = self.as_ref() {
            inner.archive::<TAG>(w);
        }
    }
}
