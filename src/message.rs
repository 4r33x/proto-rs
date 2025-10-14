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
use crate::EncodeError;
use crate::encoding::DecodeContext;
use crate::encoding::decode_key;
use crate::encoding::message;
use crate::encoding::varint::encode_varint;
use crate::encoding::varint::encoded_len_varint;
use crate::encoding::wire_type::WireType;

/// A Protocol Buffers message.
pub trait ProtoExt: Sized {
    /// The lightweight protobuf representation used while encoding and decoding
    /// values of `Self`.
    type Shadow;

    /// Returns the default value for this type according to protobuf semantics.
    /// This is used internally for decoding and should not be called directly.
    #[doc(hidden)]
    fn proto_default() -> Self::Shadow;

    /// Encodes the message using its shadow representation to a buffer.
    #[doc(hidden)]
    fn encode_shadow(shadow: &Self::Shadow, buf: &mut impl BufMut);

    /// Decodes a field from a buffer, and merges it into `shadow`.
    #[doc(hidden)]
    fn merge_field(shadow: &mut Self::Shadow, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    /// Returns the encoded length of the message without a length delimiter for
    /// the provided shadow value.
    fn encoded_len_shadow(shadow: &Self::Shadow) -> usize;

    /// Clears the shadow representation, resetting all fields to their default.
    fn clear_shadow(shadow: &mut Self::Shadow);

    /// Finalises decoding by converting the fully-populated shadow
    /// representation into the domain type.
    fn post_decode(shadow: Self::Shadow) -> Self;

    /// Produces a shadow representation from the domain value prior to encoding
    /// or incremental decoding.
    fn cast_shadow(value: &Self) -> Self::Shadow;

    /// Merges a single field occurrence directly into the domain value.
    ///
    /// The default implementation converts `value` into a shadow
    /// representation, delegates to [`Self::merge_field`], and then
    /// reconstructs the domain value via [`Self::post_decode`]. Types with
    /// specialised storage requirements (e.g. smart pointers) can override this
    /// method to reuse allocations during incremental decoding.
    fn merge_into(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let mut shadow = Self::cast_shadow(value);
        Self::merge_field(&mut shadow, tag, wire_type, buf, ctx)?;
        *value = Self::post_decode(shadow);
        Ok(())
    }

    /// Rebuilds the domain value from a fully merged shadow.
    ///
    /// Types can override this to avoid reallocations during incremental
    /// decoding. By default the shadow is consumed via [`Self::post_decode`].
    fn rebuild_from_shadow(value: &mut Self, shadow: Self::Shadow) {
        *value = Self::post_decode(shadow);
    }

    /// Encodes the message to a buffer.
    ///
    /// This method will panic if the buffer has insufficient capacity.
    ///
    /// Meant to be used only by `Message` implementations.
    #[doc(hidden)]
    fn encode_raw(&self, buf: &mut impl BufMut) {
        let shadow = Self::cast_shadow(self);
        Self::encode_shadow(&shadow, buf);
    }

    /// Returns the encoded length of the message without a length delimiter.
    fn encoded_len(&self) -> usize {
        let shadow = Self::cast_shadow(self);
        Self::encoded_len_shadow(&shadow)
    }

    /// Encodes the message to a buffer.
    ///
    /// An error will be returned if the buffer does not have sufficient capacity.
    fn encode(&self, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        let required = self.encoded_len();
        let remaining = buf.remaining_mut();
        if required > remaining {
            return Err(EncodeError::new(required, remaining));
        }

        self.encode_raw(buf);
        Ok(())
    }

    /// Encodes the message to a newly allocated buffer.
    fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.encoded_len());

