#![allow(clippy::inline_always)]
#![allow(clippy::wrong_self_convention)]
use core::marker::PhantomData;

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
use crate::zero_copy::ZeroCopyBuffer;

// ---------- conversion trait users implement ----------
pub trait ProtoShadow<T>: Sized {
    /// Borrowed or owned form used during encoding.
    type Sun<'a>;

    /// The value returned after decoding — can be fully owned
    /// (e.g. `D128`, `String`) or a zero-copy wrapper `ZeroCopyAccess<T>`.
    type OwnedSun: Sized;

    /// The *resulting* shadow type when constructed from a given Sun<'b>, it could be just zero-copy view so we can encode it to buffer
    type View<'a>;

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
            ProtoKind::Repeated(_) => "Repeated",
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

#[track_caller]
#[allow(clippy::extra_unused_type_parameters)]
pub const fn const_test_validate_with_ext<T: ProtoWire>() -> ! {
    const_panic::concat_panic!("Type has validator with ext and it should not be used in infallible rpc methods", T::KIND.dbg_name())
}

pub trait ProtoWire: Sized {
    type EncodeInput<'a>;
    const KIND: ProtoKind;
    const WIRE_TYPE: WireType = Self::KIND.wire_type();

    const _REPEATED_SUPPORT: Option<&'static str> = None;

    const _TEST_REPEATED: () = {
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

    #[inline(always)]
    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
        if Self::is_default_impl(value) { 0 } else { unsafe { Self::encoded_len_impl_raw(value) } }
    }

    #[inline(always)]
    fn encode_with_tag(tag: u32, value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        if Self::is_default_impl(&value) {
            return;
        }
        encode_key(tag, Self::WIRE_TYPE, buf);
        Self::encode_entrypoint(value, buf);
    }

    #[inline(always)]
    fn encode_entrypoint(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        if Self::WIRE_TYPE == WireType::LengthDelimited {
            Self::encode_length_delimited(value, buf);
        } else {
            Self::encode_raw_unchecked(value, buf);
        }
    }

    #[inline(always)]
    fn encode_length_delimited(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        let body_len = unsafe { Self::encoded_len_impl_raw(&value) };
        encode_varint(body_len as u64, buf);
        Self::encode_raw_unchecked(value, buf);
    }

    #[allow(clippy::missing_safety_doc)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize;
    /// Encode *this value only* (no field tag and no default check).
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut);

    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    /// default value used for decoding
    fn proto_default() -> Self;
    /// Reset to default.
    fn clear(&mut self);
}

pub trait EncodeInputFromRef<'a>: ProtoWire {
    fn encode_input_from_ref(value: &'a Self) -> Self::EncodeInput<'a>;
}

trait EncodeInputFromRefValue<'a, T: ?Sized> {
    type Output;
    fn encode_input_from_ref(value: &'a T) -> Self::Output;
}

impl<'a, T> EncodeInputFromRefValue<'a, T> for &'a T {
    type Output = &'a T;

    #[inline(always)]
    fn encode_input_from_ref(value: &'a T) -> Self::Output {
        value
    }
}

impl<'a, T> EncodeInputFromRefValue<'a, T> for T
where
    T: Clone,
{
    type Output = T;

    #[inline(always)]
    fn encode_input_from_ref(value: &'a T) -> Self::Output {
        value.clone()
    }
}

