use alloc::collections::VecDeque;
use alloc::vec::Vec;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
use crate::encoding::skip_field;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;

#[doc(hidden)]
pub struct ArchivedRepeated<'a, T: ProtoArchive + ProtoExt> {
    items: Vec<T::Archived<'a>>,
    len: usize,
}

fn repeated_payload_len<T: ProtoArchive + ProtoExt>(archived: &T::Archived<'_>) -> usize {
    let item_len = T::len(archived);
    match T::KIND {
        ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => item_len,
        ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => encoded_len_varint(item_len as u64) + item_len,
        ProtoKind::Repeated(_) => unreachable!(),
    }
}

fn encode_repeated_value<T: ProtoArchive + ProtoExt>(archived: T::Archived<'_>, buf: &mut impl BufMut) {
    match T::KIND {
        ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => unsafe {
            T::encode(archived, buf);
        },
        ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
            let len = T::len(&archived);
            encode_varint(len as u64, buf);
            unsafe { T::encode(archived, buf) };
        }
        ProtoKind::Repeated(_) => unreachable!(),
    }
}

impl<T: ProtoExt> ProtoExt for Vec<T> {
    const KIND: ProtoKind = ProtoKind::Repeated(&T::KIND);
    const _REPEATED_SUPPORT: Option<&'static str> = Some("Vec");
}

impl<T: ProtoDecoder + ProtoExt> ProtoDecoder for Vec<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        Vec::new()
    }

    #[inline(always)]
    fn clear(&mut self) {
        Vec::clear(self);
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if wire_type == WireType::LengthDelimited {
                    let len = decode_varint(buf)? as usize;
                    let mut slice = buf.take(len);
                    while slice.has_remaining() {
                        let mut v = T::proto_default();
                        T::merge(&mut v, T::WIRE_TYPE, &mut slice, ctx)?;
                        self.push(v);
                    }
                    debug_assert!(!slice.has_remaining());
                } else {
                    let mut v = T::proto_default();
                    T::merge(&mut v, wire_type, buf, ctx)?;
                    self.push(v);
                }
                Ok(())
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                let mut v = T::proto_default();
                T::merge(&mut v, wire_type, buf, ctx)?;
                self.push(v);
                Ok(())
            }
            ProtoKind::Repeated(_) => unreachable!(),
        }
    }
}

impl<T: ProtoDecode> ProtoDecode for Vec<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = Vec<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<Vec<U>> for Vec<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<Vec<U>, DecodeError> {
        self.into_iter().map(T::to_sun).collect()
    }
}

impl<'a, T> ProtoArchive for Vec<T>
where
    T: ProtoArchive + ProtoExt,
{
    type Archived<'x> = ArchivedRepeated<'x, T>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        archived.len
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        for item in archived.items {
            encode_repeated_value::<T>(item, buf);
        }
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        let mut items = Vec::with_capacity(self.len());
        let mut len = 0;
        for item in self {
            let archived = item.archive();
            len += repeated_payload_len::<T>(&archived);
            items.push(archived);
        }
        ArchivedRepeated { items, len }
    }
}

impl<T: ProtoEncode> ProtoEncode for Vec<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
{
    type Shadow<'a> = Vec<T::Shadow<'a>>;
}

impl<'a, T, S> ProtoShadowEncode<'a, Vec<T>> for Vec<S>
where
    S: ProtoShadowEncode<'a, T>,
    T: ProtoEncode,
{
    #[inline]
    fn from_sun(value: &'a Vec<T>) -> Self {
        value.iter().map(S::from_sun).collect()
    }
}

impl<T: ProtoExt> ProtoExt for VecDeque<T> {
    const KIND: ProtoKind = ProtoKind::Repeated(&T::KIND);
    const _REPEATED_SUPPORT: Option<&'static str> = Some("VecDeque");
}

impl<T: ProtoDecoder + ProtoExt> ProtoDecoder for VecDeque<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        VecDeque::new()
    }

    #[inline(always)]
    fn clear(&mut self) {
        VecDeque::clear(self);
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if wire_type == WireType::LengthDelimited {
                    let len = decode_varint(buf)? as usize;
                    let mut slice = buf.take(len);
                    while slice.has_remaining() {
                        let mut v = T::proto_default();
                        T::merge(&mut v, T::WIRE_TYPE, &mut slice, ctx)?;
                        self.push_back(v);
                    }
                    debug_assert!(!slice.has_remaining());
                } else {
                    let mut v = T::proto_default();
                    T::merge(&mut v, wire_type, buf, ctx)?;
                    self.push_back(v);
                }
                Ok(())
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                let mut v = T::proto_default();
                T::merge(&mut v, wire_type, buf, ctx)?;
                self.push_back(v);
                Ok(())
            }
            ProtoKind::Repeated(_) => unreachable!(),
        }
    }
}

impl<T: ProtoDecode> ProtoDecode for VecDeque<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = VecDeque<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<VecDeque<U>> for VecDeque<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<VecDeque<U>, DecodeError> {
        self.into_iter().map(T::to_sun).collect()
    }
}

impl<'a, T> ProtoArchive for VecDeque<T>
where
    T: ProtoArchive + ProtoExt,
{
    type Archived<'x> = ArchivedRepeated<'x, T>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        archived.len
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        for item in archived.items {
            encode_repeated_value::<T>(item, buf);
        }
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        let mut items = Vec::with_capacity(self.len());
        let mut len = 0;
        for item in self {
            let archived = item.archive();
            len += repeated_payload_len::<T>(&archived);
            items.push(archived);
        }
        ArchivedRepeated { items, len }
    }
}

impl<T: ProtoEncode> ProtoEncode for VecDeque<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
{
    type Shadow<'a> = VecDeque<T::Shadow<'a>>;
}

impl<'a, T, S> ProtoShadowEncode<'a, VecDeque<T>> for VecDeque<S>
where
    S: ProtoShadowEncode<'a, T>,
    T: ProtoEncode,
{
    #[inline]
    fn from_sun(value: &'a VecDeque<T>) -> Self {
        value.iter().map(S::from_sun).collect()
    }
}
