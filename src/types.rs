//! Protocol Buffers well-known wrapper types.
//!
//! This module provides implementations of `Message` for Rust standard library types which
//! correspond to a Protobuf well-known wrapper type. The remaining well-known types are defined in
//! the `prost-types` crate in order to avoid a cyclic dependency between `prost` and
//! `prost-build`.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use ::bytes::Buf;
use ::bytes::BufMut;
use ::bytes::Bytes;

use crate::DecodeError;
use crate::Name;
use crate::ProtoExt;
use crate::encoding::DecodeContext;
use crate::encoding::bool;
use crate::encoding::bytes;
use crate::encoding::check_wire_type;
use crate::encoding::double;
use crate::encoding::float;
use crate::encoding::int32;
use crate::encoding::int64;
use crate::encoding::skip_field;
use crate::encoding::string;
use crate::encoding::uint32;
use crate::encoding::uint64;
use crate::encoding::wire_type::WireType;
use crate::traits::ProtoShadow;

macro_rules! impl_google_wrapper {
    // ---------- Main entry ----------
    ($ty:ty, $module:ident, $name:literal, $mode:ident,
        $is_default_encode:tt, $is_default_len:tt, $clear_spec:tt
    ) => {
        impl ProtoShadow for $ty {
            type Sun<'a> = impl_google_wrapper!(@sun_ty, $mode, $ty);
            type OwnedSun = $ty;
            type View<'a> = impl_google_wrapper!(@view_ty, $mode, $ty);

            #[inline(always)]
            fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> { Ok(self) }

            #[inline(always)]
            fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> { value }
        }

        impl crate::traits::ProtoWire for $ty {
            type EncodeInput<'b> = impl_google_wrapper!(@encode_ty, $mode, $ty);

            const KIND: crate::traits::ProtoKind =
                impl_google_wrapper!(@kind_ty, $ty);

            #[inline(always)]
            fn encoded_len_impl(v: &Self::EncodeInput<'_>) -> usize {
                if impl_google_wrapper!(@is_default_len, $mode, $is_default_len, v) {
                    0
                } else {
                    impl_google_wrapper!(@len_call, $mode, $module, 1, v)
                }
            }

            #[inline(always)]
            fn encode_raw(v: Self::EncodeInput<'_>, buf: &mut impl BufMut)
            {
                if impl_google_wrapper!(@is_default_encode, $mode, $is_default_encode, v) {
                   impl_google_wrapper!(@encode_call, $mode, $module, 1, v, buf);
                }
            }

            #[inline(always)]
            fn decode_into(wire_type: WireType, value: &mut Self,buf: &mut impl Buf,  ctx: DecodeContext) -> Result<(), DecodeError> {
                ::proto_rs::encoding::$module::merge(wire_type, value, buf, ctx)
            }

            #[inline(always)]
            fn proto_default() -> Self { Default::default() }

            #[inline(always)]
            fn is_default(&self) -> bool {
                impl_google_wrapper!(@is_default_len, $mode, $is_default_len, self)
            }

            #[inline(always)]
            fn clear(&mut self) {
                impl_google_wrapper!(@clear, $mode, $clear_spec, self)
            }
        }

        impl ProtoExt for $ty {
            type Shadow<'b> = $ty where $ty: 'b;
            #[inline(always)]
            fn merge_field(
                value: &mut Self::Shadow<'_>,
                tag: u32,
                wire_type: WireType,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                if tag == 1 {
                    $module::merge(wire_type, value, buf, ctx)
                } else {
                    skip_field(wire_type, tag, buf, ctx)
                }
            }
        }

        impl Name for $ty {
            const NAME: &'static str = $name;
            const PACKAGE: &'static str = "google.protobuf";
            fn type_url() -> String {
                format!("type.googleapis.com/{}.{}", Self::PACKAGE, Self::NAME)
            }
        }
    };

    // ---------- Helpers for predicates and clear ----------

    // by_value: pass (== rhs)
    (@is_default_len, by_value, ($op:tt $rhs:expr), $len:expr) => { (*$len) $op $rhs };
    (@is_default_encode,  by_value, ($op:tt $rhs:expr), $v:expr)    => { ($v)   $op $rhs };
    (@clear,           by_value, ($rhs:expr), $this:expr)        => { *$this = $rhs};

    // by_ref: pass (is_empty) and (clear)
    (@is_default_len, by_ref, ($meth:ident), $len:expr) => { ($len).$meth() };
    (@is_default_encode, by_ref, ($meth:ident), $v:expr)    => { ($v).$meth() };

    (@is_default_len,    by_ref, ($op:tt $rhs:expr), $len:expr) => { (*$len) $op $rhs };
    (@is_default_encode, by_ref, ($op:tt $rhs:expr), $v:expr)   => { ($v)   $op $rhs };

    (@clear,           by_ref, (clear), $this:expr)       => { ($this).clear() };

    // ---------- MODE EXPANSIONS ----------
    (@sun_ty, by_value, $ty:ty) => { $ty };
    (@view_ty, by_value, $ty:ty) => { $ty };
    (@encode_ty, by_value, $ty:ty) => { $ty };

    (@sun_ty, by_ref, $ty:ty) => { &'a $ty };
    (@view_ty, by_ref, $ty:ty) => { &'a $ty };
    (@encode_ty, by_ref, $ty:ty) => { &'b $ty };

    (@len_call, by_value, $module:ident, $tag:expr, $v:ident) => {
        $module::encoded_len($tag, *$v)
    };
    (@encode_call, by_value, $module:ident, $tag:expr, $v:ident, $buf:ident) => {
        $module::encode($tag, $v, $buf)
    };
    (@len_call, by_ref, $module:ident, $tag:expr, $v:ident) => {
        $module::encoded_len($tag, $v)
    };
    (@encode_call, by_ref, $module:ident, $tag:expr, $v:ident, $buf:ident) => {
        $module::encode($tag, $v, $buf)
    };

    // ---------- TYPE → KIND EXPANSION ----------
    (@kind_ty, bool)    => { crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::Bool) };
    (@kind_ty, u32)     => { crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::U32) };
    (@kind_ty, u64)     => { crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::U64) };
    (@kind_ty, i32)     => { crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I32) };
    (@kind_ty, i64)     => { crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I64) };
    (@kind_ty, f32)     => { crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::F32) };
    (@kind_ty, f64)     => { crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::F64) };
    (@kind_ty, String)  => { crate::traits::ProtoKind::String };
    (@kind_ty, Vec<u8>) => { crate::traits::ProtoKind::Bytes };
    (@kind_ty, Bytes)   => { crate::traits::ProtoKind::Bytes };
    (@kind_ty, $other:ty) => { crate::traits::ProtoKind::Message };
}
impl_google_wrapper!(bool,  bool,   "BoolValue",   by_value, (!= false), (== false), (false));
impl_google_wrapper!(u32,   uint32, "UInt32Value", by_value, (!= 0),     (== 0),     (0));
impl_google_wrapper!(u64,   uint64, "UInt64Value", by_value, (!= 0),     (== 0),     (0));
impl_google_wrapper!(i32,   int32,  "Int32Value",  by_value, (!= 0),     (== 0),     (0));
impl_google_wrapper!(i64,   int64,  "Int64Value",  by_value, (!= 0),     (== 0),     (0));
impl_google_wrapper!(f32,   float,  "FloatValue",  by_value, (!= 0.0),   (== 0.0),   (0.0));
impl_google_wrapper!(f64,   double, "DoubleValue", by_value, (!= 0.0),   (== 0.0),   (0.0));

