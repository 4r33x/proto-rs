use alloc::vec::Vec;
use std::collections::HashSet;

use crate::DecodeError;
use crate::ProtoArchive;
use crate::bytes::Buf;
use crate::bytes::BufMut;
use crate::encoding::bytes as bytes_encoding;
use crate::encoding::skip_field;
use crate::traits::PrimitiveKind;
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

impl<T: ProtoExt + Eq + core::hash::Hash, S> ProtoExt for HashSet<T, S> {
    const KIND: ProtoKind = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => ProtoKind::Bytes,
        _ => ProtoKind::Repeated(&T::KIND),
    };
    const _REPEATED_SUPPORT: Option<&'static str> = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => None,
        _ => Some("HashSet"),
    };
}

impl<T: ProtoDecode + Eq + core::hash::Hash, S> ProtoDecode for HashSet<T, S>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt + Eq + core::hash::Hash,
    Vec<<T as ProtoDecode>::ShadowDecoded>: ProtoShadowDecode<HashSet<T, S>>,
{
    type ShadowDecoded = Vec<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<HashSet<U>> for Vec<T>
where
    T: ProtoShadowDecode<U>,
    U: Eq + core::hash::Hash,
{
    #[inline]
    fn to_sun(self) -> Result<HashSet<U>, DecodeError> {
        self.into_iter().map(T::to_sun).collect()
    }
}

impl<T: ProtoEncode + Eq + core::hash::Hash, S> ProtoEncode for HashSet<T, S>
where
    for<'a> T: 'a + ProtoExt,
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
    for<'a> &'a HashSet<T, S>: ProtoArchive + ProtoExt,
    for<'a> S: 'a,
{
    type Shadow<'a> = &'a HashSet<T, S>;
}

impl<'a, T, S> ProtoShadowEncode<'a, HashSet<T, S>> for &'a HashSet<T, S>
where
    T: Eq + core::hash::Hash,
{
    #[inline]
    fn from_sun(value: &'a HashSet<T, S>) -> Self {
        value
    }
}

impl<T, S> ProtoArchive for &HashSet<T, S>
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

impl<T, S> ProtoDecoder for HashSet<T, S>
where
    T: ProtoDecoder + ProtoExt + Eq + core::hash::Hash,
    S: core::hash::BuildHasher + Default,
{
    #[inline(always)]
    fn proto_default() -> Self {
        HashSet::with_hasher(S::default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.clear();
    }

    #[inline(always)]
    fn merge_field(
        value: &mut Self,
        tag: u32,
        wire_type: crate::encoding::WireType,
        buf: &mut impl Buf,
        ctx: crate::encoding::DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: crate::encoding::WireType, buf: &mut impl Buf, ctx: crate::encoding::DecodeContext) -> Result<(), DecodeError> {
        let mut tmp = Vec::<T>::new();
        <Vec<T> as ProtoDecoder>::merge(&mut tmp, wire_type, buf, ctx)?;
        self.extend(tmp);
        Ok(())
    }
}
