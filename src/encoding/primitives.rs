use crate::DecodeError;
use crate::bytes::Buf;
use crate::bytes::BufMut;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::check_wire_type;
use crate::encoding::decode_varint;
use crate::encoding::encode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
use crate::encoding::key_len;

/// Helper macro which emits an `encode_repeated` function for the type.
macro_rules! encode_repeated {
    ($ty:ty, by_value) => {
        pub fn encode_repeated(tag: u32, values: &[$ty], buf: &mut impl BufMut) {
            for &value in values {
                encode_tagged(tag, value, buf);
            }
        }
    };
    ($ty:ty, by_ref) => {
        pub fn encode_repeated(tag: u32, values: &[$ty], buf: &mut impl BufMut) {
            for value in values {
                encode_tagged(tag, value, buf);
            }
        }
    };
}

/// Helper macro which emits a `merge_repeated` function for the numeric type.
macro_rules! merge_repeated_numeric {
    ($ty:ty,
     $wire_type:expr,
     $merge:ident,
     $merge_repeated:ident) => {
        pub fn $merge_repeated(wire_type: WireType, values: &mut Vec<$ty>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
            if wire_type == WireType::LengthDelimited {
                // Packed.
                merge_loop(values, buf, ctx, |values, buf, ctx| {
                    let mut value = Default::default();
                    $merge($wire_type, &mut value, buf, ctx)?;
                    values.push(value);
                    Ok(())
                })
            } else {
                // Unpacked.
                check_wire_type($wire_type, wire_type)?;
                let mut value = Default::default();
                $merge(wire_type, &mut value, buf, ctx)?;
                values.push(value);
                Ok(())
            }
        }
    };
}

/// Macro which emits a module containing a set of encoding functions for a
/// variable-width numeric type.
macro_rules! varint {
    ($ty:ty, $proto_ty:ident) => {
        varint!($ty, $proto_ty,
            to_uint64(v) { v as u64 },
            from_uint64(v) { v as $ty });
    };

    ($ty:ty,
     $proto_ty:ident,
     to_uint64($v:ident) $to_uint64:expr,
     from_uint64($fv:ident) $from_uint64:expr) => {
        pub mod $proto_ty {
            use crate::encoding::*;

            #[inline(always)]
            pub fn encode_tagged(tag: u32, $v: $ty, buf: &mut impl BufMut) {
                encode_key(tag, WireType::Varint, buf);
                encode_varint($to_uint64, buf);
            }

            #[inline(always)]
            pub fn encode($v: $ty, buf: &mut impl BufMut) {
                encode_varint($to_uint64, buf);
            }

            #[inline(always)]
            pub(crate) fn _encode_by_ref_tagged(tag: u32, value: &$ty, buf: &mut impl BufMut) {
                encode_tagged(tag, *value, buf);
            }
            #[inline]
            pub fn merge(
                wire_type: WireType,
                value: &mut $ty,
                buf: &mut impl Buf,
                _ctx: DecodeContext,
            ) -> Result<(), DecodeError> {
                check_wire_type(WireType::Varint, wire_type)?;
                let $fv = decode_varint(buf)?;
                *value = $from_uint64;
                Ok(())
            }

            encode_repeated!($ty, by_value);
            #[inline(always)]
            pub fn encode_packed(tag: u32, values: &[$ty], buf: &mut impl BufMut) {
                if values.is_empty() {
                    return;
                }

                encode_key(tag, WireType::LengthDelimited, buf);
                let len: usize = values.iter()
                    .map(|&$v| encoded_len_varint($to_uint64))
                    .sum();
                encode_varint(len as u64, buf);

                for &$v in values {
                    encode_varint($to_uint64, buf);
                }
            }

            merge_repeated_numeric!($ty, WireType::Varint, merge, merge_repeated);

            #[inline(always)]
            pub fn encoded_len_tagged(tag: u32, $v: $ty) -> usize {
                key_len(tag) + encoded_len_varint($to_uint64)
            }

            #[inline(always)]
            pub fn encoded_len($v: $ty) -> usize {
                encoded_len_varint($to_uint64)
            }

            #[inline(always)]
            pub(crate) fn _encoded_len_by_ref_tagged(tag: u32, value: &$ty) -> usize {
                encoded_len_tagged(tag, *value)
            }

            #[inline(always)]
            pub fn encoded_len_repeated(tag: u32, values: &[$ty]) -> usize {
                key_len(tag) * values.len()
                    + values.iter().map(|&$v| encoded_len_varint($to_uint64)).sum::<usize>()
            }

           #[inline(always)]
            pub fn encoded_len_packed(tag: u32, values: &[$ty]) -> usize {
                if values.is_empty() {
                    0
                } else {
                    let len = values.iter().map(|&$v| encoded_len_varint($to_uint64)).sum::<usize>();
                    key_len(tag) + encoded_len_varint(len as u64) + len
                }
            }
        }
    };
}
varint!(bool, bool,
        to_uint64(value) u64::from(value),
        from_uint64(value) value != 0);
