//! Protocol Buffers well-known wrapper types.
//!
//! This module provides implementations of the new proto traits for Rust standard library types which
//! correspond to a Protobuf well-known wrapper type.

use alloc::format;
use alloc::string::String;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicI8;
use core::sync::atomic::AtomicI16;
use core::sync::atomic::AtomicI32;
use core::sync::atomic::AtomicI64;
use core::sync::atomic::AtomicIsize;
use core::sync::atomic::AtomicU8;
use core::sync::atomic::AtomicU16;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use ::bytes::Buf;
use ::bytes::Bytes;

use crate::DecodeError;
use crate::Name;
use crate::ProtoDecoder;
use crate::ProtoDefault;
use crate::ProtoEncode;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::check_wire_type;
use crate::encoding::skip_field;
use crate::traits::ArchivedProtoField;
use crate::traits::PrimitiveKind;
use crate::traits::ProtoArchive;
use crate::traits::ProtoDecode;
use crate::traits::ProtoExt;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadowDecode;
use crate::traits::ProtoShadowEncode;
use crate::traits::buffer::RevWriter;
// ============================================================================
// Macro for by-value primitives (bool, i32, i64, u32, u64, f32, f64)
// ============================================================================

macro_rules! impl_proto_primitive_varint_by_value {
    ($ty:ty, $module:ident, $name:literal, $kind:expr, $default:expr, to_u64($v:ident) $to_u64:expr) => {
        impl ProtoExt for $ty {
            const KIND: ProtoKind = $kind;
        }

        impl ProtoShadowDecode<$ty> for $ty {
            #[inline(always)]
            fn to_sun(self) -> Result<$ty, DecodeError> {
                Ok(self)
            }
        }

        impl<'a> ProtoShadowEncode<'a, $ty> for $ty {
            #[inline(always)]
            fn from_sun(value: &'a $ty) -> Self {
                *value
            }
        }

        impl ProtoDecoder for $ty {
            #[inline(always)]
            fn merge_field(
                value: &mut Self,
                tag: u32,
                wire_type: WireType,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                if tag == 1 {
                    crate::encoding::$module::merge(wire_type, value, buf, ctx)
                } else {
                    skip_field(wire_type, tag, buf, ctx)
                }
            }

            #[inline(always)]
            fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
                crate::encoding::$module::merge(wire_type, self, buf, ctx)
            }
        }

        impl ProtoDefault for $ty {
            #[inline(always)]
            fn proto_default() -> Self {
                $default
            }
        }

        impl ProtoDecode for $ty {
            type ShadowDecoded = Self;
        }

        impl ProtoArchive for $ty {
            #[inline(always)]
            fn is_default(&self) -> bool {
                *self == $default
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
                let $v: $ty = *self;
                let value = $to_u64;
                w.put_varint(value);
                if TAG != 0 {
                    ArchivedProtoField::<TAG, Self>::put_key(w);
                }
            }
        }

        impl ProtoEncode for $ty {
            type Shadow<'a> = $ty;
        }

        impl Name for $ty {
            const NAME: &'static str = $name;
            const PACKAGE: &'static str = "google.protobuf";
            fn type_url() -> String {
                format!("type.googleapis.com/{}.{}", Self::PACKAGE, Self::NAME)
            }
        }
    };
}

