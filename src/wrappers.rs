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
use crate::traits::ProtoShadow;
use crate::traits::Shadow;
use crate::traits::ViewOf;

impl<T> SingularField for T
where
    T: MessageField,
{
    fn encode_singular_field(tag: u32, value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
        let len = <Self as ProtoExt>::Shadow::encoded_len(&value);
        if len != 0 {
            crate::encoding::message::encode(tag, value, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self::Shadow<'_>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        crate::encoding::message::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_singular_field(tag: u32, value: &ViewOf<'_, Self>) -> usize {
        if Shadow::encoded_len(value) == 0 { 0 } else { crate::encoding::message::encoded_len(tag, value) }
    }
}

impl<T> RepeatedField for T
where
    T: MessageField,
{
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        crate::encoding::message::encode_repeated(tag, values, buf);
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        crate::encoding::message::merge_repeated(wire_type, values, buf, ctx)
    }

    fn encoded_len_repeated_field(tag: u32, values: &[ViewOf<'_, T>]) -> usize {
        crate::encoding::message::encoded_len_repeated(tag, values)
    }
}

impl<M> ProtoExt for Box<M>
where
    M: ProtoExt,
{
    fn proto_default() -> Self {
        Box::new(M::proto_default())
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        (**self).encode_raw(buf);
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        (**self).merge_field(tag, wire_type, buf, ctx)
    }

    fn encoded_len(&self) -> usize {
        (**self).encoded_len()
    }

    fn clear(&mut self) {
        (**self).clear();
    }
}

impl<M> MessageField for Box<M> where M: MessageField {}

impl<M> ProtoExt for Arc<M>
where
    M: ProtoExt,
{
    fn proto_default() -> Self {
        Arc::new(M::proto_default())
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        (**self).encode_raw(buf);
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if let Some(v) = Arc::get_mut(self) {
            M::merge_field(v, tag, wire_type, buf, ctx)
        } else {
            unreachable!("There should be no other Arc instances")
        }
    }

    fn encoded_len(&self) -> usize {
        (**self).encoded_len()
    }

    fn clear(&mut self) {
        if let Some(v) = Arc::get_mut(self) {
            M::clear(v);
        } else {
            unreachable!("There should be no other Arc instances")
        }
    }
}

// `Arc::make_mut` requires the inner value to be `Clone` so that shared
// storage can be detached before mutating during a merge.
impl<M> MessageField for Arc<M> where M: MessageField {}

impl<T> ProtoExt for Vec<T>
where
    T: RepeatedField,
{
    #[inline]
    fn proto_default() -> Self {
        Vec::new()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.is_empty() {
            T::encode_repeated_field(1, self, buf);
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            T::merge_repeated_field(wire_type, self, buf, ctx)
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if self.is_empty() { 0 } else { T::encoded_len_repeated_field(1, self) }
    }

    fn clear(&mut self) {
        Vec::clear(self);
    }
}

impl<K, V> ProtoExt for BTreeMap<K, V>
where
    K: SingularField + Default + Eq + Hash + Ord,
    V: SingularField + Default + PartialEq,
{
    #[inline]
    fn proto_default() -> Self {
        BTreeMap::new()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.is_empty() {
            crate::encoding::btree_map::encode(
                |tag, key, buf| <K as SingularField>::encode_singular_field(tag, key, buf),
                |tag, key| <K as SingularField>::encoded_len_singular_field(tag, key),
                |tag, value, buf| <V as SingularField>::encode_singular_field(tag, value, buf),
                |tag, value| <V as SingularField>::encoded_len_singular_field(tag, value),
                1,
                self,
                buf,
            );
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            crate::encoding::btree_map::merge(
                |wire_type, key, buf, ctx| <K as SingularField>::merge_singular_field(wire_type, key, buf, ctx),
                |wire_type, value, buf, ctx| <V as SingularField>::merge_singular_field(wire_type, value, buf, ctx),
                self,
                buf,
                ctx,
            )
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        crate::encoding::btree_map::encoded_len(
            |tag, key| <K as SingularField>::encoded_len_singular_field(tag, key),
            |tag, value| <V as SingularField>::encoded_len_singular_field(tag, value),
            1,
            self,
        )
    }

    fn clear(&mut self) {
        BTreeMap::clear(self);
    }
}

#[cfg(feature = "std")]
impl<K, V, S> ProtoExt for HashMap<K, V, S>
where
    K: SingularField + Default + Eq + Hash + Ord,
    V: SingularField + Default + PartialEq,
    S: BuildHasher + Default,
{
    #[inline]
    fn proto_default() -> Self {
        HashMap::default()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.is_empty() {
            crate::encoding::hash_map::encode(
                |tag, key, buf| <K as SingularField>::encode_singular_field(tag, key, buf),
                |tag, key| <K as SingularField>::encoded_len_singular_field(tag, key),
                |tag, value, buf| <V as SingularField>::encode_singular_field(tag, value, buf),
                |tag, value| <V as SingularField>::encoded_len_singular_field(tag, value),
                1,
                self,
                buf,
            );
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            crate::encoding::hash_map::merge(
                |wire_type, key, buf, ctx| <K as SingularField>::merge_singular_field(wire_type, key, buf, ctx),
                |wire_type, value, buf, ctx| <V as SingularField>::merge_singular_field(wire_type, value, buf, ctx),
                self,
                buf,
                ctx,
            )
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        crate::encoding::hash_map::encoded_len(
            |tag, key| <K as SingularField>::encoded_len_singular_field(tag, key),
            |tag, value| <V as SingularField>::encoded_len_singular_field(tag, value),
            1,
            self,
        )
    }

    fn clear(&mut self) {
        HashMap::clear(self);
    }
}

impl<T> ProtoExt for BTreeSet<T>
where
    T: RepeatedField + Clone + Ord,
{
    #[inline]
    fn proto_default() -> Self {
        BTreeSet::new()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.is_empty() {
            let values: alloc::vec::Vec<T> = self.iter().cloned().collect();
            T::encode_repeated_field(1, &values, buf);
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            let mut values: alloc::vec::Vec<T> = alloc::vec::Vec::new();
            T::merge_repeated_field(wire_type, &mut values, buf, ctx)?;
            for value in values {
                self.insert(value);
            }
            Ok(())
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if self.is_empty() {
            0
        } else {
            let values: alloc::vec::Vec<T> = self.iter().cloned().collect();
            T::encoded_len_repeated_field(1, &values)
        }
    }

    fn clear(&mut self) {
        BTreeSet::clear(self);
    }
}

#[cfg(feature = "std")]
impl<T, S> ProtoExt for HashSet<T, S>
where
    T: RepeatedField + Clone + Eq + Hash,
    S: BuildHasher + Default,
{
    #[inline]
    fn proto_default() -> Self {
        HashSet::default()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.is_empty() {
            let values: alloc::vec::Vec<T> = self.iter().cloned().collect();
            T::encode_repeated_field(1, &values, buf);
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            let mut values: alloc::vec::Vec<T> = alloc::vec::Vec::new();
            T::merge_repeated_field(wire_type, &mut values, buf, ctx)?;
            for value in values {
                self.insert(value);
            }
            Ok(())
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if self.is_empty() {
            0
        } else {
            let values: alloc::vec::Vec<T> = self.iter().cloned().collect();
            T::encoded_len_repeated_field(1, &values)
        }
    }

    fn clear(&mut self) {
        HashSet::clear(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const _MESSAGE_IS_OBJECT_SAFE: Option<&dyn ProtoExt> = None;
}
