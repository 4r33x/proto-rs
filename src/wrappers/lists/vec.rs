use alloc::vec::Vec;
use core::ptr;

use bytes::Buf;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::bytes as bytes_encoding;
use crate::encoding::decode_varint;
use crate::encoding::skip_field;
use crate::traits::ArchivedProtoField;
use crate::traits::PrimitiveKind;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::traits::buffer::RevWriter;

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
    type Shadow = Self;

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
                    let remaining = buf.remaining();
                    if len > remaining {
                        return Err(DecodeError::new("buffer underflow"));
                    }
                    // Use limit-based decoding to avoid Take wrapper overhead
                    let limit = remaining - len;
                    while buf.remaining() > limit {
                        let mut v = T::proto_default();
                        T::merge(&mut v, T::WIRE_TYPE, buf, ctx)?;
                        self.push(v);
                    }
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
    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        if T::KIND.is_bytes_kind() {
            // SAFETY: only executed for Vec<u8>.
            let bytes = unsafe { (*(ptr::from_ref(self).cast::<Vec<u8>>())).as_slice() };
            w.put_slice(bytes);
            if TAG != 0 {
                w.put_varint(bytes.len() as u64);
                ArchivedProtoField::<TAG, Self>::put_key(w);
            }
            return;
        }

        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                let mark = w.mark();
                for item in self.iter().rev() {
                    item.archive::<0>(w);
                }
                if TAG != 0 {
                    let payload_len = w.written_since(mark);
                    w.put_varint(payload_len as u64);
                    ArchivedProtoField::<TAG, Self>::put_key(w);
                }
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                for item in self.iter().rev() {
                    ArchivedProtoField::<TAG, T>::new_always(item, w);
                }
            }
            ProtoKind::Repeated(_) => unreachable!(),
        }
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
