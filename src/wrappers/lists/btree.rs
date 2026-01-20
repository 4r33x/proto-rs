use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::encoding::bytes as bytes_encoding;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
use crate::encoding::skip_field;
use crate::traits::PrimitiveKind;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::wrappers::lists::ArchivedRepeated;
use crate::wrappers::lists::ArchivedVec;
use crate::wrappers::lists::encode_repeated_value;
use crate::wrappers::lists::repeated_payload_len;

impl<T: ProtoExt + Ord> ProtoExt for BTreeSet<T> {
    const KIND: ProtoKind = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => ProtoKind::Bytes,
        _ => ProtoKind::Repeated(&T::KIND),
    };
    const _REPEATED_SUPPORT: Option<&'static str> = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => None,
        _ => Some("BTreeSet"),
    };
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
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt + ProtoShadowEncode<'a, T>,
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

impl<T> ProtoArchive for &BTreeSet<T>
where
    T: ProtoArchive + ProtoExt,
{
    type Archived<'x> = ArchivedVec<'x, T>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        match archived {
            ArchivedVec::Bytes(bytes) => bytes.len(),
            ArchivedVec::Owned(repeated) => repeated.len,
        }
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        match archived {
            ArchivedVec::Bytes(bytes) => bytes_encoding::encode(&bytes, buf),
            ArchivedVec::Owned(repeated) => {
                for item in repeated.items {
                    encode_repeated_value::<T>(item, buf);
                }
            }
        }
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        let mut items = Vec::with_capacity(self.len());
        let mut len = 0;
        for item in *self {
            let archived = item.archive();
            len += repeated_payload_len::<T>(&archived);
            items.push(archived);
        }
        ArchivedVec::Owned(ArchivedRepeated { items, len })
    }
}

impl<T> ProtoArchive for BTreeSet<T>
where
    T: ProtoArchive + ProtoExt,
{
    type Archived<'x> = ArchivedVec<'x, T>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        match archived {
            ArchivedVec::Bytes(bytes) => bytes.len(),
            ArchivedVec::Owned(repeated) => repeated.len,
        }
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        match archived {
            ArchivedVec::Bytes(bytes) => bytes_encoding::encode(&bytes, buf),
            ArchivedVec::Owned(repeated) => {
                for item in repeated.items {
                    encode_repeated_value::<T>(item, buf);
                }
            }
        }
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        let mut items = Vec::with_capacity(self.len());
        let mut len = 0;
        for item in self {
            let archived = item.archive();
            len += repeated_payload_len::<T>(&archived);
            items.push(archived);
        }
        ArchivedVec::Owned(ArchivedRepeated { items, len })
    }
}
