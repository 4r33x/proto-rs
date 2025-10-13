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
use crate::MessageField;
use crate::Name;
use crate::ProtoExt;
use crate::RepeatedField;
use crate::SingularField;
use crate::encoding::DecodeContext;
use crate::encoding::bool;
use crate::encoding::bytes;
use crate::encoding::double;
use crate::encoding::float;
use crate::encoding::int32;
use crate::encoding::int64;
use crate::encoding::skip_field;
use crate::encoding::string;
use crate::encoding::uint32;
use crate::encoding::uint64;
use crate::encoding::wire_type::WireType;

macro_rules! impl_google_wrapper {
    ($ty:ty, $module:ident, $name:literal, |$value:ident| $is_default:expr, |$clear_value:ident| $clear_body:expr) => {
        impl ProtoExt for $ty {
            #[inline]
            fn proto_default() -> Self {
                Default::default()
            }

            fn encode_raw(&self, buf: &mut impl BufMut) {
                if !{
                    let $value: &$ty = self;
                    $is_default
                } {
                    $module::encode(1, self, buf);
                }
            }

            fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
                if tag == 1 {
                    $module::merge(wire_type, self, buf, ctx)
                } else {
                    skip_field(wire_type, tag, buf, ctx)
                }
            }

            fn encoded_len(&self) -> usize {
                if {
                    let $value: &$ty = self;
                    $is_default
                } {
                    0
                } else {
                    $module::encoded_len(1, self)
                }
            }

            fn clear(&mut self) {
                let $clear_value: &mut $ty = self;
                $clear_body
            }
        }

        impl Name for $ty {
            const NAME: &'static str = $name;
            const PACKAGE: &'static str = "google.protobuf";

            fn type_url() -> String {
                googleapis_type_url_for::<Self>()
            }
        }

        impl RepeatedField for $ty {
            fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
                $module::encode_repeated(tag, values, buf);
            }

            fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
                $module::merge_repeated(wire_type, values, buf, ctx)
            }

            fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
                $module::encoded_len_repeated(tag, values)
            }
        }

        impl SingularField for $ty {
            fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
                if !{
                    let $value: &$ty = value;
                    $is_default
                } {
                    $module::encode(tag, value, buf);
                }
            }

            fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
                $module::merge(wire_type, value, buf, ctx)
            }

            fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
                if {
                    let $value: &$ty = value;
                    $is_default
                } {
                    0
                } else {
                    $module::encoded_len(tag, value)
                }
            }
        }
    };
}

impl_google_wrapper!(bool, bool, "BoolValue", |value| !*value, |value| *value = false);
impl_google_wrapper!(u32, uint32, "UInt32Value", |value| *value == 0, |value| *value = 0);
impl_google_wrapper!(u64, uint64, "UInt64Value", |value| *value == 0, |value| *value = 0);
impl_google_wrapper!(i32, int32, "Int32Value", |value| *value == 0, |value| *value = 0);
impl_google_wrapper!(i64, int64, "Int64Value", |value| *value == 0, |value| *value = 0);
impl_google_wrapper!(f32, float, "FloatValue", |value| *value == 0.0, |value| *value = 0.0);
impl_google_wrapper!(f64, double, "DoubleValue", |value| *value == 0.0, |value| *value = 0.0);
impl_google_wrapper!(String, string, "StringValue", |value| value.is_empty(), |value| value.clear());
impl_google_wrapper!(Vec<u8>, bytes, "BytesValue", |value| value.is_empty(), |value| value.clear());
impl_google_wrapper!(Bytes, bytes, "BytesValue", |value| value.is_empty(), |value| value.clear());

/// `google.protobuf.Empty`
impl ProtoExt for () {
    #[inline]
    fn proto_default() -> Self {}

    fn encode_raw(&self, _buf: &mut impl BufMut) {}

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        skip_field(wire_type, tag, buf, ctx)
    }

    fn encoded_len(&self) -> usize {
        0
    }

    fn clear(&mut self) {}
}

