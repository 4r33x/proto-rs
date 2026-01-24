use core::array;
use core::mem::MaybeUninit;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
use crate::encoding::skip_field;
use crate::traits::PrimitiveKind;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::wrappers::lists::encode_repeated_value;
use crate::wrappers::lists::repeated_payload_len;

#[cfg(feature = "stable")]
#[inline]
#[allow(clippy::needless_pass_by_value)]
unsafe fn assume_init_array<T, const N: usize>(arr: [MaybeUninit<T>; N]) -> [T; N] {
    let ptr = (&raw const arr).cast::<[T; N]>();
    unsafe { core::ptr::read(ptr) }
}

#[cfg(not(feature = "stable"))]
#[inline]
#[allow(clippy::needless_pass_by_value)]
unsafe fn assume_init_array<T, const N: usize>(arr: [MaybeUninit<T>; N]) -> [T; N] {
    unsafe { MaybeUninit::array_assume_init(arr) }
}

#[doc(hidden)]
pub struct ArchivedArray<'a, T: ProtoArchive + ProtoExt, const N: usize> {
    items: [T::Archived<'a>; N],
    len: usize,
}

impl<T: ProtoExt, const N: usize> ProtoExt for [T; N] {
    const KIND: ProtoKind = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => ProtoKind::Bytes,
        _ => ProtoKind::Repeated(&T::KIND),
    };
    const _REPEATED_SUPPORT: Option<&'static str> = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => None,
        _ => Some("Array"),
    };
}

impl<T: ProtoDecoder + ProtoExt, const N: usize> ProtoDecoder for [T; N] {
    #[inline(always)]
    fn proto_default() -> Self {
        array::from_fn(|_| T::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        for v in self.iter_mut() {
            v.clear();
        }
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if T::KIND.is_bytes_kind() {
            // SAFETY: only executed for [u8]
            let mut bytes: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr().cast::<u8>(), self.len()) };
            return super::bytes_encoding::merge(wire_type, &mut bytes, buf, ctx);
        }
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if wire_type == WireType::LengthDelimited {
                    let len = decode_varint(buf)? as usize;
                    let mut slice = buf.take(len);
                    for v in self.iter_mut() {
                        if !slice.has_remaining() {
                            break;
                        }
                        T::merge(v, T::WIRE_TYPE, &mut slice, ctx)?;
                    }
                    debug_assert!(!slice.has_remaining());
                } else {
                    for v in self.iter_mut() {
                        T::merge(v, wire_type, buf, ctx)?;
                    }
                }
                Ok(())
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                for v in self.iter_mut() {
                    T::merge(v, wire_type, buf, ctx)?;
                }
                Ok(())
            }
            ProtoKind::Repeated(_) => unreachable!(),
        }
    }
}

impl<T: ProtoDecode, const N: usize> ProtoDecode for [T; N]
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = [T::ShadowDecoded; N];
}

impl<T, U, const N: usize> ProtoShadowDecode<[U; N]> for [T; N]
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<[U; N], DecodeError> {
        let mut out: [MaybeUninit<U>; N] = [const { MaybeUninit::uninit() }; N];
        for (i, elem) in self.into_iter().enumerate() {
            match elem.to_sun() {
                Ok(value) => {
                    out[i].write(value);
                }
                Err(err) => {
                    for entry in out.iter_mut().take(i) {
                        unsafe { entry.assume_init_drop() };
                    }
                    return Err(err);
                }
            }
        }
        Ok(unsafe { assume_init_array(out) })
    }
}

