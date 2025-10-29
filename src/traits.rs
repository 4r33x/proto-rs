#![allow(clippy::inline_always)]
#![allow(clippy::wrong_self_convention)]
use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::EncodeError;
use crate::alloc::vec::Vec;
use crate::encoding;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::check_wire_type;
use crate::encoding::decode_key;
use crate::encoding::encode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
use crate::encoding::key_len;

// pub trait RepeatedCollection<T>: Sized {
//     /// The iterator type returned by `.iter()` or `.into_iter()`.
//     ///
//     /// This associated type is parameterized by how `self` is accessed.
//     /// - For `Self` (owned), the iterator yields `T`.
//     /// - For `&Self`, it yields `&T`.
//     /// - For `&mut Self`, it could yield `&mut T` if desired.
//     type Iter<'a>: Iterator<Item = <Self::Item<'a> as std::ops::Deref>::Target>
//     where
//         Self: 'a,
//         T: 'a;

//     /// The item type produced by iteration, parameterized by `self`'s lifetime form.
//     /// Typically:
//     /// - For `Self`, this is `T`
//     /// - For `&Self`, this is `&'a T`
//     /// - For `&mut Self`, this is `&'a mut T`
//     type Item<'a>: std::ops::Deref<Target = T> + 'a
//     where
//         Self: 'a,
//         T: 'a;

//     fn len(&self) -> usize;

//     fn is_empty(&self) -> bool {
//         self.len() == 0
//     }

//     /// Shared iteration (`&self`).
//     fn iter(&self) -> Self::Iter<'_>;

//     /// Consuming iteration (`self`).
//     fn into_iter(self) -> Self::Iter<'static>;
// }

// pub trait RepeatedCollectionMut<T>: RepeatedCollection<T> + FromIterator<T> {
//     fn from_iter<I>(iter: I) -> Self
//     where
//         I: IntoIterator<Item = T>,
//     {
//         iter.into_iter().collect()
//     }
//     fn new_reserved(capacity: usize) -> Self;
//     fn push(&mut self, value: T);
//     fn extend<I>(&mut self, iter: I)
//     where
//         I: IntoIterator<Item = T>,
//     {
//         for value in iter {
//             self.push(value);
//         }
//     }
// }

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

pub enum ProtoKind {
    Primitive(PrimitiveKind),
    SimpleEnum,
    Message,
    Bytes,
    String,
    Repeated(&'static ProtoKind),
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
    pub const fn dbg_name(&'static self) -> &'static str {
        match self {
            ProtoKind::Primitive(_) => "Primitive",
            ProtoKind::SimpleEnum => "SimpleEnum",
            ProtoKind::Message => "Message",
            ProtoKind::Bytes => "Bytes",
            ProtoKind::String => "String",
            ProtoKind::Repeated(v) => "Repeated",
        }
    }
    #[inline(always)]
    pub const fn for_vec(inner: &'static ProtoKind) -> ProtoKind {
        ProtoKind::Repeated(inner)
    }
    #[inline(always)]
    pub const fn is_packable(&self) -> bool {
        matches!(self, ProtoKind::Primitive(_) | ProtoKind::SimpleEnum)
    }
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
            ProtoKind::Repeated(_) | ProtoKind::Message | ProtoKind::Bytes | ProtoKind::String => WireType::LengthDelimited,
        }
    }
}

// pub trait AsEncodeInput<'a, T: ?Sized> {
//     fn as_encode_input(&'a self) -> &'a T;
// }

// impl<'a, T: ?Sized> AsEncodeInput<'a, T> for T {
//     #[inline(always)]
//     fn as_encode_input(&self) -> &'a T {
//         unsafe { &*(self as *const T as *const &'a T) }
//     }
// }

// impl<'a, T: ?Sized> AsEncodeInput<'a, T> for &T {
//     #[inline(always)]
//     fn as_encode_input(&self) -> &T {
//         *self
//     }
// }

