use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use bytes::Buf;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
use crate::encoding::skip_field;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;

impl<T: ProtoExt + Ord> ProtoExt for BTreeSet<T> {
    const KIND: ProtoKind = ProtoKind::Repeated(&T::KIND);
    const _REPEATED_SUPPORT: Option<&'static str> = Some("BTreeSet");
}

impl<T: ProtoDecoder + ProtoExt + Ord> ProtoDecoder for BTreeSet<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        BTreeSet::new()
    }

    #[inline(always)]
    fn clear(&mut self) {
        BTreeSet::clear(self);
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if wire_type == WireType::LengthDelimited {
                    let len = decode_varint(buf)? as usize;
                    let mut slice = buf.take(len);
                    while slice.has_remaining() {
                        let mut v = T::proto_default();
                        T::merge(&mut v, T::WIRE_TYPE, &mut slice, ctx)?;
                        self.insert(v);
                    }
                    debug_assert!(!slice.has_remaining());
                } else {
                    let mut v = T::proto_default();
                    T::merge(&mut v, wire_type, buf, ctx)?;
                    self.insert(v);
                }
                Ok(())
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                let mut v = T::proto_default();
                T::merge(&mut v, wire_type, buf, ctx)?;
                self.insert(v);
                Ok(())
            }
            ProtoKind::Repeated(_) => unreachable!(),
        }
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

impl<T: ProtoEncode + Ord> ProtoEncode for BTreeSet<T>
where
    for<'a> T::Shadow<'a>: ProtoShadowEncode<'a, T>,
    for<'a> Vec<T::Shadow<'a>>: crate::traits::ProtoArchive + ProtoExt,
{
    type Shadow<'a> = Vec<T::Shadow<'a>>;
}

impl<'a, T, S> ProtoShadowEncode<'a, BTreeSet<T>> for Vec<S>
where
    S: ProtoShadowEncode<'a, T>,
    T: Ord,
{
    #[inline]
    fn from_sun(value: &'a BTreeSet<T>) -> Self {
        value.iter().map(S::from_sun).collect()
    }
}
