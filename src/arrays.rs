//! `ProtoExt` implementations for fixed-size arrays using new trait system

use core::array;
use std::mem::MaybeUninit;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::skip_field;
use crate::traits::ProtoShadow;
use crate::traits::ViewOf;

// -----------------------------------------------------------------------------
// ProtoShadow for arrays — only provides structural wrapping for borrow/own view
// -----------------------------------------------------------------------------
impl<T: ProtoShadow, const N: usize> ProtoShadow for [T; N] {
    type Sun<'a> = [T::Sun<'a>; N];
    type OwnedSun = [T::OwnedSun; N];
    type View<'a> = [T::View<'a>; N];

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        // Create an uninitialized array
        let mut out: [MaybeUninit<T::OwnedSun>; N] = [const { MaybeUninit::uninit() }; N];

        let mut written = 0;
        for (i, elem) in self.into_iter().enumerate() {
            match elem.to_sun() {
                Ok(v) => {
                    out[i].write(v);
                    written += 1;
                }
                Err(e) => {
                    // Drop initialized elements
                    for j in 0..written {
                        unsafe { out[j].assume_init_drop() };
                    }
                    return Err(e);
                }
            }
        }

        // SAFETY: all N elements are initialized
        Ok(unsafe { array_assume_init(out) })
    }

    #[inline]
    fn from_sun<'a>(v: Self::Sun<'a>) -> Self::View<'a> {
        let mut out: [MaybeUninit<T::View<'a>>; N] = [const { MaybeUninit::uninit() }; N];

        for (idx, x) in v.into_iter().enumerate() {
            out[idx].write(T::from_sun(x));
        }

        unsafe { array_assume_init(out) }
    }
}

/// Stable replacement for `MaybeUninit::array_assume_init`
#[inline]
unsafe fn array_assume_init<T, const N: usize>(arr: [MaybeUninit<T>; N]) -> [T; N] {
    // SAFETY: Caller guarantees all elements are initialized
    let ptr = &arr as *const [MaybeUninit<T>; N] as *const [T; N];
    unsafe { core::ptr::read(ptr) }
}
// -----------------------------------------------------------------------------
// ProtoExt for arrays — placeholder behavior (encoded/decoded by parent struct)
// -----------------------------------------------------------------------------
impl<T: ProtoExt, const N: usize> ProtoExt for [T; N] {
    // The array shadow is an array of element shadows.
    type Shadow<'a>
        = [T::Shadow<'a>; N]
    where
        T: 'a;

    #[inline]
    fn proto_default<'a>() -> Self::Shadow<'a> {
        array::from_fn(|_| T::proto_default())
    }

    #[inline]
    fn encoded_len(_: &ViewOf<'_, Self>) -> usize {
        // Arrays are encoded by the parent struct’s codegen.
        0
    }

    #[inline]
    fn encode_raw<'a>(_: ViewOf<'a, Self>, _: &mut impl BufMut) {
        // Arrays are encoded by the parent struct’s codegen.
    }

    #[inline]
    fn merge_field(_: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        // Arrays are decoded by the parent struct’s codegen.
        skip_field(wire_type, tag, buf, ctx)
    }

    #[inline]
    fn clear(&mut self) {
        // We own `[T; N]`; clear each element in place using T::clear().
        for elem in self.iter_mut() {
            elem.clear();
        }
    }
}
