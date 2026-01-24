use alloc::vec::Vec;
use core::hash::Hash;
use std::collections::HashMap;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
use crate::encoding::encode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
use crate::encoding::key_len;
use crate::encoding::skip_field;
use crate::traits::ArchivedProtoField;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::wrappers::maps::MapEntryDecoded;

impl<'a, K, V, S> ProtoShadowEncode<'a, HashMap<K, V, S>> for &'a HashMap<K, V, S>
where
    K: ProtoEncode + Eq + Hash,
    V: ProtoEncode,
{
    #[inline]
    fn from_sun(value: &'a HashMap<K, V, S>) -> Self {
        value
    }
}

impl<K, V, S> ProtoArchive for &HashMap<K, V, S>
where
    K: ProtoEncode + Eq + Hash,
    V: ProtoEncode + ProtoExt,
    for<'b> <K as ProtoEncode>::Shadow<'b>: ProtoArchive + ProtoExt,
    for<'b> <V as ProtoEncode>::Shadow<'b>: ProtoArchive + ProtoExt,
{
    type Archived<'x> = Vec<u8>;

    #[inline]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn len(archived: &Self::Archived<'_>) -> usize {
        archived.len()
    }

    #[inline]
    unsafe fn encode<const TAG: u32>(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        buf.put_slice(archived.as_slice());
    }

    #[inline]
    fn archive<const TAG: u32>(&self) -> Self::Archived<'_> {
        let tag_len = if TAG == 0 { 0 } else { key_len(TAG) };
        let mut bytes = Vec::new();
        for entry in *self {
            let key = <K as ProtoEncode>::Shadow::from_sun(entry.0);
            let key_archived = ArchivedProtoField::<1, <K as ProtoEncode>::Shadow<'_>>::new(&key);
            let value = <V as ProtoEncode>::Shadow::from_sun(entry.1);
            let value_archived = ArchivedProtoField::<2, <V as ProtoEncode>::Shadow<'_>>::new(&value);
            let entry_len = key_archived.len() + value_archived.len();
            bytes.reserve(tag_len + encoded_len_varint(entry_len as u64) + entry_len);
            if TAG != 0 {
                encode_key(TAG, WireType::LengthDelimited, &mut bytes);
            }
            encode_varint(entry_len as u64, &mut bytes);
            key_archived.encode(&mut bytes);
            value_archived.encode(&mut bytes);
        }
        bytes
    }
}

impl<K, V, S> ProtoExt for HashMap<K, V, S>
{
    const KIND: ProtoKind = ProtoKind::Repeated(&crate::wrappers::maps::MAP_ENTRY_KIND);
    const _REPEATED_SUPPORT: Option<&'static str> = Some("HashMap");
}

impl<K, V, S: Default + std::hash::BuildHasher> ProtoDecoder for HashMap<K, V, S>
where
    K: ProtoDecode + Eq + Hash,
    V: ProtoDecode + ProtoExt,
    K::ShadowDecoded: ProtoDecoder + ProtoExt,
    V::ShadowDecoded: ProtoDecoder + ProtoExt,
    MapEntryDecoded<K::ShadowDecoded, V::ShadowDecoded>: ProtoDecoder + ProtoExt,
{
    #[inline]
    fn proto_default() -> Self {
        HashMap::default()
    }

    #[inline]
    fn clear(&mut self) {
        HashMap::clear(self);
    }

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
        if wire_type != WireType::LengthDelimited {
            return Err(DecodeError::new("map entry must be length-delimited"));
        }
        let len = decode_varint(buf)? as usize;
        let mut slice = buf.take(len);
        if !slice.has_remaining() {
            let entry = MapEntryDecoded::<K::ShadowDecoded, V::ShadowDecoded>::proto_default();
            let (key, value) = entry.to_sun()?;
            self.insert(key, value);
            return Ok(());
        }
        while slice.has_remaining() {
            let mut entry = MapEntryDecoded::<K::ShadowDecoded, V::ShadowDecoded>::proto_default();
            MapEntryDecoded::<K::ShadowDecoded, V::ShadowDecoded>::decode_into(&mut entry, &mut slice, ctx)?;
            let (key, value) = entry.to_sun()?;
            self.insert(key, value);
        }
        Ok(())
    }
}

impl<K, V, S> ProtoDecode for HashMap<K, V, S>
where
    K: ProtoDecode + Eq + Hash,
    V: ProtoDecode,
    K::ShadowDecoded: Ord,
    Vec<MapEntryDecoded<K::ShadowDecoded, V::ShadowDecoded>>: ProtoDecoder + ProtoExt,
    Vec<MapEntryDecoded<<K as ProtoDecode>::ShadowDecoded, <V as ProtoDecode>::ShadowDecoded>>: ProtoShadowDecode<HashMap<K, V, S>>,
{
    type ShadowDecoded = Vec<MapEntryDecoded<K::ShadowDecoded, V::ShadowDecoded>>;
}

impl<K, V> ProtoShadowDecode<HashMap<K, V>> for Vec<MapEntryDecoded<K::ShadowDecoded, V::ShadowDecoded>>
where
    K: ProtoDecode + Eq + Hash,
    V: ProtoDecode,
    K::ShadowDecoded: ProtoShadowDecode<K>,
    V::ShadowDecoded: ProtoShadowDecode<V>,
{
    #[inline]
    fn to_sun(self) -> Result<HashMap<K, V>, DecodeError> {
        let mut out = HashMap::new();
        for entry in self {
            let (key, value) = entry.to_sun()?;
            out.insert(key, value);
        }
        Ok(out)
    }
}

impl<K, V, S> ProtoEncode for HashMap<K, V, S>
where
    for<'b> K: 'b + ProtoEncode + Eq + Hash,
    for<'b> V: 'b + ProtoEncode + ProtoExt,
    for<'b> S: 'b,
{
    type Shadow<'a> = &'a HashMap<K, V, S>;

    // for<'b> <K as ProtoEncode>::Shadow<'a>: ProtoArchive + ProtoExt,
    // for<'b> <V as ProtoEncode>::Shadow<'a>: ProtoArchive + ProtoExt;
}
