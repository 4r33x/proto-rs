use alloc::vec::Vec;
use core::hash::BuildHasher;
use core::hash::Hash;

use bytes::Buf;
use papaya::HashMap;

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
    S: BuildHasher,
    for<'b> <K as ProtoEncode>::Shadow<'b>: ProtoArchive + ProtoExt,
    for<'b> <V as ProtoEncode>::Shadow<'b>: ProtoArchive + ProtoExt,
{
    #[inline]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        let guard = self.pin();
        let entries: Vec<(&K, &V)> = guard.iter().collect();
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

impl<K, V, S> ProtoExt for HashMap<K, V, S> {
    const KIND: ProtoKind = ProtoKind::Repeated(&crate::wrappers::maps::MAP_ENTRY_KIND);
    const _REPEATED_SUPPORT: Option<&'static str> = Some("papaya::HashMap");
}

impl<K, V, S> ProtoDecoder for HashMap<K, V, S>
where
    K: ProtoDecode + Eq + Hash,
    V: ProtoDecode + ProtoExt,
    S: BuildHasher + Default,
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
        let guard = self.pin();
        guard.clear();
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
        let remaining = buf.remaining();
        if len > remaining {
            return Err(DecodeError::new("buffer underflow"));
        }
        let guard = self.pin();
        // Each merge call handles exactly one map entry
        let mut entry = MapEntryDecoded::<K::ShadowDecoded, V::ShadowDecoded>::proto_default();
        if len > 0 {
            // Use limit-based decoding to avoid Take wrapper overhead
            let limit = remaining - len;
            while buf.remaining() > limit {
                MapEntryDecoded::<K::ShadowDecoded, V::ShadowDecoded>::decode_one_field(&mut entry, buf, ctx)?;
            }
        }
        let (key, value) = entry.to_sun()?;
        guard.insert(key, value);
        Ok(())
    }
}

impl<K, V, S> ProtoDecode for HashMap<K, V, S>
where
    K: ProtoDecode + Eq + Hash,
    V: ProtoDecode,
    S: BuildHasher + Default,
    K::ShadowDecoded: Ord,
    Vec<MapEntryDecoded<K::ShadowDecoded, V::ShadowDecoded>>: ProtoDecoder + ProtoExt,
    Vec<MapEntryDecoded<<K as ProtoDecode>::ShadowDecoded, <V as ProtoDecode>::ShadowDecoded>>: ProtoShadowDecode<HashMap<K, V, S>>,
{
    type ShadowDecoded = Vec<MapEntryDecoded<K::ShadowDecoded, V::ShadowDecoded>>;
}

impl<K, V, S> ProtoShadowDecode<HashMap<K, V, S>> for Vec<MapEntryDecoded<K::ShadowDecoded, V::ShadowDecoded>>
where
    K: ProtoDecode + Eq + Hash,
    V: ProtoDecode,
    S: BuildHasher + Default,
    K::ShadowDecoded: ProtoShadowDecode<K>,
    V::ShadowDecoded: ProtoShadowDecode<V>,
{
    #[inline]
    fn to_sun(self) -> Result<HashMap<K, V, S>, DecodeError> {
        let out = HashMap::default();
        let guard = out.pin();
        for entry in self {
            let (key, value) = entry.to_sun()?;
            guard.insert(key, value);
        }
        drop(guard);
        Ok(out)
    }
}

impl<K, V, S> ProtoEncode for HashMap<K, V, S>
where
    for<'b> K: 'b + ProtoEncode + Eq + Hash,
    for<'b> V: 'b + ProtoEncode + ProtoExt,
    for<'b> S: 'b + BuildHasher,
{
    type Shadow<'a> = &'a HashMap<K, V, S>;
}
