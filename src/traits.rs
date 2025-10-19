use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::EncodeError;
use crate::alloc::vec::Vec;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;

// ---------- conversion trait users implement ----------
pub trait ProtoShadow: Sized {
    /// Borrowed or owned form used during encoding.
    type Sun<'a>: 'a;

    /// The value returned after decoding — can be fully owned
    /// (e.g. `D128`, `String`) or a zero-copy wrapper `ZeroCopyAccess<T>`.
    type OwnedSun: Sized;

    /// The *resulting* shadow type when constructed from a given Sun<'b>, it could be just zero-copy view so we can encode it to buffer
    type View<'a>: 'a;

    /// Decoder to owned value
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError>;

    /// Build a shadow from an existing Sun (borrowed or owned).
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_>;
}

// Helper alias to shorten signatures:
pub type Shadow<'a, T> = <T as ProtoExt>::Shadow<'a>;
pub type SunOf<'a, T> = <Shadow<'a, T> as ProtoShadow>::Sun<'a>;
pub type OwnedSunOf<'a, T> = <Shadow<'a, T> as ProtoShadow>::OwnedSun;
pub type ViewOf<'a, T> = <Shadow<'a, T> as ProtoShadow>::View<'a>;

pub trait ProtoExt: Sized {
    type Shadow<'a>: ProtoShadow<OwnedSun = Self>
    where
        Self: 'a;

    fn proto_default<'a>() -> Self::Shadow<'a>;
    fn encoded_len(value: &ViewOf<'_, Self>) -> usize;
    #[doc(hidden)]
    fn encode_raw(value: ViewOf<'_, Self>, buf: &mut impl BufMut);

    #[doc(hidden)]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    #[inline]
    fn post_decode(value: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        value.to_sun()
    }

    #[inline]
    fn with_shadow<R, F>(value: SunOf<'_, Self>, f: F) -> R
    where
        F: FnOnce(ViewOf<'_, Self>) -> R,
    {
        let shadow = Self::Shadow::from_sun(value);
        f(shadow)
    }

    #[inline]
    fn ensure_capacity(buf: &mut impl BufMut, required: usize) -> Result<(), EncodeError> {
        let remaining = buf.remaining_mut();
        if required > remaining { Err(EncodeError::new(required, remaining)) } else { Ok(()) }
    }

    #[inline]
    fn length_delimited_capacity(len: usize) -> usize {
        len + encoded_len_varint(len as u64)
    }

    // -------- Encoding entry points (Sun -> Shadow -> write)
    #[inline]
    fn encode(value: SunOf<'_, Self>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        Self::with_shadow(value, |shadow| {
            let required = Self::encoded_len(&shadow);
            Self::ensure_capacity(buf, required)?;
            Self::encode_raw(shadow, buf);
            Ok(())
        })
    }
    #[inline]
    fn encode_to_vec(value: SunOf<'_, Self>) -> Vec<u8> {
        Self::with_shadow(value, |shadow| {
            let len = Self::encoded_len(&shadow);
            let mut buf = Vec::with_capacity(len);
            Self::encode_raw(shadow, &mut buf);
            buf
        })
    }
    #[inline]
    fn encode_to_array<const N: usize>(value: SunOf<'_, Self>) -> [u8; N] {
        Self::with_shadow(value, |shadow| {
            let len = Self::encoded_len(&shadow);
            debug_assert!(len <= N, "encode_to_array called with insufficient capacity");
            let mut buf = [0; N];
            Self::encode_raw(shadow, &mut buf.as_mut_slice());
            buf
        })
    }

    #[inline]
    fn encode_length_delimited(value: SunOf<'_, Self>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        Self::with_shadow(value, |shadow| {
            let len = Self::encoded_len(&shadow);
            let required = Self::length_delimited_capacity(len);
            Self::ensure_capacity(buf, required)?;

            encode_varint(len as u64, buf);
            Self::encode_raw(shadow, buf);
            Ok(())
        })
    }

