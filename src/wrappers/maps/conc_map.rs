use core::hash::BuildHasher;
use core::hash::Hash;

use bytes::Buf;
use papaya::HashMap;

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
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        super::size_hint::<K, V, TAG>(self.len())
    }

    #[inline]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        let guard = self.pin();
        for (key_value, value_value) in &guard {
            super::archive_entry::<K, V, TAG>(key_value, value_value, w);
        }
    }
}

impl<K, V, S> ProtoExt for HashMap<K, V, S> {
    const KIND: ProtoKind = ProtoKind::Repeated(&crate::wrappers::maps::MAP_ENTRY_KIND);
    const REPEATED_SUPPORT: Option<&'static str> = Some("papaya::HashMap");
}

impl<K, V, S> ProtoDecoder for HashMap<K, V, S>
where
    K: ProtoDecode + Eq + Hash,
    V: ProtoDecode,
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
        let (key, value) = super::decode_entry::<K, V>(wire_type, buf, ctx)?;
        self.pin().insert(key, value);
        Ok(())
    }
}

impl<K, V, S> ProtoDefault for HashMap<K, V, S>
where
    S: BuildHasher + Default,
{
    #[inline]
    fn proto_default() -> Self {
        HashMap::default()
    }
}

impl<K, V, S> ProtoDecode for HashMap<K, V, S>
where
    K: ProtoDecode + Eq + Hash,
    V: ProtoDecode,
    S: Default + std::hash::BuildHasher,
{
    type ShadowDecoded = Self;
}

impl<K, V, S> ProtoShadowDecode<HashMap<K, V, S>> for HashMap<K, V, S> {
    #[inline]
    fn to_sun(self) -> Result<Self, DecodeError> {
        Ok(self)
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
