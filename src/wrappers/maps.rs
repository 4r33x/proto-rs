use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
use crate::encoding::skip_field;
use crate::traits::ArchivedProtoInner;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;

#[doc(hidden)]
pub struct MapEntryShadow<'a, K: ProtoEncode + ?Sized, V: ProtoEncode + ?Sized> {
    key: <K as ProtoEncode>::Shadow<'a>,
    value: <V as ProtoEncode>::Shadow<'a>,
}

#[doc(hidden)]
pub struct MapEntryArchived {
    bytes: Vec<u8>,
    len: usize,
}

#[doc(hidden)]
pub struct MapEntryDecoded<K, V> {
    key: K,
    value: V,
}

impl<'a, K: ProtoEncode + ?Sized, V: ProtoEncode + ?Sized> ProtoExt for MapEntryShadow<'a, K, V> {
    const KIND: ProtoKind = ProtoKind::Message;
}

impl<K, V> ProtoExt for MapEntryDecoded<K, V> {
    const KIND: ProtoKind = ProtoKind::Message;
}

impl<'a, K, V> ProtoShadowEncode<'a, (K, V)> for MapEntryShadow<'a, K, V>
where
    K: ProtoEncode,
    V: ProtoEncode,
{
    #[inline]
    fn from_sun(value: &'a (K, V)) -> Self {
        Self {
            key: <K as ProtoEncode>::Shadow::from_sun(&value.0),
            value: <V as ProtoEncode>::Shadow::from_sun(&value.1),
        }
    }
}

impl<'a, K, V> ProtoArchive for MapEntryShadow<'a, K, V>
where
    K: ProtoEncode,
    V: ProtoEncode,
    <K as ProtoEncode>::Shadow<'a>: ProtoArchive + ProtoExt,
    <V as ProtoEncode>::Shadow<'a>: ProtoArchive + ProtoExt,
{
    type Archived<'x> = MapEntryArchived;

    #[inline]
    fn is_default(&self) -> bool {
        <K as ProtoEncode>::Shadow::is_default(&self.key) && <V as ProtoEncode>::Shadow::is_default(&self.value)
    }

    #[inline]
    fn len(archived: &Self::Archived<'_>) -> usize {
        archived.len
    }

    #[inline]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        buf.put_slice(&archived.bytes);
    }

    #[inline]
    fn archive(&self) -> Self::Archived<'_> {
        let key = ArchivedProtoInner::<1, <K as ProtoEncode>::Shadow<'a>>::new(&self.key);
        let value = ArchivedProtoInner::<2, <V as ProtoEncode>::Shadow<'a>>::new(&self.value);
        let len = key.len() + value.len();
        let mut bytes = Vec::with_capacity(len);
        key.encode(&mut bytes);
        value.encode(&mut bytes);
        MapEntryArchived { bytes, len }
    }
}

impl<Kd, Vd> ProtoDecoder for MapEntryDecoded<Kd, Vd>
where
    Kd: ProtoDecoder,
    Vd: ProtoDecoder,
{
    #[inline]
    fn proto_default() -> Self {
        Self {
            key: Kd::proto_default(),
            value: Vd::proto_default(),
        }
    }

    #[inline]
    fn clear(&mut self) {
        self.key.clear();
        self.value.clear();
    }

    #[inline]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match tag {
            1 => value.key.merge(wire_type, buf, ctx),
            2 => value.value.merge(wire_type, buf, ctx),
            _ => skip_field(wire_type, tag, buf, ctx),
        }
    }
}

impl<K, V> ProtoShadowDecode<(K, V)> for MapEntryDecoded<<K as ProtoDecode>::ShadowDecoded, <V as ProtoDecode>::ShadowDecoded>
where
    K: ProtoDecode,
    V: ProtoDecode,
    <K as ProtoDecode>::ShadowDecoded: ProtoShadowDecode<K>,
    <V as ProtoDecode>::ShadowDecoded: ProtoShadowDecode<V>,
{
    #[inline]
    fn to_sun(self) -> Result<(K, V), DecodeError> {
        let key = <K as ProtoDecode>::ShadowDecoded::to_sun(self.key)?;
        let value = <V as ProtoDecode>::ShadowDecoded::to_sun(self.value)?;
        Ok((key, value))
    }
}

impl<K, V> ProtoExt for BTreeMap<K, V>
where
    V: ProtoExt,
{
    const KIND: ProtoKind = ProtoKind::Repeated(&V::KIND);
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
    K: ProtoEncode + Ord,
    V: ProtoEncode,
    for<'a> <K as ProtoEncode>::Shadow<'a>: ProtoArchive + ProtoExt,
    for<'a> <V as ProtoEncode>::Shadow<'a>: ProtoArchive + ProtoExt,
{
    type Shadow<'a> = Vec<MapEntryShadow<'a, K, V>>;
}

impl<'a, K, V> ProtoShadowEncode<'a, BTreeMap<K, V>> for Vec<MapEntryShadow<'a, K, V>>
where
    K: ProtoEncode + Ord,
    V: ProtoEncode,
{
    #[inline]
    fn from_sun(value: &'a BTreeMap<K, V>) -> Self {
        value
            .iter()
            .map(|(k, v)| MapEntryShadow {
                key: <K as ProtoEncode>::Shadow::from_sun(k),
                value: <V as ProtoEncode>::Shadow::from_sun(v),
            })
            .collect()
    }
}
