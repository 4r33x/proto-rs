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
        // Build [T::OwnedSun; N] fallibly, dropping initialized items on error
        let mut out: [MaybeUninit<T::OwnedSun>; N] = [const { MaybeUninit::uninit() }; N];
        let mut written = 0;

        for (i, elem) in self.into_iter().enumerate() {
            match elem.to_sun() {
                Ok(v) => {
                    out[i].write(v);
                    written += 1;
                }
                Err(e) => {
                    // Drop any initialized elements
                    for j in 0..written {
                        // SAFETY: Only the first `written` elements were initialized.
                        unsafe {
                            out[j].assume_init_drop();
                        }
                    }
                    return Err(e);
                }
            }
        }

        // SAFETY: All N elements have been initialized above.
        Ok(unsafe { MaybeUninit::array_assume_init(out) })
    }

    #[inline]
    fn from_sun<'a>(v: Self::Sun<'a>) -> Self::View<'a> {
        // Consume the input array by value and map each element.
        let mut out: [MaybeUninit<T::View<'a>>; N] = [const { MaybeUninit::uninit() }; N];
        let mut it = v.into_iter();

        for i in 0..N {
            // We own `v`, moving each element out is fine.
            let s = it.next().expect("length N mismatch");
            out[i].write(T::from_sun(s));
        }

        // SAFETY: All N elements were initialized in the loop.
        unsafe { MaybeUninit::array_assume_init(out) }
    }
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
