use core::marker::PhantomData;

use bytes::BufMut;

use crate::coders::AsBytes;
use crate::error::EncodeError;
use crate::traits::PrimitiveKind;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::buffer::RevVec;
use crate::traits::buffer::RevWriter;
use crate::traits::utils::VarintConst;
use crate::traits::utils::encode_varint_const;

pub trait ProtoShadowEncode<'a, T: ?Sized> {
    fn from_sun(value: &'a T) -> Self;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Type-level capacity estimate for reverse encoding.
///
/// `size` sums the minimum wire contribution of statically known fields or elements.
/// It may overestimate values whose default fields are omitted. `exact` means a value
/// that is emitted occupies exactly `size` bytes; default omission is handled separately.
pub struct EncodeSizeHint {
    pub size: usize,
    pub exact: bool,
}

impl EncodeSizeHint {
    pub const UNKNOWN: Self = Self { size: 0, exact: false };

    pub const fn new(size: usize, exact: bool) -> Self {
        Self { size, exact }
    }

    pub const fn from_kind(kind: &ProtoKind) -> Self {
        match kind {
            ProtoKind::Primitive(PrimitiveKind::Bool) => Self::new(1, true),
            ProtoKind::Primitive(PrimitiveKind::F32 | PrimitiveKind::Fixed32 | PrimitiveKind::SFixed32) => Self::new(4, true),
            ProtoKind::Primitive(PrimitiveKind::F64 | PrimitiveKind::Fixed64 | PrimitiveKind::SFixed64) => Self::new(8, true),
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => Self::new(1, false),
            ProtoKind::Message | ProtoKind::Bytes | ProtoKind::String | ProtoKind::Repeated(_) => Self::UNKNOWN,
        }
    }

    #[must_use]
    pub const fn add_field<const TAG: u32>(self, field: Self, wire_type: crate::encoding::WireType) -> Self {
        let delimiter = if wire_type.is_length_delimited() { 1 } else { 0 };
        Self {
            size: self.size.saturating_add(crate::encoding::key_len(TAG)).saturating_add(delimiter).saturating_add(field.size),
            exact: false,
        }
    }

    #[must_use]
    pub const fn repeated(self, count: usize) -> Self {
        Self {
            size: self.size.saturating_mul(count),
            exact: count == 0 || self.exact,
        }
    }
}

pub trait ProtoArchive {
    fn is_default(&self) -> bool;
    /// Reverse one-pass archive into a [`RevWriter`].
    ///
    /// TAG semantics:
    /// - TAG == 0 => top-level payload (no field key/len wrapper)
    /// - TAG != 0 => field encoding (payload, then len/key as required by wire type)
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter);
}

pub type ArchivedProtoMessageWriter<T> = ArchivedProtoMessage<T, RevVec>;

pub trait ProtoEncode {
    type Shadow<'a>: ProtoArchive + ProtoExt + ProtoShadowEncode<'a, Self>;

    #[inline]
    fn encode(&self, buf: &mut impl BufMut) -> Result<(), EncodeError>
    where
        Self: ProtoExt,
    {
        let value: ArchivedProtoMessageWriter<Self> = match ArchivedProtoMessage::new(self) {
            Some(v) => v,
            None => return Ok(()),
        };

        ArchivedProtoMessage::encode(value, buf)?;

        Ok(())
    }

    #[inline]
    fn encode_to_vec(&self) -> Vec<u8>
    where
        Self: ProtoExt,
    {
        let value: ArchivedProtoMessageWriter<Self> = match ArchivedProtoMessage::new(self) {
            Some(v) => v,
            None => return vec![],
        };
        value.to_vec_tight()
    }

    #[inline]
    fn to_zero_copy(&self) -> ZeroCopy<Self>
    where
        Self: ProtoExt,
        for<'s> <Self as ProtoEncode>::Shadow<'s>: ProtoArchive,
    {
        ZeroCopy::new(self)
    }
}

pub struct ArchivedProtoMessage<T: ProtoEncode, W: RevWriter> {
    inner: W,
    _pd: PhantomData<T>,
}

impl<T: ProtoEncode, W: RevWriter> ProtoExt for ArchivedProtoMessage<T, W> {
    const KIND: ProtoKind = T::Shadow::KIND;
}

impl<T: ProtoEncode, W: RevWriter> ArchivedProtoMessage<T, W>
where
    T: ProtoEncode + ProtoExt,
    for<'s> <T as ProtoEncode>::Shadow<'s>: ProtoArchive,
{
    const INIT_CAP: usize = 64;
    #[inline]
    pub fn new(input: &T) -> Option<Self> {
        let s = T::Shadow::from_sun(input);
        if !matches!(T::KIND, ProtoKind::Message) && <<T as ProtoEncode>::Shadow<'_> as ProtoArchive>::is_default(&s) {
            return None;
        }
        let hint = <<T as ProtoEncode>::Shadow<'_> as ProtoExt>::ENCODED_SIZE_HINT;
        let capacity = if hint.exact { hint.size } else { hint.size.max(Self::INIT_CAP) };
        let mut w = W::with_capacity(capacity);

        if matches!(T::KIND, ProtoKind::SimpleEnum) {
            s.archive::<1>(&mut w);
        } else {
            s.archive::<0>(&mut w);
        }

        if w.is_empty() {
            return None;
        }

        Some(Self {
            inner: w,
            _pd: PhantomData,
        })
    }

    #[inline]
    pub fn encode(self, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        let v = self.inner.as_written_slice();

        let remaining = buf.remaining_mut();
        let total = v.len();

        if total > remaining {
            return Err(EncodeError::new(total, remaining));
        }

        buf.put_slice(v);
        Ok(())
    }
}

