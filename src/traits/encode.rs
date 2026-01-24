use bytes::BufMut;

use crate::encoding::encode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
use crate::encoding::key_len;
use crate::encoding::WireType;
use crate::error::EncodeError;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::utils::VarintConst;
use crate::traits::utils::encode_varint_const;
use crate::zero_copy::ZeroCopyBuffer;

pub trait ProtoShadowEncode<'a, T: ?Sized>: Sized {
    fn from_sun(value: &'a T) -> Self;
}

pub trait ProtoArchive: Sized {
    type Archived<'a>;

    fn is_default(&self) -> bool;
    fn len(archived: &Self::Archived<'_>) -> usize;
    /// # Safety
    ///
    /// DO NOT CALL IT, ONLY IMPLEMENTATION ALLOWED
    unsafe fn encode<const TAG: u32>(arhived: Self::Archived<'_>, buf: &mut impl BufMut);
    //when tag == 0, do not encode payload with tag, because its top level message (not a field)
    fn archive<const TAG: u32>(&self) -> Self::Archived<'_>;
}

pub trait ProtoEncode: Sized {
    type Shadow<'a>: ProtoArchive + ProtoExt + ProtoShadowEncode<'a, Self>;

    #[inline(always)]
    fn encode(&self, buf: &mut impl BufMut) -> Result<(), EncodeError>
    where
        Self: ProtoExt,
    {
        let shadow = Self::Shadow::from_sun(self);
        let value: ArchivedProtoMessage<Self> = match ArchivedProtoMessage::new(&shadow) {
            Some(v) => v,
            None => return Ok(()),
        };
        let total = value.len;
        let remaining = buf.remaining_mut();
        if total > value.len {
            return Err(EncodeError::new(total, remaining));
        }
        ArchivedProtoMessage::encode(value, buf);

        Ok(())
    }

    #[inline(always)]
    fn encode_to_vec(&self) -> Vec<u8>
    where
        Self: ProtoExt,
    {
        let shadow = Self::Shadow::from_sun(self);
        let value: ArchivedProtoMessage<Self> = match ArchivedProtoMessage::new(&shadow) {
            Some(v) => v,
            None => return vec![],
        };
        let mut buf = Vec::with_capacity(value.len);
        ArchivedProtoMessage::encode(value, &mut buf);
        buf
    }
    #[inline(always)]
    fn encode_to_zerocopy(&self) -> ZeroCopyBuffer
    where
        Self: ProtoExt,
    {
        let shadow = Self::Shadow::from_sun(self);
        let value: ArchivedProtoMessage<Self> = match ArchivedProtoMessage::new(&shadow) {
            Some(v) => v,
            None => return ZeroCopyBuffer::new(),
        };
        let mut buf = ZeroCopyBuffer::with_capacity(value.len);
        value.encode(&mut buf);
        buf
    }
}

pub struct ArchivedProtoMessage<'a, 's, T: ProtoEncode>
where
    's: 'a,
{
    inner: <<T as ProtoEncode>::Shadow<'s> as ProtoArchive>::Archived<'a>, //None when default
    len: usize,                                                            // 0 when inner=None
}
impl<T: ProtoEncode> ProtoExt for ArchivedProtoMessage<'_, '_, T> {
    const KIND: ProtoKind = T::Shadow::KIND;
}

impl<'a, 's, T> ArchivedProtoMessage<'a, 's, T>
where
    T: ProtoEncode + ProtoExt,
    's: 'a,
    <T as ProtoEncode>::Shadow<'s>: ProtoArchive,
{
    pub fn new(input: &'a T::Shadow<'s>) -> Option<Self> {
        // Check is_default first - for enums with value 0, is_default returns true
        // even though len > 0 (varint encoding of 0 is 1 byte)
        if <<T as ProtoEncode>::Shadow<'s> as ProtoArchive>::is_default(input) {
            return None;
        }
        let archived = input.archive::<0>();

        let mut len = <<T as ProtoEncode>::Shadow<'s> as ProtoArchive>::len(&archived);
        if matches!(T::KIND, ProtoKind::SimpleEnum) {
            len += key_len(1);
        }
        Some(Self { len, inner: archived })
    }

    #[inline(always)]
    pub const fn is_default(&self) -> bool {
        self.len == 0
    }

    //used for preallocating buffers
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len
    }
    #[inline(always)]
    pub fn encode(self, buf: &mut impl BufMut) {
        let len = self.len;
        debug_assert!(len != 0);

        if matches!(T::KIND, ProtoKind::SimpleEnum) {
            encode_key(1, WireType::Varint, buf);
            unsafe { T::Shadow::encode::<0>(self.inner, buf) };
        } else {
            unsafe { T::Shadow::encode::<0>(self.inner, buf) };
        }
    }
}

