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
use crate::traits::OwnedSunOf;
use crate::traits::ProtoShadow;
use crate::traits::Shadow;
use crate::traits::ViewOf;

macro_rules! impl_google_wrapper {
    ($ty:ty, $module:ident, $name:literal, |$value:ident| $is_default:expr, |$clear_value:ident| $clear_body:expr) => {
        impl ProtoShadow for $ty {
            type Sun<'a> = &'a Self;
            type OwnedSun = Self;
            type View<'a> = &'a Self;

            fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
                Ok(self)
            }

            fn from_sun<'a>(value: Self::Sun<'_>) -> Self::View<'_> {
                value
            }
        }

        impl ProtoExt for $ty {
            type Shadow<'a> = Self;

            #[inline]
            fn proto_default<'a>() -> Self::Shadow<'a> {
                Default::default()
            }

            fn encoded_len(value: &ViewOf<'_, Self>) -> usize {
                let inner: &$ty = *value;
                if {
                    let $value: &$ty = inner;
                    $is_default
                } {
                    0
                } else {
                    $module::encoded_len(1, inner)
                }
            }

            fn encode_raw<'a>(value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
                if !{
                    let $value: &$ty = value;
                    $is_default
                } {
                    $module::encode(1, value, buf);
                }
            }

            fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
                if tag == 1 {
                    $module::merge(wire_type, value, buf, ctx)
                } else {
                    skip_field(wire_type, tag, buf, ctx)
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
            fn encode_repeated_field(tag: u32, values: &[OwnedSunOf<'_, Self>], buf: &mut impl BufMut) {
                for value in values {
                    $module::encode(tag, value, buf);
                }
            }

            fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self::Shadow<'_>>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
                $module::merge_repeated(wire_type, values, buf, ctx)
            }

            fn encoded_len_repeated_field(tag: u32, values: &[OwnedSunOf<'_, Self>]) -> usize {
                values.iter().map(|value| $module::encoded_len(tag, value)).sum()
            }
        }

        impl SingularField for $ty {
            fn encode_singular_field(tag: u32, value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
                if !{
                    let $value: &$ty = value;
                    $is_default
                } {
                    $module::encode(tag, value, buf);
                }
            }

            fn merge_singular_field(wire_type: WireType, value: &mut Self::Shadow<'_>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
                $module::merge(wire_type, value, buf, ctx)
            }

            fn encoded_len_singular_field(tag: u32, value: &ViewOf<'_, Self>) -> usize {
                let inner: &$ty = *value;
                if {
                    let $value: &$ty = inner;
                    $is_default
                } {
                    0
                } else {
                    $module::encoded_len(tag, inner)
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
impl ProtoShadow for () {
    type Sun<'a> = Self;
    type OwnedSun = Self;
    type View<'a> = Self;

    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(())
    }
    fn from_sun<'a>(_value: Self::Sun<'_>) -> Self::View<'_> {}
}

/// `google.protobuf.Empty`
impl ProtoExt for () {
    type Shadow<'a> = Self;

    #[inline]
    fn proto_default<'a>() -> Self::Shadow<'a> {}

    fn encoded_len(_value: &ViewOf<'_, Self>) -> usize {
        0
    }

    fn encode_raw<'a>(_value: ViewOf<'_, Self>, _buf: &mut impl BufMut) {}

    fn merge_field(_value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        skip_field(wire_type, tag, buf, ctx)
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
        impl ProtoShadow for $ty {
            type Sun<'a> = &'a Self;
            type OwnedSun = Self;
            type View<'a> = &'a Self;

            fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
                Ok(self)
            }

            fn from_sun<'a>(value: Self::Sun<'_>) -> Self::View<'_> {
                value
            }
        }

        impl ProtoExt for $ty {
            type Shadow<'a> = Self;

            #[inline]
            fn proto_default<'a>() -> Self::Shadow<'a> {
                Self::default()
            }

            fn encoded_len(value: &ViewOf<'_, Self>) -> usize {
                let inner: &$ty = *value;
                if *inner == Self::default() {
                    0
                } else {
                    let widened: $wide_ty = (*inner).into();
                    $module::encoded_len(1, &widened)
                }
            }

            fn encode_raw<'a>(value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
                if *value != Self::default() {
                    let widened: $wide_ty = (*value).into();
                    $module::encode(1, &widened, buf);
                }
            }

            fn merge_field(
                value: &mut Self::Shadow<'_>,
                tag: u32,
                wire_type: WireType,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                if tag == 1 {
                    let mut widened: $wide_ty = <$wide_ty as Default>::default();
                    $module::merge(wire_type, &mut widened, buf, ctx)?;
                    *value = widened.try_into().map_err(|_| DecodeError::new($err))?;
                    Ok(())
                } else {
                    skip_field(wire_type, tag, buf, ctx)
                }
            }

            fn clear(&mut self) {
                *self = Self::default();
            }
        }

        impl SingularField for $ty {
            fn encode_singular_field(tag: u32, value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
                if *value != Self::default() {
                    let widened: $wide_ty = (*value).into();
                    $module::encode(tag, &widened, buf);
                }
            }

            fn merge_singular_field(
                wire_type: WireType,
                value: &mut Self::Shadow<'_>,
                buf: &mut impl Buf,
                ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                let mut widened: $wide_ty = <$wide_ty as Default>::default();
                $module::merge(wire_type, &mut widened, buf, ctx)?;
                *value = widened.try_into().map_err(|_| DecodeError::new($err))?;
                Ok(())
            }

            fn encoded_len_singular_field(tag: u32, value: &ViewOf<'_, Self>) -> usize {
                let inner: &$ty = *value;
                if *inner == Self::default() {
                    0
                } else {
                    let widened: $wide_ty = (*inner).into();
                    $module::encoded_len(tag, &widened)
                }
            }
        }

        impl_narrow_varint!(@maybe_repeated $with_repeated, $ty, $wide_ty, $module, $err);
    };
    (@maybe_repeated true, $ty:ty, $wide_ty:ty, $module:ident, $err:literal) => {
       impl RepeatedField for $ty {
            fn encode_repeated_field(tag: u32, values: &[OwnedSunOf<'_, Self>], buf: &mut impl BufMut) {
                for value in values {
                    let widened: $wide_ty = (*value).into();
                    $module::encode(tag, &widened, buf);
                }
            }

            fn merge_repeated_field(
                wire_type: WireType,
                values: &mut Vec<Self::Shadow<'_>>,
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

            fn encoded_len_repeated_field(tag: u32, values: &[OwnedSunOf<'_, Self>]) -> usize {
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
impl<T: ProtoShadow> ProtoShadow for Option<T> {
    type Sun<'a> = Option<T::Sun<'a>>;

    type OwnedSun = Option<T::OwnedSun>;
    type View<'a> = Option<T::View<'a>>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        // Map Option<T> → Option<T::OwnedSun>
        self.map(T::to_sun).transpose()
    }

    #[inline]
    fn from_sun<'a>(v: Self::Sun<'_>) -> Self::View<'_> {
        v.map(T::from_sun)
    }
}

impl<T: ProtoExt> ProtoExt for Option<T> {
    type Shadow<'a>
        = Option<Shadow<'a, T>>
    where
        T: 'a;

    #[inline]
    fn proto_default<'a>() -> Self::Shadow<'a> {
        None
    }

    fn encoded_len(value: &ViewOf<'_, Self>) -> usize {
        value.as_ref().map_or(0, |inner| T::encoded_len(inner))
    }

    fn encode_raw<'a>(value: ViewOf<'_, Self>, buf: &mut impl BufMut) {
        if let Some(inner) = value {
            T::encode_raw(inner, buf);
        }
    }

    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let slot = value.get_or_insert_with(T::proto_default);
        T::merge_field(slot, tag, wire_type, buf, ctx)
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
