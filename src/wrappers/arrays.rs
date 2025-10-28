//! `ProtoExt` implementations for fixed-size arrays using new trait system

use core::array;
use core::mem::MaybeUninit;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::EncodeError;
use crate::ProtoWire;
use crate::encoding;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
use crate::encoding::encode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
use crate::encoding::key_len;
use crate::traits::ProtoKind;
use crate::traits::ProtoShadow;

// -----------------------------------------------------------------------------
// ProtoShadow for arrays — only provides structural wrapping for borrow/own view
// -----------------------------------------------------------------------------
impl<T: ProtoShadow, const N: usize> ProtoShadow for [T; N] {
    type Sun<'a> = [T::Sun<'a>; N];
    type OwnedSun = [T::OwnedSun; N];
    type View<'a> = [T::View<'a>; N];

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        // Create an uninitialized array
        let mut out: [MaybeUninit<T::OwnedSun>; N] = [const { MaybeUninit::uninit() }; N];

        for (i, elem) in self.into_iter().enumerate() {
            match elem.to_sun() {
                Ok(v) => {
                    out[i].write(v);
                }
                Err(e) => {
                    // Drop initialized elements
                    for j in out.iter_mut().take(i) {
                        unsafe { j.assume_init_drop() };
                    }
                    return Err(e);
                }
            }
        }

        // SAFETY: all N elements are initialized
        Ok(unsafe { assume_init_array(out) })
    }

    #[inline]
    fn from_sun<'a>(v: Self::Sun<'a>) -> Self::View<'a> {
        let mut out: [MaybeUninit<T::View<'a>>; N] = [const { MaybeUninit::uninit() }; N];

        for (idx, x) in v.into_iter().enumerate() {
            out[idx].write(T::from_sun(x));
        }

        unsafe { assume_init_array(out) }
    }
}

#[cfg(feature = "stable")]
#[inline]
#[allow(clippy::needless_pass_by_value)]
unsafe fn assume_init_array<T, const N: usize>(arr: [MaybeUninit<T>; N]) -> [T; N] {
    // SAFETY: Caller guarantees all elements are initialized
    let ptr = (&raw const arr).cast::<[T; N]>();
    unsafe { core::ptr::read(ptr) }
}

#[cfg(not(feature = "stable"))]
#[inline]
#[allow(clippy::needless_pass_by_value)]
unsafe fn assume_init_array<T, const N: usize>(arr: [MaybeUninit<T>; N]) -> [T; N] {
    unsafe { MaybeUninit::array_assume_init(arr) }
}

