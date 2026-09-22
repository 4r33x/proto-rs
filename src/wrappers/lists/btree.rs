use alloc::collections::BTreeSet;

use bytes::Buf;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoDefault;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoFieldMerge;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;

impl<T: ProtoExt + Ord> ProtoExt for BTreeSet<T> {
    const KIND: ProtoKind = ProtoKind::Repeated(&T::KIND);
    const REPEATED_SUPPORT: Option<&'static str> = Some("BTreeSet");
}

impl<T: ProtoFieldMerge + ProtoDefault + Ord> ProtoDecoder for BTreeSet<T> {
    #[inline]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        super::merge_repeated(
            self,
            wire_type,
            buf,
            ctx,
            |_, _| {},
            |values, value| {
                values.insert(value);
                Ok(())
            },
        )
    }
}

impl<T> ProtoDefault for BTreeSet<T> {
    #[inline]
    fn proto_default() -> Self {
        BTreeSet::new()
    }
}

impl<T: ProtoDecode + Ord> ProtoDecode for BTreeSet<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt + Ord,
{
    type ShadowDecoded = BTreeSet<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<BTreeSet<U>> for BTreeSet<T>
where
    T: ProtoShadowDecode<U>,
    U: Ord,
{
    #[inline]
    fn to_sun(self) -> Result<BTreeSet<U>, DecodeError> {
        self.into_iter().map(T::to_sun).collect()
    }
}

impl<T: ProtoEncode + ProtoExt + Ord + 'static> ProtoEncode for BTreeSet<T> {
    type Shadow<'a> = &'a BTreeSet<T>;
}

impl<'a, T: Ord> ProtoShadowEncode<'a, BTreeSet<T>> for &'a BTreeSet<T> {
    fn from_sun(value: &'a BTreeSet<T>) -> Self {
        value
    }
}

impl<T: ProtoEncode + ProtoExt + Ord> ProtoArchive for &BTreeSet<T> {
    fn is_default(&self) -> bool {
        self.is_empty()
    }
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        super::repeated_size_hint::<T::Shadow<'_>, TAG>(self.len())
    }
    fn archive<const TAG: u32>(&self, w: &mut impl crate::RevWriter) {
        super::archive_repeated::<TAG, _>(self.iter().rev().map(T::Shadow::from_sun), w);
    }
}