        self.encode_raw(&mut buf);
        buf
    }

    /// Encodes the message with a length-delimiter to a buffer.
    ///
    /// An error will be returned if the buffer does not have sufficient capacity.
    fn encode_length_delimited(&self, buf: &mut impl BufMut) -> Result<(), EncodeError> {
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
    fn encode_length_delimited_to_vec(&self) -> Vec<u8> {
        let len = self.encoded_len();
        let mut buf = Vec::with_capacity(len + encoded_len_varint(len as u64));

        encode_varint(len as u64, &mut buf);
        self.encode_raw(&mut buf);
        buf
    }

    /// Decodes an instance of the message from a buffer.
    ///
    /// The entire buffer will be consumed.
    fn decode(mut buf: impl Buf) -> Result<Self, DecodeError> {
        let mut shadow = Self::proto_default();
        let ctx = DecodeContext::default();
        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            Self::merge_field(&mut shadow, tag, wire_type, &mut buf, ctx)?;
        }
        Ok(Self::post_decode(shadow))
    }

    /// Decodes a length-delimited instance of the message from the buffer.
    fn decode_length_delimited(buf: impl Buf) -> Result<Self, DecodeError> {
        let mut message = Self::proto_default();
        Self::merge_length_delimited_shadow(&mut message, buf)?;
        Ok(Self::post_decode(message))
    }

    /// Decodes an instance of the message from a buffer, and merges it into `self`.
    ///
    /// The entire buffer will be consumed.
    fn merge(&mut self, mut buf: impl Buf) -> Result<(), DecodeError> {
        let ctx = DecodeContext::default();
        let mut shadow = Self::cast_shadow(self);
        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            Self::merge_field(&mut shadow, tag, wire_type, &mut buf, ctx)?;
        }
        *self = Self::post_decode(shadow);
        Ok(())
    }

    /// Decodes a length-delimited instance of the message from buffer, and
    /// merges it into `self`.
    fn merge_length_delimited(&mut self, buf: impl Buf) -> Result<(), DecodeError> {
        let mut shadow = Self::cast_shadow(self);
        Self::merge_length_delimited_shadow(&mut shadow, buf)?;
        *self = Self::post_decode(shadow);
        Ok(())
    }

    /// Clears the message, resetting all fields to their default.
    fn clear(&mut self) {
        let mut shadow = Self::proto_default();
        Self::clear_shadow(&mut shadow);
        *self = Self::post_decode(shadow);
    }

    /// Helper for merging a length-delimited payload into a shadow value.
    #[doc(hidden)]
    fn merge_length_delimited_shadow(shadow: &mut Self::Shadow, mut buf: impl Buf) -> Result<(), DecodeError> {
        message::merge_shadow::<Self::Shadow, _, _>(WireType::LengthDelimited, shadow, &mut buf, DecodeContext::default(), Self::merge_field)
    }
}

/// Marker trait for message-like types which can be embedded inside other
/// messages (e.g. nested structs, enums with fields, etc.).
///
/// This trait is automatically implemented for all types generated by the
/// `#[proto_message]` macro and is used internally to provide blanket
/// implementations for collections of nested messages.
pub trait MessageField: ProtoExt {}

/// Helper trait for lightweight proto representations that act as a "shadow"
/// for another Rust type.
///
/// Types implementing this trait can be used with `#[proto_message(convert =
/// ...)]` to automatically bridge encoding/decoding logic between the shadow
/// struct generated by the macro and an existing domain type.
pub trait ProtoShadow: Sized {
    /// The domain type represented by this shadow.
    type Sun;

    /// Convert the shadow into the domain type after decoding completes.
    fn to_sun(self) -> Self::Sun;

