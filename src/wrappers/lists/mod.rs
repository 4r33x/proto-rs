use crate::ProtoArchive;
use crate::ProtoExt;
use crate::ProtoKind;
use crate::traits::ArchivedProtoField;
use crate::traits::PrimitiveKind;
use crate::traits::buffer::RevWriter;

mod arrays;
mod btree;
#[cfg(feature = "papaya")]
mod conc_set;
mod deque;
mod hash_set;
mod vec;

impl<T: ProtoExt> ProtoExt for &[T] {
    const KIND: ProtoKind = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => ProtoKind::Bytes,
        _ => ProtoKind::Repeated(&T::KIND),
    };
}

impl<T> ProtoArchive for &[T]
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
            // SAFETY: only executed for &[u8].
            let bytes = unsafe { core::slice::from_raw_parts((*self).as_ptr().cast::<u8>(), (*self).len()) };
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