pub struct ArchivedProtoField<'a, const TAG: u32, T: ProtoArchive> {
    inner: Option<T::Archived<'a>>, //None when default
    len: usize,                     // 0 when inner=None
}
impl<const TAG: u32, T: ProtoArchive + ProtoExt> ProtoExt for ArchivedProtoField<'_, TAG, T> {
    const KIND: ProtoKind = T::KIND;
}

impl<'a, const TAG: u32, T: ProtoArchive + ProtoExt> ArchivedProtoField<'a, TAG, T> {
    const _TAG_VARINT: VarintConst<10> = encode_varint_const(((TAG as u64) << 3) | (T::WIRE_TYPE as u64));
    const TAG_LEN: usize = Self::_TAG_VARINT.len;

    pub fn new(input: &'a T) -> Self {
        if <T as ProtoArchive>::is_default(input) {
            return Self { inner: None, len: 0 };
        }
        let archived = input.archive::<{ TAG }>();
        let len = <T as ProtoArchive>::len(&archived);
        Self {
            len,
            inner: Some(archived),
        }
    }

    /// Creates an ArchivedProtoField that will always encode, even if the value is default.
    /// Use this for enum tuple variants where the variant selection must be preserved.
    pub fn new_always(input: &'a T) -> Self {
        let archived = input.archive::<{ TAG }>();
        let len = <T as ProtoArchive>::len(&archived);
        Self {
            len,
            inner: Some(archived),
        }
    }

    #[inline(always)]
    pub fn put_key(buf: &mut impl bytes::BufMut) {
        buf.put_slice(&Self::_TAG_VARINT.bytes[..Self::TAG_LEN]);
    }

    #[inline(always)]
    pub const fn is_default(&self) -> bool {
        self.inner.is_none()
    }

    #[allow(clippy::len_without_is_empty)]
    //used for preallocating buffers
    #[inline(always)]
    pub fn len(&self) -> usize {
        if self.inner.is_none() {
            return 0;
        }
        // For repeated non-packable types, self.len already includes all keys and length prefixes
        // computed by repeated_payload_len in the archive() method
        if T::KIND.is_repeated_non_packable() {
            self.len
        } else if T::WIRE_TYPE.is_length_delimited() {
            Self::TAG_LEN + encoded_len_varint(self.len as u64) + self.len
        } else {
            Self::TAG_LEN + self.len
        }
    }

    pub fn encode(self, buf: &mut impl BufMut) {
        let Some(value) = self.inner else { return };
        // For repeated non-packable types (Vec<String>, Vec<Message>, etc.),
        // each element needs its own field tag. Use encode_repeated which handles this.
        if T::KIND.is_repeated_non_packable() {
            unsafe { <T as ProtoArchive>::encode::<{ TAG }>(value, buf) };
            return;
        }
        Self::put_key(buf);
        if T::WIRE_TYPE.is_length_delimited() {
            encode_varint(self.len as u64, buf);
        }
        unsafe { <T as ProtoArchive>::encode::<0>(value, buf) };
    }
}
