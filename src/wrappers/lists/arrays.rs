use core::array;
use core::mem::MaybeUninit;

use bytes::Buf;

use crate::DecodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::check_wire_type;
use crate::encoding::skip_field;
use crate::traits::ArchivedProtoField;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoDecoder;
use crate::traits::ProtoDefault;
use crate::traits::ProtoEncode;
use crate::traits::ProtoExt;
use crate::traits::ProtoFieldMerge;
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
    const KIND: ProtoKind = if T::IS_BYTE {
        ProtoKind::Bytes
    } else {
        ProtoKind::Repeated(&T::KIND)
    };
    const REPEATED_SUPPORT: Option<&'static str> = if T::IS_BYTE { None } else { Some("Array") };
    const ENCODED_SIZE_HINT: crate::EncodeSizeHint = if T::IS_BYTE {
        crate::EncodeSizeHint::new(N, true)
    } else {
        T::ENCODED_SIZE_HINT.repeated(N)
    };
}

impl<T: ProtoFieldMerge + ProtoDefault, const N: usize> ProtoDecoder for [T; N] {
    #[inline]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        self.merge_with_state(wire_type, buf, ctx, &crate::DecodeState::default())
    }

    fn merge_field_with_state(
        value: &mut Self,
        tag: u32,
        wire: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
        state: &crate::DecodeState<'_>,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            value.merge_with_state(wire, buf, ctx, state)
        } else {
            skip_field(wire, tag, buf, ctx)
        }
    }

    fn merge_with_state(
        &mut self,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
        state: &crate::DecodeState<'_>,
    ) -> Result<(), DecodeError> {
        if let Some(bytes) = T::byte_slice_mut(self) {
            check_wire_type(WireType::LengthDelimited, wire_type)?;
            let len = crate::encoding::decode_length_delimiter(&mut *buf)?;
            if len != N {
                return Err(DecodeError::new(format!(
                    "invalid length for fixed byte array: expected {N} got {len}"
                )));
            }
            if len > buf.remaining() {
                return Err(DecodeError::new("buffer underflow"));
            }
            buf.copy_to_slice(bytes);
            return Ok(());
        }
        let mut index = state.index();
        let result = super::merge_repeated(
            self,
            wire_type,
            buf,
            ctx,
            |_, _| {},
            |values, value| {
                let slot = values.get_mut(index).ok_or_else(|| DecodeError::new("packed array has too many elements"))?;
                *slot = value;
                index += 1;
                Ok(())
            },
        );
        state.set_index(index);
        result
    }
}

impl<T: ProtoDefault, const N: usize> ProtoDefault for [T; N] {
    #[inline]
    fn proto_default() -> Self {
        array::from_fn(|_| <T as ProtoDefault>::proto_default())
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
    #[inline]
    fn is_default(&self) -> bool {
        self.iter().all(|item| <T as ProtoArchive>::is_default(item))
    }

    #[inline]
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        if self.is_default() {
            crate::EncodeSizeHint::EMPTY
        } else {
            super::collection_size_hint::<T, TAG>(N)
        }
    }

    #[inline]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        if let Some(bytes) = T::byte_slice(self) {
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

/// Wrapper type for array shadows that preserves array default semantics.
/// Arrays are considered default when all elements are default, unlike slices/vecs
/// which are only default when empty.
#[doc(hidden)]
pub struct ArrayShadow<'a, T: ProtoArchive + ProtoExt, const N: usize> {
    slice: &'a [T],
}

impl<T: ProtoArchive + ProtoExt, const N: usize> ProtoExt for ArrayShadow<'_, T, N> {
    const KIND: ProtoKind = <[T; N] as ProtoExt>::KIND;
    const ENCODED_SIZE_HINT: crate::EncodeSizeHint = <[T; N] as ProtoExt>::ENCODED_SIZE_HINT;
    const REPEATED_SUPPORT: Option<&'static str> = <[T; N] as ProtoExt>::REPEATED_SUPPORT;
}

impl<T: ProtoArchive + ProtoExt, const N: usize> ProtoArchive for ArrayShadow<'_, T, N> {
    #[inline]
    fn is_default(&self) -> bool {
        // Arrays are default when all elements are default (unlike slices which are default when empty)
        self.slice.iter().all(|item| <T as ProtoArchive>::is_default(item))
    }

    #[inline]
    fn encoded_size_hint<const TAG: u32>(&self) -> crate::EncodeSizeHint {
        self.slice.encoded_size_hint::<TAG>()
    }

    #[inline]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        self.slice.archive::<TAG>(w);
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
