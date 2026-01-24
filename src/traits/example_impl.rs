use core::marker::PhantomData;

use bytes::Buf;
use bytes::BufMut;

use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::{self};
use crate::error::DecodeError;
use crate::traits::ArchivedProtoField;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
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
pub struct IDShadow<'a, K: ProtoEncode + ?Sized, V: ProtoEncode + ?Sized> {
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

/// ---------- Archived container for IDShadow encoding ----------
pub struct IDArchived<'x, 'a, K: ProtoEncode + ?Sized, V: ProtoEncode + ?Sized> {
    pub f1: ArchivedProtoField<'x, 1, u64>,
    pub f2: ArchivedProtoField<'x, 2, <K as ProtoEncode>::Shadow<'a>>,
    pub f3: ArchivedProtoField<'x, 3, <V as ProtoEncode>::Shadow<'a>>,
    pub len: usize,
}

// ---------------- ProtoExt glue ----------------

impl<K, V> ProtoExt for ID<'_, K, V> {
    const KIND: ProtoKind = ProtoKind::Message;
}

impl<K: ProtoEncode + ?Sized, V: ProtoEncode + ?Sized> ProtoExt for IDShadow<'_, K, V> {
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

impl<'a, K, V> ProtoArchive for IDShadow<'a, K, V>
where
    K: ProtoEncode,
    V: ProtoEncode,
    <K as ProtoEncode>::Shadow<'a>: ProtoArchive + ProtoExt,
    <V as ProtoEncode>::Shadow<'a>: ProtoArchive + ProtoExt,
    u64: ProtoArchive + ProtoExt,
{
    type Archived<'x> = IDArchived<'x, 'a, K, V>;

    #[inline(always)]
    fn archive(&self) -> <Self as ProtoArchive>::Archived<'_> {
        // Each ArchivedProtoField::new() handles "default => None" internally.
        let f1 = ArchivedProtoField::<1, u64>::new(&self.id);
        let f2 = ArchivedProtoField::<2, <K as ProtoEncode>::Shadow<'a>>::new(&self.k);
        let f3 = ArchivedProtoField::<3, <V as ProtoEncode>::Shadow<'a>>::new(&self.v);

        let len = f1.len() + f2.len() + f3.len();

        IDArchived { f1, f2, f3, len }
    }

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.id == 0 && <K as ProtoEncode>::Shadow::is_default(&self.k) && <V as ProtoEncode>::Shadow::is_default(&self.v)
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        archived.len
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        archived.f1.encode(buf);
        archived.f2.encode(buf);
        archived.f3.encode(buf);
    }
}

// ---------------- Decoding: ShadowDecoded (ProtoDecoder) ----------------

impl<Kd, Vd> ProtoDecoder for IDDecoded<Kd, Vd>
where
    Kd: ProtoDecoder,
    Vd: ProtoDecoder,
{
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
