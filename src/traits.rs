#![allow(clippy::inline_always)]
#[cfg(feature = "std")]
use std::collections::HashSet;
#[cfg(feature = "std")]
use std::hash::Hash;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::EncodeError;
use crate::alloc::collections::BTreeSet;
use crate::alloc::vec::Vec;
use crate::encoding;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::check_wire_type;
use crate::encoding::decode_key;
use crate::encoding::encode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;

pub trait RepeatedCollection<T>: Sized + FromIterator<T> {
    fn new_reserved(capacity: usize) -> Self;
    fn push(&mut self, value: T);
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        iter.into_iter().collect()
    }

    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for value in iter {
            self.push(value);
        }
    }
}

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

pub enum ProtoKind {
    Primitive(PrimitiveKind),
    SimpleEnum,
    Message,
    Bytes,
    String,
}
pub enum PrimitiveKind {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Fixed32,
    Fixed64,
    SFixed32,
    SFixed64,
    SInt32,
    SInt64,
}

impl ProtoKind {
    #[inline(always)]
    pub const fn is_packable(&self) -> bool {
        matches!(self, ProtoKind::Primitive(_) | ProtoKind::SimpleEnum)
    }
}

pub trait AsEncodeInput<'a, T: ?Sized> {
    fn as_encode_input(&self) -> &T;
}

impl<'a, T: ?Sized> AsEncodeInput<'a, T> for &T {
    #[inline(always)]
    fn as_encode_input(&self) -> &T {
        *self
    }
}

impl<'a, T: ?Sized> AsEncodeInput<'a, T> for &&T {
    #[inline(always)]
    fn as_encode_input(&self) -> &T {
        **self
    }
}

impl ProtoKind {
    #[inline(always)]
    pub const fn wire_type(&self) -> WireType {
        match self {
            ProtoKind::Primitive(p) => match p {
                PrimitiveKind::Bool
                | PrimitiveKind::I8
                | PrimitiveKind::I16
                | PrimitiveKind::I32
                | PrimitiveKind::I64
                | PrimitiveKind::U8
                | PrimitiveKind::U16
                | PrimitiveKind::U32
                | PrimitiveKind::U64
                | PrimitiveKind::SInt32
                | PrimitiveKind::SInt64 => WireType::Varint,

                PrimitiveKind::Fixed32 | PrimitiveKind::SFixed32 | PrimitiveKind::F32 => WireType::ThirtyTwoBit,

                PrimitiveKind::Fixed64 | PrimitiveKind::SFixed64 | PrimitiveKind::F64 => WireType::SixtyFourBit,
            },

            ProtoKind::SimpleEnum => WireType::Varint,
            ProtoKind::Message | ProtoKind::Bytes | ProtoKind::String => WireType::LengthDelimited,
        }
    }
}
/// ---------- atomic, tag-agnostic wire codec ----------
pub trait ProtoWire: Sized {
    type EncodeInput<'b>; //mostly self or &self

    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize;

    #[inline(always)]
    fn encoded_len<'a, X>(value: X) -> usize
    where
        X: AsEncodeInput<'a, Self::EncodeInput<'a>>,
    {
        let v = value.as_encode_input();
        Self::encoded_len_impl(v)
    }

