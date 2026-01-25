use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use bytes::Buf;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
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
use crate::traits::buffer::RevWriter;
use crate::wrappers::maps::MapEntryDecoded;

impl<'a, K, V> ProtoShadowEncode<'a, BTreeMap<K, V>> for &'a BTreeMap<K, V>
where
    K: ProtoEncode + Ord,
    V: ProtoEncode,
{
    #[inline]
    fn from_sun(value: &'a BTreeMap<K, V>) -> Self {
        value
    }
}

impl<K, V> ProtoArchive for &BTreeMap<K, V>
where
    K: ProtoEncode + Ord,
    V: ProtoEncode + ProtoExt,
    for<'b> <K as ProtoEncode>::Shadow<'b>: ProtoArchive + ProtoExt,
    for<'b> <V as ProtoEncode>::Shadow<'b>: ProtoArchive + ProtoExt,
{
    #[inline]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        let entries: Vec<(&K, &V)> = self.iter().collect();
        for (key_value, value_value) in entries.into_iter().rev() {
            let key = <K as ProtoEncode>::Shadow::from_sun(key_value);
            let value = <V as ProtoEncode>::Shadow::from_sun(value_value);
            let mark = w.mark();
            ArchivedProtoField::<2, <V as ProtoEncode>::Shadow<'_>>::archive(&value, w);
            ArchivedProtoField::<1, <K as ProtoEncode>::Shadow<'_>>::archive(&key, w);
            if TAG != 0 {
                let payload_len = w.written_since(mark);
                w.put_varint(payload_len as u64);
                ArchivedProtoField::<TAG, Self>::put_key(w);
            }
        }
    }
}

impl<K, V> ProtoExt for BTreeMap<K, V> {
    const KIND: ProtoKind = ProtoKind::Repeated(&crate::wrappers::maps::MAP_ENTRY_KIND);
    const _REPEATED_SUPPORT: Option<&'static str> = Some("BTreeMap");
}

impl<K, V> ProtoDecoder for BTreeMap<K, V>
where
    K: ProtoDecode + Ord,
    V: ProtoDecode + ProtoExt,
    K::ShadowDecoded: ProtoDecoder + ProtoExt + Ord,
    V::ShadowDecoded: ProtoDecoder + ProtoExt,
    MapEntryDecoded<K::ShadowDecoded, V::ShadowDecoded>: ProtoDecoder + ProtoExt,
{
    #[inline]
    fn proto_default() -> Self {
        BTreeMap::new()
    }

    #[inline]
    fn clear(&mut self) {
        BTreeMap::clear(self);
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

impl<K, V> ProtoDecode for BTreeMap<K, V>
where
    K: ProtoDecode + Ord,
    V: ProtoDecode,
    K::ShadowDecoded: Ord,
    Vec<MapEntryDecoded<K::ShadowDecoded, V::ShadowDecoded>>: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = Vec<MapEntryDecoded<K::ShadowDecoded, V::ShadowDecoded>>;
}

impl<K, V> ProtoShadowDecode<BTreeMap<K, V>> for Vec<MapEntryDecoded<K::ShadowDecoded, V::ShadowDecoded>>
where
    K: ProtoDecode + Ord,
    V: ProtoDecode,
    K::ShadowDecoded: ProtoShadowDecode<K> + Ord,
    V::ShadowDecoded: ProtoShadowDecode<V>,
{
    #[inline]
    fn to_sun(self) -> Result<BTreeMap<K, V>, DecodeError> {
        let mut out = BTreeMap::new();
        for entry in self {
            let (key, value) = entry.to_sun()?;
            out.insert(key, value);
        }
        Ok(out)
    }
}

impl<K, V> ProtoEncode for BTreeMap<K, V>
where
    for<'b> K: 'b + ProtoEncode + Ord,
    for<'b> V: 'b + ProtoEncode + ProtoExt,
{
    type Shadow<'a> = &'a BTreeMap<K, V>;
}