    /// Create a shadow representation from a domain value prior to encoding.
    fn cast_shadow(value: &Self::Sun) -> Self;
}

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
            let mut value = Self::post_decode(Self::proto_default());
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
    type Shadow = M::Shadow;

    fn proto_default() -> Self::Shadow {
        M::proto_default()
    }

    fn encode_shadow(shadow: &Self::Shadow, buf: &mut impl BufMut) {
        M::encode_shadow(shadow, buf);
    }

    fn merge_field(shadow: &mut Self::Shadow, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        M::merge_field(shadow, tag, wire_type, buf, ctx)
    }

    fn encoded_len_shadow(shadow: &Self::Shadow) -> usize {
        M::encoded_len_shadow(shadow)
    }

    fn clear_shadow(shadow: &mut Self::Shadow) {
        M::clear_shadow(shadow);
    }

    fn post_decode(shadow: Self::Shadow) -> Self {
        Box::new(M::post_decode(shadow))
    }

    fn cast_shadow(value: &Self) -> Self::Shadow {
        M::cast_shadow(value.as_ref())
    }

    fn rebuild_from_shadow(value: &mut Self, shadow: Self::Shadow) {
        **value = M::post_decode(shadow);
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        M::encode_raw(self.as_ref(), buf);
    }

    fn encoded_len(&self) -> usize {
        M::encoded_len(self.as_ref())
    }

    fn merge_into(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        M::merge_into(value.as_mut(), tag, wire_type, buf, ctx)
    }

    fn merge(&mut self, buf: impl Buf) -> Result<(), DecodeError> {
        M::merge(self.as_mut(), buf)
    }

    fn merge_length_delimited(&mut self, buf: impl Buf) -> Result<(), DecodeError> {
        M::merge_length_delimited(self.as_mut(), buf)
    }

    fn clear(&mut self) {
        M::clear(self.as_mut());
    }
}

impl<M> MessageField for Box<M> where M: MessageField {}

impl<M> ProtoExt for Arc<M>
where
    M: ProtoExt,
{
    type Shadow = M::Shadow;

    fn proto_default() -> Self::Shadow {
        M::proto_default()
    }

    fn encode_shadow(shadow: &Self::Shadow, buf: &mut impl BufMut) {
        M::encode_shadow(shadow, buf);
    }

    fn merge_field(shadow: &mut Self::Shadow, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        M::merge_field(shadow, tag, wire_type, buf, ctx)
    }

    fn encoded_len_shadow(shadow: &Self::Shadow) -> usize {
        M::encoded_len_shadow(shadow)
    }

    fn clear_shadow(shadow: &mut Self::Shadow) {
        M::clear_shadow(shadow);
    }

    fn post_decode(shadow: Self::Shadow) -> Self {
        Arc::new(M::post_decode(shadow))
    }

    fn cast_shadow(value: &Self) -> Self::Shadow {
        M::cast_shadow(value.as_ref())
    }

    fn rebuild_from_shadow(value: &mut Self, shadow: Self::Shadow) {
        if let Some(inner) = Arc::get_mut(value) {
            *inner = M::post_decode(shadow);
        } else {
            *value = Arc::new(M::post_decode(shadow));
        }
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        M::encode_raw(self.as_ref(), buf);
    }

    fn encoded_len(&self) -> usize {
        M::encoded_len(self.as_ref())
    }

    fn merge_into(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if let Some(inner) = Arc::get_mut(value) {
            M::merge_into(inner, tag, wire_type, buf, ctx)
        } else {
            let mut shadow = M::cast_shadow(value.as_ref());
            M::merge_field(&mut shadow, tag, wire_type, buf, ctx)?;
            *value = Arc::new(M::post_decode(shadow));
            Ok(())
        }
    }

    fn merge(&mut self, buf: impl Buf) -> Result<(), DecodeError> {
        if let Some(inner) = Arc::get_mut(self) {
            M::merge(inner, buf)
        } else {
            let ctx = DecodeContext::default();
            let mut shadow = M::cast_shadow(self.as_ref());
            let mut buf = buf;
            while buf.has_remaining() {
                let (tag, wire_type) = decode_key(&mut buf)?;
                M::merge_field(&mut shadow, tag, wire_type, &mut buf, ctx)?;
            }
            *self = Arc::new(M::post_decode(shadow));
            Ok(())
        }
    }

    fn merge_length_delimited(&mut self, buf: impl Buf) -> Result<(), DecodeError> {
        if let Some(inner) = Arc::get_mut(self) {
            M::merge_length_delimited(inner, buf)
        } else {
            let mut shadow = M::cast_shadow(self.as_ref());
            M::merge_length_delimited_shadow(&mut shadow, buf)?;
            *self = Arc::new(M::post_decode(shadow));
            Ok(())
        }
    }

    fn clear(&mut self) {
        if let Some(inner) = Arc::get_mut(self) {
            M::clear(inner);
        } else {
            *self = Arc::new(M::post_decode(M::proto_default()));
        }
    }
}

