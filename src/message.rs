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
use std::sync::Arc;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::EncodeError;
use crate::encoding::DecodeContext;
use crate::encoding::decode_key;
use crate::encoding::message;
use crate::encoding::varint::encode_varint;
use crate::encoding::varint::encoded_len_varint;
use crate::encoding::wire_type::WireType;

/// A Protocol Buffers message.
pub trait ProtoExt {
    /// Returns the default value for this type according to protobuf semantics.
    /// This is used internally for decoding and should not be called directly.
    #[doc(hidden)]
    fn proto_default() -> Self
    where
        Self: Sized;

    /// Encodes the message to a buffer.
    ///
    /// This method will panic if the buffer has insufficient capacity.
    ///
    /// Meant to be used only by `Message` implementations.
    #[doc(hidden)]
    fn encode_raw(&self, buf: &mut impl BufMut)
    where
        Self: Sized;

    /// Decodes a field from a buffer, and merges it into `self`.
    ///
    /// Meant to be used only by `Message` implementations.
    #[doc(hidden)]
    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>
    where
        Self: Sized;

    /// Hook that is invoked after a successful decode/merge pass completes.
    ///
    /// The default implementation is a no-op. Code generated via
    /// `#[proto_message]` overrides this to run post-processing that depends on
    /// all fields having been decoded.
    fn post_decode(&mut self) {}

    /// Returns the encoded length of the message without a length delimiter.
    fn encoded_len(&self) -> usize;

    /// Encodes the message to a buffer.
    ///
    /// An error will be returned if the buffer does not have sufficient capacity.
    fn encode(&self, buf: &mut impl BufMut) -> Result<(), EncodeError>
    where
        Self: Sized,
    {
        let required = self.encoded_len();
        let remaining = buf.remaining_mut();
        if required > remaining {
            return Err(EncodeError::new(required, remaining));
        }

        self.encode_raw(buf);
        Ok(())
    }

    /// Encodes the message to a newly allocated buffer.
    fn encode_to_vec(&self) -> Vec<u8>
    where
        Self: Sized,
    {
        let mut buf = Vec::with_capacity(self.encoded_len());

        self.encode_raw(&mut buf);
        buf
    }

    /// Encodes the message with a length-delimiter to a buffer.
    ///
    /// An error will be returned if the buffer does not have sufficient capacity.
    fn encode_length_delimited(&self, buf: &mut impl BufMut) -> Result<(), EncodeError>
    where
        Self: Sized,
    {
        let len = self.encoded_len();
        let required = len + encoded_len_varint(len as u64);
        let remaining = buf.remaining_mut();
        if required > remaining {
            return Err(EncodeError::new(required, remaining));
        }
        encode_varint(len as u64, buf);
        self.encode_raw(buf);
        Ok(())
    }

    /// Encodes the message with a length-delimiter to a newly allocated buffer.
    fn encode_length_delimited_to_vec(&self) -> Vec<u8>
    where
        Self: Sized,
    {
        let len = self.encoded_len();
        let mut buf = Vec::with_capacity(len + encoded_len_varint(len as u64));

        encode_varint(len as u64, &mut buf);
        self.encode_raw(&mut buf);
        buf
    }

    /// Decodes an instance of the message from a buffer.
    ///
    /// The entire buffer will be consumed.
    fn decode(mut buf: impl Buf) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        let mut message = Self::proto_default();
        Self::merge(&mut message, &mut buf).map(|_| message)
    }

    /// Decodes a length-delimited instance of the message from the buffer.
    fn decode_length_delimited(buf: impl Buf) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        let mut message = Self::proto_default();
        message.merge_length_delimited(buf)?;
        Ok(message)
    }

    /// Decodes an instance of the message from a buffer, and merges it into `self`.
    ///
    /// The entire buffer will be consumed.
    fn merge(&mut self, mut buf: impl Buf) -> Result<(), DecodeError>
    where
        Self: Sized,
    {
        let ctx = DecodeContext::default();
        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            self.merge_field(tag, wire_type, &mut buf, ctx)?;
        }
        self.post_decode();
        Ok(())
    }

    /// Decodes a length-delimited instance of the message from buffer, and
    /// merges it into `self`.
    fn merge_length_delimited(&mut self, mut buf: impl Buf) -> Result<(), DecodeError>
    where
        Self: Sized,
    {
        message::merge(WireType::LengthDelimited, self, &mut buf, DecodeContext::default())
    }

    /// Clears the message, resetting all fields to their default.
    fn clear(&mut self);
}

