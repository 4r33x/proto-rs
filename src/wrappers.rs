#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap;
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeSet;
#[cfg(not(feature = "std"))]
use alloc::sync::Arc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::hash::Hash;
#[cfg(feature = "std")]
use std::collections::BTreeMap;
#[cfg(feature = "std")]
use std::collections::BTreeSet;
#[cfg(feature = "std")]
use std::collections::HashMap;
#[cfg(feature = "std")]
use std::collections::HashSet;
#[cfg(feature = "std")]
use std::hash::BuildHasher;
#[cfg(feature = "std")]
use std::sync::Arc;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::MessageField;
use crate::ProtoExt;
use crate::RepeatedField;
use crate::SingularField;
use crate::encoding::DecodeContext;
use crate::encoding::wire_type::WireType;
use crate::traits::OwnedSunOf;
use crate::traits::ProtoShadow;
use crate::traits::Shadow;
use crate::traits::SunOf;
use crate::traits::ViewOf;

// ---------------- Blanket impls for MessageField ----------------

impl<T> SingularField for T
where
    T: MessageField,
{
    #[inline]
    fn encode_singular_field(tag: u32, value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
        let len = <Self as ProtoExt>::encoded_len(&value);
        if len != 0 {
            crate::encoding::message::encode::<Self>(tag, value, buf);
        }
    }

    #[inline]
    fn merge_singular_field(wire_type: WireType, value: &mut Self::Shadow<'_>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        crate::encoding::message::merge::<Self, _>(wire_type, value, buf, ctx)
    }

    #[inline]
    fn encoded_len_singular_field(tag: u32, value: &ViewOf<'_, Self>) -> usize {
        if <Self as ProtoExt>::encoded_len(value) == 0 {
            0
        } else {
            crate::encoding::message::encoded_len::<Self>(tag, value)
        }
    }
}