// by_ref (length-delimited)
impl_google_wrapper!(String, string, "StringValue", by_ref, (!= ""), (== ""), (clear));
impl_google_wrapper!(Vec<u8>, bytes, "BytesValue", by_ref, (!= b"" as &[u8]) , (== b"" as &[u8]), (clear));
impl_google_wrapper!(Bytes, bytes, "BytesValue", by_ref, (!=  b"" as &[u8]), (==  b"" as &[u8]), (clear));

impl ProtoShadow for () {
    type Sun<'a> = Self;
    type OwnedSun = Self;
    type View<'a> = Self;

    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(())
    }
    fn from_sun<'a>(_value: Self::Sun<'_>) -> Self::View<'_> {}
}

impl ProtoExt for () {
    type Shadow<'b>
        = Self
    where
        Self: 'b;

    #[inline(always)]
    fn merge_field(_value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        skip_field(wire_type, tag, buf, ctx)
    }
}

impl crate::traits::ProtoWire for () {
    type EncodeInput<'b> = Self;
    const KIND: crate::traits::ProtoKind = crate::traits::ProtoKind::Message;

    #[inline(always)]
    fn encoded_len_impl(_v: &Self::EncodeInput<'_>) -> usize {
        0
    }

    #[inline(always)]
    fn encode_raw(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {}

    #[inline(always)]
    fn is_default(&self) -> bool {
        true
    }

    #[inline(always)]
    fn proto_default() -> Self {}

    #[inline(always)]
    fn clear(&mut self) {}

    fn decode_into(_wire_type: WireType, _value: &mut Self, _buf: &mut impl Buf, _ctx: DecodeContext) -> Result<(), DecodeError> {
        Ok(())
    }
}

/// `google.protobuf.Empty`
impl Name for () {
    const NAME: &'static str = "Empty";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// Compute the type URL for the given `google.protobuf` type, using `type.googleapis.com` as the
/// authority for the URL.
fn googleapis_type_url_for<T: Name>() -> String {
    format!("type.googleapis.com/{}.{}", T::PACKAGE, T::NAME)
}

// Additional implementations for smaller primitive types
// These are not part of protobuf well-known types but needed for internal use

macro_rules! impl_narrow_varint {
    // $mod: encoding module (uint32, int32, sint32, etc.)
    // $prim_kind: PrimitiveKind variant for reflection
    // $wide_ty: widened intermediate type
    // $err: error message on overflow
    ($ty:ty, $wide_ty:ty, $mod:ident, $prim_kind:ident, $err:literal) => {
        /* ---------- ProtoShadow ---------- */
        impl crate::traits::ProtoShadow for $ty {
            type Sun<'a> = Self;
            type OwnedSun = Self;
            type View<'a> = Self;

            #[inline(always)]
            fn to_sun(self) -> Result<Self::OwnedSun, crate::DecodeError> {
                Ok(self)
            }

            #[inline(always)]
            fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
                value
            }
        }

        /* ---------- ProtoWire (atomic encoding) ---------- */
        impl crate::traits::ProtoWire for $ty {
            type EncodeInput<'b> = Self;
            const KIND: crate::traits::ProtoKind = crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::$prim_kind);
            // wire_type() = Varint automatically

            #[inline(always)]
            fn encoded_len_impl(v: &Self::EncodeInput<'_>) -> usize {
                let widened: $wide_ty = *v as $wide_ty;
                crate::encoding::encoded_len_varint(widened as u64)
            }

            #[inline(always)]
            fn encode_raw(value: Self::EncodeInput<'_>, buf: &mut impl ::bytes::BufMut) {
                let widened: $wide_ty = value as $wide_ty;
                crate::encoding::encode_varint(widened as u64, buf);
            }

            #[inline(always)]
            fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, _ctx: DecodeContext) -> Result<(), DecodeError> {
                check_wire_type(WireType::Varint, wire_type)?;
                let widened: $wide_ty = crate::encoding::decode_varint(buf)? as $wide_ty;
                *value = widened.try_into().map_err(|_| crate::DecodeError::new($err))?;
                Ok(())
            }

