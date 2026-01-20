use alloc::vec::Vec;
use core::hash::BuildHasher;
use core::hash::Hash;

use bytes::Buf;
use bytes::BufMut;
use papaya::HashSet;

use crate::DecodeError;
use crate::ProtoArchive;
use crate::encoding::skip_field;
use crate::traits::PrimitiveKind;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::wrappers::lists::encode_repeated_value;
use crate::wrappers::lists::repeated_payload_len;

impl<T: ProtoExt + Eq + Hash, S> ProtoExt for HashSet<T, S> {
    const KIND: ProtoKind = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => ProtoKind::Bytes,
        _ => ProtoKind::Repeated(&T::KIND),
    };
    const _REPEATED_SUPPORT: Option<&'static str> = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => None,
        _ => Some("papaya::HashSet"),
    };
}

impl<T: ProtoDecode + Eq + Hash, S> ProtoDecode for HashSet<T, S>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt + Eq + Hash,
    S: BuildHasher + Default,
    Vec<<T as ProtoDecode>::ShadowDecoded>: ProtoShadowDecode<HashSet<T, S>>,
{
    type ShadowDecoded = Vec<T::ShadowDecoded>;
}

impl<T, U, S> ProtoShadowDecode<HashSet<U, S>> for Vec<T>
where
    T: ProtoShadowDecode<U>,
    U: Eq + Hash,
    S: BuildHasher + Default,
{
    #[inline]
    fn to_sun(self) -> Result<HashSet<U, S>, DecodeError> {
        let out = HashSet::default();
        let guard = out.pin();
        for item in self {
            guard.insert(item.to_sun()?);
        }
        drop(guard);
        Ok(out)
    }
}

impl<T: ProtoEncode + Eq + Hash, S> ProtoEncode for HashSet<T, S>
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
    T: Eq + Hash,
{
    #[inline]
    fn from_sun(value: &'a HashSet<T, S>) -> Self {
        value
    }
}

impl<T, S> ProtoArchive for &HashSet<T, S>
where
    T: ProtoExt + Eq + Hash,
    for<'a> &'a T: ProtoArchive + ProtoExt,
    S: BuildHasher,
{
    type Archived<'x> = Vec<u8>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        archived.len()
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        buf.put_slice(archived.as_slice());
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        let mut bytes = Vec::new();
        let guard = self.pin();
        for item in &guard {
            let archived = item.archive();
            let len = repeated_payload_len::<T>(&archived);
            bytes.reserve(len);
            encode_repeated_value::<T>(archived, &mut bytes);
        }
        bytes
    }
}

impl<T, S> ProtoDecoder for HashSet<T, S>
where
    T: ProtoDecoder + ProtoExt + Eq + Hash,
    S: BuildHasher + Default,
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
        let guard = self.pin();
        for item in tmp {
            guard.insert(item);
        }
        Ok(())
    }
}