impl<'a, T> EncodeInputFromRef<'a> for T
where
    T: ProtoWire,
    <T as ProtoWire>::EncodeInput<'a>: EncodeInputFromRefValue<'a, T, Output = <T as ProtoWire>::EncodeInput<'a>>,
{
    #[inline(always)]
    fn encode_input_from_ref(value: &'a Self) -> Self::EncodeInput<'a> {
        <<T as ProtoWire>::EncodeInput<'a> as EncodeInputFromRefValue<'a, T>>::encode_input_from_ref(value)
    }
}

 

// Helper alias to shorten signatures:
pub type Shadow<'a, T> = <T as ProtoExt>::Shadow<'a>;
pub type SunOf<'a, T> = <Shadow<'a, T> as ProtoShadow<T>>::Sun<'a>;
pub type OwnedSunOf<'a, T> = <Shadow<'a, T> as ProtoShadow<T>>::OwnedSun;
pub type ViewOf<'a, T> = <Shadow<'a, T> as ProtoShadow<T>>::View<'a>;

pub trait ProtoExt: Sized {
    /// The shadow is the *actual codec unit*; it must also implement ProtoWire.
    type Shadow<'b>: ProtoShadow<Self, OwnedSun = Self> + ProtoWire<EncodeInput<'b> = ViewOf<'b, Self>>;

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
    fn encode(value: SunOf<'_, Self>, mut buf: &mut impl BufMut) -> Result<(), EncodeError> {
        Self::with_shadow(value, |shadow| {
            let len = <Self::Shadow<'_> as ProtoWire>::encoded_len_impl(&shadow);
            if len == 0 {
                return Ok(());
            }
            let remaining = buf.remaining_mut();
            // TODO use std::hint::unlikely when stable
            if matches!(<Self::Shadow<'_> as ProtoWire>::KIND, ProtoKind::SimpleEnum) {
                let total = key_len(1) + len;
                if total > remaining {
                    return Err(EncodeError::new(total, remaining));
                }
                <Self::Shadow<'_> as ProtoWire>::encode_with_tag(1, shadow, &mut buf);
            } else {
                if len > remaining {
                    return Err(EncodeError::new(len, remaining));
                }
                <Self::Shadow<'_> as ProtoWire>::encode_raw_unchecked(shadow, &mut buf);
            }
            Ok(())
        })
    }

    #[inline(always)]
    fn encode_to_vec(value: SunOf<'_, Self>) -> Vec<u8> {
        Self::with_shadow(value, |shadow| {
            let len = <Self::Shadow<'_> as ProtoWire>::encoded_len_impl(&shadow);
            if len == 0 {
                return Vec::new();
            }
            // TODO use std::hint::unlikely when stable
            if matches!(<Self::Shadow<'_> as ProtoWire>::KIND, ProtoKind::SimpleEnum) {
                let total = key_len(1) + len;
                let mut buf = Vec::with_capacity(total);
                <Self::Shadow<'_> as ProtoWire>::encode_with_tag(1, shadow, &mut buf);
                buf
            } else {
                let mut buf = Vec::with_capacity(len);
                <Self::Shadow<'_> as ProtoWire>::encode_raw_unchecked(shadow, &mut buf);
                buf
            }
        })
    }
    #[inline(always)]
    fn encode_to_zerocopy(value: SunOf<'_, Self>) -> ZeroCopyBuffer {
        Self::with_shadow(value, |shadow| {
            let len = <Self::Shadow<'_> as ProtoWire>::encoded_len_impl(&shadow);
            if len == 0 {
                return ZeroCopyBuffer::new();
            }
            // TODO use std::hint::unlikely when stable
            if matches!(<Self::Shadow<'_> as ProtoWire>::KIND, ProtoKind::SimpleEnum) {
                let total = key_len(1) + len;
                let mut buf = ZeroCopyBuffer::with_capacity(total);
                <Self::Shadow<'_> as ProtoWire>::encode_with_tag(1, shadow, buf.inner_mut());
                buf
            } else {
                let mut buf = ZeroCopyBuffer::with_capacity(len);
                <Self::Shadow<'_> as ProtoWire>::encode_raw_unchecked(shadow, buf.inner_mut());
                buf
            }
        })
    }

    const VALIDATE_WITH_EXT: bool = false;

    #[inline(always)]
    fn validate_with_ext(_value: &mut Self, _ext: &tonic::Extensions) -> Result<(), DecodeError> {
        Ok(())
    }
}

//Example implementation with lifetimes and generics
#[expect(dead_code)]
struct ID<'b, K, V> {
    id: u64,
    k: K,
    v: V,
    _pd: PhantomData<&'b ()>,
}

impl<'b, K, V> ProtoShadow<ID<'b, K, V>> for ID<'b, K, V>
where
    K: ProtoShadow<K, OwnedSun = K>,
    V: ProtoShadow<V, OwnedSun = V>,
{
    type Sun<'a> = ID<'a, K, V>;
    type OwnedSun = ID<'b, K, V>;
    type View<'a> = ID<'a, K, V>;

    #[inline(always)]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline(always)]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value
    }
}

impl<'b, K, V> ProtoExt for ID<'b, K, V>
where
    K: ProtoExt + ProtoWire + ProtoShadow<K, OwnedSun = K>,
    V: ProtoExt + ProtoWire + ProtoShadow<V, OwnedSun = V>,
    // IMPORTANT: required so that ID can be its own shadow and its View is ID<'a, K, V>
    for<'a> ID<'b, K, V>: ProtoWire<EncodeInput<'a> = ID<'a, K, V>>,
{
    type Shadow<'a> = ID<'b, K, V>;

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

            // If you have tags for k/v, they should delegate into their shadows:
            // 2 => K::merge_field(&mut value.k, /*...*/, buf, ctx),
            // 3 => V::merge_field(&mut value.v, /*...*/, buf, ctx),
            _ => encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }
}

impl<K, V> ProtoWire for ID<'_, K, V>
where
    K: ProtoWire + ProtoExt + ProtoShadow<K, OwnedSun = K>,
    V: ProtoWire + ProtoExt + ProtoShadow<V, OwnedSun = V>,
    for<'a> K: ProtoWire<EncodeInput<'a> = K>,
    for<'a> V: ProtoWire<EncodeInput<'a> = V>,
{
    type EncodeInput<'a> = ID<'a, K, V>;

    const KIND: ProtoKind = ProtoKind::Message;
    const WIRE_TYPE: WireType = WireType::LengthDelimited;

    #[inline(always)]
    fn proto_default() -> Self {
        Self {
            id: <u64 as ProtoWire>::proto_default(),
            k: K::proto_default(),
            v: V::proto_default(),
            _pd: PhantomData,
        }
    }

    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        <u64 as ProtoWire>::is_default_impl(&value.id) && K::is_default_impl(&value.k) && V::is_default_impl(&value.v)
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.id.clear();
        self.k.clear();
        self.v.clear();
    }

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(v: &Self::EncodeInput<'_>) -> usize {
        let mut len = 0;
        len += <u64 as ProtoWire>::encoded_len_tagged_impl(&v.id, 1);
        len += K::encoded_len_tagged_impl(&v.k, 2);
        len += V::encoded_len_tagged_impl(&v.v, 3);
        len
    }

    #[inline(always)]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        <u64 as ProtoWire>::encode_with_tag(1, value.id, buf);
        K::encode_with_tag(2, value.k, buf);
        V::encode_with_tag(3, value.v, buf);
    }

    #[inline(always)]
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        *value = ID::decode_length_delimited(buf, ctx)?;
        Ok(())
    }
}
