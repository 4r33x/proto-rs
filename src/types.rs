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

impl RepeatedField for bool {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        bool::encode_repeated(tag, values, buf);
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        bool::merge_repeated(wire_type, values, buf, ctx)
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        bool::encoded_len_repeated(tag, values)
    }
}

impl SingularField for bool {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if *value {
            bool::encode(tag, value, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        bool::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if *value { bool::encoded_len(tag, value) } else { 0 }
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

impl RepeatedField for u32 {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        uint32::encode_repeated(tag, values, buf);
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        uint32::merge_repeated(wire_type, values, buf, ctx)
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        uint32::encoded_len_repeated(tag, values)
    }
}

impl SingularField for u32 {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if *value != 0 {
            uint32::encode(tag, value, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        uint32::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if *value != 0 { uint32::encoded_len(tag, value) } else { 0 }
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

impl RepeatedField for u64 {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        uint64::encode_repeated(tag, values, buf);
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        uint64::merge_repeated(wire_type, values, buf, ctx)
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        uint64::encoded_len_repeated(tag, values)
    }
}

impl SingularField for u64 {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if *value != 0 {
            uint64::encode(tag, value, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        uint64::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if *value != 0 { uint64::encoded_len(tag, value) } else { 0 }
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

impl RepeatedField for i32 {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        int32::encode_repeated(tag, values, buf);
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        int32::merge_repeated(wire_type, values, buf, ctx)
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        int32::encoded_len_repeated(tag, values)
    }
}

impl SingularField for i32 {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if *value != 0 {
            int32::encode(tag, value, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        int32::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if *value != 0 { int32::encoded_len(tag, value) } else { 0 }
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

impl RepeatedField for i64 {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        int64::encode_repeated(tag, values, buf);
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        int64::merge_repeated(wire_type, values, buf, ctx)
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        int64::encoded_len_repeated(tag, values)
    }
}

impl SingularField for i64 {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if *value != 0 {
            int64::encode(tag, value, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        int64::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if *value != 0 { int64::encoded_len(tag, value) } else { 0 }
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

impl RepeatedField for f32 {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        float::encode_repeated(tag, values, buf);
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        float::merge_repeated(wire_type, values, buf, ctx)
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        float::encoded_len_repeated(tag, values)
    }
}

impl SingularField for f32 {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if *value != 0.0 {
            float::encode(tag, value, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        float::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if *value != 0.0 { float::encoded_len(tag, value) } else { 0 }
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

impl RepeatedField for f64 {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        double::encode_repeated(tag, values, buf);
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        double::merge_repeated(wire_type, values, buf, ctx)
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        double::encoded_len_repeated(tag, values)
    }
}

impl SingularField for f64 {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if *value != 0.0 {
            double::encode(tag, value, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        double::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if *value != 0.0 { double::encoded_len(tag, value) } else { 0 }
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

impl RepeatedField for String {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        string::encode_repeated(tag, values, buf);
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        string::merge_repeated(wire_type, values, buf, ctx)
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        string::encoded_len_repeated(tag, values)
    }
}

impl SingularField for String {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if !value.is_empty() {
            string::encode(tag, value, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        string::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if value.is_empty() { 0 } else { string::encoded_len(tag, value) }
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

impl RepeatedField for Vec<u8> {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        bytes::encode_repeated(tag, values, buf);
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        bytes::merge_repeated(wire_type, values, buf, ctx)
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        bytes::encoded_len_repeated(tag, values)
    }
}

impl SingularField for Vec<u8> {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if !value.is_empty() {
            bytes::encode(tag, value, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        bytes::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if value.is_empty() { 0 } else { bytes::encoded_len(tag, value) }
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

impl RepeatedField for Bytes {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        bytes::encode_repeated(tag, values, buf);
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        bytes::merge_repeated(wire_type, values, buf, ctx)
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        bytes::encoded_len_repeated(tag, values)
    }
}

impl SingularField for Bytes {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if !value.is_empty() {
            bytes::encode(tag, value, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        bytes::merge(wire_type, value, buf, ctx)
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if value.is_empty() { 0 } else { bytes::encoded_len(tag, value) }
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

impl MessageField for () {}

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

impl SingularField for u8 {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if *value != 0 {
            let converted: u32 = (*value).into();
            uint32::encode(tag, &converted, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let mut temp: u32 = 0;
        uint32::merge(wire_type, &mut temp, buf, ctx)?;
        *value = temp.try_into().map_err(|_| DecodeError::new("u8 overflow"))?;
        Ok(())
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if *value == 0 {
            0
        } else {
            let converted: u32 = (*value).into();
            uint32::encoded_len(tag, &converted)
        }
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

impl SingularField for u16 {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if *value != 0 {
            let converted: u32 = (*value).into();
            uint32::encode(tag, &converted, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let mut temp: u32 = 0;
        uint32::merge(wire_type, &mut temp, buf, ctx)?;
        *value = temp.try_into().map_err(|_| DecodeError::new("u16 overflow"))?;
        Ok(())
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if *value == 0 {
            0
        } else {
            let converted: u32 = (*value).into();
            uint32::encoded_len(tag, &converted)
        }
    }
}

impl RepeatedField for u16 {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        for value in values {
            let widened = u32::from(*value);
            uint32::encode(tag, &widened, buf);
        }
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if wire_type == WireType::LengthDelimited {
            crate::encoding::merge_loop(values, buf, ctx, |values, buf, ctx| {
                let mut widened: u32 = 0;
                uint32::merge(WireType::Varint, &mut widened, buf, ctx)?;
                values.push(widened.try_into().map_err(|_| DecodeError::new("u16 overflow"))?);
                Ok(())
            })
        } else {
            crate::encoding::check_wire_type(WireType::Varint, wire_type)?;
            let mut widened: u32 = 0;
            uint32::merge(wire_type, &mut widened, buf, ctx)?;
            values.push(widened.try_into().map_err(|_| DecodeError::new("u16 overflow"))?);
            Ok(())
        }
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        values
            .iter()
            .map(|value| {
                let widened = u32::from(*value);
                uint32::encoded_len(tag, &widened)
            })
            .sum()
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

impl SingularField for i8 {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if *value != 0 {
            let converted: i32 = (*value).into();
            int32::encode(tag, &converted, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let mut temp: i32 = 0;
        int32::merge(wire_type, &mut temp, buf, ctx)?;
        *value = temp.try_into().map_err(|_| DecodeError::new("i8 overflow"))?;
        Ok(())
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if *value == 0 {
            0
        } else {
            let converted: i32 = (*value).into();
            int32::encoded_len(tag, &converted)
        }
    }
}

impl RepeatedField for i8 {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        for value in values {
            let widened = i32::from(*value);
            int32::encode(tag, &widened, buf);
        }
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if wire_type == WireType::LengthDelimited {
            crate::encoding::merge_loop(values, buf, ctx, |values, buf, ctx| {
                let mut widened: i32 = 0;
                int32::merge(WireType::Varint, &mut widened, buf, ctx)?;
                values.push(widened.try_into().map_err(|_| DecodeError::new("i8 overflow"))?);
                Ok(())
            })
        } else {
            crate::encoding::check_wire_type(WireType::Varint, wire_type)?;
            let mut widened: i32 = 0;
            int32::merge(wire_type, &mut widened, buf, ctx)?;
            values.push(widened.try_into().map_err(|_| DecodeError::new("i8 overflow"))?);
            Ok(())
        }
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        values
            .iter()
            .map(|value| {
                let widened = i32::from(*value);
                int32::encoded_len(tag, &widened)
            })
            .sum()
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

impl SingularField for i16 {
    fn encode_singular_field(tag: u32, value: &Self, buf: &mut impl BufMut) {
        if *value != 0 {
            let converted: i32 = (*value).into();
            int32::encode(tag, &converted, buf);
        }
    }

    fn merge_singular_field(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let mut temp: i32 = 0;
        int32::merge(wire_type, &mut temp, buf, ctx)?;
        *value = temp.try_into().map_err(|_| DecodeError::new("i16 overflow"))?;
        Ok(())
    }

    fn encoded_len_singular_field(tag: u32, value: &Self) -> usize {
        if *value == 0 {
            0
        } else {
            let converted: i32 = (*value).into();
            int32::encoded_len(tag, &converted)
        }
    }
}

impl RepeatedField for i16 {
    fn encode_repeated_field(tag: u32, values: &[Self], buf: &mut impl BufMut) {
        for value in values {
            let widened = i32::from(*value);
            int32::encode(tag, &widened, buf);
        }
    }

    fn merge_repeated_field(wire_type: WireType, values: &mut Vec<Self>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if wire_type == WireType::LengthDelimited {
            crate::encoding::merge_loop(values, buf, ctx, |values, buf, ctx| {
                let mut widened: i32 = 0;
                int32::merge(WireType::Varint, &mut widened, buf, ctx)?;
                values.push(widened.try_into().map_err(|_| DecodeError::new("i16 overflow"))?);
                Ok(())
            })
        } else {
            crate::encoding::check_wire_type(WireType::Varint, wire_type)?;
            let mut widened: i32 = 0;
            int32::merge(wire_type, &mut widened, buf, ctx)?;
            values.push(widened.try_into().map_err(|_| DecodeError::new("i16 overflow"))?);
            Ok(())
        }
    }

    fn encoded_len_repeated_field(tag: u32, values: &[Self]) -> usize {
        values
            .iter()
            .map(|value| {
                let widened = i32::from(*value);
                int32::encoded_len(tag, &widened)
            })
            .sum()
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
