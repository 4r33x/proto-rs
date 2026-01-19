use bytes::BufMut;

use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
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
    unsafe fn encode(arhived: Self::Archived<'_>, buf: &mut impl BufMut);
    fn archive(&self) -> Self::Archived<'_>;
}

pub trait ProtoEncode: Sized {
    type Shadow<'a>: ProtoArchive + ProtoExt + ProtoShadowEncode<'a, Self>;

    #[inline(always)]
    fn encode(&self, buf: &mut impl BufMut) -> Result<(), EncodeError> {
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
    fn encode_to_vec(&self) -> Vec<u8> {
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
    fn encode_to_zerocopy(&self) -> ZeroCopyBuffer {
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

impl<'a, 's, T: ProtoEncode> ArchivedProtoMessage<'a, 's, T>
where
    's: 'a,
    <T as ProtoEncode>::Shadow<'s>: ProtoArchive,
{
    pub fn new(input: &'a T::Shadow<'s>) -> Option<Self> {
        let archived = input.archive();
        let len = <<T as ProtoEncode>::Shadow<'s> as ProtoArchive>::len(&archived);
        if len == 0 {
            return None;
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
        const TAG_VARINT_ONE: VarintConst<5> = encode_varint_const(1);
        const TAG_LEN_ONE: usize = TAG_VARINT_ONE.len;

        let len = self.len;
        debug_assert!(len != 0);

        if matches!(Self::KIND, ProtoKind::SimpleEnum) {
            buf.put_slice(&TAG_VARINT_ONE.bytes[..TAG_LEN_ONE]);
            unsafe { T::Shadow::encode(self.inner, buf) };
        } else {
            unsafe { T::Shadow::encode(self.inner, buf) };
        }
    }
}

pub struct ArchivedProtoInner<'a, const TAG: u32, T: ProtoArchive> {
    inner: Option<T::Archived<'a>>, //None when default
    len: usize,                     // 0 when inner=None
}
impl<const TAG: u32, T: ProtoArchive + ProtoExt> ProtoExt for ArchivedProtoInner<'_, TAG, T> {
    const KIND: ProtoKind = T::KIND;
}

impl<'a, const TAG: u32, T: ProtoArchive + ProtoExt> ArchivedProtoInner<'a, TAG, T> {
    const _TAG_VARINT: VarintConst<10> = encode_varint_const(TAG as u64);
    const TAG_LEN: usize = Self::_TAG_VARINT.len;

    pub fn new(input: &'a T) -> Self {
        if <T as ProtoArchive>::is_default(input) {
            return Self { inner: None, len: 0 };
        }
        let archived = input.archive();
        let len = <T as ProtoArchive>::len(&archived);
        Self {
            len,
            inner: Some(archived),
        }
    }

    #[inline(always)]
    pub fn put_tag(buf: &mut impl bytes::BufMut) {
        buf.put_slice(&Self::_TAG_VARINT.bytes[..Self::TAG_LEN]);
    }

    #[inline(always)]
    pub const fn is_default(&self) -> bool {
        self.inner.is_none()
    }

    //used for preallocating buffers
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len + encoded_len_varint(self.len as u64) + Self::TAG_LEN
    }

    pub fn encode(self, buf: &mut impl BufMut) {
        let Some(value) = self.inner else { return };
        if T::WIRE_TYPE.is_length_delimited() {
            Self::put_tag(buf);
            encode_varint(self.len as u64, buf);
        } else {
            Self::put_tag(buf);
        }
        unsafe { <T as ProtoArchive>::encode(value, buf) };
    }
}