impl<T> RepeatedField for T
where
    T: MessageField,
    for<'a> T::Shadow<'a>: ProtoShadow<Sun<'a> = &'a T, OwnedSun = T>,
{
    #[inline]
    fn encode_repeated_field(tag: u32, values: &[OwnedSunOf<'_, Self>], buf: &mut impl BufMut) {
        crate::encoding::message::encode_repeated::<Self>(tag, values, buf);
    }

    #[inline]
    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        crate::encoding::message::merge_repeated::<Self>(wire_type, values, buf, ctx)
    }

    #[inline]
    fn encoded_len_repeated_field(tag: u32, values: &[OwnedSunOf<'_, Self>]) -> usize {
        crate::encoding::message::encoded_len_repeated::<Self>(tag, values)
    }
}

// ---------------- Vec<T> ----------------

impl<T> ProtoExt for Vec<T>
where
    T: RepeatedField,
{
    // Shadow of Vec<T> is a vector of element shadows. This is zero-alloc
    // during encode/size/merge until you actually build an owned Vec<T>
    // in post_decode via T’s own post_decode path.
    type Shadow<'a>
        = Vec<Shadow<'a, T>>
    where
        T: 'a;

    #[inline]
    fn proto_default<'a>() -> Self::Shadow<'a> {
        Vec::new()
    }

    #[inline]
    fn encoded_len(values: &ViewOf<'_, Self>) -> usize {
        if values.is_empty() { 0 } else { T::encoded_len_repeated_field(1, values) }
    }

    #[inline]
    fn encode_raw<'a>(values: ViewOf<'a, Self>, buf: &mut impl BufMut) {
        if !values.is_empty() {
            T::encode_repeated_field(1, values, buf);
        }
    }

    #[inline]
    fn merge_field(values: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            T::merge_repeated_field(wire_type, values, buf, ctx)
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    // Use T’s post_decode to build the final owned Vec<T>.
    #[inline]
    fn post_decode(shadow: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        shadow.into_iter().map(T::post_decode).collect()
    }

    #[inline]
    fn clear(&mut self) {
        Vec::clear(self);
    }
}

// ---------------- BTreeMap<K, V> ----------------

impl<K, V> ProtoExt for BTreeMap<K, V>
where
    K: SingularField + Default + Eq + Hash + Ord,
    V: SingularField + Default + PartialEq,
{
    // The shadow pairs the element shadows.
    type Shadow<'a>
        = BTreeMap<Shadow<'a, K>, Shadow<'a, V>>
    where
        K: 'a,
        V: 'a;

    #[inline]
    fn proto_default<'a>() -> Self::Shadow<'a> {
        BTreeMap::new()
    }

    #[inline]
    fn encoded_len(value: &ViewOf<'_, Self>) -> usize {
        crate::encoding::btree_map::encoded_len(
            |tag, k| <K as SingularField>::encoded_len_singular_field(tag, k),
            |tag, v| <V as SingularField>::encoded_len_singular_field(tag, v),
            1,
            value,
        )
    }

    #[inline]
    fn encode_raw<'a>(value: ViewOf<'a, Self>, buf: &mut impl BufMut) {
        if !value.is_empty() {
            crate::encoding::btree_map::encode(
                |tag, k, b| <K as SingularField>::encode_singular_field(tag, k, b),
                |tag, k| <K as SingularField>::encoded_len_singular_field(tag, k),
                |tag, v, b| <V as SingularField>::encode_singular_field(tag, v, b),
                |tag, v| <V as SingularField>::encoded_len_singular_field(tag, v),
                1,
                value,
                buf,
            );
        }
    }

    #[inline]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            crate::encoding::btree_map::merge(
                |wt, k, b, c| <K as SingularField>::merge_singular_field(wt, k, b, c),
                |wt, v, b, c| <V as SingularField>::merge_singular_field(wt, v, b, c),
                value,
                buf,
                ctx,
            )
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn post_decode(map_shadow: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        let mut out = BTreeMap::new();
        for (k_s, v_s) in map_shadow {
            let k = <K as ProtoExt>::post_decode(k_s)?;
            let v = <V as ProtoExt>::post_decode(v_s)?;
            out.insert(k, v);
        }
        Ok(out)
    }

    #[inline]
    fn clear(&mut self) {
        BTreeMap::clear(self);
    }
}

// ---------------- HashMap<K, V> ----------------
#[cfg(feature = "std")]
impl<K, V, S> ProtoExt for HashMap<K, V, S>
where
    K: SingularField + Default + Eq + Hash + Ord,
    V: SingularField + Default + PartialEq,
    S: BuildHasher + Default,
{
    type Shadow<'a>
        = HashMap<Shadow<'a, K>, Shadow<'a, V>, S>
    where
        K: 'a,
        V: 'a;

    #[inline]
    fn proto_default<'a>() -> Self::Shadow<'a> {
        HashMap::default()
    }

    #[inline]
    fn encoded_len(value: &ViewOf<'_, Self>) -> usize {
        crate::encoding::hash_map::encoded_len(
            |tag, k| <K as SingularField>::encoded_len_singular_field(tag, k),
            |tag, v| <V as SingularField>::encoded_len_singular_field(tag, v),
            1,
            value,
        )
    }

    #[inline]
    fn encode_raw<'a>(value: ViewOf<'a, Self>, buf: &mut impl BufMut) {
        if !value.is_empty() {
            crate::encoding::hash_map::encode(
                |tag, k, b| <K as SingularField>::encode_singular_field(tag, k, b),
                |tag, k| <K as SingularField>::encoded_len_singular_field(tag, k),
                |tag, v, b| <V as SingularField>::encode_singular_field(tag, v, b),
                |tag, v| <V as SingularField>::encoded_len_singular_field(tag, v),
                1,
                value,
                buf,
            );
        }
    }

    #[inline]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            crate::encoding::hash_map::merge(
                |wt, k, b, c| <K as SingularField>::merge_singular_field(wt, k, b, c),
                |wt, v, b, c| <V as SingularField>::merge_singular_field(wt, v, b, c),
                value,
                buf,
                ctx,
            )
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn post_decode(map_shadow: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        let mut out = HashMap::default();
        for (k_s, v_s) in map_shadow {
            let k = <K as ProtoExt>::post_decode(k_s)?;
            let v = <V as ProtoExt>::post_decode(v_s)?;
            out.insert(k, v);
        }
        Ok(out)
    }

    #[inline]
    fn clear(&mut self) {
        HashMap::clear(self);
    }
}