// `Arc::make_mut` requires the inner value to be `Clone` so that shared
// storage can be detached before mutating during a merge.
impl<M> MessageField for Arc<M> where M: MessageField {}

impl<T> ProtoExt for Vec<T>
where
    T: RepeatedField + Clone,
{
    type Shadow = Self;

    #[inline]
    fn proto_default() -> Self::Shadow {
        Vec::new()
    }

    fn encode_shadow(shadow: &Self::Shadow, buf: &mut impl BufMut) {
        if !shadow.is_empty() {
            T::encode_repeated_field(1, shadow, buf);
        }
    }

    fn merge_field(shadow: &mut Self::Shadow, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            T::merge_repeated_field(wire_type, shadow, buf, ctx)
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len_shadow(shadow: &Self::Shadow) -> usize {
        if shadow.is_empty() { 0 } else { T::encoded_len_repeated_field(1, shadow) }
    }

    fn clear_shadow(shadow: &mut Self::Shadow) {
        Vec::clear(shadow);
    }

    fn post_decode(shadow: Self::Shadow) -> Self {
        shadow
    }

    fn cast_shadow(value: &Self) -> Self::Shadow {
        value.clone()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        Self::encode_shadow(self, buf);
    }

    fn encoded_len(&self) -> usize {
        Self::encoded_len_shadow(self)
    }

    fn merge_into(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        Self::merge_field(value, tag, wire_type, buf, ctx)
    }

    fn merge(&mut self, mut buf: impl Buf) -> Result<(), DecodeError> {
        let ctx = DecodeContext::default();
        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            Self::merge_field(self, tag, wire_type, &mut buf, ctx)?;
        }
        Ok(())
    }

    fn merge_length_delimited(&mut self, buf: impl Buf) -> Result<(), DecodeError> {
        Self::merge_length_delimited_shadow(self, buf)
    }

    fn clear(&mut self) {
        Self::clear_shadow(self);
    }
}

impl<K, V> ProtoExt for BTreeMap<K, V>
where
    K: SingularField + Default + Eq + Hash + Ord + Clone,
    V: SingularField + Default + PartialEq + Clone,
{
    type Shadow = Self;

    #[inline]
    fn proto_default() -> Self::Shadow {
        BTreeMap::new()
    }

    fn encode_shadow(shadow: &Self::Shadow, buf: &mut impl BufMut) {
        if !shadow.is_empty() {
            crate::encoding::btree_map::encode(
                |tag, key, buf| <K as SingularField>::encode_singular_field(tag, key, buf),
                |tag, key| <K as SingularField>::encoded_len_singular_field(tag, key),
                |tag, value, buf| <V as SingularField>::encode_singular_field(tag, value, buf),
                |tag, value| <V as SingularField>::encoded_len_singular_field(tag, value),
                1,
                shadow,
                buf,
            );
        }
    }

    fn merge_field(shadow: &mut Self::Shadow, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            crate::encoding::btree_map::merge(
                |wire_type, key, buf, ctx| <K as SingularField>::merge_singular_field(wire_type, key, buf, ctx),
                |wire_type, value, buf, ctx| <V as SingularField>::merge_singular_field(wire_type, value, buf, ctx),
                shadow,
                buf,
                ctx,
            )
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len_shadow(shadow: &Self::Shadow) -> usize {
        crate::encoding::btree_map::encoded_len(
            |tag, key| <K as SingularField>::encoded_len_singular_field(tag, key),
            |tag, value| <V as SingularField>::encoded_len_singular_field(tag, value),
            1,
            shadow,
        )
    }

    fn clear_shadow(shadow: &mut Self::Shadow) {
        BTreeMap::clear(shadow);
    }

    fn post_decode(shadow: Self::Shadow) -> Self {
        shadow
    }

    fn cast_shadow(value: &Self) -> Self::Shadow {
        value.clone()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        Self::encode_shadow(self, buf);
    }

    fn encoded_len(&self) -> usize {
        Self::encoded_len_shadow(self)
    }

    fn merge_into(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        Self::merge_field(value, tag, wire_type, buf, ctx)
    }

    fn merge(&mut self, mut buf: impl Buf) -> Result<(), DecodeError> {
        let ctx = DecodeContext::default();
        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            Self::merge_field(self, tag, wire_type, &mut buf, ctx)?;
        }
        Ok(())
    }

    fn merge_length_delimited(&mut self, buf: impl Buf) -> Result<(), DecodeError> {
        Self::merge_length_delimited_shadow(self, buf)
    }

    fn clear(&mut self) {
        Self::clear_shadow(self);
    }
}

