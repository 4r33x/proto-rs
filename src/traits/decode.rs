use bytes::Buf;

use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_key;
use crate::encoding::decode_varint;
use crate::error::DecodeError;
use crate::traits::ProtoExt;

pub trait ProtoShadowDecode<T> {
    /// Convert shadow -> final owned type.
    fn to_sun(self) -> Result<T, DecodeError>;
}

/// “Message-level” decoder: knows how to dispatch tags inside a message.
pub trait ProtoDecoder: ProtoExt {
    /// User (or macro-generated code) implements this.
    ///
    /// Contract:
    /// - If `tag` is unknown, call `skip_field(tag, wire_type, buf, ctx)` (or equivalent).
    /// - Must fully consume the field payload from `buf` (or skip it).
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    /// Merge an entire message payload
    #[inline]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        // Not work :C :C :C
        // const _: () = {
        //     assert_eq!(Self::WIRE_TYPE, WireType::LengthDelimited);
        // };
        if wire_type != WireType::LengthDelimited {
            return Err(DecodeError::new(format!("invalid wire type {}", Self::KIND.dbg_name())));
        }
        // Check recursion limit once at recursion boundary (not per-field)
        ctx.limit_reached()?;
        let inner_ctx = ctx.enter_recursion();
        let len = decode_varint(buf)? as usize;
        let remaining = buf.remaining();
        if len > remaining {
            return Err(DecodeError::new("buffer underflow"));
        }
        // Use limit-based decoding to avoid Buf::take wrapper overhead
        let limit = remaining - len;
        while buf.remaining() > limit {
            Self::decode_one_field(self, buf, inner_ctx)?;
        }
        if buf.remaining() != limit {
            return Err(DecodeError::new("delimited length exceeded"));
        }
        Ok(())
    }

    ///top level decode entrypoint
    /// Decode a whole message from a buffer (top-level, not length-delimited wrapper).
    #[inline]
    fn decode(mut buf: impl Buf, ctx: DecodeContext) -> Result<Self, DecodeError>
    where
        Self: ProtoDefault,
    {
        // Check recursion limit at top-level entry
        ctx.limit_reached()?;
        let mut sh = <Self as ProtoDefault>::proto_default();
        Self::decode_into(&mut sh, &mut buf, ctx)?;
        Ok(sh)
    }
    /// Decode until `buf` is exhausted. Caller must check ctx.limit_reached() before calling.
    #[inline]
    fn decode_into(value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        while buf.has_remaining() {
            Self::decode_one_field(value, buf, ctx)?;
        }
        Ok(())
    }

    /// Decode one field from the buffer. This is an internal function - `ctx.limit_reached()`
    /// must be checked before the first call to this function (it's checked in `merge` before recursion).
    #[inline]
    fn decode_one_field(value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let (tag, wire) = decode_key(buf)?;
        Self::merge_field(value, tag, wire, buf, ctx)
    }
}

pub trait ProtoDecode: Sized {
    type ShadowDecoded: ProtoDecoder + ProtoExt + ProtoShadowDecode<Self> + ProtoDefault;
    #[inline]
    fn decode(mut buf: impl Buf, ctx: DecodeContext) -> Result<Self, DecodeError> {
        ctx.limit_reached()?;
        let mut sh = <Self::ShadowDecoded as ProtoDefault>::proto_default();
        Self::ShadowDecoded::decode_into(&mut sh, &mut buf, ctx)?;
        Self::post_decode(sh)
    }

    #[inline]
    fn post_decode(value: Self::ShadowDecoded) -> Result<Self, DecodeError> {
        Self::ShadowDecoded::to_sun(value)
    }

    const VALIDATE_WITH_EXT: bool = false;

    #[inline]
    fn validate_with_ext(_value: &mut Self, _ext: &crate::grpc::Extensions) -> Result<(), DecodeError> {
        Ok(())
    }
}

pub trait ProtoFieldMerge: ProtoExt {
    /// Merge a single *field occurrence* into `self` given the field wire type.
    fn merge_value(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;
}

pub trait ProtoDefault: Sized {
    /// default value used for decoding
    /// should be real default value as protobuf spec
    fn proto_default() -> Self;
}

impl<T> ProtoFieldMerge for T
where
    T: ProtoDecoder,
{
    #[inline]
    fn merge_value(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        <T as ProtoDecoder>::merge(self, wire_type, buf, ctx)
    }
}

pub trait DecodeIrBuilder<T> {
    fn build_ir(&self) -> Result<T, ::proto_rs::DecodeError>;
}

#[cfg(all(test, not(feature = "no-recursion-limit")))]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;
    use crate::encoding::encode_varint;
    use crate::encoding::skip_field;
    use crate::traits::ProtoKind;

    #[derive(Debug, Default)]
    struct RecursiveMessage;

    impl ProtoExt for RecursiveMessage {
        const KIND: ProtoKind = ProtoKind::Message;
    }

    impl ProtoDefault for RecursiveMessage {
        fn proto_default() -> Self {
            Self
        }
    }

    impl ProtoDecoder for RecursiveMessage {
        fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
            if tag == 1 {
                Self::merge(value, wire_type, buf, ctx)
            } else {
                skip_field(wire_type, tag, buf, ctx)
            }
        }
    }

    #[test]
    fn nested_messages_enforce_the_recursion_limit() {
        let mut encoded = Vec::new();
        for _ in 0..=crate::RECURSION_LIMIT {
            let mut outer = vec![0x0a];
            encode_varint(encoded.len() as u64, &mut outer);
            outer.extend_from_slice(&encoded);
            encoded = outer;
        }

        let error = <RecursiveMessage as ProtoDecoder>::decode(encoded.as_slice(), DecodeContext::default())
            .expect_err("deeply nested message must exceed recursion limit");
        assert!(error.to_string().contains("recursion limit reached"));
    }
}