macro_rules! impl_proto_primitive_fixed_by_value {
    ($ty:ty, $module:ident, $name:literal, $kind:expr, $default:expr, to_bits($v:ident) $to_bits:expr, $put:ident) => {
        impl ProtoExt for $ty {
            const KIND: ProtoKind = $kind;
        }

        impl ProtoShadowDecode<$ty> for $ty {
            #[inline(always)]
            fn to_sun(self) -> Result<$ty, DecodeError> {
                Ok(self)
            }
        }

        impl<'a> ProtoShadowEncode<'a, $ty> for $ty {
            #[inline(always)]
            fn from_sun(value: &'a $ty) -> Self {
                *value
            }
        }

        impl ProtoDecoder for $ty {
            #[inline(always)]
            fn merge_field(
                value: &mut Self,
                tag: u32,
                wire_type: WireType,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                if tag == 1 {
                    crate::encoding::$module::merge(wire_type, value, buf, ctx)
                } else {
                    skip_field(wire_type, tag, buf, ctx)
                }
            }

            #[inline(always)]
            fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
                crate::encoding::$module::merge(wire_type, self, buf, ctx)
            }
        }

        impl ProtoDefault for $ty {
            #[inline(always)]
            fn proto_default() -> Self {
                $default
            }
        }

        impl ProtoDecode for $ty {
            type ShadowDecoded = Self;
        }

        impl ProtoArchive for $ty {
            #[inline(always)]
            fn is_default(&self) -> bool {
                *self == $default
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
                let $v: $ty = *self;
                let bits = $to_bits;
                w.$put(bits);
                if TAG != 0 {
                    ArchivedProtoField::<TAG, Self>::put_key(w);
                }
            }
        }

        impl ProtoEncode for $ty {
            type Shadow<'a> = $ty;
        }

        impl Name for $ty {
            const NAME: &'static str = $name;
            const PACKAGE: &'static str = "google.protobuf";
            fn type_url() -> String {
                format!("type.googleapis.com/{}.{}", Self::PACKAGE, Self::NAME)
            }
        }
    };
}

// ============================================================================
// Macro for by-ref primitives (String, Vec<u8>, Bytes)
// ============================================================================

macro_rules! impl_proto_primitive_by_ref {
    ($ty:ty, $module:ident, $name:literal, $kind:expr) => {
        impl ProtoExt for $ty {
            const KIND: ProtoKind = $kind;
        }

        impl ProtoShadowDecode<$ty> for $ty {
            #[inline(always)]
            fn to_sun(self) -> Result<$ty, DecodeError> {
                Ok(self)
            }
        }

        impl<'a> ProtoShadowEncode<'a, $ty> for &'a $ty {
            #[inline(always)]
            fn from_sun(value: &'a $ty) -> Self {
                &value
            }
        }

        impl ProtoDecoder for $ty {
            #[inline(always)]
            fn merge_field(
                value: &mut Self,
                tag: u32,
                wire_type: WireType,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                if tag == 1 {
                    crate::encoding::$module::merge(wire_type, value, buf, ctx)
                } else {
                    skip_field(wire_type, tag, buf, ctx)
                }
            }

            #[inline(always)]
            fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
                crate::encoding::$module::merge(wire_type, self, buf, ctx)
            }
        }

        impl ProtoDefault for $ty {
            #[inline(always)]
            fn proto_default() -> Self {
                Default::default()
            }
        }

        impl ProtoDecode for $ty {
            type ShadowDecoded = Self;
        }

        impl<'a> ProtoArchive for &'a $ty {
            #[inline(always)]
            fn is_default(&self) -> bool {
                (*self).is_empty()
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
                let bytes = self.as_ref();
                w.put_slice(bytes);
                if TAG != 0 {
                    w.put_varint(bytes.len() as u64);
                    ArchivedProtoField::<TAG, Self>::put_key(w);
                }
            }
        }

        impl ProtoEncode for $ty {
            type Shadow<'a> = &'a $ty;
        }

        impl Name for $ty {
            const NAME: &'static str = $name;
            const PACKAGE: &'static str = "google.protobuf";
            fn type_url() -> String {
                format!("type.googleapis.com/{}.{}", Self::PACKAGE, Self::NAME)
            }
        }
    };
}

// ============================================================================
// Implement by-value primitives
// ============================================================================

impl_proto_primitive_varint_by_value!(
    bool,
    bool,
    "BoolValue",
    ProtoKind::Primitive(PrimitiveKind::Bool),
    false,
    to_u64(value) { value as u64 }
);

impl_proto_primitive_varint_by_value!(
    u32,
    uint32,
    "UInt32Value",
    ProtoKind::Primitive(PrimitiveKind::U32),
    0,
    to_u64(value) { value as u64 }
);

impl_proto_primitive_varint_by_value!(
    u64,
    uint64,
    "UInt64Value",
    ProtoKind::Primitive(PrimitiveKind::U64),
    0,
    to_u64(value) { value as u64 }
);

impl_proto_primitive_varint_by_value!(
    i32,
    int32,
    "Int32Value",
    ProtoKind::Primitive(PrimitiveKind::I32),
    0,
    to_u64(value) { value as u64 }
);