/// Marker trait for message-like types which can be embedded inside other
/// messages (e.g. nested structs, enums with fields, etc.).
///
/// This trait is automatically implemented for all types generated by the
/// `#[proto_message]` macro and is used internally to provide blanket
/// implementations for collections of nested messages.
pub trait MessageField: ProtoExt {}

/// Marker trait for enums encoded as plain `int32` values on the wire.
///
/// Derive macros mark unit enums with this trait so other generated code can
/// reliably treat them as scalar fields. Manual implementations can opt in to
/// the same behaviour by providing the conversions required here alongside the
/// appropriate [`ProtoExt`], [`SingularField`], and [`RepeatedField`]
/// implementations.
pub trait ProtoEnum: Copy + Sized {
    /// Default value used when decoding absent fields.
    const DEFAULT_VALUE: Self;

    /// Convert a raw `i32` value into the enum, returning a [`DecodeError`]
    /// when the value is not recognised.
    fn from_i32(value: i32) -> Result<Self, DecodeError>;

    /// Convert the enum into its raw `i32` representation.
    fn to_i32(self) -> i32;
}

/// Trait describing how to encode, decode, and size a single field value.
///
/// Implementations exist for all scalar protobuf types, as well as the message
/// types generated via `#[proto_message]`. Codegen can rely on this trait to
/// drive both singular fields and optional wrappers without having to know the
/// concrete wire representation of `Self`.
pub trait SingularField: ProtoExt + Sized {
    /// Encodes `value` as a singular field with the provided tag.
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut);

    /// Merges a single field occurrence into `value`.
    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    /// Computes the encoded length for a singular field with the provided tag.
    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize;

    /// Encodes an optional field by delegating to [`Self::encode_singular_field`].
    fn encode_option_field(tag: u32, value: &Option<Self>, buf: &mut impl BufMut) {
        if let Some(inner) = value.as_ref() {
            Self::encode_singular_field(tag, inner, buf);
        }
    }

    /// Decodes an optional field occurrence and stores the result inside
    /// `target`.
    fn merge_option_field(wire_type: WireType, target: &mut Option<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if let Some(value) = target.as_mut() {
            Self::merge_singular_field(wire_type, value, buf, ctx)
        } else {
            let mut value = Self::proto_default();
            Self::merge_singular_field(wire_type, &mut value, buf, ctx)?;
            *target = Some(value);
            Ok(())
        }
    }

    /// Computes the encoded length for an optional field.
    fn encoded_len_option_field(tag: u32, value: &Option<Self>) -> usize {
        value.as_ref().map_or(0, |inner| Self::encoded_len_singular_field(tag, inner))
    }
}

impl<T> SingularField for T
where
    T: MessageField,
{
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        let len = ProtoExt::encoded_len(value);
        if len != 0 {
            crate::encoding::message::encode(tag, value, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        crate::encoding::message::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if ProtoExt::encoded_len(value) == 0 {
            0
        } else {
            crate::encoding::message::encoded_len(tag, value)
        }
    }
}

/// Trait describing how to encode and decode a repeated field of a particular
/// element type. This is used to support nested `Vec<T>` values inside
/// generated structs and enums without requiring ad-hoc implementations for
/// every possible `T`.
pub trait RepeatedField: ProtoExt + Sized {
    /// Encodes `values` as a repeated field with the provided tag.
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut);

    /// Merges repeated field occurrences into `values`.
    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    /// Returns the encoded length of a repeated field with the provided tag.
    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize;
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

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
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
        (**self).encode_raw(buf)
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        (**self).merge_field(tag, wire_type, buf, ctx)
    }

    fn encoded_len(&self) -> usize {
        (**self).encoded_len()
    }

    fn clear(&mut self) {
        (**self).clear()
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
        (**self).encode_raw(buf)
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
impl<K, V> ProtoExt for HashMap<K, V>
where
    K: SingularField + Default + Eq + Hash + Ord,
    V: SingularField + Default + PartialEq,
{
    #[inline]
    fn proto_default() -> Self {
        HashMap::new()
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
impl<T> ProtoExt for HashSet<T>
where
    T: RepeatedField + Clone + Eq + Hash,
{
    #[inline]
    fn proto_default() -> Self {
        HashSet::new()
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
