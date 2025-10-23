#[cfg(feature = "std")]
use core::hash::BuildHasher;
#[cfg(feature = "std")]
use std::collections::HashSet;
#[cfg(feature = "std")]
use std::hash::Hash;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::ViewOf;
use crate::alloc::collections::BTreeSet;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::check_wire_type;
use crate::encoding::decode_varint;
use crate::encoding::encode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
use crate::encoding::key_len;
use crate::encoding::{self};
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

    #[inline]
    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
        match T::KIND {
            // ---- Packed numeric fields -------------------------------------
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if value.is_empty() {
                    0
                } else {
                    let body_len = value.iter().map(|value: &T| T::encoded_len_impl(&value)).sum::<usize>();
                    key_len(0) + encoded_len_varint(body_len as u64) + body_len
                }
            }

            // ---- Repeated string -------------------------------------------
            ProtoKind::String => value.iter().map(|s| key_len(0) + encoded_len_varint(s.len() as u64) + s.len()).sum(),

            // ---- Repeated bytes --------------------------------------------
            ProtoKind::Bytes => value.iter().map(|b| key_len(0) + encoded_len_varint(b.len() as u64) + b.len()).sum(),

            // ---- Repeated messages -----------------------------------------
            ProtoKind::Message => value
                .iter()
                .map(|m| {
                    let len = T::encoded_len_impl(m);
                    key_len(0) + encoded_len_varint(len as u64) + len
                })
                .sum(),

            _ => unreachable!("unsupported kind in Vec<T>"),
        }
    }

    // -------------------------------------------------------------------------
    // encode_raw
    // -------------------------------------------------------------------------
    #[inline]
    fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        match T::KIND {
            // ---- Packed numeric --------------------------------------------
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                let body_len = value.iter().map(|value: &T| T::encoded_len_impl(&value)).sum::<usize>();
                encode_varint(body_len as u64, buf);
                for v in value {
                    T::encode_raw_unchecked(v, buf);
                }
            }

            // ---- Repeated messages -----------------------------------------
            ProtoKind::Bytes | ProtoKind::String | ProtoKind::Message => {
                for m in value {
                    let len = T::encoded_len_impl(&m);
                    encode_key(0, WireType::LengthDelimited, buf);
                    encode_varint(len as u64, buf);
                    T::encode_atomic(m, buf);
                }
            }

            _ => {
                const {
                    // causes a compile error if reached during constant evaluation
                    panic!("unsupported kind in Vec<T>")
                }
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
                    let len = decode_varint(buf)?;
                    let mut slice = buf.take(len as usize);
                    while slice.has_remaining() {
                        let mut v = T::proto_default();
                        T::decode_into(T::WIRE_TYPE, &mut v, &mut slice, ctx)?;
                        values.push(v);
                    }
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
                T::decode_into(wire_type, &mut v, buf, ctx);
                values.push(v);
                Ok(())
            }

            _ => Err(DecodeError::new("unsupported kind for Vec<T>")),
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
        self.clear()
    }
}

impl<T> ProtoExt for Vec<T>
where
    for<'b> T: ProtoExt + ProtoShadow + ProtoWire<EncodeInput<'b> = &'b T> + 'b,
{
    type Shadow<'a> = Vec<T>;

    #[inline(always)]
    fn merge_field(values: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match T::Shadow::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => encoding::merge_repeated_packed_field(tag, wire, value, buf, ctx),
            ProtoKind::Message | ProtoKind::Bytes => {}
            ProtoKind::String => encoding::string::merge_repeated(wire_type, values, buf, ctx),
        }
    }
}
