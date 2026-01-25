use core::marker::PhantomData;

use bytes::BufMut;

use crate::error::EncodeError;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::buffer::RevVec;
use crate::traits::buffer::RevWriter;
use crate::traits::utils::VarintConst;
use crate::traits::utils::encode_varint_const;

pub trait ProtoShadowEncode<'a, T: ?Sized>: Sized {
    fn from_sun(value: &'a T) -> Self;
}

pub trait ProtoArchive: Sized {
    fn is_default(&self) -> bool;
    /// Reverse one-pass archive into a [`RevWriter`].
    ///
    /// TAG semantics:
    /// - TAG == 0 => top-level payload (no field key/len wrapper)
    /// - TAG != 0 => field encoding (payload, then len/key as required by wire type)
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter);
}

pub type ArchivedProtoMessageWriter<T> = ArchivedProtoMessage<T, RevVec>;

pub trait ProtoEncode: Sized {
    type Shadow<'a>: ProtoArchive + ProtoExt + ProtoShadowEncode<'a, Self>;

    #[inline(always)]
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

    #[inline(always)]
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

    /// Pre-encode the message into a `ZeroCopy<Self>` wrapper.
    ///
    /// This allows you to serialize the message once and reuse the serialized
    /// form multiple times without re-encoding. Returns `None` if the message
    /// is empty/default.
    #[inline(always)]
    fn to_zero_copy(&self) -> Option<ZeroCopy<Self>>
    where
        Self: ProtoExt,
        for<'s> <Self as ProtoEncode>::Shadow<'s>: ProtoArchive,
    {
        ArchivedProtoMessage::new(self).map(ZeroCopy::new)
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
        if <<T as ProtoEncode>::Shadow<'_> as ProtoArchive>::is_default(&s) {
            return None;
        }
        let mut w = W::with_capacity(Self::INIT_CAP);

        if matches!(T::KIND, ProtoKind::SimpleEnum) {
            s.archive::<1>(&mut w);
        } else {
            s.archive::<0>(&mut w);
        }

        Some(Self {
            inner: w,
            _pd: PhantomData,
        })
    }

    #[inline(always)]
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
    #[inline(always)]
    pub fn to_vec_tight(self) -> Vec<u8> {
        self.inner.finish_tight()
    }
    #[inline(always)]
    pub fn to_vec_raw(self) -> Vec<u8> {
        self.inner.finish_raw()
    }
}

/// A zero-copy wrapper around a pre-encoded proto message.
///
/// `ZeroCopy<T>` holds an already-serialized proto message in a buffer,
/// allowing it to be sent over the wire without re-encoding. This is useful
/// when you want to pre-compute the serialization (e.g., for caching) and
/// avoid repeated encoding costs.
///
/// Both server responses and client requests can use `ZeroCopy<T>` instead
/// of `T` directly.
pub struct ZeroCopy<T: ProtoEncode + ProtoExt>
where
    for<'s> <T as ProtoEncode>::Shadow<'s>: ProtoArchive,
{
    inner: ArchivedProtoMessage<T, RevVec>,
}

impl<T: ProtoEncode + ProtoExt> ZeroCopy<T>
where
    for<'s> <T as ProtoEncode>::Shadow<'s>: ProtoArchive,
{
    /// Create a new `ZeroCopy<T>` from an `ArchivedProtoMessage`.
    #[inline]
    pub fn new(inner: ArchivedProtoMessage<T, RevVec>) -> Self {
        Self { inner }
    }

    /// Get the underlying bytes as a slice.
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        self.inner.inner.as_written_slice()
    }

    /// Encode the pre-serialized message into a buffer.
    #[inline(always)]
    pub fn encode(self, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        self.inner.encode(buf)
    }

    /// Convert to a tight Vec<u8> with data at offset 0.
    #[inline(always)]
    pub fn into_vec(self) -> crate::alloc::vec::Vec<u8> {
        self.inner.to_vec_tight()
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

    #[inline(always)]
    pub fn put_key(w: &mut impl RevWriter) {
        w.put_slice(&Self::_TAG_VARINT.bytes[..Self::TAG_LEN]);
    }
}
