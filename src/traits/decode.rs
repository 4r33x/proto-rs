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

pub trait ProtoDecoder: ProtoExt {
    type Shadow: ProtoDecoder + ProtoExt;
    /// default value used for decoding
    /// should be real default value as protobuf spec
    fn proto_default() -> Self;
    /// Reset to default.
    fn clear(&mut self);

    /// User (or macro-generated code) implements this.
    ///
    /// Contract:
    /// - If `tag` is unknown, call `skip_field(tag, wire_type, buf, ctx)` (or equivalent).
    /// - Must fully consume the field payload from `buf` (or skip it).
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    #[inline(always)]
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
        let len = decode_varint(buf)? as usize;
        let remaining = buf.remaining();
        if len > remaining {
            return Err(DecodeError::new("buffer underflow"));
        }
        // Use limit-based decoding to avoid Buf::take wrapper overhead
        let limit = remaining - len;
        while buf.remaining() > limit {
            Self::decode_one_field(self, buf, ctx)?;
        }
        Ok(())
    }

    ///top level decode entrypoint
    #[inline(always)]
    fn decode(mut buf: impl Buf, ctx: DecodeContext) -> Result<Self, DecodeError> {
        // Check recursion limit at top-level entry
        ctx.limit_reached()?;
        let mut sh = Self::proto_default();
        Self::decode_into(&mut sh, &mut buf, ctx)?;
        Ok(sh)
    }
    /// Decode until `buf` is exhausted. Caller must check ctx.limit_reached() before calling.
    #[inline(always)]
    fn decode_into(value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        while buf.has_remaining() {
            Self::decode_one_field(value, buf, ctx)?;
        }
        Ok(())
    }

    /// Decode one field from the buffer. This is an internal function - `ctx.limit_reached()`
    /// must be checked before the first call to this function (it's checked in `merge` before recursion).
    #[inline(always)]
    fn decode_one_field(value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        let (tag, wire) = decode_key(buf)?;
        if tag == 0 {
            return Err(DecodeError::new("invalid tag 0"));
        }
        Self::merge_field(value, tag, wire, buf, ctx)
    }
}

pub trait ProtoDecode: Sized {
    type ShadowDecoded: ProtoDecoder + ProtoExt + ProtoShadowDecode<Self>;
    #[inline(always)]
    fn decode(mut buf: impl Buf, ctx: DecodeContext) -> Result<Self, DecodeError> {
        let mut sh = Self::ShadowDecoded::proto_default();
        Self::ShadowDecoded::decode_into(&mut sh, &mut buf, ctx)?;
        Self::post_decode(sh)
    }

    #[inline(always)]
    fn post_decode(value: Self::ShadowDecoded) -> Result<Self, DecodeError> {
        Self::ShadowDecoded::to_sun(value)
    }

    const VALIDATE_WITH_EXT: bool = false;

    #[cfg(feature = "tonic")]
    #[inline(always)]
    fn validate_with_ext(_value: &mut Self, _ext: &tonic::Extensions) -> Result<(), DecodeError> {
        Ok(())
    }
}