varint!(i32, int32);
varint!(i64, int64);
varint!(u32, uint32);
varint!(u64, uint64);
varint!(i32, sint32,
to_uint64(value) {
    ((value << 1) ^ (value >> 31)) as u32 as u64
},
from_uint64(value) {
    let value = value as u32;
    ((value >> 1) as i32) ^ (-((value & 1) as i32))
});
varint!(i64, sint64,
to_uint64(value) {
    ((value << 1) ^ (value >> 63)) as u64
},
from_uint64(value) {
    ((value >> 1) as i64) ^ (-((value & 1) as i64))
});

/// Macro which emits a module containing a set of encoding functions for a
/// fixed width numeric type.
macro_rules! fixed_width {
    ($ty:ty,
     $width:expr,
     $wire_type:expr,
     $proto_ty:ident,
     $put:ident,
     $get:ident) => {
        pub mod $proto_ty {
            use crate::encoding::*;

            #[inline(always)]
            pub fn encode_tagged(tag: u32, value: $ty, buf: &mut impl BufMut) {
                encode_key(tag, $wire_type, buf);
                buf.$put(value);
            }
            #[inline(always)]
            pub fn encode(value: $ty, buf: &mut impl BufMut) {
                buf.$put(value);
            }

            #[inline(always)]
            pub(crate) fn _encode_by_ref_tagged(tag: u32, value: &$ty, buf: &mut impl BufMut) {
                encode_tagged(tag, *value, buf);
            }
            #[inline]
            pub fn merge(wire_type: WireType, value: &mut $ty, buf: &mut impl Buf, _ctx: DecodeContext) -> Result<(), DecodeError> {
                check_wire_type($wire_type, wire_type)?;
                if buf.remaining() < $width {
                    return Err(DecodeError::new("buffer underflow"));
                }
                *value = buf.$get();
                Ok(())
            }

            encode_repeated!($ty, by_value);
            #[inline(always)]
            pub fn encode_packed(tag: u32, values: &[$ty], buf: &mut impl BufMut) {
                if values.is_empty() {
                    return;
                }

                encode_key(tag, WireType::LengthDelimited, buf);
                let len = values.len() as u64 * $width;
                encode_varint(len, buf);

                for &v in values {
                    buf.$put(v);
                }
            }

            merge_repeated_numeric!($ty, $wire_type, merge, merge_repeated);

            #[inline(always)]
            pub fn encoded_len(_value: $ty) -> usize {
                $width
            }
            #[inline(always)]
            pub fn encoded_len_tagged(tag: u32, _value: $ty) -> usize {
                key_len(tag) + $width
            }
            #[inline(always)]
            pub(crate) fn _encoded_len_by_ref_tagged(tag: u32, _value: &$ty) -> usize {
                encoded_len_tagged(tag, *_value)
            }

            #[inline(always)]
            pub fn encoded_len_repeated(tag: u32, values: &[$ty]) -> usize {
                (key_len(tag) + $width) * values.len()
            }

            #[inline(always)]
            pub fn encoded_len_packed(tag: u32, values: &[$ty]) -> usize {
                if values.is_empty() {
                    0
                } else {
                    let len = $width * values.len();
                    key_len(tag) + encoded_len_varint(len as u64) + len
                }
            }
        }
    };
}

fixed_width!(f32, 4, WireType::ThirtyTwoBit, float, put_f32_le, get_f32_le);
fixed_width!(f64, 8, WireType::SixtyFourBit, double, put_f64_le, get_f64_le);
fixed_width!(u32, 4, WireType::ThirtyTwoBit, fixed32, put_u32_le, get_u32_le);
fixed_width!(u64, 8, WireType::SixtyFourBit, fixed64, put_u64_le, get_u64_le);
fixed_width!(i32, 4, WireType::ThirtyTwoBit, sfixed32, put_i32_le, get_i32_le);
fixed_width!(i64, 8, WireType::SixtyFourBit, sfixed64, put_i64_le, get_i64_le);

