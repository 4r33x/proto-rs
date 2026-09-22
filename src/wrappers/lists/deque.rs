use alloc::collections::VecDeque;

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

impl<T: ProtoExt> ProtoExt for VecDeque<T> {
    const KIND: ProtoKind = if T::IS_BYTE {
        ProtoKind::Bytes
    } else {
        ProtoKind::Repeated(&T::KIND)
    };
    const REPEATED_SUPPORT: Option<&'static str> = if T::IS_BYTE { None } else { Some("VecDeque") };
}

impl<T: ProtoFieldMerge + ProtoDefault> ProtoDecoder for VecDeque<T> {
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
        if let Some(bytes) = T::byte_deque_mut(self) {
            return bytes_encoding::merge_one_copy(wire_type, bytes, buf, ctx);
        }
        super::merge_repeated(self, wire_type, buf, ctx, Self::reserve, |values, value| {
            values.push_back(value);
            Ok(())
        })
    }
}

impl<T> ProtoDefault for VecDeque<T> {
    #[inline]
    fn proto_default() -> Self {
        VecDeque::new()
    }
}

impl<T: ProtoDecode> ProtoDecode for VecDeque<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
    VecDeque<T::ShadowDecoded>: ProtoDecoder + ProtoExt,
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

impl<T> ProtoArchive for VecDeque<T>
where
    T: ProtoArchive + ProtoExt,
{
    #[inline]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        super::collection_size_hint::<T, TAG>(self.len())
    }

    #[inline]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        let (front, back) = self.as_slices();
        if let (Some(front), Some(back)) = (T::byte_slice(front), T::byte_slice(back)) {
            w.put_slice(back);
            w.put_slice(front);
            let len = front.len() + back.len();
            if TAG != 0 {
                w.put_varint(len as u64);
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

impl<T: ProtoEncode + ProtoExt + 'static> ProtoEncode for VecDeque<T> {
    type Shadow<'a> = &'a VecDeque<T>;
}
impl<'a, T> ProtoShadowEncode<'a, VecDeque<T>> for &'a VecDeque<T> {
    fn from_sun(value: &'a VecDeque<T>) -> Self {
        value
    }
}
impl<T: ProtoEncode + ProtoExt> ProtoArchive for &VecDeque<T> {
    fn is_default(&self) -> bool {
        self.is_empty()
    }
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        if T::IS_BYTE {
            super::collection_size_hint::<T, TAG>(self.len())
        } else {
            super::repeated_size_hint::<T::Shadow<'_>, TAG>(self.len())
        }
    }
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        let (front, back) = self.as_slices();
        if let (Some(front), Some(back)) = (T::byte_slice(front), T::byte_slice(back)) {
            w.put_slice(back);
            w.put_slice(front);
            if TAG != 0 {
                w.put_varint((front.len() + back.len()) as u64);
                ArchivedProtoField::<TAG, Self>::put_key(w);
            }
        } else {
            super::archive_repeated::<TAG, _>(self.iter().rev().map(T::Shadow::from_sun), w);
        }
    }
}