// ---------------- BTreeSet<T> ----------------

impl<T> ProtoExt for BTreeSet<T>
where
    T: RepeatedField + Clone + Ord,
{
    // Represent as a set of element shadows while merging, then lift to owned T in post_decode.
    type Shadow<'a>
        = BTreeSet<Shadow<'a, T>>
    where
        T: 'a;

    #[inline]
    fn proto_default<'a>() -> Self::Shadow<'a> {
        BTreeSet::new()
    }

    #[inline]
    fn encoded_len(value: &ViewOf<'_, Self>) -> usize {
        if value.is_empty() {
            0
        } else {
            // NOTE: this collects to a Vec to reuse the RepeatedField API (same as your legacy).
            let vals: Vec<_> = value.iter().cloned().collect();
            T::encoded_len_repeated_field(1, &vals)
        }
    }

    #[inline]
    fn encode_raw<'a>(value: ViewOf<'a, Self>, buf: &mut impl BufMut) {
        if !value.is_empty() {
            let vals: Vec<_> = value.iter().cloned().collect();
            T::encode_repeated_field(1, &vals, buf);
        }
    }

    #[inline]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            let mut tmp: Vec<Shadow<'_, T>> = Vec::new();
            T::merge_repeated_field(wire_type, &mut tmp, buf, ctx)?;
            for v in tmp {
                value.insert(v);
            }
            Ok(())
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn post_decode(set_shadow: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        let mut out = BTreeSet::new();
        for s in set_shadow {
            out.insert(<T as ProtoExt>::post_decode(s)?);
        }
        Ok(out)
    }

    #[inline]
    fn clear(&mut self) {
        BTreeSet::clear(self);
    }
}

// ---------------- HashSet<T> ----------------

#[cfg(feature = "std")]
impl<T, S> ProtoExt for HashSet<T, S>
where
    T: RepeatedField + Clone + Eq + Hash,
    S: BuildHasher + Default,
{
    type Shadow<'a>
        = HashSet<Shadow<'a, T>, S>
    where
        T: 'a;

    #[inline]
    fn proto_default<'a>() -> Self::Shadow<'a> {
        HashSet::default()
    }

    #[inline]
    fn encoded_len(value: &ViewOf<'_, Self>) -> usize {
        if value.is_empty() {
            0
        } else {
            let vals: Vec<_> = value.iter().cloned().collect();
            T::encoded_len_repeated_field(1, &vals)
        }
    }

    #[inline]
    fn encode_raw<'a>(value: ViewOf<'a, Self>, buf: &mut impl BufMut) {
        if !value.is_empty() {
            let vals: Vec<_> = value.iter().cloned().collect();
            T::encode_repeated_field(1, &vals, buf);
        }
    }

    #[inline]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            let mut tmp: Vec<Shadow<'_, T>> = Vec::new();
            T::merge_repeated_field(wire_type, &mut tmp, buf, ctx)?;
            for v in tmp {
                value.insert(v);
            }
            Ok(())
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline]
    fn post_decode(set_shadow: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        let mut out = HashSet::default();
        for s in set_shadow {
            out.insert(<T as ProtoExt>::post_decode(s)?);
        }
        Ok(out)
    }

    #[inline]
    fn clear(&mut self) {
        HashSet::clear(self);
    }
}
