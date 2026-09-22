use core::hash::BuildHasher;
use core::hash::Hash;

use bytes::Buf;
use papaya::HashSet;

use super::Repeated;
use crate::DecodeError;
use crate::ProtoArchive;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;
use crate::traits::ArchivedProtoField;
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

impl<T: ProtoExt + Eq + Hash, S> ProtoExt for HashSet<T, S> {
    const KIND: ProtoKind = ProtoKind::Repeated(&T::KIND);
    const REPEATED_SUPPORT: Option<&'static str> = Some("papaya::HashSet");
}

impl<T: ProtoDecode + Eq + Hash, S> ProtoDecode for HashSet<T, S>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
    S: BuildHasher + Default,
    Repeated<<T as ProtoDecode>::ShadowDecoded>: ProtoShadowDecode<HashSet<T, S>>,
{
    type ShadowDecoded = Repeated<T::ShadowDecoded>;
}

impl<T, S> ProtoDecoder for HashSet<T, S>
where
    T: ProtoFieldMerge + ProtoDefault + Eq + Hash,
    S: BuildHasher + Default,
{
    #[inline]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let mut guard = self.pin();
        super::merge_repeated(
            &mut guard,
            wire_type,
            buf,
            ctx,
            |_, _| {},
            |values, value| {
                values.insert(value);
                Ok(())
            },
        )
    }
}

impl<T, S> ProtoDefault for HashSet<T, S>
where
    S: BuildHasher + Default,
{
    #[inline]
    fn proto_default() -> Self {
        HashSet::default()
    }
}

impl<T, U, S> ProtoShadowDecode<HashSet<U, S>> for Repeated<T>
where
    T: ProtoShadowDecode<U>,
    U: Eq + Hash,
    S: BuildHasher + Default,
{
    #[inline]
    fn to_sun(self) -> Result<HashSet<U, S>, DecodeError> {
        let out = HashSet::default();
        let guard = out.pin();
        for item in self.0 {
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
    #[inline]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        super::repeated_size_hint::<T, TAG>(self.len())
    }

    #[inline]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        let guard = self.pin();
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                let mark = w.mark();
                for item in &guard {
                    item.archive::<0>(w);
                }
                if TAG != 0 {
                    let payload_len = w.written_since(mark);
                    w.put_varint(payload_len as u64);
                    ArchivedProtoField::<TAG, Self>::put_key(w);
                }
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                for item in &guard {
                    ArchivedProtoField::<TAG, T>::new_always(item, w);
                }
            }
            ProtoKind::Repeated(_) => unreachable!(),
        }
    }
}