    const KIND: ProtoKind;
    const WIRE_TYPE: WireType = Self::KIND.wire_type();
    /// Encode *this value only* (no field tag).
    fn encode_raw(value: Self::EncodeInput<'_>, buf: &mut impl BufMut);
    #[inline(always)]
    fn encode_with_tag(tag: u32, value: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        encode_key(tag, Self::WIRE_TYPE, buf);
        Self::encode_maybe_length_delimited(value, buf)
    }

    #[inline(always)]
    fn encode_maybe_length_delimited(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        match Self::WIRE_TYPE {
            WireType::LengthDelimited => Self::encode_length_delimited(value, buf),
            _ => Ok(Self::encode_raw(value, buf)),
        }
    }

    #[inline(always)]
    fn encode_length_delimited(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        let len = Self::encoded_len(&value);
        let required = len + encoded_len_varint(len as u64);
        let remaining = buf.remaining_mut();
        if required > remaining {
            return Err(EncodeError::new(required, remaining));
        }
        encode_varint(len as u64, buf);
        Ok(Self::encode_raw(value, buf))
    }

    /// Decode *this value only* (no field tag).
    //fn decode_atomic(buf: &mut impl Buf) -> Result<Self, DecodeError>;
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;
    /// Default-ness used to skip fields.
    fn is_default(&self) -> bool;
    /// default value used for decoding
    fn proto_default() -> Self;
    /// Reset to default.
    fn clear(&mut self);
}

pub trait ProtoExt: Sized {
    /// The shadow is the *actual codec unit*; it must also implement ProtoWire.
    type Shadow<'b>: ProtoShadow<OwnedSun = Self> + ProtoWire<EncodeInput<'b> = ViewOf<'b, Self>>;

    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    #[inline(always)]
    fn with_shadow<R, F>(value: SunOf<'_, Self>, f: F) -> R
    where
        F: FnOnce(ViewOf<'_, Self>) -> R,
    {
        let shadow = Self::Shadow::from_sun(value);
        f(shadow)
    }
    #[inline(always)]
    fn post_decode(value: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        value.to_sun()
    }

    #[inline(always)]
    fn decode(mut buf: impl Buf) -> Result<Self, DecodeError> {
        let mut sh = Self::Shadow::proto_default();
        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            Self::merge_field(&mut sh, tag, wire_type, &mut buf, DecodeContext::default())?;
        }
        Self::post_decode(sh)
    }

    #[inline(always)]
    fn decode_length_delimited(mut buf: impl Buf, ctx: DecodeContext) -> Result<Self, DecodeError> {
        let mut sh = Self::Shadow::proto_default();
        Self::merge_length_delimited(&mut sh, &mut buf, ctx)?;
        Self::post_decode(sh)
    }

    #[inline(always)]
    fn merge_length_delimited<B: Buf>(value: &mut Self::Shadow<'_>, buf: &mut B, ctx: DecodeContext) -> Result<(), DecodeError> {
        ctx.limit_reached()?;
        crate::encoding::merge_loop(value, buf, ctx.enter_recursion(), |msg: &mut Shadow<'_, Self>, buf: &mut B, ctx| {
            let (tag, wire_type) = decode_key(buf)?;
            Self::merge_field(msg, tag, wire_type, buf, ctx)
        })
    }

    #[inline(always)]
    fn encode(value: SunOf<'_, Self>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        Self::with_shadow(value, |shadow| {
            let remaining = buf.remaining_mut();
            let len = Self::Shadow::encoded_len_impl(&shadow);
            if len > remaining {
                return Err(EncodeError::new(len, remaining));
            }
            Ok(<Self::Shadow<'_> as ProtoWire>::encode_raw(shadow, buf))
        })
    }
    //TODO probably should add Result here
    #[inline(always)]
    fn encode_to_vec(value: SunOf<'_, Self>) -> Vec<u8> {
        Self::with_shadow(value, |shadow| {
            let len = Self::Shadow::encoded_len_impl(&shadow);
            let mut buf = Vec::with_capacity(len);
            <Self::Shadow<'_> as ProtoWire>::encode_raw(shadow, &mut buf);
            buf
        })
    }
}

//Example implementation
struct ID {
    id: u64,
}
impl ProtoShadow for ID {
    type Sun<'a> = &'a Self; // borrowed during encoding
    type OwnedSun = Self; // owned form after decoding
    type View<'a> = &'a Self;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value
    }
}
impl ProtoExt for ID {
    type Shadow<'b>
        = ID
    where
        Self: 'b;

    #[inline(always)]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match tag {
            1 => {
                if wire_type != WireType::Varint {
                    return Err(DecodeError::new("invalid wire type for ID.id"));
                }
                value.id = encoding::decode_varint(buf)? as u64;
                Ok(())
            }
            _ => encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }
}

impl ProtoWire for ID {
    type EncodeInput<'b> = &'b Self;
    const KIND: ProtoKind = ProtoKind::Message;
    const WIRE_TYPE: WireType = WireType::LengthDelimited;

    #[inline(always)]
    fn proto_default() -> Self {
        Self { id: ProtoWire::proto_default() }
    }

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.id.is_default()
    }
    #[inline(always)]
    fn clear(&mut self) {
        self.id.clear();
    }

    #[inline(always)]
    /// Returns the encoded length of the message without a length delimiter.
    fn encoded_len_impl(v: &Self::EncodeInput<'_>) -> usize {
        if v.is_default() { 0 } else { encoding::key_len(1) + u64::encoded_len(&v.id) }
    }

    #[inline(always)]
    fn encode_raw(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        // write internal field(s)
        if value.id != 0 {
            encode_key(1, WireType::Varint, buf);
            encode_varint(value.id, buf);
        }
    }

    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        check_wire_type(WireType::Varint, wire_type)?;
        *value = ID::decode_length_delimited(buf, ctx)?;
        Ok(())
    }
}
