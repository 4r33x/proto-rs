use alloc::collections::BTreeMap;

use bytes::Buf;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoDefault;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::traits::buffer::RevWriter;

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
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        super::size_hint::<K, V, TAG>(self.len())
    }

    #[inline]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        for (key_value, value_value) in self.iter().rev() {
            super::archive_entry::<K, V, TAG>(key_value, value_value, w);
        }
    }
}

impl<K, V> ProtoExt for BTreeMap<K, V> {
    const KIND: ProtoKind = ProtoKind::Repeated(&crate::wrappers::maps::MAP_ENTRY_KIND);
    const REPEATED_SUPPORT: Option<&'static str> = Some("BTreeMap");
}

impl<K, V> ProtoDecoder for BTreeMap<K, V>
where
    K: ProtoDecode + Ord,
    V: ProtoDecode,
{
    #[inline]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            <Self as ProtoDecoder>::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let (key, value) = super::decode_entry::<K, V>(wire_type, buf, ctx)?;
        self.insert(key, value);
        Ok(())
    }
}

impl<K, V> ProtoDefault for BTreeMap<K, V> {
    #[inline]
    fn proto_default() -> Self {
        BTreeMap::new()
    }
}

impl<K, V> ProtoDecode for BTreeMap<K, V>
where
    K: ProtoDecode + Ord,
    V: ProtoDecode,
{
    type ShadowDecoded = Self;
}

impl<K, V> ProtoShadowDecode<BTreeMap<K, V>> for BTreeMap<K, V> {
    #[inline]
    fn to_sun(self) -> Result<Self, DecodeError> {
        Ok(self)
    }
}

impl<K, V> ProtoEncode for BTreeMap<K, V>
where
    for<'b> K: 'b + ProtoEncode + Ord,
    for<'b> V: 'b + ProtoEncode + ProtoExt,
{
    type Shadow<'a> = &'a BTreeMap<K, V>;
}
