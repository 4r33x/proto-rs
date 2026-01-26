use core::array;
use core::mem::MaybeUninit;

use bytes::Buf;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::check_wire_type;
use crate::encoding::decode_varint;
use crate::encoding::skip_field;
use crate::traits::ArchivedProtoField;
use crate::traits::PrimitiveKind;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::traits::buffer::RevWriter;

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
            check_wire_type(WireType::LengthDelimited, wire_type)?;
            let len = decode_varint(buf)? as usize;
            if len != N {
                return Err(DecodeError::new(format!(
                    "invalid length for fixed byte array: expected {N} got {len}"
                )));
            }
            if len > buf.remaining() {
                return Err(DecodeError::new("buffer underflow"));
            }
            // SAFETY: only executed for [u8]
            let bytes: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr().cast::<u8>(), self.len()) };
            buf.copy_to_slice(bytes);
            return Ok(());
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
    T: ProtoEncode + ProtoExt,
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        self.iter().all(|item| {
            let shadow = T::Shadow::from_sun(item);
            <T::Shadow<'_> as ProtoArchive>::is_default(&shadow)
        })
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        if T::KIND.is_bytes_kind() {
            let bytes: &[u8] = unsafe { core::slice::from_raw_parts(self.as_ptr().cast::<u8>(), N) };
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
                    let shadow = T::Shadow::from_sun(item);
                    shadow.archive::<0>(w);
                }
                if TAG != 0 {
                    let payload_len = w.written_since(mark);
                    w.put_varint(payload_len as u64);
                    ArchivedProtoField::<TAG, Self>::put_key(w);
                }
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                for item in self.iter().rev() {
                    let shadow = T::Shadow::from_sun(item);
                    ArchivedProtoField::<TAG, T::Shadow<'_>>::new_always(&shadow, w);
                }
            }
            ProtoKind::Repeated(_) => unreachable!(),
        }
    }
}

/// Wrapper type for array shadows that preserves array default semantics.
/// Arrays are considered default when all elements are default, unlike slices/vecs
/// which are only default when empty.
#[doc(hidden)]
pub struct ArrayShadow<'a, T: ProtoEncode + ProtoExt, const N: usize> {
    slice: &'a [T],
}

impl<T: ProtoEncode + ProtoExt, const N: usize> ProtoExt for ArrayShadow<'_, T, N> {
    const KIND: ProtoKind = <[T; N] as ProtoExt>::KIND;
    const _REPEATED_SUPPORT: Option<&'static str> = <[T; N] as ProtoExt>::_REPEATED_SUPPORT;
}

impl<T: ProtoEncode + ProtoExt, const N: usize> ProtoArchive for ArrayShadow<'_, T, N>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
{
    #[inline(always)]
    fn is_default(&self) -> bool {
        // Arrays are default when all elements are default (unlike slices which are default when empty)
        self.slice.iter().all(|item| {
            let shadow = T::Shadow::from_sun(item);
            <T::Shadow<'_> as ProtoArchive>::is_default(&shadow)
        })
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        self.slice.archive::<TAG>(w);
    }
}

impl<T: ProtoEncode, const N: usize> ProtoEncode for [T; N]
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
    for<'a> T: 'a + ProtoExt,
{
    type Shadow<'a> = ArrayShadow<'a, T, N>;
}

impl<'a, T: ProtoEncode + ProtoExt, const N: usize> ProtoShadowEncode<'a, [T; N]> for ArrayShadow<'a, T, N> {
    #[inline]
    fn from_sun(value: &'a [T; N]) -> Self {
        ArrayShadow { slice: value.as_slice() }
    }
}