impl_proto_primitive_varint_by_value!(
    i64,
    int64,
    "Int64Value",
    ProtoKind::Primitive(PrimitiveKind::I64),
    0,
    to_u64(value) { value as u64 }
);

impl_proto_primitive_fixed_by_value!(
    f32,
    float,
    "FloatValue",
    ProtoKind::Primitive(PrimitiveKind::F32),
    0.0,
    to_bits(value) { value.to_bits() },
    put_fixed32
);

impl_proto_primitive_fixed_by_value!(
    f64,
    double,
    "DoubleValue",
    ProtoKind::Primitive(PrimitiveKind::F64),
    0.0,
    to_bits(value) { value.to_bits() },
    put_fixed64
);

// ============================================================================
// Implement by-ref primitives
// ============================================================================

impl_proto_primitive_by_ref!(String, string, "StringValue", ProtoKind::String);

impl_proto_primitive_by_ref!(Bytes, bytes, "BytesValue", ProtoKind::Bytes);

impl ProtoArchive for String {
    #[inline(always)]
    fn is_default(&self) -> bool {
        <&String as ProtoArchive>::is_default(&self)
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        (&self).archive::<TAG>(w);
    }
}

impl ProtoArchive for Bytes {
    #[inline(always)]
    fn is_default(&self) -> bool {
        <&Bytes as ProtoArchive>::is_default(&self)
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
        (&self).archive::<TAG>(w);
    }
}

// ============================================================================
// Narrow primitives (u8, u16, i8, i16)
// ============================================================================

macro_rules! impl_narrow_varint {
    ($ty:ty, $wide_ty:ty, $prim_kind:ident, $err:literal) => {
        impl ProtoExt for $ty {
            const KIND: ProtoKind = ProtoKind::Primitive(PrimitiveKind::$prim_kind);
        }

        impl ProtoShadowDecode<$ty> for $ty {
            #[inline(always)]
            fn to_sun(self) -> Result<$ty, DecodeError> {
                Ok(self)
            }
        }

        impl<'a> ProtoShadowEncode<'a, $ty> for $ty {
            #[inline(always)]
            fn from_sun(value: &'a $ty) -> Self {
                *value
            }
        }

        impl ProtoDecoder for $ty {
            #[inline(always)]
            fn merge_field(
                value: &mut Self,
                tag: u32,
                wire_type: WireType,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                if tag == 1 {
                    Self::merge(value, wire_type, buf, ctx)
                } else {
                    skip_field(wire_type, tag, buf, ctx)
                }
            }

            #[inline(always)]
            fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, _ctx: DecodeContext) -> Result<(), DecodeError> {
                check_wire_type(WireType::Varint, wire_type)?;
                let widened: $wide_ty = crate::encoding::decode_varint(buf)? as $wide_ty;
                *self = widened.try_into().map_err(|_| DecodeError::new($err))?;
                Ok(())
            }
        }

        impl ProtoDefault for $ty {
            #[inline(always)]
            fn proto_default() -> Self {
                0
            }
        }

        impl ProtoDecode for $ty {
            type ShadowDecoded = Self;
        }

        impl ProtoArchive for $ty {
            #[inline(always)]
            fn is_default(&self) -> bool {
                *self == 0
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
                let widened: $wide_ty = *self as $wide_ty;
                w.put_varint(widened as u64);
                if TAG != 0 {
                    ArchivedProtoField::<TAG, Self>::put_key(w);
                }
            }
        }

        impl ProtoEncode for $ty {
            type Shadow<'a> = $ty;
        }
    };
}

impl_narrow_varint!(u8, u32, U8, "u8 overflow");
impl_narrow_varint!(u16, u32, U16, "u16 overflow");
impl_narrow_varint!(i8, i32, I8, "i8 overflow");
impl_narrow_varint!(i16, i32, I16, "i16 overflow");

// ============================================================================
// Atomic primitives
// ============================================================================

