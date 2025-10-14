use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::EncodeError;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;

// ---------- conversion trait users implement ----------
pub trait ProtoShadow<'a>: Sized {
    /// Borrowed or owned form used during encoding.
    type Sun<'b>: 'b;

    /// The value returned after decoding — can be fully owned
    /// (e.g. `D128`, `String`) or a zero-copy wrapper that still
    /// implements `ZeroCopyAccess<T>`.
    type OwnedSun: Sized;

    /// The *resulting* shadow type when constructed from a given Sun<'b>.
    ///
    /// Example:
    /// - If Sun<'b> = &'b Self::OwnedSun → ResultShadow<'b> = &'b Self
    /// - If Sun<'b> = Self::OwnedSun → ResultShadow<'b> = Self
    type View<'b>: 'b;

    /// Convert this shadow into whatever "owned" representation we chose.
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError>;

    /// Build a shadow from an existing Sun (borrowed or owned).
    fn from_sun(value: Self::Sun<'a>) -> Self::View<'a>;

    fn proto_default() -> Self;
    fn encoded_len(value: &Self::View<'_>) -> usize;
}

// Helper alias to shorten signatures:
pub type Shadow<'a, T> = <T as ProtoExt>::Shadow<'a>;
pub type SunOf<'a, T> = <Shadow<'a, T> as ProtoShadow<'a>>::Sun<'a>;
pub type OwnedSunOf<'a, T> = <Shadow<'a, T> as ProtoShadow<'a>>::OwnedSun;
pub type ViewOf<'a, T> = <Shadow<'a, T> as ProtoShadow<'a>>::View<'a>;

pub trait ProtoExt: Sized {
    type Shadow<'a>: ProtoShadow<'a, OwnedSun = Self>
    where
        Self: 'a;

    #[doc(hidden)]
    fn encode_raw<'a>(value: ViewOf<'a, Self>, buf: &mut impl BufMut);

    #[doc(hidden)]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    fn post_decode(value: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        value.to_sun()
    }

    // -------- Encoding entry points (Sun -> Shadow -> write)

    fn encode<'a>(value: SunOf<'a, Self>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        let shadow = Self::Shadow::from_sun(value);

        let required = Self::Shadow::encoded_len(&shadow);
        let remaining = buf.remaining_mut();
        if required > remaining {
            return Err(EncodeError::new(required, remaining));
        }
        Self::encode_raw(shadow, buf);
        Ok(())
    }

    fn encode_to_vec<'a>(value: SunOf<'a, Self>) -> Vec<u8> {
        let shadow = Self::Shadow::from_sun(value);
        let len = Self::Shadow::encoded_len(&shadow);
        let mut buf = Vec::with_capacity(len);
        Self::encode_raw(shadow, &mut buf);
        buf
    }
    fn encode_to_array<'a, const N: usize>(value: SunOf<'a, Self>) -> [u8; N] {
        let shadow = Self::Shadow::from_sun(value);
        let mut buf = [0; N];
        Self::encode_raw(shadow, &mut buf.as_mut_slice());
        buf
    }

    fn encode_length_delimited<'a>(value: SunOf<'a, Self>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        let shadow = Self::Shadow::from_sun(value);
        let len = Self::Shadow::encoded_len(&shadow);
        let required = len + encoded_len_varint(len as u64);
        let remaining = buf.remaining_mut();
        if required > remaining {
            return Err(EncodeError::new(required, remaining));
        }
        encode_varint(len as u64, buf);
        Self::encode_raw(shadow, buf);
        Ok(())
    }

    fn encode_length_delimited_to_vec<'a>(value: SunOf<'a, Self>) -> Vec<u8> {
        let shadow = Self::Shadow::from_sun(value);
        let len = Self::Shadow::encoded_len(&shadow);
        let mut buf = Vec::with_capacity(len + encoded_len_varint(len as u64));
        encode_varint(len as u64, &mut buf);
        Self::encode_raw(shadow, &mut buf);
        buf
    }
    //N should include encoded_len_varint
    fn encode_length_delimited_to_array<'a, const VAR_INT_LEN: usize>(value: SunOf<'a, Self>) -> [u8; VAR_INT_LEN] {
        let shadow = Self::Shadow::from_sun(value);
        let len = Self::Shadow::encoded_len(&shadow);
        let mut buf = [0; VAR_INT_LEN];
        encode_varint(len as u64, &mut buf.as_mut_slice());
        Self::encode_raw(shadow, &mut buf.as_mut_slice());
        buf
    }

    // -------- Decoding (read -> Shadow -> post_decode -> Self)

    fn decode(mut buf: impl Buf) -> Result<Self, DecodeError> {
        let mut shadow = Self::Shadow::proto_default();
        Self::merge(&mut shadow, &mut buf)?;
        Self::post_decode(shadow)
    }

    fn decode_length_delimited(buf: impl Buf) -> Result<Self, DecodeError> {
        let mut shadow = Self::Shadow::proto_default();
        Self::merge_length_delimited(&mut shadow, buf)?;
        Self::post_decode(shadow)
    }

    fn merge_length_delimited(value: &mut Self::Shadow<'_>, mut buf: impl Buf) -> Result<(), DecodeError> {
        crate::encoding::message::merge::<Self, _>(WireType::LengthDelimited, value, &mut buf, DecodeContext::default())
    }

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

    /// Encodes an optional field by delegating to [`Self::encode_singular_field`].
    fn encode_option_field(tag: u32, value: Option<ViewOf<'_, Self>>, buf: &mut impl BufMut) {
        if let Some(inner) = value {
            Self::encode_singular_field(tag, inner, buf);
        }
    }

    /// Decodes an optional field occurrence and stores the result inside
    /// `target`.
    fn merge_option_field(wire_type: WireType, target: &mut Option<Self::Shadow<'_>>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if let Some(value) = target.as_mut() {
            Self::merge_singular_field(wire_type, value, buf, ctx)
        } else {
            let mut value = Self::Shadow::proto_default();
            Self::merge_singular_field(wire_type, &mut value, buf, ctx)?;
            *target = Some(value);
            Ok(())
        }
    }

    /// Computes the encoded length for an optional field.
    fn encoded_len_option_field(tag: u32, value: Option<ViewOf<'_, Self>>) -> usize {
        value.as_ref().map_or(0, |inner| Self::encoded_len_singular_field(tag, inner))
    }
}

/// Trait describing how to encode and decode a repeated field of a particular
/// element type. This is used to support nested `Vec<T>` values inside
/// generated structs and enums without requiring ad-hoc implementations for
/// every possible `T`.
pub trait RepeatedField: ProtoExt + Sized {
    /// Encodes `values` as a repeated field with the provided tag.
    fn encode_repeated_field(tag: u32, values: &[ViewOf<'_, Self>], buf: &mut impl BufMut);

    /// Merges repeated field occurrences into `values`.
    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Shadow<'_, Self>>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    /// Returns the encoded length of a repeated field with the provided tag.
    fn encoded_len_repeated_field(tag: u32, values: &[ViewOf<'_, Self>]) -> usize;
}