#[cfg(feature = "std")]
impl<K, V, S> ProtoExt for HashMap<K, V, S>
where
    K: SingularField + Default + Eq + Hash + Ord + Clone,
    V: SingularField + Default + PartialEq + Clone,
    S: BuildHasher + Default + Clone,
{
    type Shadow = Self;

    #[inline]
    fn proto_default() -> Self::Shadow {
        HashMap::default()
    }

    fn encode_shadow(shadow: &Self::Shadow, buf: &mut impl BufMut) {
        if !shadow.is_empty() {
            crate::encoding::hash_map::encode(
                |tag, key, buf| <K as SingularField>::encode_singular_field(tag, key, buf),
                |tag, key| <K as SingularField>::encoded_len_singular_field(tag, key),
                |tag, value, buf| <V as SingularField>::encode_singular_field(tag, value, buf),
                |tag, value| <V as SingularField>::encoded_len_singular_field(tag, value),
                1,
                shadow,
                buf,
            );
        }
    }

    fn merge_field(shadow: &mut Self::Shadow, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            crate::encoding::hash_map::merge(
                |wire_type, key, buf, ctx| <K as SingularField>::merge_singular_field(wire_type, key, buf, ctx),
                |wire_type, value, buf, ctx| <V as SingularField>::merge_singular_field(wire_type, value, buf, ctx),
                shadow,
                buf,
                ctx,
            )
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len_shadow(shadow: &Self::Shadow) -> usize {
        crate::encoding::hash_map::encoded_len(
            |tag, key| <K as SingularField>::encoded_len_singular_field(tag, key),
            |tag, value| <V as SingularField>::encoded_len_singular_field(tag, value),
            1,
            shadow,
        )
    }

    fn clear_shadow(shadow: &mut Self::Shadow) {
        HashMap::clear(shadow);
    }

    fn post_decode(shadow: Self::Shadow) -> Self {
        shadow
    }

    fn cast_shadow(value: &Self) -> Self::Shadow {
        value.clone()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        Self::encode_shadow(self, buf);
    }

    fn encoded_len(&self) -> usize {
        Self::encoded_len_shadow(self)
    }

    fn merge_into(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        Self::merge_field(value, tag, wire_type, buf, ctx)
    }

    fn merge(&mut self, mut buf: impl Buf) -> Result<(), DecodeError> {
        let ctx = DecodeContext::default();
        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            Self::merge_field(self, tag, wire_type, &mut buf, ctx)?;
        }
        Ok(())
    }

    fn merge_length_delimited(&mut self, buf: impl Buf) -> Result<(), DecodeError> {
        Self::merge_length_delimited_shadow(self, buf)
    }

    fn clear(&mut self) {
        Self::clear_shadow(self);
    }
}