macro_rules! length_delimited_encode {
    ($ty:ty) => {
        encode_repeated!($ty, by_ref);
        #[allow(clippy::ptr_arg)]
        #[inline(always)]
        pub fn encoded_len_tagged(tag: u32, value: &$ty) -> usize {
            key_len(tag) + encoded_len_varint(value.len() as u64) + value.len()
        }

        #[allow(clippy::ptr_arg)]
        #[inline(always)]
        pub fn encoded_len(value: &$ty) -> usize {
            encoded_len_varint(value.len() as u64) + value.len()
        }

        #[inline(always)]
        pub(crate) fn _encoded_len_by_ref_tagged(tag: u32, value: &$ty) -> usize {
            encoded_len_tagged(tag, value)
        }

        #[inline(always)]
        pub fn encoded_len_repeated(tag: u32, values: &[$ty]) -> usize {
            key_len(tag) * values.len() + values.iter().map(|v| encoded_len_varint(v.len() as u64) + v.len()).sum::<usize>()
        }
    };
}

macro_rules! length_delimited_decode {
    ($ty:ty) => {
        pub fn merge_repeated(wire_type: WireType, values: &mut Vec<$ty>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
            check_wire_type(WireType::LengthDelimited, wire_type)?;
            let mut value = Default::default();
            merge(wire_type, &mut value, buf, ctx)?;
            values.push(value);
            Ok(())
        }
    };
}

pub mod string {
    use super::Buf;
    use super::BufMut;
    use super::DecodeContext;
    use super::DecodeError;
    use super::WireType;
    use super::bytes;
    use super::check_wire_type;
    use super::encode_key;
    use super::encode_varint;
    use super::encoded_len_varint;
    use super::key_len;
    #[inline(always)]
    pub fn encode_tagged(tag: u32, value: &String, buf: &mut impl BufMut) {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(value.len() as u64, buf);
        buf.put_slice(value.as_bytes());
    }
    #[inline(always)]
    pub fn encode(value: &String, buf: &mut impl BufMut) {
        buf.put_slice(value.as_bytes());
    }
    pub fn _encode_by_ref_tagged(tag: u32, value: &String, buf: &mut impl BufMut) {
        encode_tagged(tag, value, buf);
    }
    #[inline]
    pub fn merge(wire_type: WireType, value: &mut String, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        // ## Unsafety
        //
        // `string::merge` reuses `bytes::merge`, with an additional check of utf-8
        // well-formedness. If the utf-8 is not well-formed, or if any other error occurs, then the
        // string is cleared, so as to avoid leaking a string field with invalid data.
        //
        // This implementation uses the unsafe `String::as_mut_vec` method instead of the safe
        // alternative of temporarily swapping an empty `String` into the field, because it results
        // in up to 10% better performance on the protobuf message decoding benchmarks.
        //
        // It's required when using `String::as_mut_vec` that invalid utf-8 data not be leaked into
        // the backing `String`. To enforce this, even in the event of a panic in `bytes::merge` or
        // in the buf implementation, a drop guard is used.
        unsafe {
            struct DropGuard<'a>(&'a mut Vec<u8>);
            impl Drop for DropGuard<'_> {
                #[inline]
                fn drop(&mut self) {
                    self.0.clear();
                }
            }

            let drop_guard = DropGuard(value.as_mut_vec());
            bytes::merge_one_copy(wire_type, drop_guard.0, buf, ctx)?;
            match str::from_utf8(drop_guard.0) {
                Ok(_) => {
                    // Success; do not clear the bytes.
                    core::mem::forget(drop_guard);
                    Ok(())
                }
                Err(_) => Err(DecodeError::new("invalid string value: data is not UTF-8 encoded")),
            }
        }
    }

    length_delimited_encode!(String);
    length_delimited_decode!(String);

    #[cfg(test)]
    mod test {
        use proptest::prelude::*;

        use super::*;
        use crate::encoding::MAX_TAG;
        use crate::encoding::MIN_TAG;
        use crate::encoding::test::check_type;

        proptest! {
            #[test]
            fn check(value: String, tag in MIN_TAG..=MAX_TAG) {
               check_type(value, tag, WireType::LengthDelimited,
                                        encode_tagged, merge, encoded_len_tagged)?;
            }
            #[test]
            fn check_repeated(value: Vec<String>, tag in MIN_TAG..=MAX_TAG) {
               crate::encoding::test::check_collection_type(value, tag, WireType::LengthDelimited,
                                                   encode_repeated, merge_repeated,
                                                   encoded_len_repeated)?;
            }
        }
    }
}

