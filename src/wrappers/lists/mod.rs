use crate::ProtoArchive;
use crate::ProtoExt;
use crate::ProtoKind;
use crate::bytes::BufMut;
use crate::encoding::bytes as bytes_encoding;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;

mod arrays;
mod btree;
#[cfg(feature = "papaya")]
mod conc_set;
mod deque;
mod hash_set;
mod vec;

#[doc(hidden)]
pub struct ArchivedRepeated<'a, T: ProtoArchive + ProtoExt> {
    items: Vec<T::Archived<'a>>,
    len: usize,
}

pub enum ArchivedVec<'a, T: ProtoArchive + ProtoExt> {
    Bytes(&'a [u8]),
    Owned(ArchivedRepeated<'a, T>),
}

impl ProtoExt for &[u8] {
    const KIND: ProtoKind = ProtoKind::Bytes;
}

impl ProtoArchive for &[u8] {
    type Archived<'x> = &'x [u8];

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        archived.len()
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        bytes_encoding::encode(&archived, buf);
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        self
    }
}

#[inline(always)]
fn repeated_payload_len<T: ProtoArchive + ProtoExt>(archived: &T::Archived<'_>) -> usize {
    let item_len = T::len(archived);
    match T::KIND {
        ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => item_len,
        ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => encoded_len_varint(item_len as u64) + item_len,
        ProtoKind::Repeated(_) => unreachable!(),
    }
}

#[inline(always)]
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
