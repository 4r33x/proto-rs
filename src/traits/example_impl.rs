use core::marker::PhantomData;

use bytes::Buf;

use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::{self};
use crate::error::DecodeError;
use crate::traits::ArchivedProtoField; // NEW: helper for field-vs-root semantics + const tag bytes
use crate::traits::ProtoArchive; // NEW: single-pass reverse archive(TAG, writer)
use crate::traits::ProtoDecode;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::buffer::RevWriter; // NEW: reverse writer trait
use crate::traits::decode::ProtoDecoder;
use crate::traits::encode::ProtoShadowEncode;

/// ---------- Sun type ----------
#[expect(dead_code)]
pub struct ID<'b, K, V> {
    pub id: u64,
    pub k: K,
    pub v: V,
    pub _pd: PhantomData<&'b ()>,
}

/// ---------- Shadow (borrows Sun for encoding) ----------
pub struct IDShadow<'a, K: ProtoEncode , V: ProtoEncode > {
    pub id: u64,
    pub k: <K as ProtoEncode>::Shadow<'a>,
    pub v: <V as ProtoEncode>::Shadow<'a>,
    pub _pd: PhantomData<&'a ()>,
}

/// ---------- decoded-shadow (owned, used for decoding) ----------
pub struct IDDecoded<Kd, Vd> {
    pub id: u64,
    pub k: Kd,
    pub v: Vd,
}

// ---------------- ProtoExt glue ----------------

impl<K, V> ProtoExt for ID<'_, K, V> {
    const KIND: ProtoKind = ProtoKind::Message;
}

impl<K: ProtoEncode, V: ProtoEncode> ProtoExt for IDShadow<'_, K, V> {
    const KIND: ProtoKind = ProtoKind::Message;
}

impl<Kd, Vd> ProtoExt for IDDecoded<Kd, Vd> {
    const KIND: ProtoKind = ProtoKind::Message;
}

// ---------------- Encoding: Sun -> Shadow ----------------

impl<'a, 'b, K, V> ProtoShadowEncode<'a, ID<'b, K, V>> for IDShadow<'a, K, V>
where
    K: ProtoEncode,
    V: ProtoEncode,
{
    #[inline(always)]
    fn from_sun(value: &'a ID<'b, K, V>) -> Self {
        Self {
            id: value.id,
            k: <K as ProtoEncode>::Shadow::from_sun(&value.k),
            v: <V as ProtoEncode>::Shadow::from_sun(&value.v),
            _pd: PhantomData,
        }
    }
}

impl<K, V> ProtoEncode for ID<'_, K, V>
where
    K: ProtoEncode,
    V: ProtoEncode,
    for<'a> <K as ProtoEncode>::Shadow<'a>: ProtoArchive + ProtoExt + ProtoShadowEncode<'a, K>,
    for<'a> <V as ProtoEncode>::Shadow<'a>: ProtoArchive + ProtoExt + ProtoShadowEncode<'a, V>,
{
    type Shadow<'a> = IDShadow<'a, K, V>;
}

// ---------------- Encoding: Reverse single-pass (NEW) ----------------

impl<'a, K, V> ProtoArchive for IDShadow<'a, K, V>
where
    K: ProtoEncode,
    V: ProtoEncode,
    <K as ProtoEncode>::Shadow<'a>: ProtoArchive + ProtoExt,
    <V as ProtoEncode>::Shadow<'a>: ProtoArchive + ProtoExt,
    u64: ProtoArchive + ProtoExt,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        self.id == 0 && <K as ProtoEncode>::Shadow::is_default(&self.k) && <V as ProtoEncode>::Shadow::is_default(&self.v)
    }

    /// Reverse one-pass encoding.
    ///
    /// TAG semantics (framework-wide):
    /// - TAG == 0 => root payload (no len/key wrapper)
    /// - TAG != 0 => field encoding: payload + (len if length-delimited) + key
    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        // Mark start of *this message payload* to compute length if we need to wrap (TAG != 0).
        let mark = w.mark();

        // IMPORTANT: reverse writer => emit fields in REVERSE order for deterministic bytes.
        // Each ArchivedProtoField::<N, T>::archive calls `T::archive::<N>` which writes
        // (payload + prefixes) for that field, skipping default values.
        ArchivedProtoField::<3, <V as ProtoEncode>::Shadow<'a>>::archive(&self.v, w);
        ArchivedProtoField::<2, <K as ProtoEncode>::Shadow<'a>>::archive(&self.k, w);
        ArchivedProtoField::<1, u64>::archive(&self.id, w);

        // If this message is embedded as a field, wrap it as length-delimited + key.
        // (Message wire type is LengthDelimited.)
        if TAG != 0 {
            let payload_len = w.written_since(mark);
            w.put_varint(payload_len as u64);

            // Emit the field key using the const-tag fast path.
            // This writes the varint bytes for (TAG<<3 | wire_type) in forward byte order,
            // placed backwards in memory by the reverse writer.
            ArchivedProtoField::<TAG, Self>::put_key(w);
        }
    }
}

// ---------------- Decoding: ShadowDecoded (ProtoDecoder) ----------------

impl<Kd, Vd> ProtoDecoder for IDDecoded<Kd, Vd>
where
    Kd: ProtoDecoder,
    Vd: ProtoDecoder,
{
    type Shadow = Self;

    #[inline(always)]
    fn proto_default() -> Self {
        Self {
            id: 0,
            k: Kd::proto_default(),
            v: Vd::proto_default(),
        }
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.id = 0;
        self.k.clear();
        self.v.clear();
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match tag {
            1 => value.id.merge(wire_type, buf, ctx),
            2 => value.k.merge(wire_type, buf, ctx),
            3 => value.v.merge(wire_type, buf, ctx),
            _ => {
                // Contract: unknown tags must be skipped.
                encoding::skip_field(wire_type, tag, buf, ctx)?;
                Ok(())
            }
        }
    }
}

// ---------------- Decoding: ShadowDecoded -> Sun ----------------

impl<'b, K, V> ProtoShadowDecode<ID<'b, K, V>> for IDDecoded<<K as ProtoDecode>::ShadowDecoded, <V as ProtoDecode>::ShadowDecoded>
where
    K: ProtoDecode,
    V: ProtoDecode,
    <K as ProtoDecode>::ShadowDecoded: ProtoShadowDecode<K>,
    <V as ProtoDecode>::ShadowDecoded: ProtoShadowDecode<V>,
{
    #[inline(always)]
    fn to_sun(self) -> Result<ID<'b, K, V>, DecodeError> {
        let k = <K as ProtoDecode>::ShadowDecoded::to_sun(self.k)?;
        let v = <V as ProtoDecode>::ShadowDecoded::to_sun(self.v)?;
        Ok(ID {
            id: self.id,
            k,
            v,
            _pd: PhantomData,
        })
    }
}

impl<K, V> ProtoDecode for ID<'_, K, V>
where
    K: ProtoDecode,
    V: ProtoDecode,
{
    type ShadowDecoded = IDDecoded<<K as ProtoDecode>::ShadowDecoded, <V as ProtoDecode>::ShadowDecoded>;
}
