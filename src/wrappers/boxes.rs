use alloc::boxed::Box;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::ProtoDecoder;
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

impl<T: ProtoExt> ProtoExt for Box<T> {
    const KIND: ProtoKind = T::KIND;
}

impl<T: ProtoDecoder + ProtoExt> ProtoDecoder for Box<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        Box::new(T::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        T::clear(self.as_mut());
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            T::merge(value.as_mut(), wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        T::merge(self.as_mut(), wire_type, buf, ctx)
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
    type Archived<'a> = T::Archived<'a>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        T::is_default(self.as_ref())
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        T::len(archived)
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        unsafe { T::encode(archived, buf) };
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        T::archive(self.as_ref())
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

impl<'a, T: ProtoExt> ProtoExt for &'a Box<T> {
    const KIND: ProtoKind = T::KIND;
}

impl<'a, T> ProtoArchive for &'a Box<T>
where
    T: ProtoArchive,
{
    type Archived<'x> = T::Archived<'x>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        T::is_default(self.as_ref())
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        T::len(archived)
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        unsafe { T::encode(archived, buf) };
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        T::archive(self.as_ref())
    }
}
