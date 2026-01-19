use alloc::vec::Vec;
use core::ptr;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::bytes as bytes_encoding;
use crate::encoding::decode_varint;
use crate::encoding::skip_field;
use crate::traits::PrimitiveKind;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::wrappers::lists::ArchivedRepeated;
use crate::wrappers::lists::ArchivedVec;
use crate::wrappers::lists::encode_repeated_value;
use crate::wrappers::lists::repeated_payload_len;

impl<T: ProtoExt> ProtoExt for Vec<T> {
    const KIND: ProtoKind = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => ProtoKind::Bytes,
        _ => ProtoKind::Repeated(&T::KIND),
    };
    const _REPEATED_SUPPORT: Option<&'static str> = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => None,
        _ => Some("Vec"),
    };
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
        if T::KIND.is_bytes_kind() {
            // SAFETY: only executed for Vec<u8>
            let bytes = unsafe { &mut *(ptr::from_mut(self).cast::<Vec<u8>>()) };
            return bytes_encoding::merge(wire_type, bytes, buf, ctx);
        }
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
    Vec<T::ShadowDecoded>: ProtoDecoder + ProtoExt,
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

impl<T> ProtoArchive for Vec<T>
where
    T: ProtoArchive + ProtoExt,
{
    type Archived<'x> = ArchivedVec<'x, T>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        match archived {
            ArchivedVec::Bytes(bytes) => bytes.len(),
            ArchivedVec::Owned(repeated) => repeated.len,
        }
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        match archived {
            ArchivedVec::Bytes(bytes) => bytes_encoding::encode(&bytes, buf),
            ArchivedVec::Owned(repeated) => {
                for item in repeated.items {
                    encode_repeated_value::<T>(item, buf);
                }
            }
        }
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        if T::KIND.is_bytes_kind() {
            // SAFETY: only executed for Vec<u8>.
            let bytes = unsafe { (*(ptr::from_ref(self).cast::<Vec<u8>>())).as_slice() };
            return ArchivedVec::Bytes(bytes);
        }

        let mut items = Vec::with_capacity(self.len());
        let mut len = 0;
        for item in self {
            let archived = item.archive();
            len += repeated_payload_len::<T>(&archived);
            items.push(archived);
        }
        ArchivedVec::Owned(ArchivedRepeated { items, len })
    }
}

impl<T: ProtoEncode> ProtoEncode for Vec<T>
where
    for<'a> T: 'a + ProtoExt,
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
    for<'a> &'a [T]: ProtoArchive + ProtoExt,
{
    type Shadow<'a> = &'a [T];
}

impl<'a, T> ProtoShadowEncode<'a, Vec<T>> for &'a [T]
where
    T: ProtoEncode,
{
    #[inline]
    fn from_sun(value: &'a Vec<T>) -> Self {
        value.as_slice()
    }
}