macro_rules! impl_atomic_primitive {
    ($ty:ty, $prim:expr, $default:expr, $base:ty, $module:ident,
        load = $load:expr,
        store = $store:expr
    ) => {
        impl ProtoExt for $ty {
            const KIND: ProtoKind = ProtoKind::Primitive($prim);
        }

        impl ProtoShadowDecode<$ty> for $ty {
            #[inline(always)]
            fn to_sun(self) -> Result<$ty, DecodeError> {
                Ok(self)
            }
        }

        impl<'a> ProtoShadowEncode<'a, $ty> for &'a $ty {
            #[inline(always)]
            fn from_sun(value: &'a $ty) -> Self {
                &value
            }
        }

        impl ProtoDecoder for $ty {
            #[inline(always)]
            fn merge_field(
                value: &mut Self,
                tag: u32,
                wire_type: WireType,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                if tag == 1 {
                    Self::merge(value, wire_type, buf, ctx)
                } else {
                    skip_field(wire_type, tag, buf, ctx)
                }
            }

            #[inline(always)]
            fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
                let mut raw: $base = ($load)(&*self);
                crate::encoding::$module::merge(wire_type, &mut raw, buf, ctx)?;
                ($store)(self, raw);
                Ok(())
            }
        }

        impl ProtoDefault for $ty {
            #[inline(always)]
            fn proto_default() -> Self {
                Self::new($default)
            }
        }

        impl ProtoDecode for $ty {
            type ShadowDecoded = Self;
        }

        impl ProtoArchive for $ty {
            #[inline(always)]
            fn is_default(&self) -> bool {
                ($load)(self) == $default
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
                let value: $base = ($load)(self);
                value.archive::<TAG>(w);
            }
        }

        impl ProtoEncode for $ty {
            type Shadow<'a> = &'a $ty;
        }

        impl<'a> ProtoArchive for &'a $ty {
            #[inline(always)]
            fn is_default(&self) -> bool {
                ($load)(*self) == $default
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
                let value: $base = ($load)(*self);
                value.archive::<TAG>(w);
            }
        }
    };
}

macro_rules! impl_atomic_narrow_primitive {
    (
        $ty:ty,
        $prim_kind:ident,
        $default:expr,
        narrow = $narrow:ty,
        wide = $wide:ty,
        module = $module:ident,
        load = $load:expr,
        store = $store:expr
    ) => {
        impl ProtoExt for $ty {
            const KIND: ProtoKind = ProtoKind::Primitive(PrimitiveKind::$prim_kind);
        }

        impl ProtoShadowDecode<$ty> for $ty {
            #[inline(always)]
            fn to_sun(self) -> Result<$ty, DecodeError> {
                Ok(self)
            }
        }

        impl<'a> ProtoShadowEncode<'a, $ty> for &'a $ty {
            #[inline(always)]
            fn from_sun(value: &'a $ty) -> Self {
                &value
            }
        }

        impl ProtoDecoder for $ty {
            #[inline(always)]
            fn merge_field(
                value: &mut Self,
                tag: u32,
                wire_type: WireType,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                if tag == 1 {
                    Self::merge(value, wire_type, buf, ctx)
                } else {
                    skip_field(wire_type, tag, buf, ctx)
                }
            }

            #[inline(always)]
            fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
                let mut raw: $wide = ($load)(&*self) as $wide;
                crate::encoding::$module::merge(wire_type, &mut raw, buf, ctx)?;
                let narrowed: $narrow =
                    <$narrow>::try_from(raw).map_err(|_| DecodeError::new(concat!(stringify!($narrow), " overflow")))?;
                ($store)(self, narrowed);
                Ok(())
            }
        }

        impl ProtoDefault for $ty {
            #[inline(always)]
            fn proto_default() -> Self {
                Self::new($default)
            }
        }

        impl ProtoDecode for $ty {
            type ShadowDecoded = Self;
        }

        impl ProtoArchive for $ty {
            #[inline(always)]
            fn is_default(&self) -> bool {
                ($load)(self) == $default
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
                let value: $wide = ($load)(self) as $wide;
                w.put_varint(value as u64);
                if TAG != 0 {
                    ArchivedProtoField::<TAG, Self>::put_key(w);
                }
            }
        }

        impl ProtoEncode for $ty {
            type Shadow<'a> = &'a $ty;
        }

        impl<'a> ProtoArchive for &'a $ty {
            #[inline(always)]
            fn is_default(&self) -> bool {
                ($load)(*self) == $default
            }

            #[inline(always)]
            fn archive<const TAG: u32>(&self, w: &mut impl RevWriter) {
                let value: $wide = ($load)(*self) as $wide;
                w.put_varint(value as u64);
                if TAG != 0 {
                    ArchivedProtoField::<TAG, Self>::put_key(w);
                }
            }
        }
    };
}

