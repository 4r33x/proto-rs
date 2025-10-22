// ---------- imports (adjust for no_std) ----------
extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::mem::MaybeUninit;
use std::collections::BTreeSet;
#[cfg(feature = "std")]
use std::collections::HashSet;
#[cfg(feature = "std")]
use std::hash::Hash;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::OwnedSunOf;
use crate::ProtoExt;
use crate::RepeatedCollection;
use crate::encoding::DecodeContext;
use crate::encoding::check_wire_type;
use crate::encoding::wire_type::WireType;
use crate::traits::ProtoShadow;
use crate::traits::ProtoWire;
use crate::traits::ViewOf;

mod arcs;
mod boxes;
mod lists;
mod options;

// /// Generic implementation for Option<T>
// impl<T: ProtoShadow> ProtoShadow for Option<T> {
//     type Sun<'a> = Option<T::Sun<'a>>;

//     type OwnedSun = Option<T::OwnedSun>;
//     type View<'a> = Option<T::View<'a>>;

//     #[inline]
//     fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
//         // Map Option<T> → Option<T::OwnedSun>
//         self.map(T::to_sun).transpose()
//     }

//     #[inline]
//     fn from_sun<'a>(v: Self::Sun<'_>) -> Self::View<'_> {
//         v.map(T::from_sun)
//     }
// }

// impl<T> RepeatedCollection<T> for Vec<T> {
//     #[inline]
//     fn reserve_hint(&mut self, additional: usize) {
//         Vec::reserve(self, additional);
//     }

//     #[inline]
//     fn push(&mut self, value: T) {
//         Vec::push(self, value);
//     }
// }

// impl<T: Ord> RepeatedCollection<T> for BTreeSet<T> {
//     #[inline]
//     fn push(&mut self, value: T) {
//         let _ = BTreeSet::insert(self, value);
//     }
// }

// #[cfg(feature = "std")]
// impl<T: Eq + Hash, S: std::hash::BuildHasher> RepeatedCollection<T> for HashSet<T, S> {
//     #[inline]
//     fn reserve_hint(&mut self, additional: usize) {
//         HashSet::reserve(self, additional);
//     }

//     #[inline]
//     fn push(&mut self, value: T) {
//         let _ = HashSet::insert(self, value);
//     }
// }