// impl<'a, T: ?Sized> AsEncodeInput<'a, T> for &&T {
//     #[inline(always)]
//     fn as_encode_input(&self) -> &T {
//         **self
//     }
// }

#[track_caller]
#[allow(clippy::extra_unused_type_parameters)]
pub const fn const_unreachable<T: ProtoWire>(structure_name: &'static str) -> ! {
    match T::KIND {
        crate::ProtoKind::Primitive(_) | crate::ProtoKind::SimpleEnum | crate::ProtoKind::Message | crate::ProtoKind::Bytes | crate::ProtoKind::String => {
            const_panic::concat_panic!("SHOULD BE SUPPORTED kind: ", T::KIND.dbg_name(), "in", structure_name)
        }
        crate::ProtoKind::Repeated(proto_kind) => {
            const_panic::concat_panic!("unsupported REPEATED kind: ", proto_kind.dbg_name(), "in", structure_name)
        }
    }
}

pub trait ProtoWire: Sized {
    type EncodeInput<'a>;
    const KIND: ProtoKind;
    const WIRE_TYPE: WireType = Self::KIND.wire_type();

    const _REPEATED_SUPPORT: Option<&'static str> = None;
    const _TEST: () = {
        if let Some(name) = Self::_REPEATED_SUPPORT
            && let ProtoKind::Repeated(_) = Self::KIND
        {
            const_unreachable::<Self>(name);
        }
    };

    #[inline(always)]
    fn is_default(&self) -> bool
    where
        for<'b> Self: ProtoWire<EncodeInput<'b> = &'b Self>,
    {
        Self::is_default_impl(&self)
    }

    #[inline(always)]
    fn is_default_by_val(self) -> bool
    where
        for<'b> Self: ProtoWire<EncodeInput<'b> = Self>,
    {
        Self::is_default_impl(&self)
    }
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool;

    #[inline(always)]
    fn encoded_len(&self) -> usize
    where
        for<'b> Self: ProtoWire<EncodeInput<'b> = &'b Self>,
    {
        Self::encoded_len_impl(&self)
    }

    #[inline(always)]
    fn encoded_len_by_val(self) -> usize
    where
        for<'b> Self: ProtoWire<EncodeInput<'b> = Self>,
    {
        Self::encoded_len_impl(&self)
    }

    #[inline(always)]
    fn encoded_len_tagged(&self, tag: u32) -> usize
    where
        for<'b> Self: ProtoWire<EncodeInput<'b> = &'b Self>,
    {
        let len = Self::encoded_len(self);
        if len != 0 {
            if Self::WIRE_TYPE == WireType::LengthDelimited {
                key_len(tag) + encoded_len_varint(len as u64) + len
            } else {
                key_len(tag) + len
            }
        } else {
            0
        }
    }
    #[inline(always)]
    fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
        let len = Self::encoded_len_impl(value);
        if len != 0 {
            if Self::WIRE_TYPE == WireType::LengthDelimited {
                key_len(tag) + encoded_len_varint(len as u64) + len
            } else {
                key_len(tag) + len
            }
        } else {
            0
        }
    }
    #[allow(clippy::missing_safety_doc)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize;

    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
        if Self::is_default_impl(value) {
            0
        } else {
            unsafe { Self::encoded_len_impl_raw(value) }
            // match Self::WIRE_TYPE {
            //     WireType::LengthDelimited => {
            //         let len = unsafe { Self::encoded_len_impl_raw(value) };
            //         len + encoded_len_varint(len as u64)
            //     }
            //     _ => unsafe { Self::encoded_len_impl_raw(value) },
            // }
        }
    }

    /// Encode *this value only* (no field tag and no default check).
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut);

    #[inline(always)]
    fn encode_with_tag(tag: u32, value: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        if Self::is_default_impl(&value) {
            Ok(())
        } else {
            encode_key(tag, Self::WIRE_TYPE, buf);
            Self::encode_entrypoint(value, buf)
        }
    }

    #[inline(always)]
    fn encode_entrypoint(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        if Self::WIRE_TYPE == WireType::LengthDelimited {
            Self::encode_length_delimited(value, buf)
        } else {
            Self::encode_raw_unchecked(value, buf);
            Ok(())
        }
    }

    #[inline(always)]
    fn encode_length_delimited(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        if Self::is_default_impl(&value) {
            return Ok(());
        }

        let body_len = unsafe { Self::encoded_len_impl_raw(&value) };
        let required = body_len + encoded_len_varint(body_len as u64);
        let remaining = buf.remaining_mut();
        if required > remaining {
            return Err(EncodeError::new(required, remaining));
        }
        encode_varint(body_len as u64, buf);
        Self::encode_raw_unchecked(value, buf);
        Ok(())
    }

    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    /// default value used for decoding
    fn proto_default() -> Self;
    /// Reset to default.
    fn clear(&mut self);
}