impl_atomic_primitive!(
    AtomicBool,
    PrimitiveKind::Bool,
    false,
    bool,
    bool,
    load = |value: &AtomicBool| value.load(Ordering::Relaxed),
    store = |value: &AtomicBool, raw: bool| value.store(raw, Ordering::Relaxed)
);

impl_atomic_primitive!(
    AtomicI32,
    PrimitiveKind::I32,
    0i32,
    i32,
    int32,
    load = |value: &AtomicI32| value.load(Ordering::Relaxed),
    store = |value: &AtomicI32, raw: i32| value.store(raw, Ordering::Relaxed)
);

impl_atomic_primitive!(
    AtomicI64,
    PrimitiveKind::I64,
    0i64,
    i64,
    int64,
    load = |value: &AtomicI64| value.load(Ordering::Relaxed),
    store = |value: &AtomicI64, raw: i64| value.store(raw, Ordering::Relaxed)
);

impl_atomic_primitive!(
    AtomicU32,
    PrimitiveKind::U32,
    0u32,
    u32,
    uint32,
    load = |value: &AtomicU32| value.load(Ordering::Relaxed),
    store = |value: &AtomicU32, raw: u32| value.store(raw, Ordering::Relaxed)
);

impl_atomic_primitive!(
    AtomicU64,
    PrimitiveKind::U64,
    0u64,
    u64,
    uint64,
    load = |value: &AtomicU64| value.load(Ordering::Relaxed),
    store = |value: &AtomicU64, raw: u64| value.store(raw, Ordering::Relaxed)
);

impl_atomic_narrow_primitive!(
    AtomicI8,
    I8,
    0i8,
    narrow = i8,
    wide = i32,
    module = int32,
    load = |value: &AtomicI8| value.load(Ordering::Relaxed),
    store = |value: &AtomicI8, raw: i8| value.store(raw, Ordering::Relaxed)
);

impl_atomic_narrow_primitive!(
    AtomicI16,
    I16,
    0i16,
    narrow = i16,
    wide = i32,
    module = int32,
    load = |value: &AtomicI16| value.load(Ordering::Relaxed),
    store = |value: &AtomicI16, raw: i16| value.store(raw, Ordering::Relaxed)
);

impl_atomic_narrow_primitive!(
    AtomicU8,
    AtomicU8,
    0u8,
    narrow = u8,
    wide = u32,
    module = uint32,
    load = |value: &AtomicU8| value.load(Ordering::Relaxed),
    store = |value: &AtomicU8, raw: u8| value.store(raw, Ordering::Relaxed)
);

impl_atomic_narrow_primitive!(
    AtomicU16,
    U16,
    0u16,
    narrow = u16,
    wide = u32,
    module = uint32,
    load = |value: &AtomicU16| value.load(Ordering::Relaxed),
    store = |value: &AtomicU16, raw: u16| value.store(raw, Ordering::Relaxed)
);

impl_atomic_narrow_primitive!(
    AtomicIsize,
    I64,
    0isize,
    narrow = isize,
    wide = i64,
    module = int64,
    load = |value: &AtomicIsize| value.load(Ordering::Relaxed),
    store = |value: &AtomicIsize, raw: isize| value.store(raw, Ordering::Relaxed)
);

impl_atomic_narrow_primitive!(
    AtomicUsize,
    U64,
    0usize,
    narrow = usize,
    wide = u64,
    module = uint64,
    load = |value: &AtomicUsize| value.load(Ordering::Relaxed),
    store = |value: &AtomicUsize, raw: usize| value.store(raw, Ordering::Relaxed)
);

// ============================================================================
// Unit type ()
// ============================================================================

impl ProtoExt for () {
    const KIND: ProtoKind = ProtoKind::Message;
}

impl ProtoShadowDecode<()> for () {
    #[inline(always)]
    fn to_sun(self) -> Result<(), DecodeError> {
        Ok(())
    }
}

