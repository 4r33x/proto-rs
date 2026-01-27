use alloc::vec::Vec;
use std::collections::HashSet;

use crate::DecodeError;
use crate::ProtoArchive;
use crate::bytes::Buf;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
use crate::encoding::skip_field;
use crate::traits::ArchivedProtoField;
use crate::traits::PrimitiveKind;
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
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
    Vec<<T as ProtoDecode>::ShadowDecoded>: ProtoShadowDecode<HashSet<T, S>>,
{
    type ShadowDecoded = Vec<T::ShadowDecoded>;
}

impl<T, S> ProtoDecoder for HashSet<T, S>
where
    T: ProtoFieldMerge + ProtoDefault + Eq + core::hash::Hash,
    S: Default + core::hash::BuildHasher,
{
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
                        let mut v = <T as ProtoDefault>::proto_default();
                        T::merge_value(&mut v, T::WIRE_TYPE, &mut slice, ctx)?;
                        self.insert(v);
                    }
                    debug_assert!(!slice.has_remaining());
                } else {
                    let mut v = <T as ProtoDefault>::proto_default();
                    T::merge_value(&mut v, wire_type, buf, ctx)?;
                    self.insert(v);
                }
                Ok(())
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                let mut v = <T as ProtoDefault>::proto_default();
                T::merge_value(&mut v, wire_type, buf, ctx)?;
                self.insert(v);
                Ok(())
            }
            ProtoKind::Repeated(_) => unreachable!(),
        }
    }
}

impl<T, S> ProtoDefault for HashSet<T, S>
where
    S: Default + core::hash::BuildHasher,
{
    #[inline(always)]
    fn proto_default() -> Self {
        HashSet::default()
    }
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
    T: ProtoArchive + ProtoExt + Eq + core::hash::Hash,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                let items: Vec<&T> = self.iter().collect();
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
                let items: Vec<&T> = self.iter().collect();
                for item in items.into_iter().rev() {
                    ArchivedProtoField::<TAG, T>::new_always(item, w);
                }
            }
            ProtoKind::Repeated(_) => unreachable!(),
        }
    }
}