impl<T, const N: usize> ProtoArchive for [T; N]
where
    T: ProtoArchive + ProtoExt,
{
    type Archived<'x> = ArchivedArray<'x, T, N>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.iter().all(|item| <T as ProtoArchive>::is_default(item))
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        // For byte arrays [u8; N], the length is always N (raw bytes, not varints)
        if T::KIND.is_bytes_kind() {
            return N;
        }
        archived.len
    }

    #[inline(always)]
    unsafe fn encode<const TAG: u32>(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        // For byte arrays [u8; N], write raw bytes directly
        if T::KIND.is_bytes_kind() {
            // SAFETY: When T::KIND.is_bytes_kind(), T = u8 and T::Archived<'a> = u8
            // The archived.items is [u8; N] and we write each byte directly
            let bytes: &[u8] = unsafe {
                core::slice::from_raw_parts(archived.items.as_ptr().cast::<u8>(), N)
            };
            buf.put_slice(bytes);
            return;
        }
        for item in archived.items {
            encode_repeated_value::<T, TAG>(item, buf);
        }
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self) -> Self::Archived<'_> {
        let mut items: [MaybeUninit<T::Archived<'_>>; N] = [const { MaybeUninit::uninit() }; N];
        let mut len = 0;
        for (idx, item) in self.iter().enumerate() {
            let archived = item.archive::<0>();
            // For byte arrays, len will be N (not computed from varints)
            if !T::KIND.is_bytes_kind() {
                len += repeated_payload_len::<T, TAG>(&archived);
            }
            items[idx].write(archived);
        }
        // For byte arrays, set len to N
        if T::KIND.is_bytes_kind() {
            len = N;
        }
        let items = unsafe { assume_init_array(items) };
        ArchivedArray { items, len }
    }
}

/// Wrapper type for array shadows that preserves array default semantics.
/// Arrays are considered default when all elements are default, unlike slices/vecs
/// which are only default when empty.
#[doc(hidden)]
pub struct ArrayShadow<'a, T: ProtoArchive + ProtoExt, const N: usize> {
    slice: &'a [T],
}

impl<T: ProtoArchive + ProtoExt, const N: usize> ProtoExt for ArrayShadow<'_, T, N> {
    const KIND: ProtoKind = <[T; N] as ProtoExt>::KIND;
    const _REPEATED_SUPPORT: Option<&'static str> = <[T; N] as ProtoExt>::_REPEATED_SUPPORT;
}

impl<T: ProtoArchive + ProtoExt, const N: usize> ProtoArchive for ArrayShadow<'_, T, N> {
    type Archived<'x> = ArchivedArray<'x, T, N>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        // Arrays are default when all elements are default (unlike slices which are default when empty)
        self.slice.iter().all(|item| <T as ProtoArchive>::is_default(item))
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        // For byte arrays [u8; N], the length is always N (raw bytes, not varints)
        if T::KIND.is_bytes_kind() {
            return N;
        }
        archived.len
    }

    #[inline(always)]
    unsafe fn encode<const TAG: u32>(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        // For byte arrays [u8; N], write raw bytes directly
        if T::KIND.is_bytes_kind() {
            // SAFETY: When T::KIND.is_bytes_kind(), T = u8 and T::Archived<'a> = u8
            // The archived.items is [u8; N] and we write each byte directly
            let bytes: &[u8] = unsafe {
                core::slice::from_raw_parts(archived.items.as_ptr().cast::<u8>(), N)
            };
            buf.put_slice(bytes);
            return;
        }
        for item in archived.items {
            encode_repeated_value::<T, TAG>(item, buf);
        }
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self) -> Self::Archived<'_> {
        let mut items: [MaybeUninit<T::Archived<'_>>; N] = [const { MaybeUninit::uninit() }; N];
        let mut len = 0;
        for (idx, item) in self.slice.iter().enumerate() {
            let archived = item.archive::<0>();
            // For byte arrays, len will be N (not computed from varints)
            if !T::KIND.is_bytes_kind() {
                len += repeated_payload_len::<T, TAG>(&archived);
            }
            items[idx].write(archived);
        }
        // For byte arrays, set len to N
        if T::KIND.is_bytes_kind() {
            len = N;
        }
        let items = unsafe { assume_init_array(items) };
        ArchivedArray { items, len }
    }
}

impl<T: ProtoEncode, const N: usize> ProtoEncode for [T; N]
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
    for<'a> T: 'a + ProtoExt + ProtoArchive,
{
    type Shadow<'a> = ArrayShadow<'a, T, N>;
}

impl<'a, T: ProtoArchive + ProtoExt, const N: usize> ProtoShadowEncode<'a, [T; N]> for ArrayShadow<'a, T, N> {
    #[inline]
    fn from_sun(value: &'a [T; N]) -> Self {
        ArrayShadow { slice: value.as_slice() }
    }
}