impl<'a> ProtoShadowEncode<'a, ()> for () {
    #[inline(always)]
    fn from_sun(_value: &'a ()) -> Self {}
}

impl ProtoDecoder for () {
    #[inline(always)]
    fn merge_field(_value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        skip_field(wire_type, tag, buf, ctx)
    }
}

impl ProtoDefault for () {
    #[inline(always)]
    fn proto_default() -> Self {}
}

impl ProtoDecode for () {
    type ShadowDecoded = Self;
}

impl ProtoArchive for () {
    #[inline(always)]
    fn is_default(&self) -> bool {
        true
    }

    #[inline(always)]
    fn archive<const TAG: u32>(&self, _w: &mut impl RevWriter) {}
}

impl ProtoEncode for () {
    type Shadow<'a> = ();
}

impl Name for () {
    const NAME: &'static str = "Empty";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        format!("type.googleapis.com/{}.{}", Self::PACKAGE, Self::NAME)
    }
}

impl Name for Vec<u8> {
    const NAME: &'static str = "BytesValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        format!("type.googleapis.com/{}.{}", Self::PACKAGE, Self::NAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impl_name() {
        assert_eq!("BoolValue", bool::NAME);
        assert_eq!("google.protobuf", bool::PACKAGE);
        assert_eq!("google.protobuf.BoolValue", bool::full_name());
        assert_eq!("type.googleapis.com/google.protobuf.BoolValue", bool::type_url());

        assert_eq!("UInt32Value", u32::NAME);
        assert_eq!("google.protobuf", u32::PACKAGE);
        assert_eq!("google.protobuf.UInt32Value", u32::full_name());
        assert_eq!("type.googleapis.com/google.protobuf.UInt32Value", u32::type_url());

        assert_eq!("UInt64Value", u64::NAME);
        assert_eq!("google.protobuf", u64::PACKAGE);
        assert_eq!("google.protobuf.UInt64Value", u64::full_name());
        assert_eq!("type.googleapis.com/google.protobuf.UInt64Value", u64::type_url());

        assert_eq!("Int32Value", i32::NAME);
        assert_eq!("google.protobuf", i32::PACKAGE);
        assert_eq!("google.protobuf.Int32Value", i32::full_name());
        assert_eq!("type.googleapis.com/google.protobuf.Int32Value", i32::type_url());

        assert_eq!("Int64Value", i64::NAME);
        assert_eq!("google.protobuf", i64::PACKAGE);
        assert_eq!("google.protobuf.Int64Value", i64::full_name());
        assert_eq!("type.googleapis.com/google.protobuf.Int64Value", i64::type_url());

        assert_eq!("FloatValue", f32::NAME);
        assert_eq!("google.protobuf", f32::PACKAGE);
        assert_eq!("google.protobuf.FloatValue", f32::full_name());
        assert_eq!("type.googleapis.com/google.protobuf.FloatValue", f32::type_url());

        assert_eq!("DoubleValue", f64::NAME);
        assert_eq!("google.protobuf", f64::PACKAGE);
        assert_eq!("google.protobuf.DoubleValue", f64::full_name());
        assert_eq!("type.googleapis.com/google.protobuf.DoubleValue", f64::type_url());

        assert_eq!("StringValue", String::NAME);
        assert_eq!("google.protobuf", String::PACKAGE);
        assert_eq!("google.protobuf.StringValue", String::full_name());
        assert_eq!("type.googleapis.com/google.protobuf.StringValue", String::type_url());

        assert_eq!("BytesValue", Vec::<u8>::NAME);
        assert_eq!("google.protobuf", Vec::<u8>::PACKAGE);
        assert_eq!("google.protobuf.BytesValue", Vec::<u8>::full_name());
        assert_eq!("type.googleapis.com/google.protobuf.BytesValue", Vec::<u8>::type_url());

        assert_eq!("BytesValue", Bytes::NAME);
        assert_eq!("google.protobuf", Bytes::PACKAGE);
        assert_eq!("google.protobuf.BytesValue", Bytes::full_name());
        assert_eq!("type.googleapis.com/google.protobuf.BytesValue", Bytes::type_url());

        assert_eq!("Empty", <()>::NAME);
        assert_eq!("google.protobuf", <()>::PACKAGE);
        assert_eq!("google.protobuf.Empty", <()>::full_name());
        assert_eq!("type.googleapis.com/google.protobuf.Empty", <()>::type_url());
    }
}
