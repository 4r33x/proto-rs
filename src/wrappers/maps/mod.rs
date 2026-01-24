use bytes::Buf;

use crate::DecodeError;
use crate::ProtoDecode;
use crate::ProtoDecoder;
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