// Helper alias to shorten signatures:
pub type Shadow<'a, T> = <T as ProtoExt>::Shadow<'a>;
pub type SunOf<'a, T> = <Shadow<'a, T> as ProtoShadow>::Sun<'a>;
pub type OwnedSunOf<'a, T> = <Shadow<'a, T> as ProtoShadow>::OwnedSun;
pub type ViewOf<'a, T> = <Shadow<'a, T> as ProtoShadow>::View<'a>;
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
            match <Self::Shadow<'_> as ProtoWire>::WIRE_TYPE {
                WireType::LengthDelimited => {
                    let len = unsafe { <Self::Shadow<'_> as ProtoWire>::encoded_len_impl_raw(&shadow) };
                    let remaining = buf.remaining_mut();
                    if len > remaining {
                        return Err(EncodeError::new(len, remaining));
                    }
                    <Self::Shadow<'_> as ProtoWire>::encode_raw_unchecked(shadow, buf);
                    Ok(())
                }
                _ => {
                    let remaining = buf.remaining_mut();
                    let len = <Self::Shadow<'_> as ProtoWire>::encoded_len_impl(&shadow);
                    if len > remaining {
                        return Err(EncodeError::new(len, remaining));
                    }
                    <Self::Shadow<'_> as ProtoWire>::encode_entrypoint(shadow, buf)
                }
            }
        })
    }
    //TODO probably should add Result here
    #[inline(always)]
    fn encode_to_vec(value: SunOf<'_, Self>) -> Vec<u8> {
        Self::with_shadow(value, |shadow| {
            match <Self::Shadow<'_> as ProtoWire>::WIRE_TYPE {
                WireType::LengthDelimited => {
                    let len = unsafe { <Self::Shadow<'_> as ProtoWire>::encoded_len_impl_raw(&shadow) };
                    let mut buf = Vec::with_capacity(len);
                    <Self::Shadow<'_> as ProtoWire>::encode_raw_unchecked(shadow, &mut buf);
                    buf
                }
                _ => {
                    let len = <Self::Shadow<'_> as ProtoWire>::encoded_len_impl(&shadow);
                    let mut buf = Vec::with_capacity(len);
                    let _ = <Self::Shadow<'_> as ProtoWire>::encode_entrypoint(shadow, &mut buf);
                    buf
                }
            }
        })
    }
}

//Example implementation
#[expect(dead_code)]
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
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        value.id.is_default_by_val()
    }
    #[inline(always)]
    fn clear(&mut self) {
        self.id.clear();
    }

    #[inline(always)]
    /// Returns the encoded length of the message without a length delimiter.
    unsafe fn encoded_len_impl_raw(v: &Self::EncodeInput<'_>) -> usize {
        encoding::key_len(1) + u64::encoded_len_impl(&v.id)
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        // write internal field(s)
        encode_key(1, WireType::Varint, buf);
        encode_varint(value.id, buf);
    }
    #[inline(always)]
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        check_wire_type(WireType::Varint, wire_type)?;
        *value = ID::decode_length_delimited(buf, ctx)?;
        Ok(())
    }
}
