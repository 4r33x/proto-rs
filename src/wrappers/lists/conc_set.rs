use alloc::vec::Vec;
use core::hash::BuildHasher;
use core::hash::Hash;

use bytes::Buf;
use papaya::HashSet;

use crate::DecodeError;
use crate::ProtoArchive;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
use crate::encoding::skip_field;
use crate::traits::ArchivedProtoField;
use crate::traits::PrimitiveKind;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::traits::buffer::RevWriter;

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

impl<T, S> ProtoDecoder for HashSet<T, S>
where
    T: ProtoDecoder + ProtoExt + Eq + Hash,
    S: BuildHasher + Default,
{
    #[inline(always)]
    fn proto_default() -> Self {
        HashSet::default()
    }

    #[inline(always)]
    fn clear(&mut self) {
        let guard = self.pin();
        guard.clear();
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
        let guard = self.pin();
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if wire_type == WireType::LengthDelimited {
                    let len = decode_varint(buf)? as usize;
                    let mut slice = buf.take(len);
                    while slice.has_remaining() {
                        let mut v = T::proto_default();
                        T::merge(&mut v, T::WIRE_TYPE, &mut slice, ctx)?;
                        guard.insert(v);
                    }
                    debug_assert!(!slice.has_remaining());
                } else {
                    let mut v = T::proto_default();
                    T::merge(&mut v, wire_type, buf, ctx)?;
                    guard.insert(v);
                }
                Ok(())
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                let mut v = T::proto_default();
                T::merge(&mut v, wire_type, buf, ctx)?;
                guard.insert(v);
                Ok(())
            }
            ProtoKind::Repeated(_) => unreachable!(),
        }
    }
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
    T: ProtoArchive + ProtoExt + Eq + Hash,
    S: BuildHasher,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        let guard = self.pin();
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                let items: Vec<&T> = guard.iter().collect();
                let mark = w.mark();
                for item in items.into_iter().rev() {
                    item.archive::<0>(w);
                }
                if TAG != 0 {
                    let payload_len = w.written_since(mark);
                    w.put_varint(payload_len as u64);
                    ArchivedProtoField::<TAG, Self>::put_key(w);
                }
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                let items: Vec<&T> = guard.iter().collect();
                for item in items.into_iter().rev() {
                    ArchivedProtoField::<TAG, T>::new_always(item, w);
                }
            }
            ProtoKind::Repeated(_) => unreachable!(),
        }
    }
}
