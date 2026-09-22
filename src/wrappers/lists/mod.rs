use crate::ProtoArchive;
use crate::ProtoExt;
use crate::ProtoKind;
use crate::encoding::WireType;
use crate::traits::ArchivedProtoField;
use crate::traits::buffer::RevWriter;

#[inline]
pub(crate) const fn collection_size_hint<T: ProtoExt, const TAG: u32>(len: usize) -> crate::EncodeSizeHint {
    if len == 0 {
        return crate::EncodeSizeHint::EMPTY;
    }
    if T::IS_BYTE {
        return crate::EncodeSizeHint::new(len, true).for_field::<TAG>(WireType::LengthDelimited);
    }
    repeated_size_hint::<T, TAG>(len)
}

pub(crate) const fn repeated_size_hint<T: ProtoExt, const TAG: u32>(len: usize) -> crate::EncodeSizeHint {
    if len == 0 {
        return crate::EncodeSizeHint::EMPTY;
    }
    match T::KIND {
        ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => T::ENCODED_SIZE_HINT.repeated(len).for_field::<TAG>(WireType::LengthDelimited),
        ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => T::ENCODED_SIZE_HINT.for_field::<TAG>(T::WIRE_TYPE).repeated(len),
        ProtoKind::Repeated(_) => crate::EncodeSizeHint::UNKNOWN,
    }
}

/// A bounded prepass for small scalar/byte/string lists avoids reverse-buffer
/// growth and compaction. Never walk message trees or unbounded collections.
#[inline]
pub(crate) fn slice_size_hint<T: ProtoArchive + ProtoExt, const TAG: u32>(values: &[T]) -> crate::EncodeSizeHint {
    let hint = collection_size_hint::<T, TAG>(values.len());
    if hint.exact || values.len() > 16 || matches!(T::KIND, ProtoKind::Message | ProtoKind::Repeated(_)) {
        return hint;
    }
    if T::KIND.is_packable() {
        values
            .iter()
            .fold(crate::EncodeSizeHint::EMPTY, |hint, value| {
                hint.add(crate::wrappers::options::present_size_hint::<T, 0>(value))
            })
            .for_field::<TAG>(WireType::LengthDelimited)
    } else {
        values.iter().fold(crate::EncodeSizeHint::EMPTY, |hint, value| {
            hint.add(crate::wrappers::options::present_size_hint::<T, TAG>(value))
        })
    }
}

/// Decode one occurrence, sharing framing and bounded speculative allocation.
/// Repeated u8 values remain varints here; byte containers use separate safe hooks.
#[inline]
pub(crate) fn merge_repeated<T, C>(
    collection: &mut C,
    wire_type: WireType,
    buf: &mut impl crate::bytes::Buf,
    ctx: crate::DecodeContext,
    reserve: impl FnOnce(&mut C, usize),
    mut push: impl FnMut(&mut C, T) -> Result<(), crate::DecodeError>,
) -> Result<(), crate::DecodeError>
where
    T: crate::ProtoFieldMerge + crate::ProtoDefault,
{
    if T::KIND.is_packable() && wire_type == WireType::LengthDelimited {
        let len = crate::encoding::decode_length_delimiter(&mut *buf)?;
        if len > buf.remaining() {
            return Err(crate::DecodeError::new("buffer underflow"));
        }
        let width = match T::WIRE_TYPE {
            WireType::ThirtyTwoBit => 4,
            WireType::SixtyFourBit => 8,
            _ => 1,
        };
        if len % width != 0 {
            return Err(crate::DecodeError::new("invalid packed field length"));
        }
        // Bound allocation before examining attacker-controlled elements.
        let capacity = (len / width).min(4096 / core::mem::size_of::<T>().max(1));
        // A single element already uses the collection's ordinary insertion
        // growth path; a separate speculative reserve only duplicates that work.
        if capacity > 1 {
            reserve(collection, capacity);
        }
        let limit = buf.remaining() - len;
        while buf.remaining() > limit {
            let mut value = T::proto_default();
            T::merge_value(&mut value, T::WIRE_TYPE, buf, ctx)?;
            if buf.remaining() < limit {
                return Err(crate::DecodeError::new("delimited length exceeded"));
            }
            push(collection, value)?;
        }
    } else {
        let mut value = T::proto_default();
        T::merge_value(&mut value, wire_type, buf, ctx)?;
        push(collection, value)?;
    }
    Ok(())
}

pub(crate) fn archive_repeated<const TAG: u32, T: ProtoArchive + ProtoExt>(values: impl Iterator<Item = T>, w: &mut impl RevWriter) {
    let mark = w.mark();
    for value in values {
        if T::KIND.is_packable() {
            value.archive::<0>(w);
        } else {
            value.archive::<TAG>(w);
        }
    }
    if TAG != 0 && T::KIND.is_packable() {
        w.put_varint(w.written_since(mark) as u64);
        w.put_varint(((TAG << 3) | WireType::LengthDelimited as u32) as u64);
    }
}

/// A repeated-element decode shadow that never treats u8 as a byte buffer.
pub struct Repeated<T>(pub(crate) Vec<T>);

impl<T: ProtoExt> ProtoExt for Repeated<T> {
    const KIND: ProtoKind = ProtoKind::Repeated(&T::KIND);
}
impl<T> crate::ProtoDefault for Repeated<T> {
    fn proto_default() -> Self {
        Self(Vec::new())
    }
}
impl<T: crate::ProtoFieldMerge + crate::ProtoDefault> crate::ProtoDecoder for Repeated<T> {
    fn merge_field(
        value: &mut Self,
        tag: u32,
        wire: WireType,
        buf: &mut impl crate::bytes::Buf,
        ctx: crate::DecodeContext,
    ) -> Result<(), crate::DecodeError> {
        if tag == 1 {
            Self::merge(value, wire, buf, ctx)
        } else {
            crate::encoding::skip_field(wire, tag, buf, ctx)
        }
    }
    fn merge(&mut self, wire: WireType, buf: &mut impl crate::bytes::Buf, ctx: crate::DecodeContext) -> Result<(), crate::DecodeError> {
        merge_repeated(&mut self.0, wire, buf, ctx, Vec::reserve, |values, value| {
            values.push(value);
            Ok(())
        })
    }
}

mod arrays;
mod btree;
#[cfg(feature = "papaya")]
mod conc_set;
mod deque;
mod hash_set;
mod vec;

impl<T: ProtoExt> ProtoExt for &[T] {
    const KIND: ProtoKind = if T::IS_BYTE {
        ProtoKind::Bytes
    } else {
        ProtoKind::Repeated(&T::KIND)
    };
}

impl<T> ProtoArchive for &[T]
where
    T: ProtoArchive + ProtoExt,
{
    #[inline]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        slice_size_hint::<T, TAG>(self)
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