            #[inline(always)]
            fn is_default(&self) -> bool {
                *self == Self::default()
            }

            #[inline(always)]
            fn proto_default() -> Self {
                Self::default()
            }

            #[inline(always)]
            fn clear(&mut self) {
                *self = Self::default();
            }
        }

        /* ---------- ProtoExt (field-level merge) ---------- */
        impl crate::traits::ProtoExt for $ty {
            type Shadow<'b>
                = Self
            where
                Self: 'b;

            #[inline(always)]
            fn merge_field(
                value: &mut Self::Shadow<'_>,
                tag: u32,
                wire_type: crate::encoding::WireType,
                buf: &mut impl ::bytes::Buf,
                ctx: crate::encoding::DecodeContext,
            ) -> Result<(), crate::DecodeError> {
                if tag == 1 {
                    <Self as crate::traits::ProtoWire>::decode_into(wire_type, value, buf, ctx)
                } else {
                    skip_field(wire_type, tag, buf, ctx)
                }
            }
        }
    };
}

// Unsigned narrow varints (plain varint)
impl_narrow_varint!(u8, u32, uint32, U8, "u8 overflow");
impl_narrow_varint!(u16, u32, uint32, U16, "u16 overflow");
impl_narrow_varint!(i8, i32, sint32, I8, "i8 overflow");
impl_narrow_varint!(i16, i32, sint32, I16, "i16 overflow");

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