pub mod bytes {

    use super::Buf;
    use super::BufMut;
    use super::DecodeContext;
    use super::DecodeError;
    use super::WireType;
    use super::check_wire_type;
    use super::decode_varint;
    use super::encode_key;
    use super::encode_varint;
    use super::encoded_len_varint;
    use super::key_len;
    use crate::encoding::BytesAdapterDecode;
    use crate::encoding::BytesAdapterEncode;

    #[inline(always)]
    pub fn encode_tagged(tag: u32, value: &impl BytesAdapterEncode, buf: &mut impl BufMut) {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(value.len() as u64, buf);
        value.append_to(buf);
    }

    #[inline(always)]
    pub fn encode(value: &impl BytesAdapterEncode, buf: &mut impl BufMut) {
        value.append_to(buf);
    }
    #[inline(always)]
    pub fn _encode_by_ref_tagged(tag: u32, value: &impl BytesAdapterEncode, buf: &mut impl BufMut) {
        encode_tagged(tag, value, buf);
    }
    #[inline]
    pub fn merge(wire_type: WireType, value: &mut impl BytesAdapterDecode, buf: &mut impl Buf, _ctx: DecodeContext) -> Result<(), DecodeError> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = decode_varint(buf)?;
        if len > buf.remaining() as u64 {
            return Err(DecodeError::new("buffer underflow"));
        }
        let len = len as usize;

        // Clear the existing value. This follows from the following rule in the encoding guide[1]:
        //
        // > Normally, an encoded message would never have more than one instance of a non-repeated
        // > field. However, parsers are expected to handle the case in which they do. For numeric
        // > types and strings, if the same field appears multiple times, the parser accepts the
        // > last value it sees.
        //
        // [1]: https://protobuf.dev/programming-guides/encoding/#last-one-wins
        //
        // This is intended for A and B both being Bytes so it is zero-copy.
        // Some combinations of A and B types may cause a double-copy,
        // in which case merge_one_copy() should be used instead.
        value.replace_with(buf.copy_to_bytes(len));
        Ok(())
    }
    #[inline]
    pub(super) fn merge_one_copy(wire_type: WireType, value: &mut impl BytesAdapterDecode, buf: &mut impl Buf, _ctx: DecodeContext) -> Result<(), DecodeError> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = decode_varint(buf)?;
        if len > buf.remaining() as u64 {
            return Err(DecodeError::new("buffer underflow"));
        }
        let len = len as usize;

        // If we must copy, make sure to copy only once.
        value.replace_with(buf.take(len));
        Ok(())
    }

    length_delimited_encode!(impl BytesAdapterEncode);
    length_delimited_decode!(impl BytesAdapterDecode);

    #[cfg(test)]
    mod test {
        use ::bytes::Bytes;
        use proptest::prelude::*;

        use super::*;
        use crate::encoding::MAX_TAG;
        use crate::encoding::MIN_TAG;
        proptest! {
            #[test]
            fn check_vec(value: Vec<u8>, tag in MIN_TAG..=MAX_TAG) {
                crate::encoding::test::check_type::<Vec<u8>, Vec<u8>>(value, tag, WireType::LengthDelimited,
                                                            encode_tagged, merge, encoded_len_tagged)?;
            }

            #[test]
            fn check_bytes(value: Vec<u8>, tag in MIN_TAG..=MAX_TAG) {
                let value = Bytes::from(value);
                crate::encoding::test::check_type::<Bytes, Bytes>(value, tag, WireType::LengthDelimited,
                                                        encode_tagged, merge, encoded_len_tagged)?;
            }

            #[test]
            fn check_repeated_vec(value: Vec<Vec<u8>>, tag in MIN_TAG..=MAX_TAG) {
                crate::encoding::test::check_collection_type(value, tag, WireType::LengthDelimited,
                                                   encode_repeated, merge_repeated,
                                                   encoded_len_repeated)?;
            }

            #[test]
            fn check_repeated_bytes(value: Vec<Vec<u8>>, tag in MIN_TAG..=MAX_TAG) {
                let value = value.into_iter().map(Bytes::from).collect();
                crate::encoding::test::check_collection_type(value, tag, WireType::LengthDelimited,
                                                   encode_repeated, merge_repeated,
                                                   encoded_len_repeated)?;
            }
        }
    }
}
