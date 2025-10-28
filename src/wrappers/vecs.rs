use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::EncodeError;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
use crate::encoding::encode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
use crate::encoding::key_len;
use crate::traits::ProtoKind;

impl<T> ProtoShadow for Vec<T>
where
    for<'a> T: ProtoShadow + 'a + ProtoWire<EncodeInput<'a> = &'a T>,
{
    type Sun<'a> = &'a Vec<T>;

    type OwnedSun = Vec<T>;
    type View<'a> = &'a Vec<T>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline]
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value
    }
}

impl<T: ProtoWire> ProtoWire for Vec<T>
where
    for<'a> T: ProtoWire<EncodeInput<'a> = &'a T> + 'a,
{
    type EncodeInput<'a> = &'a Vec<T>;
    const KIND: ProtoKind = ProtoKind::for_vec(&T::KIND);
    const _REPEATED_SUPPORT: Option<&'static str> = Some("Vec");

    #[inline(always)]
    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
        unsafe { Self::encoded_len_impl_raw(value) }
    }

    #[inline(always)]
    fn encoded_len_tagged(&self, tag: u32) -> usize
    where
        for<'b> Self: ProtoWire<EncodeInput<'b> = &'b Self>,
    {
        Self::encoded_len_tagged_impl(&self, tag)
    }

    #[inline(always)]
    fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
        match T::KIND {
            // ---- Packed numeric fields -------------------------------------
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if value.is_empty() {
                    0
                } else {
                    let len = unsafe { Self::encoded_len_impl_raw(value) };
                    key_len(tag) + encoded_len_varint(len as u64) + len
                }
            }

            // ---- Repeated messages -----------------------------------------
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                let len = value.len();
                if len == 0 { 0 } else { key_len(tag) * len + unsafe { Self::encoded_len_impl_raw(value) } }
            }

            ProtoKind::Repeated(_) => {
                unreachable!()
            }
        }
    }

    #[inline]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        match T::KIND {
            // ---- Packed numeric fields -------------------------------------
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => value.iter().map(|value: &T| unsafe { T::encoded_len_impl_raw(&value) }).sum::<usize>(),

            // ---- Repeated messages -----------------------------------------
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => value
                .iter()
                .map(|m| {
                    let len = unsafe { T::encoded_len_impl_raw(&m) };
                    encoded_len_varint(len as u64) + len
                })
                .sum(),

            ProtoKind::Repeated(_) => {
                unreachable!()
            }
        }
    }

    // -------------------------------------------------------------------------
    // encode_raw
    // -------------------------------------------------------------------------
    #[inline]
    fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {
        panic!("Do not call encode_raw_unchecked on Vec<T>")
    }

    #[inline]
    fn encode_with_tag(tag: u32, value: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        match T::KIND {
            // ---- Packed numeric --------------------------------------------
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if value.is_empty() {
                    return Ok(());
                }
                encode_key(tag, WireType::LengthDelimited, buf);
                let body_len = value.iter().map(|value: &T| T::encoded_len_impl(&value)).sum::<usize>();
                encode_varint(body_len as u64, buf);
                for v in value {
                    T::encode_raw_unchecked(v, buf);
                }
                Ok(())
            }

            // ---- Repeated messages -----------------------------------------
            ProtoKind::Bytes | ProtoKind::String | ProtoKind::Message => {
                for m in value {
                    let len = T::encoded_len_impl(&m);
                    encode_key(tag, WireType::LengthDelimited, buf);
                    encode_varint(len as u64, buf);
                    T::encode_raw_unchecked(m, buf);
                }
                Ok(())
            }

            ProtoKind::Repeated(_) => {
                unreachable!()
            }
        }
    }

    // -------------------------------------------------------------------------
    // decode_into
    // -------------------------------------------------------------------------
    #[inline]
    fn decode_into(wire_type: WireType, values: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match T::KIND {
            // ---- Packed numeric or enum ------------------------------------
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if wire_type == WireType::LengthDelimited {
                    let len = decode_varint(buf)? as usize;
                    let mut slice = buf.take(len);
                    while slice.has_remaining() {
                        let mut v = T::proto_default();
                        T::decode_into(T::WIRE_TYPE, &mut v, &mut slice, ctx)?;
                        values.push(v);
                    }
                    buf.advance(len);
                } else {
                    let mut v = T::proto_default();
                    T::decode_into(wire_type, &mut v, buf, ctx)?;
                    values.push(v);
                }
                Ok(())
            }

            // ---- Repeated message ------------------------------------------
            ProtoKind::Bytes | ProtoKind::String | ProtoKind::Message => {
                let mut v = T::proto_default();
                T::decode_into(wire_type, &mut v, buf, ctx)?;
                values.push(v);
                Ok(())
            }

            ProtoKind::Repeated(_) => {
                unreachable!()
            }
        }
    }

    #[inline]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        value.is_empty()
    }

    #[inline]
    fn proto_default() -> Self {
        Vec::new()
    }

    #[inline]
    fn clear(&mut self) {
        Vec::clear(self);
    }
}

