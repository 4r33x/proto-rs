use bytes::Buf;

use crate::DecodeError;
use crate::ProtoDecode;
use crate::ProtoDecoder;
use crate::ProtoDefault;
use crate::ProtoFieldMerge;
use crate::ProtoExt;
use crate::ProtoKind;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;
use crate::traits::ProtoShadowDecode;

mod btree;
#[cfg(feature = "papaya")]
mod conc_map;
mod hash_map;

pub(crate) const MAP_ENTRY_KIND: ProtoKind = ProtoKind::Message;

pub struct MapEntryDecoded<K, V> {
    key: K,
    value: V,
}

impl<K, V> ProtoExt for MapEntryDecoded<K, V> {
    const KIND: ProtoKind = ProtoKind::Message;
}

impl<Kd, Vd> ProtoDecoder for MapEntryDecoded<Kd, Vd>
where
    Kd: ProtoDecoder,
    Vd: ProtoDecoder,
{
    #[inline]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match tag {
            1 => ProtoFieldMerge::merge_value(&mut value.key, wire_type, buf, ctx),
            2 => ProtoFieldMerge::merge_value(&mut value.value, wire_type, buf, ctx),
            _ => skip_field(wire_type, tag, buf, ctx),
        }
    }
}

impl<Kd, Vd> ProtoDefault for MapEntryDecoded<Kd, Vd>
where
    Kd: ProtoDefault,
    Vd: ProtoDefault,
{
    #[inline]
    fn proto_default() -> Self {
        Self {
            key: <Kd as ProtoDefault>::proto_default(),
            value: <Vd as ProtoDefault>::proto_default(),
        }
    }
}

impl<K, V> ProtoShadowDecode<MapEntryDecoded<K, V>> for MapEntryDecoded<K, V> {
    #[inline]
    fn to_sun(self) -> Result<MapEntryDecoded<K, V>, DecodeError> {
        Ok(self)
    }
}

impl<K, V> ProtoDecode for MapEntryDecoded<K, V>
where
    K: ProtoDecoder + ProtoDefault,
    V: ProtoDecoder + ProtoDefault,
{
    type ShadowDecoded = Self;
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
