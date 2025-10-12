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
use crate::encoding::double;
use crate::encoding::float;
use crate::encoding::int32;
use crate::encoding::int64;
use crate::encoding::skip_field;
use crate::encoding::string;
use crate::encoding::uint32;
use crate::encoding::uint64;
use crate::encoding::wire_type::WireType;

/// `google.protobuf.BoolValue`
impl ProtoExt for bool {
    #[inline]
    fn proto_default() -> Self {
        false
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self {
            bool::encode(1, self, buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 { bool::merge(wire_type, self, buf, ctx) } else { skip_field(wire_type, tag, buf, ctx) }
    }

    fn encoded_len(&self) -> usize {
        if *self { 2 } else { 0 }
    }

    fn clear(&mut self) {
        *self = false;
    }
}

/// `google.protobuf.BoolValue`
impl Name for bool {
    const NAME: &'static str = "BoolValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.UInt32Value`
impl ProtoExt for u32 {
    #[inline]
    fn proto_default() -> Self {
        0
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0 {
            uint32::encode(1, self, buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            uint32::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if *self != 0 { uint32::encoded_len(1, self) } else { 0 }
    }

    fn clear(&mut self) {
        *self = 0;
    }
}

/// `google.protobuf.UInt32Value`
impl Name for u32 {
    const NAME: &'static str = "UInt32Value";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.UInt64Value`
impl ProtoExt for u64 {
    #[inline]
    fn proto_default() -> Self {
        0
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0 {
            uint64::encode(1, self, buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            uint64::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if *self != 0 { uint64::encoded_len(1, self) } else { 0 }
    }

    fn clear(&mut self) {
        *self = 0;
    }
}

/// `google.protobuf.UInt64Value`
impl Name for u64 {
    const NAME: &'static str = "UInt64Value";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

impl ProtoExt for Vec<u64> {
    #[inline]
    fn proto_default() -> Self {
        Vec::new()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.is_empty() {
            uint64::encode_repeated(1, self, buf);
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            uint64::merge_repeated(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if self.is_empty() { 0 } else { uint64::encoded_len_repeated(1, self) }
    }

    fn clear(&mut self) {
        self.clear();
    }
}

/// `google.protobuf.Int32Value`
impl ProtoExt for i32 {
    #[inline]
    fn proto_default() -> Self {
        0
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0 {
            int32::encode(1, self, buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            int32::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if *self != 0 { int32::encoded_len(1, self) } else { 0 }
    }

    fn clear(&mut self) {
        *self = 0;
    }
}

/// `google.protobuf.Int32Value`
impl Name for i32 {
    const NAME: &'static str = "Int32Value";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.Int64Value`
impl ProtoExt for i64 {
    #[inline]
    fn proto_default() -> Self {
        0
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0 {
            int64::encode(1, self, buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            int64::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if *self != 0 { int64::encoded_len(1, self) } else { 0 }
    }

    fn clear(&mut self) {
        *self = 0;
    }
}

/// `google.protobuf.Int64Value`
impl Name for i64 {
    const NAME: &'static str = "Int64Value";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.FloatValue`
impl ProtoExt for f32 {
    #[inline]
    fn proto_default() -> Self {
        0.0
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0.0 {
            float::encode(1, self, buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            float::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if *self != 0.0 { float::encoded_len(1, self) } else { 0 }
    }

    fn clear(&mut self) {
        *self = 0.0;
    }
}

/// `google.protobuf.FloatValue`
impl Name for f32 {
    const NAME: &'static str = "FloatValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.DoubleValue`
impl ProtoExt for f64 {
    #[inline]
    fn proto_default() -> Self {
        0.0
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0.0 {
            double::encode(1, self, buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            double::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if *self != 0.0 { double::encoded_len(1, self) } else { 0 }
    }

    fn clear(&mut self) {
        *self = 0.0;
    }
}

/// `google.protobuf.DoubleValue`
impl Name for f64 {
    const NAME: &'static str = "DoubleValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.StringValue`
impl ProtoExt for String {
    #[inline]
    fn proto_default() -> Self {
        String::new()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.is_empty() {
            string::encode(1, self, buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            string::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if !self.is_empty() { string::encoded_len(1, self) } else { 0 }
    }

    fn clear(&mut self) {
        self.clear();
    }
}

/// `google.protobuf.StringValue`
impl Name for String {
    const NAME: &'static str = "StringValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.BytesValue`
impl ProtoExt for Vec<u8> {
    #[inline]
    fn proto_default() -> Self {
        Vec::new()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.is_empty() {
            bytes::encode(1, self, buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            bytes::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if !self.is_empty() { bytes::encoded_len(1, self) } else { 0 }
    }

    fn clear(&mut self) {
        self.clear();
    }
}

/// `google.protobuf.BytesValue`
impl Name for Vec<u8> {
    const NAME: &'static str = "BytesValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

/// `google.protobuf.BytesValue`
impl ProtoExt for Bytes {
    #[inline]
    fn proto_default() -> Self {
        Bytes::new()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if !self.is_empty() {
            bytes::encode(1, self, buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            bytes::merge(wire_type, self, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if !self.is_empty() { bytes::encoded_len(1, self) } else { 0 }
    }

    fn clear(&mut self) {
        self.clear();
    }
}

/// `google.protobuf.BytesValue`
impl Name for Bytes {
    const NAME: &'static str = "BytesValue";
    const PACKAGE: &'static str = "google.protobuf";

    fn type_url() -> String {
        googleapis_type_url_for::<Self>()
    }
}

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

/// Compute the type URL for the given `google.protobuf` type, using `type.googleapis.com` as the
/// authority for the URL.
fn googleapis_type_url_for<T: Name>() -> String {
    format!("type.googleapis.com/{}.{}", T::PACKAGE, T::NAME)
}

// Additional implementations for smaller primitive types
// These are not part of protobuf well-known types but needed for internal use

/// Internal implementation for u8
impl ProtoExt for u8 {
    #[inline]
    fn proto_default() -> Self {
        0
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0 {
            uint32::encode(1, &(*self as u32), buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            let mut temp: u32 = 0;
            uint32::merge(wire_type, &mut temp, buf, ctx)?;
            *self = temp.try_into().map_err(|_| DecodeError::new("u8 overflow"))?;
            Ok(())
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if *self != 0 { uint32::encoded_len(1, &(*self as u32)) } else { 0 }
    }

    fn clear(&mut self) {
        *self = 0;
    }
}

/// Internal implementation for u16
impl ProtoExt for u16 {
    #[inline]
    fn proto_default() -> Self {
        0
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0 {
            uint32::encode(1, &(*self as u32), buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            let mut temp: u32 = 0;
            uint32::merge(wire_type, &mut temp, buf, ctx)?;
            *self = temp.try_into().map_err(|_| DecodeError::new("u16 overflow"))?;
            Ok(())
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if *self != 0 { uint32::encoded_len(1, &(*self as u32)) } else { 0 }
    }

    fn clear(&mut self) {
        *self = 0;
    }
}

/// Internal implementation for i8
impl ProtoExt for i8 {
    #[inline]
    fn proto_default() -> Self {
        0
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0 {
            int32::encode(1, &(*self as i32), buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            let mut temp: i32 = 0;
            int32::merge(wire_type, &mut temp, buf, ctx)?;
            *self = temp.try_into().map_err(|_| DecodeError::new("i8 overflow"))?;
            Ok(())
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if *self != 0 { int32::encoded_len(1, &(*self as i32)) } else { 0 }
    }

    fn clear(&mut self) {
        *self = 0;
    }
}

/// Internal implementation for i16
impl ProtoExt for i16 {
    #[inline]
    fn proto_default() -> Self {
        0
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        if *self != 0 {
            int32::encode(1, &(*self as i32), buf)
        }
    }

    fn merge_field(&mut self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            let mut temp: i32 = 0;
            int32::merge(wire_type, &mut temp, buf, ctx)?;
            *self = temp.try_into().map_err(|_| DecodeError::new("i16 overflow"))?;
            Ok(())
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        if *self != 0 { int32::encoded_len(1, &(*self as i32)) } else { 0 }
    }

    fn clear(&mut self) {
        *self = 0;
    }
}

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
