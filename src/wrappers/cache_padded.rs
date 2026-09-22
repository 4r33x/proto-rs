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
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoFieldMerge;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::traits::buffer::RevWriter;

impl<T: ProtoExt> ProtoExt for CachePadded<T> {
    const KIND: ProtoKind = T::KIND;
    const WRAP_ROOT: bool = true;
    const ENCODED_SIZE_HINT: crate::EncodeSizeHint = T::ENCODED_SIZE_HINT;
}

impl<T: ProtoFieldMerge + ProtoDefault> ProtoDecoder for CachePadded<T> {
    #[inline]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            value.merge(wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        self.merge_with_state(wire_type, buf, ctx, &crate::DecodeState::default())
    }

    fn merge_field_with_state(
        value: &mut Self,
        tag: u32,
        wire: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
        state: &crate::DecodeState<'_>,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            value.merge_with_state(wire, buf, ctx, state)
        } else {
            skip_field(wire, tag, buf, ctx)
        }
    }

    fn merge_with_state(
        &mut self,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
        state: &crate::DecodeState<'_>,
    ) -> Result<(), DecodeError> {
        T::merge_value_with_state(self, wire_type, buf, ctx, state)
    }
}

impl<T: ProtoDefault> ProtoDefault for CachePadded<T> {
    #[inline]
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
    #[inline]
    fn is_default(&self) -> bool {
        T::is_default(self)
    }

    #[inline]
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
    #[inline]
    fn is_default(&self) -> bool {
        T::is_default(self)
    }

    #[inline]
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