impl<T> ProtoExt for BTreeSet<T>
where
    T: RepeatedField + Clone + Ord,
{
    type Shadow = Self;

    #[inline]
    fn proto_default() -> Self::Shadow {
        BTreeSet::new()
    }

    fn encode_shadow(shadow: &Self::Shadow, buf: &mut impl BufMut) {
        if !shadow.is_empty() {
            let values: alloc::vec::Vec<T> = shadow.iter().cloned().collect();
            T::encode_repeated_field(1, &values, buf);
        }
    }

    fn merge_field(shadow: &mut Self::Shadow, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            let mut values: alloc::vec::Vec<T> = alloc::vec::Vec::new();
            T::merge_repeated_field(wire_type, &mut values, buf, ctx)?;
            for value in values {
                shadow.insert(value);
            }
            Ok(())
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len_shadow(shadow: &Self::Shadow) -> usize {
        if shadow.is_empty() {
            0
        } else {
            let values: alloc::vec::Vec<T> = shadow.iter().cloned().collect();
            T::encoded_len_repeated_field(1, &values)
        }
    }

    fn clear_shadow(shadow: &mut Self::Shadow) {
        BTreeSet::clear(shadow);
    }

    fn post_decode(shadow: Self::Shadow) -> Self {
        shadow
    }

    fn cast_shadow(value: &Self) -> Self::Shadow {
        value.clone()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        Self::encode_shadow(self, buf);
    }

    fn encoded_len(&self) -> usize {
        Self::encoded_len_shadow(self)
    }

    fn merge_into(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        Self::merge_field(value, tag, wire_type, buf, ctx)
    }

    fn merge(&mut self, mut buf: impl Buf) -> Result<(), DecodeError> {
        let ctx = DecodeContext::default();
        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            Self::merge_field(self, tag, wire_type, &mut buf, ctx)?;
        }
        Ok(())
    }

    fn merge_length_delimited(&mut self, buf: impl Buf) -> Result<(), DecodeError> {
        Self::merge_length_delimited_shadow(self, buf)
    }

    fn clear(&mut self) {
        Self::clear_shadow(self);
    }
}

#[cfg(feature = "std")]
impl<T, S> ProtoExt for HashSet<T, S>
where
    T: RepeatedField + Clone + Eq + Hash,
    S: BuildHasher + Default + Clone,
{
    type Shadow = Self;

    #[inline]
    fn proto_default() -> Self::Shadow {
        HashSet::default()
    }

    fn encode_shadow(shadow: &Self::Shadow, buf: &mut impl BufMut) {
        if !shadow.is_empty() {
            let values: alloc::vec::Vec<T> = shadow.iter().cloned().collect();
            T::encode_repeated_field(1, &values, buf);
        }
    }

    fn merge_field(shadow: &mut Self::Shadow, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            let mut values: alloc::vec::Vec<T> = alloc::vec::Vec::new();
            T::merge_repeated_field(wire_type, &mut values, buf, ctx)?;
            for value in values {
                shadow.insert(value);
            }
            Ok(())
        } else {
            crate::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len_shadow(shadow: &Self::Shadow) -> usize {
        if shadow.is_empty() {
            0
        } else {
            let values: alloc::vec::Vec<T> = shadow.iter().cloned().collect();
            T::encoded_len_repeated_field(1, &values)
        }
    }

    fn clear_shadow(shadow: &mut Self::Shadow) {
        HashSet::clear(shadow);
    }

    fn post_decode(shadow: Self::Shadow) -> Self {
        shadow
    }

    fn cast_shadow(value: &Self) -> Self::Shadow {
        value.clone()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        Self::encode_shadow(self, buf);
    }

    fn encoded_len(&self) -> usize {
        Self::encoded_len_shadow(self)
    }

    fn merge_into(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        Self::merge_field(value, tag, wire_type, buf, ctx)
    }

    fn merge(&mut self, mut buf: impl Buf) -> Result<(), DecodeError> {
        let ctx = DecodeContext::default();
        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            Self::merge_field(self, tag, wire_type, &mut buf, ctx)?;
        }
        Ok(())
    }

    fn merge_length_delimited(&mut self, buf: impl Buf) -> Result<(), DecodeError> {
        Self::merge_length_delimited_shadow(self, buf)
    }

    fn clear(&mut self) {
        Self::clear_shadow(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proto_ext_is_sized() {
        fn assert_sized<T: ProtoExt + Sized>() {}
        assert_sized::<Option<()>>();
    }
}