/// Implements `ProtoWire` for `Vec<Primitive>` prost-compatible numeric types.
/// This is a specialized version of the generic `Vec<T>` implementation,
/// using packed encoding for numeric primitives.
///
/// - Uses packed `LengthDelimited` encoding
/// - Uses per-element `ProtoWire` encode/decode
/// - Skips unsafe code and generics resolution overhead
macro_rules! impl_proto_wire_vec_for_copy {
    ($($ty:ty => $kind:expr),* $(,)?) => {
        $(
            impl crate::ProtoWire for Vec<$ty> {
                type EncodeInput<'a> = &'a Vec<$ty>;
                const KIND: crate::traits::ProtoKind = $kind;
                const _REPEATED_SUPPORT: Option<&'static str> = Some("Vec");

                // -------------------------------------------------------------------------
                // encoded_len_impl / encoded_len_tagged
                // -------------------------------------------------------------------------
                #[inline(always)]
                fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
                    unsafe { Self::encoded_len_impl_raw(value) }
                }

                #[inline(always)]
                fn encoded_len_tagged(&self, tag: u32) -> usize
                where
                    for<'b> Self: crate::ProtoWire<EncodeInput<'b> = &'b Self>,
                {
                    Self::encoded_len_tagged_impl(&self, tag)
                }

                #[inline(always)]
                fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
                    if value.is_empty() {
                        0
                    } else {
                        let len = unsafe { Self::encoded_len_impl_raw(value) };
                        crate::encoding::key_len(tag)
                            + crate::encoding::encoded_len_varint(len as u64)
                            + len
                    }
                }

                #[inline(always)]
                unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                    value.iter()
                        .map(|v| <$ty as crate::ProtoWire>::encoded_len_impl(&v))
                        .sum::<usize>()
                }

                // -------------------------------------------------------------------------
                // encode_raw
                // -------------------------------------------------------------------------
                #[inline(always)]
                fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl bytes::BufMut) {
                    panic!("Do not call encode_raw_unchecked on Vec<$ty>");
                }

                #[inline(always)]
                fn encode_with_tag(
                    tag: u32,
                    value: Self::EncodeInput<'_>,
                    buf: &mut impl bytes::BufMut,
                ) -> Result<(), crate::EncodeError> {
                    use crate::encoding::{encode_key, encode_varint, WireType};
                    use crate::ProtoWire;

                    if value.is_empty() {
                        return Ok(());
                    }

                    encode_key(tag, WireType::LengthDelimited, buf);
                    let body_len = value.iter()
                        .map(|v| <$ty as ProtoWire>::encoded_len_impl(&v))
                        .sum::<usize>();
                    encode_varint(body_len as u64, buf);

                    for v in value {
                        <$ty as ProtoWire>::encode_raw_unchecked(*v, buf);
                    }

                    Ok(())
                }

                // -------------------------------------------------------------------------
                // decode_into
                // -------------------------------------------------------------------------
                #[inline(always)]
                fn decode_into(
                    wire_type: crate::encoding::WireType,
                    values: &mut Self,
                    buf: &mut impl bytes::Buf,
                    ctx: crate::encoding::DecodeContext,
                ) -> Result<(), crate::DecodeError> {
                    use crate::encoding::{WireType, decode_varint};
                    use crate::ProtoWire;
                    use bytes::Buf;

                    match wire_type {
                        WireType::LengthDelimited => {
                            let len = decode_varint(buf)? as usize;
                            let mut slice = buf.take(len);
                            while slice.has_remaining() {
                                let mut v = <$ty>::default();
                                <$ty as ProtoWire>::decode_into(
                                    <$ty as ProtoWire>::WIRE_TYPE,
                                    &mut v,
                                    &mut slice,
                                    ctx.clone(),
                                )?;
                                values.push(v);
                            }
                            buf.advance(len);
                            Ok(())
                        }
                        other => {
                            let mut v = <$ty>::default();
                            <$ty as ProtoWire>::decode_into(other, &mut v, buf, ctx)?;
                            values.push(v);
                            Ok(())
                        }
                    }
                }

                // -------------------------------------------------------------------------
                // defaults
                // -------------------------------------------------------------------------
                #[inline(always)]
                #[allow(clippy::float_cmp)]
                fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                    value.is_empty()
                }

                #[inline(always)]
                fn proto_default() -> Self {
                    Vec::new()
                }

                #[inline(always)]
                fn clear(&mut self) {
                    self.clear();
                }
            }
        )*
    }
}

// -----------------------------------------------------------------------------
// Apply for all Prost-compatible primitive numeric types
// -----------------------------------------------------------------------------
impl_proto_wire_vec_for_copy! {
    bool  => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::Bool),
    i8    => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I8),
    u16   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::U16),
    i16   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I16),
    u32   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::U32),
    i32   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I32),
    u64   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::U64),
    i64   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I64),
    f32   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::F32),
    f64   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::F64),
}