    #[inline]
    fn encode_length_delimited_to_vec(value: SunOf<'_, Self>) -> Vec<u8> {
        Self::with_shadow(value, |shadow| {
            let len = Self::encoded_len(&shadow);
            let mut buf = Vec::with_capacity(Self::length_delimited_capacity(len));
            encode_varint(len as u64, &mut buf);
            Self::encode_raw(shadow, &mut buf);
            buf
        })
    }
    #[inline]
    ///N should include encoded_len_varint
    fn encode_length_delimited_to_array<const VAR_INT_LEN: usize>(value: SunOf<'_, Self>) -> [u8; VAR_INT_LEN] {
        Self::with_shadow(value, |shadow| {
            let len = Self::encoded_len(&shadow);
            let required = Self::length_delimited_capacity(len);
            debug_assert!(required <= VAR_INT_LEN, "encode_length_delimited_to_array called with insufficient capacity");
            let mut buf = [0; VAR_INT_LEN];
            let mut slice = buf.as_mut_slice();
            encode_varint(len as u64, &mut slice);
            Self::encode_raw(shadow, &mut slice);
            buf
        })
    }

    #[inline]
    fn decode(mut buf: impl Buf) -> Result<Self, DecodeError> {
        let mut shadow = Self::proto_default();
        Self::merge(&mut shadow, &mut buf)?;
        Self::post_decode(shadow)
    }
    #[inline]
    fn decode_length_delimited(buf: impl Buf) -> Result<Self, DecodeError> {
        let mut shadow = Self::proto_default();
        Self::merge_length_delimited(&mut shadow, buf)?;
        Self::post_decode(shadow)
    }
    #[inline]
    fn merge_length_delimited(value: &mut Self::Shadow<'_>, mut buf: impl Buf) -> Result<(), DecodeError> {
        crate::encoding::message::merge::<Self, _>(WireType::LengthDelimited, value, &mut buf, DecodeContext::default())
    }
    #[inline]
    fn merge(value: &mut Self::Shadow<'_>, mut buf: impl Buf) -> Result<(), DecodeError> {
        let ctx = DecodeContext::default();
        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            Self::merge_field(value, tag, wire_type, &mut buf, ctx)?;
        }
        Ok(())
    }

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
    fn encode_singular_field(tag: u32, value: ViewOf<'_, Self>, buf: &mut impl BufMut);

    /// Merges a single field occurrence into `value`.
    fn merge_singular_field(wire_type: WireType, value: &mut Self::Shadow<'_>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    /// Computes the encoded length for a singular field with the provided tag.
    fn encoded_len_singular_field(tag: u32, value: &ViewOf<'_, Self>) -> usize;
    #[inline]
    /// Encodes an optional field by delegating to [`Self::encode_singular_field`].
    fn encode_option_field(tag: u32, value: Option<ViewOf<'_, Self>>, buf: &mut impl BufMut) {
        if let Some(inner) = value {
            Self::encode_singular_field(tag, inner, buf);
        }
    }
    #[inline]
    /// Decodes an optional field occurrence and stores the result inside
    /// `target`.
    fn merge_option_field(wire_type: WireType, target: &mut Option<Self::Shadow<'_>>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if let Some(value) = target.as_mut() {
            Self::merge_singular_field(wire_type, value, buf, ctx)
        } else {
            let mut value = Self::proto_default();
            Self::merge_singular_field(wire_type, &mut value, buf, ctx)?;
            *target = Some(value);
            Ok(())
        }
    }
    #[inline]
    /// Computes the encoded length for an optional field.
    fn encoded_len_option_field(tag: u32, value: Option<ViewOf<'_, Self>>) -> usize {
        value.as_ref().map_or(0, |inner| Self::encoded_len_singular_field(tag, inner))
    }
}

/// Trait describing how to encode and decode a repeated field of a particular
/// element type. This is used to support nested `Vec<T>` values inside
/// generated structs and enums without requiring ad-hoc implementations for
/// every possible `T`.
pub trait RepeatedField: ProtoExt {
    fn encode_repeated_field<'a, I>(tag: u32, values: I, buf: &mut impl BufMut)
    where
        Self: ProtoExt + 'a,
        I: IntoIterator<Item = ViewOf<'a, Self>>;

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self::Shadow<'_>>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    fn encoded_len_repeated_field<'a, I>(tag: u32, values: I) -> usize
    where
        Self: ProtoExt + 'a,
        I: IntoIterator<Item = ViewOf<'a, Self>>;
}