impl<T: ProtoEncode + ProtoExt> ArchivedProtoMessage<T, RevVec>
where
    for<'s> <T as ProtoEncode>::Shadow<'s>: ProtoArchive,
{
    /// Convert to a tight Vec<u8> with data at offset 0.
    ///
    /// This avoids an extra allocation compared to `finish().as_slice().to_vec()`
    /// by doing an in-place memmove within the existing buffer.
    #[inline]
    pub fn to_vec_tight(self) -> Vec<u8> {
        self.inner.finish_tight()
    }

    #[inline]
    pub fn into_bytes(self) -> bytes::Bytes {
        if self.inner.is_empty() {
            return bytes::Bytes::new();
        }
        bytes::Bytes::from_owner(self.inner)
    }
}

impl<T: ProtoEncode, W: RevWriter> ArchivedProtoMessage<T, W> {
    #[inline]
    pub fn as_written_slice(&self) -> &[u8] {
        self.inner.as_written_slice()
    }
}

pub struct ZeroCopy<T: ProtoEncode>(ArchivedProtoMessage<T, RevVec>);

impl<T: ProtoEncode> ZeroCopy<T>
where
    T: ProtoExt,
    for<'s> <T as ProtoEncode>::Shadow<'s>: ProtoArchive,
{
    #[inline]
    pub fn new(value: &T) -> Self {
        if let Some(message) = ArchivedProtoMessage::new(value) {
            return Self(message);
        }

        let empty = ArchivedProtoMessage {
            inner: <RevVec as RevWriter>::empty(),
            _pd: PhantomData,
        };
        Self(empty)
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_written_slice()
    }

    #[inline]
    pub fn into_inner(self) -> ArchivedProtoMessage<T, RevVec> {
        self.0
    }

    #[inline]
    pub fn into_bytes(self) -> bytes::Bytes {
        self.0.into_bytes()
    }
}

impl<T: ProtoEncode> From<ArchivedProtoMessage<T, RevVec>> for ZeroCopy<T> {
    #[inline]
    fn from(value: ArchivedProtoMessage<T, RevVec>) -> Self {
        Self(value)
    }
}

impl<T: ProtoEncode> AsBytes for ZeroCopy<T> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.0.as_written_slice()
    }
}

pub struct ArchivedProtoField<const TAG: u32, T: ProtoArchive + ProtoExt>(PhantomData<T>);

/// Helper for generated code: emits field keys and enforces field-vs-root semantics.
///
/// Deterministic output requires encoding message fields (and repeated elements) in reverse order
/// when using the reverse writer.
impl<const TAG: u32, T: ProtoArchive + ProtoExt> ProtoExt for ArchivedProtoField<TAG, T> {
    const KIND: ProtoKind = T::KIND;
}

impl<const TAG: u32, T: ProtoArchive + ProtoExt> ArchivedProtoField<TAG, T> {
    const _TAG_VARINT: VarintConst<10> = encode_varint_const(((TAG << 3) | Self::WIRE_TYPE as u32) as u64);
    const TAG_LEN: usize = Self::_TAG_VARINT.len;

    pub fn archive(input: &T, w: &mut impl RevWriter) {
        if <T as ProtoArchive>::is_default(input) {
            return;
        }
        input.archive::<{ TAG }>(w);
    }

    /// Creates an ArchivedProtoField that will always encode, even if the value is default.
    /// Use this for enum tuple variants where the variant selection must be preserved.
    pub fn new_always(input: &T, w: &mut impl RevWriter) {
        input.archive::<{ TAG }>(w);
    }

    #[inline]
    pub fn put_key(w: &mut impl RevWriter) {
        w.put_slice(&Self::_TAG_VARINT.bytes[..Self::TAG_LEN]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EmptyMessage;

    impl ProtoExt for EmptyMessage {
        const KIND: ProtoKind = ProtoKind::Message;
        const ENCODED_SIZE_HINT: EncodeSizeHint = EncodeSizeHint::new(0, true);
    }

    impl ProtoShadowEncode<'_, EmptyMessage> for EmptyMessage {
        fn from_sun(_value: &EmptyMessage) -> Self {
            Self
        }
    }

    impl ProtoArchive for EmptyMessage {
        fn is_default(&self) -> bool {
            panic!("top-level messages must not run a default prepass");
        }

        fn archive<const TAG: u32>(&self, _w: &mut impl RevWriter) {
            assert_eq!(TAG, 0);
        }
    }

    impl ProtoEncode for EmptyMessage {
        type Shadow<'a> = Self;
    }

    #[test]
    fn empty_top_level_message_is_detected_from_archive_output() {
        assert_eq!(EmptyMessage.encode_to_vec(), Vec::<u8>::new());
    }

    #[test]
    fn exact_size_hint_avoids_spare_reverse_capacity() {
        assert_eq!(<f64 as ProtoExt>::ENCODED_SIZE_HINT, EncodeSizeHint::new(8, true));
        assert_eq!(<[f64; 4] as ProtoExt>::ENCODED_SIZE_HINT, EncodeSizeHint::new(32, true));
        assert_eq!(<[u8; 4] as ProtoExt>::ENCODED_SIZE_HINT, EncodeSizeHint::new(4, true));
        assert_eq!(<[u64; 4] as ProtoExt>::ENCODED_SIZE_HINT, EncodeSizeHint::new(4, false));
        assert_eq!(<[String; 0] as ProtoExt>::ENCODED_SIZE_HINT, EncodeSizeHint::new(0, true));

        let encoded = 1.0f64.encode_to_vec();
        assert_eq!(encoded.len(), 8);
        assert_eq!(encoded.capacity(), 8);
    }
}
