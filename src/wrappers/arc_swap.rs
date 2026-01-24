use std::sync::Arc;

use arc_swap::ArcSwap;
use arc_swap::ArcSwapOption;
use bytes::Buf;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;

pub struct ArcSwapShadow<T> {
    bytes: Vec<u8>,
    is_default: bool,
    _marker: core::marker::PhantomData<T>,
}

pub struct ArcSwapOptionShadow<T> {
    bytes: Vec<u8>,
    is_default: bool,
    _marker: core::marker::PhantomData<T>,
}

impl<T: ProtoExt> ProtoExt for ArcSwap<T> {
    const KIND: ProtoKind = T::KIND;
}

impl<T: ProtoDecoder + ProtoExt> ProtoDecoder for ArcSwap<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        ArcSwap::from_pointee(T::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.store(Arc::new(T::proto_default()));
    }

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
        let mut inner = T::proto_default();
        T::merge(&mut inner, wire_type, buf, ctx)?;
        self.store(Arc::new(inner));
        Ok(())
    }
}

impl<T: ProtoDecode> ProtoDecode for ArcSwap<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
    ArcSwap<T::ShadowDecoded>: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = ArcSwap<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<ArcSwap<U>> for ArcSwap<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<ArcSwap<U>, DecodeError> {
        let arc = ArcSwap::into_inner(self);
        let inner = Arc::try_unwrap(arc).map_err(|_| DecodeError::new("ArcSwap shadow has extra references"))?;
        let value = inner.to_sun()?;
        Ok(ArcSwap::from_pointee(value))
    }
}

impl<T> ProtoExt for ArcSwapShadow<T>
where
    T: ProtoExt,
{
    const KIND: ProtoKind = T::KIND;
}

impl<T> ProtoArchive for ArcSwapShadow<T> {
    type Archived<'a> = &'a [u8];

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_default
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        archived.len()
    }

    #[inline(always)]
    unsafe fn encode<const TAG: u32>(archived: Self::Archived<'_>, buf: &mut impl bytes::BufMut) {
        buf.put_slice(archived);
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self) -> Self::Archived<'_> {
        self.bytes.as_slice()
    }
}

impl<T: ProtoEncode + ProtoArchive + ProtoExt> ProtoEncode for ArcSwap<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt + ProtoShadowEncode<'a, T>,
{
    type Shadow<'a> = ArcSwapShadow<T>;
}

impl<'a, T> ProtoShadowEncode<'a, ArcSwap<T>> for ArcSwapShadow<T>
where
    T: ProtoEncode + ProtoArchive + ProtoExt,
{
    #[inline]
    fn from_sun(value: &'a ArcSwap<T>) -> Self {
        let guard = value.load_full();
        let is_default = T::is_default(guard.as_ref());
        let bytes = if is_default { Vec::new() } else { guard.encode_to_vec() };
        Self {
            bytes,
            is_default,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<T: ProtoExt> ProtoExt for ArcSwapOption<T> {
    const KIND: ProtoKind = T::KIND;
}

impl<T: ProtoDecoder + ProtoExt> ProtoDecoder for ArcSwapOption<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        ArcSwapOption::from_pointee(None)
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.store(None);
    }

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
        let mut inner = T::proto_default();
        T::merge(&mut inner, wire_type, buf, ctx)?;
        self.store(Some(Arc::new(inner)));
        Ok(())
    }
}

impl<T: ProtoDecode> ProtoDecode for ArcSwapOption<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
    ArcSwapOption<T::ShadowDecoded>: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = ArcSwapOption<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<ArcSwapOption<U>> for ArcSwapOption<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<ArcSwapOption<U>, DecodeError> {
        let inner = ArcSwapOption::into_inner(self);
        let value = match inner {
            Some(arc) => {
                let inner = Arc::try_unwrap(arc).map_err(|_| DecodeError::new("ArcSwapOption shadow has extra references"))?;
                Some(inner.to_sun()?)
            }
            None => None,
        };
        Ok(ArcSwapOption::from_pointee(value))
    }
}

impl<T> ProtoExt for ArcSwapOptionShadow<T>
where
    T: ProtoExt,
{
    const KIND: ProtoKind = T::KIND;
}

impl<T> ProtoArchive for ArcSwapOptionShadow<T> {
    type Archived<'a> = &'a [u8];

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_default
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        archived.len()
    }

    #[inline(always)]
    unsafe fn encode<const TAG: u32>(archived: Self::Archived<'_>, buf: &mut impl bytes::BufMut) {
        buf.put_slice(archived);
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self) -> Self::Archived<'_> {
        self.bytes.as_slice()
    }
}

impl<T: ProtoEncode + ProtoArchive + ProtoExt> ProtoEncode for ArcSwapOption<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt + ProtoShadowEncode<'a, T>,
{
    type Shadow<'a> = ArcSwapOptionShadow<T>;
}

impl<'a, T> ProtoShadowEncode<'a, ArcSwapOption<T>> for ArcSwapOptionShadow<T>
where
    T: ProtoEncode + ProtoArchive + ProtoExt,
{
    #[inline]
    fn from_sun(value: &'a ArcSwapOption<T>) -> Self {
        let guard = value.load_full();
        match guard.as_ref() {
            Some(inner) => {
                let is_default = T::is_default(inner.as_ref());
                let bytes = if is_default { Vec::new() } else { inner.encode_to_vec() };
                Self {
                    bytes,
                    is_default,
                    _marker: core::marker::PhantomData,
                }
            }
            None => Self {
                bytes: Vec::new(),
                is_default: true,
                _marker: core::marker::PhantomData,
            },
        }
    }
}
