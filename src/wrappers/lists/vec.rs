use alloc::vec::Vec;

use bytes::Buf;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::bytes as bytes_encoding;
use crate::encoding::skip_field;
use crate::traits::ArchivedProtoField;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoDefault;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoFieldMerge;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::traits::buffer::RevWriter;

impl<T: ProtoExt> ProtoExt for Vec<T> {
    const KIND: ProtoKind = if T::IS_BYTE {
        ProtoKind::Bytes
    } else {
        ProtoKind::Repeated(&T::KIND)
    };
    const REPEATED_SUPPORT: Option<&'static str> = if T::IS_BYTE { None } else { Some("Vec") };
}

impl<T: ProtoFieldMerge + ProtoDefault> ProtoDecoder for Vec<T> {
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
        if let Some(bytes) = T::byte_vec_mut(self) {
            return bytes_encoding::merge_one_copy(wire_type, bytes, buf, ctx);
        }
        super::merge_repeated(self, wire_type, buf, ctx, Self::reserve, |values, value| {
            values.push(value);
            Ok(())
        })
    }
}

impl<T> ProtoDefault for Vec<T> {
    #[inline]
    fn proto_default() -> Self {
        Vec::new()
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
    #[inline]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        super::slice_size_hint::<T, TAG>(self)
    }

    #[inline]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        if let Some(bytes) = T::byte_slice(self) {
            w.put_bytes::<TAG>(bytes);
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