// -----------------------------------------------------------------------------
// ProtoWire for [T; N]
// -----------------------------------------------------------------------------
impl<T, const N: usize> ProtoWire for [T; N]
where
    for<'a> T: ProtoWire<EncodeInput<'a> = &'a T> + 'a,
{
    type EncodeInput<'a> = &'a [T; N];
    const KIND: ProtoKind = ProtoKind::for_vec(&T::KIND);

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
        for<'b> Self: ProtoWire<EncodeInput<'b> = &'b Self>,
    {
        let input: Self::EncodeInput<'_> = self;
        Self::encoded_len_tagged_impl(&input, tag)
    }

    #[inline(always)]
    fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
        match T::KIND {
            // ---- Packed numeric fields -------------------------------------
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if N == 0 {
                    0
                } else {
                    let len = unsafe { Self::encoded_len_impl_raw(value) };
                    key_len(tag) + encoded_len_varint(len as u64) + len
                }
            }

            // ---- Repeated message/string/bytes ------------------------------
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                if N == 0 {
                    0
                } else {
                    key_len(tag) * N + unsafe { Self::encoded_len_impl_raw(value) }
                }
            }

            ProtoKind::Repeated(_) => const { panic!("unsupported kind in [T; N]") },
        }
    }

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => value.iter().map(|v| unsafe { T::encoded_len_impl_raw(&v) }).sum(),

            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => value
                .iter()
                .map(|m| {
                    let len = unsafe { T::encoded_len_impl_raw(&m) };
                    encoded_len_varint(len as u64) + len
                })
                .sum(),

            ProtoKind::Repeated(_) => const { panic!("unsupported kind in [T; N]") },
        }
    }

    // -------------------------------------------------------------------------
    // encode_raw_unchecked / encode_with_tag
    // -------------------------------------------------------------------------
    #[inline(always)]
    fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {
        panic!("Do not call encode_raw_unchecked on [T; N]");
    }

    #[inline(always)]
    fn encode_with_tag(tag: u32, value: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if N == 0 {
                    return Ok(());
                }
                encode_key(tag, WireType::LengthDelimited, buf);
                let body_len = value.iter().map(|v| T::encoded_len_impl(&v)).sum::<usize>();
                encode_varint(body_len as u64, buf);
                for v in value {
                    T::encode_raw_unchecked(v, buf);
                }
                Ok(())
            }

            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                if value.is_default() {
                    return Ok(());
                }
                for m in value {
                    let len = T::encoded_len_impl(&m);
                    encode_key(tag, WireType::LengthDelimited, buf);
                    encode_varint(len as u64, buf);
                    T::encode_raw_unchecked(m, buf);
                }
                Ok(())
            }

            ProtoKind::Repeated(_) => const { panic!("unsupported kind in [T; N]") },
        }
    }

    // -------------------------------------------------------------------------
    // decode_into
    // -------------------------------------------------------------------------
    #[inline(always)]
    fn decode_into(wire_type: WireType, values: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if wire_type == WireType::LengthDelimited {
                    let len = decode_varint(buf)? as usize;
                    let mut slice = buf.take(len);
                    for v in values.iter_mut() {
                        if !slice.has_remaining() {
                            break;
                        }
                        T::decode_into(T::WIRE_TYPE, v, &mut slice, ctx)?;
                    }
                    buf.advance(len);
                } else {
                    for v in values.iter_mut() {
                        T::decode_into(wire_type, v, buf, ctx)?;
                    }
                }
                Ok(())
            }

            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                for v in values.iter_mut() {
                    T::decode_into(wire_type, v, buf, ctx)?;
                }
                Ok(())
            }

            ProtoKind::Repeated(_) => const { panic!("unsupported kind in [T; N]") },
        }
    }

    // -------------------------------------------------------------------------
    // defaults
    // -------------------------------------------------------------------------
    #[inline(always)]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        value.iter().all(|v| T::is_default_impl(&v))
    }

    #[inline(always)]
    fn proto_default() -> Self {
        array::from_fn(|_| T::proto_default())
    }

    #[inline(always)]
    fn clear(&mut self) {
        for v in self.iter_mut() {
            v.clear();
        }
    }
}

/// Implements `ProtoWire` for fixed-size arrays of primitive Prost copy types,
/// such as `u32`, `i32`, `f64`, etc.
///
/// This version mirrors the exact logic of the `Vec<T>` implementation:
/// - packed numeric encoding (LengthDelimited)
/// - repeated decode loop
/// - uses ProtoKind::Primitive or ProtoKind::SimpleEnum
///
/// It does **not** overlap with the generic `[T; N]` impl because each concrete
/// type is explicitly listed.
macro_rules! impl_proto_wire_array_for_copy {
    ($($ty:ty => $kind:expr),* $(,)?) => {
        $(
            impl<const N: usize> crate::ProtoWire for [$ty; N] {
                type EncodeInput<'a> = &'a [$ty; N];
                const KIND: crate::traits::ProtoKind = $kind;

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
                    if N == 0 {
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
                    // Same as Vec<T>: per-element varint/fixed width cost
                    value.iter().map(|v| <$ty as crate::ProtoWire>::encoded_len_impl(&v)).sum()
                }

                // -------------------------------------------------------------------------
                // encode_raw
                // -------------------------------------------------------------------------
                #[inline(always)]
                fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl bytes::BufMut) {
                    panic!("Do not call encode_raw_unchecked on array");
                }

                #[inline(always)]
                fn encode_with_tag(
                    tag: u32,
                    value: Self::EncodeInput<'_>,
                    buf: &mut impl bytes::BufMut,
                ) -> Result<(), crate::EncodeError> {
                    use crate::encoding::{encode_key, encode_varint, WireType};
                    use crate::ProtoWire;

                    if N == 0 {
                        return Ok(());
                    }

                    // Packed numeric field (LengthDelimited)
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
                            for v in values.iter_mut() {
                                if !slice.has_remaining() { break; }
                                <$ty as ProtoWire>::decode_into(<$ty as ProtoWire>::WIRE_TYPE, v, &mut slice, ctx.clone())?;
                            }
                            buf.advance(len);
                            Ok(())
                        }
                        other => {
                            // Non-packed single values
                            for v in values.iter_mut() {
                                <$ty as ProtoWire>::decode_into(other, v, buf, ctx.clone())?;
                            }
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
                    value.iter().all(|v| *v == <$ty>::default())
                }

                #[inline(always)]
                fn proto_default() -> Self {
                    [<$ty>::default(); N]
                }

                #[inline(always)]
                fn clear(&mut self) {
                    *self = [<$ty>::default(); N];
                }
            }
        )*
    }
}