/// `google.protobuf.Empty`
impl Name for () {
    const NAME: &'static str = "Empty";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

impl MessageField for () {}

/// Compute the type URL for the given `google.protobuf` type, using `type.googleapis.com` as the
/// authority for the URL.
fn googleapis_type_url_for<T: Name>() -> String {
    format!("type.googleapis.com/{}.{}", T::PACKAGE, T::NAME)
}

// Additional implementations for smaller primitive types
// These are not part of protobuf well-known types but needed for internal use

macro_rules! impl_narrow_varint {
    ($ty:ty, $wide_ty:ty, $module:ident, $err:literal) => {
        impl_narrow_varint!(@impl $ty, $wide_ty, $module, $err, true);
    };
    ($ty:ty, $wide_ty:ty, $module:ident, $err:literal, no_repeated) => {
        impl_narrow_varint!(@impl $ty, $wide_ty, $module, $err, false);
    };
    (@impl $ty:ty, $wide_ty:ty, $module:ident, $err:literal, $with_repeated:tt) => {
        impl ProtoExt for $ty {
            #[inline]
            fn proto_default() -> Self {
                Self::default()
            }

            fn encode_raw(&self, buf: &mut impl BufMut) {
                if *self != Self::default() {
                    let widened: $wide_ty = (*self).into();
                    $module::encode(1, &widened, buf);
                }
            }

            fn merge_field(
                &mut self,
                tag: u32,
                wire_type: WireType,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                if tag == 1 {
                    let mut widened: $wide_ty = <$wide_ty as Default>::default();
                    $module::merge(wire_type, &mut widened, buf, ctx)?;
                    *self = widened.try_into().map_err(|_| DecodeError::new($err))?;
                    Ok(())
                } else {
                    skip_field(wire_type, tag, buf, ctx)
                }
            }

            fn encoded_len(&self) -> usize {
                if *self == Self::default() {
                    0
                } else {
                    let widened: $wide_ty = (*self).into();
                    $module::encoded_len(1, &widened)
                }
            }

            fn clear(&mut self) {
                *self = Self::default();
            }
        }

        impl SingularField for $ty {
            fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
                if *value != Self::default() {
                    let widened: $wide_ty = (*value).into();
                    $module::encode(tag, &widened, buf);
                }
            }

            fn merge_singular_field(
                wire_type: WireType,
                value: &mut Self,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                let mut widened: $wide_ty = <$wide_ty as Default>::default();
                $module::merge(wire_type, &mut widened, buf, ctx)?;
                *value = widened.try_into().map_err(|_| DecodeError::new($err))?;
                Ok(())
            }

            fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
                if *value == Self::default() {
                    0
                } else {
                    let widened: $wide_ty = (*value).into();
                    $module::encoded_len(tag, &widened)
                }
            }
        }

        impl_narrow_varint!(@maybe_repeated $with_repeated, $ty, $wide_ty, $module, $err);
    };
    (@maybe_repeated true, $ty:ty, $wide_ty:ty, $module:ident, $err:literal) => {
        impl RepeatedField for $ty {
            fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
                for value in values {
                    let widened: $wide_ty = (*value).into();
                    $module::encode(tag, &widened, buf);
                }
            }

            fn merge_repeated_field(
                wire_type: WireType,
                values: &mut Vec<Self>,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                if wire_type == WireType::LengthDelimited {
                    crate::encoding::merge_loop(values, buf, ctx, |values, buf, ctx| {
                        let mut widened: $wide_ty = <$wide_ty as Default>::default();
                        $module::merge(WireType::Varint, &mut widened, buf, ctx)?;
                        values.push(widened.try_into().map_err(|_| DecodeError::new($err))?);
                        Ok(())
                    })
                } else {
                    crate::encoding::check_wire_type(WireType::Varint, wire_type)?;
                    let mut widened: $wide_ty = <$wide_ty as Default>::default();
                    $module::merge(wire_type, &mut widened, buf, ctx)?;
                    values.push(widened.try_into().map_err(|_| DecodeError::new($err))?);
                    Ok(())
                }
            }

            fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
                values
                    .iter()
                    .map(|value| {
                        let widened: $wide_ty = (*value).into();
                        $module::encoded_len(tag, &widened)
                    })
                    .sum()
            }
        }
    };
    (@maybe_repeated false, $ty:ty, $wide_ty:ty, $module:ident, $err:literal) => {};
}

impl_narrow_varint!(u8, u32, uint32, "u8 overflow", no_repeated);
impl_narrow_varint!(u16, u32, uint32, "u16 overflow");
impl_narrow_varint!(i8, i32, int32, "i8 overflow");
impl_narrow_varint!(i16, i32, int32, "i16 overflow");

/// Generic implementation for Option<T>
impl<T: ProtoExt> ProtoExt for Option<T> {
    #[inline]
    fn proto_default() -> Self {
        None
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if let Some(value) = self {
            value.encode_raw(buf);
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let mut value = self.take().unwrap_or_else(T::proto_default);
        value.merge_field(tag, wire_type, buf, ctx)?;
        *self = Some(value);
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        self.as_ref().map_or(0, |value| value.encoded_len())
    }

    fn clear(&mut self) {
        *self = None;
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
