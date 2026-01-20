use crate::ProtoArchive;
use crate::ProtoExt;
use crate::ProtoKind;
use crate::traits::PrimitiveKind;
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

impl<T: ProtoExt> ProtoExt for &[T] {
    const KIND: ProtoKind = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => ProtoKind::Bytes,
        _ => ProtoKind::Repeated(&T::KIND),
    };
    const _REPEATED_SUPPORT: Option<&'static str> = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => None,
        _ => Some("&[T]"),
    };
}

impl<T> ProtoArchive for &[T]
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
            // SAFETY: only executed for &[u8].
            let bytes = unsafe { *(core::ptr::from_ref(self).cast::<&[u8]>()) };
            return ArchivedVec::Bytes(bytes);
        }

        let mut items = Vec::with_capacity(self.len());
        let mut len = 0;
        for item in *self {
            let archived = item.archive();
            len += repeated_payload_len::<T>(&archived);
            items.push(archived);
        }
        ArchivedVec::Owned(ArchivedRepeated { items, len })
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
//this fn should probably use tag for ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message branch
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