impl_proto_wire_array_for_copy! {
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

#[inline(always)]
fn is_all_zero(slice: &[u8]) -> bool {
    slice.iter().all(|b: &u8| *b == 0u8)
}

impl<const N: usize> ProtoWire for [u8; N] {
    type EncodeInput<'a> = &'a [u8; N];
    const KIND: ProtoKind = ProtoKind::Bytes;

    #[inline(always)]
    fn encoded_len_impl(v: &Self::EncodeInput<'_>) -> usize {
        // Treat all-zero array as proto default (skip on encode).

        if is_all_zero(*v) { 0 } else { encoded_len_varint(N as u64) + N }
    }

    #[inline(always)]
    fn encoded_len_tagged_impl(v: &Self::EncodeInput<'_>, tag: u32) -> usize {
        let s = v;
        if is_all_zero(*s) { 0 } else { crate::encoding::key_len(tag) + encoded_len_varint(N as u64) + N }
    }

    #[inline(always)]
    unsafe fn encoded_len_impl_raw(_v: &Self::EncodeInput<'_>) -> usize {
        // Raw body size (no skipping, no tag): len varint + N
        encoded_len_varint(N as u64) + N
    }

    #[inline(always)]
    fn encode_raw_unchecked(v: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        // Body only: length + bytes
        encode_varint(N as u64, buf);
        buf.put_slice(v);
    }

    #[inline(always)]
    fn encode_entrypoint(v: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        encode_varint(N as u64, buf);
        buf.put_slice(v);
        Ok(())
    }

    #[inline(always)]
    fn encode_with_tag(tag: u32, v: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        if !is_all_zero(v) {
            encode_key(tag, WireType::LengthDelimited, buf);
            encode_varint(N as u64, buf);
            buf.put_slice(v);
        }
        Ok(())
    }

    #[inline(always)]
    fn decode_into(wire_type: WireType, value: &mut Self, buf: &mut impl Buf, _ctx: DecodeContext) -> Result<(), DecodeError> {
        // Decode length-delimited bytes, copy up to N, zero the rest.
        if wire_type != WireType::LengthDelimited {
            return Err(DecodeError::new("bytes field must be length-delimited"));
        }
        let len = crate::encoding::decode_varint(buf)? as usize;
        if len > buf.remaining() {
            return Err(DecodeError::new("buffer underflow"));
        }
        let to_copy = core::cmp::min(N, len);
        buf.copy_to_slice(&mut value[..to_copy]);
        if len > to_copy {
            // Skip the unused tail from the input if any.
            buf.advance(len - to_copy);
        }
        Ok(())
    }

    #[inline(always)]
    fn proto_default() -> Self {
        [0u8; N]
    }

    #[inline(always)]
    fn is_default_impl(v: &Self::EncodeInput<'_>) -> bool {
        is_all_zero(*v)
    }

    #[inline(always)]
    fn clear(&mut self) {
        self.fill(0);
    }
}
