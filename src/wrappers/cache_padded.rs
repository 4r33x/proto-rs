use bytes::Buf;
use crossbeam_utils::CachePadded;

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

impl<T: ProtoExt> ProtoExt for CachePadded<T> {
    const KIND: ProtoKind = T::KIND;
}

impl<T: ProtoFieldMerge + ProtoDefault> ProtoDecoder for CachePadded<T> {
    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            value.merge(wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        T::merge_value(self, wire_type, buf, ctx)
    }
}

impl<T: ProtoDefault> ProtoDefault for CachePadded<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        CachePadded::new(<T as ProtoDefault>::proto_default())
    }
}

impl<T: ProtoDecode> ProtoDecode for CachePadded<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
    CachePadded<T::ShadowDecoded>: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = CachePadded<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<CachePadded<U>> for CachePadded<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<CachePadded<U>, DecodeError> {
        let inner = self.into_inner();
        Ok(CachePadded::new(inner.to_sun()?))
    }
}

impl<T> ProtoArchive for CachePadded<T>
where
    T: ProtoArchive,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        T::is_default(self)
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        <T as ProtoArchive>::archive::<TAG>(self, w);
    }
}

impl<T: ProtoEncode> ProtoEncode for CachePadded<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
{
    type Shadow<'a> = T::Shadow<'a>;
}

impl<'a, T, S> ProtoShadowEncode<'a, CachePadded<T>> for S
where
    S: ProtoShadowEncode<'a, T>,
{
    #[inline]
    fn from_sun(value: &'a CachePadded<T>) -> Self {
        S::from_sun(value)
    }
}

impl<T> ProtoArchive for &CachePadded<T>
where
    T: ProtoArchive,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        T::is_default(self)
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        <T as ProtoArchive>::archive::<TAG>(self, w);
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicU8;
    use std::sync::Arc;

    use crossbeam_utils::CachePadded;
    use prosto_derive::proto_message;
    #[allow(dead_code)]
    #[proto_message(proto_path = "protos/cache_padded_test.proto")]
    pub struct AtomicOrderState {
        inner: Arc<CachePadded<AtomicU8>>,
        inner2: CachePadded<AtomicU8>,
    }
}
