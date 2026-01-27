use alloc::boxed::Box;

use bytes::Buf;

use crate::DecodeError;
use crate::ProtoDecoder;
use crate::ProtoDefault;
use crate::ProtoFieldMerge;
use crate::ProtoEncode;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::traits::buffer::RevWriter;

impl<T: ProtoExt> ProtoExt for Box<T> {
    const KIND: ProtoKind = T::KIND;
}

impl<T: ProtoFieldMerge + ProtoDefault> ProtoDecoder for Box<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        Box::new(<T as ProtoDefault>::proto_default_value())
    }

    #[inline(always)]
    fn clear(&mut self) {
        *self.as_mut() = <T as ProtoDefault>::proto_default_value();
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            T::merge_value(value.as_mut(), wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        T::merge_value(self.as_mut(), wire_type, buf, ctx)
    }
}

impl<T: ProtoDecode> ProtoDecode for Box<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = Box<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<Box<U>> for Box<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline(always)]
    fn to_sun(self) -> Result<Box<U>, DecodeError> {
        Ok(Box::write(Box::new_uninit(), (*self).to_sun()?))
    }
}

impl<T> ProtoArchive for Box<T>
where
    T: ProtoArchive,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        T::is_default(self.as_ref())
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        <T as ProtoArchive>::archive::<TAG>(self.as_ref(), w);
    }
}

// ============================================================================
// ProtoEncode for Box<T>
// ============================================================================

impl<T: ProtoEncode> ProtoEncode for Box<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
{
    type Shadow<'a> = T::Shadow<'a>;
}

impl<'a, T, S> ProtoShadowEncode<'a, Box<T>> for S
where
    S: ProtoShadowEncode<'a, T>,
{
    #[inline]
    fn from_sun(value: &'a Box<T>) -> Self {
        S::from_sun(value.as_ref())
    }
}

impl<T> ProtoArchive for &Box<T>
where
    T: ProtoArchive,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        T::is_default(self.as_ref())
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        <T as ProtoArchive>::archive::<TAG>(self.as_ref(), w);
    }
}
