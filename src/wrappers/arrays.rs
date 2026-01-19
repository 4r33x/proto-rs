//! `ProtoExt` implementations for fixed-size arrays using new trait system

use core::array;
use core::mem::MaybeUninit;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
use crate::encoding::skip_field;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;

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

fn repeated_payload_len<T: ProtoArchive + ProtoExt>(archived: &T::Archived<'_>) -> usize {
    let item_len = T::len(archived);
    match T::KIND {
        ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => item_len,
        ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => encoded_len_varint(item_len as u64) + item_len,
        ProtoKind::Repeated(_) => unreachable!(),
    }
}

fn encode_repeated_value<T: ProtoArchive + ProtoExt>(archived: T::Archived<'_>, buf: &mut impl BufMut) {
    match T::KIND {
        ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => unsafe { T::encode(archived, buf) },
        ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
            let len = T::len(&archived);
            encode_varint(len as u64, buf);
            unsafe { T::encode(archived, buf) };
        }
        ProtoKind::Repeated(_) => unreachable!(),
    }
}

impl<T: ProtoExt, const N: usize> ProtoExt for [T; N] {
    const KIND: ProtoKind = ProtoKind::Repeated(&T::KIND);
    const _REPEATED_SUPPORT: Option<&'static str> = Some("Array");
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
        archived.len
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        for item in archived.items {
            encode_repeated_value::<T>(item, buf);
        }
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        let mut items: [MaybeUninit<T::Archived<'_>>; N] = [const { MaybeUninit::uninit() }; N];
        let mut len = 0;
        for (idx, item) in self.iter().enumerate() {
            let archived = item.archive();
            len += repeated_payload_len::<T>(&archived);
            items[idx].write(archived);
        }
        let items = unsafe { assume_init_array(items) };
        ArchivedArray { items, len }
    }
}

impl<T: ProtoEncode, const N: usize> ProtoEncode for [T; N]
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
{
    type Shadow<'a> = [T::Shadow<'a>; N];
}

impl<'a, T, S, const N: usize> ProtoShadowEncode<'a, [T; N]> for [S; N]
where
    S: ProtoShadowEncode<'a, T>,
{
    #[inline]
    fn from_sun(value: &'a [T; N]) -> Self {
        let mut out: [MaybeUninit<S>; N] = [const { MaybeUninit::uninit() }; N];
        for (idx, item) in value.iter().enumerate() {
            out[idx].write(S::from_sun(item));
        }
        unsafe { assume_init_array(out) }
    }
}
