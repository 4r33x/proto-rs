use alloc::collections::BTreeSet;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::EncodeInputFromRef;
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

impl<T> ProtoShadow<Self> for BTreeSet<T>
where
    for<'a> T: ProtoShadow<T> + ProtoWire + EncodeInputFromRef<'a> + 'a,
{
    type Sun<'a> = &'a BTreeSet<T>;
    type OwnedSun = BTreeSet<T>;
    type View<'a> = &'a BTreeSet<T>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }
    #[inline]
    fn from_sun(v: Self::Sun<'_>) -> Self::View<'_> {
        v
    }
}

impl<T> ProtoWire for BTreeSet<T>
where
    for<'a> T: ProtoWire + EncodeInputFromRef<'a> + Ord + 'a,
{
    type EncodeInput<'a> = &'a BTreeSet<T>;
    const KIND: ProtoKind = ProtoKind::for_vec(&T::KIND);
    const _REPEATED_SUPPORT: Option<&'static str> = Some("BTreeSet");

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
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if value.is_empty() {
                    0
                } else {
                    let body = unsafe { Self::encoded_len_impl_raw(value) };
                    key_len(tag) + encoded_len_varint(body as u64) + body
                }
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                let n = value.len();
                if n == 0 {
                    0
                } else {
                    key_len(tag) * n + unsafe { Self::encoded_len_impl_raw(value) }
                }
            }
            ProtoKind::Repeated(_) => {
                unreachable!()
            }
        }
    }

    #[inline]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        match T::KIND {
            // packed: body only
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => value
                .iter()
                .map(|v| {
                    let input = T::encode_input_from_ref(v);
                    unsafe { T::encoded_len_impl_raw(&input) }
                })
                .sum(),
            // messages/bytes/string: per element (len varint + body)
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => value
                .iter()
                .map(|m| {
                    let input = T::encode_input_from_ref(m);
                    let len = unsafe { T::encoded_len_impl_raw(&input) };
                    encoded_len_varint(len as u64) + len
                })
                .sum(),
            ProtoKind::Repeated(_) => {
                unreachable!()
            }
        }
    }

    #[inline]
    fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {
        panic!("Do not call encode_raw_unchecked on BTreeSet<T>");
    }

    #[inline]
    fn encode_with_tag(tag: u32, value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if value.is_empty() {
                    return;
                }
                encode_key(tag, WireType::LengthDelimited, buf);
                let body_len = value
                    .iter()
                    .map(|v| {
                        let input = T::encode_input_from_ref(v);
                        T::encoded_len_impl(&input)
                    })
                    .sum::<usize>();
                encode_varint(body_len as u64, buf);
                for v in value {
                    let input = T::encode_input_from_ref(v);
                    T::encode_raw_unchecked(input, buf);
                }
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                for m in value {
                    let input = T::encode_input_from_ref(m);
                    let len = unsafe { T::encoded_len_impl_raw(&input) };
                    encode_key(tag, WireType::LengthDelimited, buf);
                    encode_varint(len as u64, buf);
                    T::encode_raw_unchecked(input, buf);
                }
            }
            ProtoKind::Repeated(_) => {
                unreachable!()
            }
        }
    }

    #[inline]
    fn decode_into(wire_type: WireType, set: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if wire_type == WireType::LengthDelimited {
                    let len = decode_varint(buf)? as usize;
                    let mut slice = buf.take(len);
                    while slice.has_remaining() {
                        let mut v = T::proto_default();
                        T::decode_into(T::WIRE_TYPE, &mut v, &mut slice, ctx)?;
                        set.insert(v);
                    }
                    debug_assert!(!slice.has_remaining());
                } else {
                    let mut v = T::proto_default();
                    T::decode_into(wire_type, &mut v, buf, ctx)?;
                    set.insert(v);
                }
                Ok(())
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                let mut v = T::proto_default();
                T::decode_into(wire_type, &mut v, buf, ctx)?;
                set.insert(v);
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
        BTreeSet::new()
    }
    #[inline]
    fn clear(&mut self) {
        BTreeSet::clear(self);
    }
}

#[cfg(feature = "std")]
mod hashset_impl {
    use std::collections::HashSet;
    use std::hash::BuildHasher;
    use std::hash::Hash;

    use bytes::Buf;
    use bytes::BufMut;

    use crate::DecodeError;
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

    impl<T, S> ProtoShadow<Self> for HashSet<T, S>
    where
        for<'a> T: ProtoShadow<T> + ProtoWire + EncodeInputFromRef<'a> + 'a,
        for<'a> S: BuildHasher + 'a,
    {
        type Sun<'a> = &'a HashSet<T, S>;
        type OwnedSun = HashSet<T, S>;
        type View<'a> = &'a HashSet<T, S>;

        #[inline]
        fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
            Ok(self)
        }
        #[inline]
        fn from_sun(v: Self::Sun<'_>) -> Self::View<'_> {
            v
        }
    }

    impl<T, S> ProtoWire for HashSet<T, S>
    where
        for<'a> T: ProtoWire + EncodeInputFromRef<'a> + Eq + Hash + 'a,
        for<'a> S: BuildHasher + Default + 'a,
    {
        type EncodeInput<'a> = &'a HashSet<T, S>;
        const KIND: ProtoKind = ProtoKind::for_vec(&T::KIND);
        const _REPEATED_SUPPORT: Option<&'static str> = Some("HashSet");

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
                ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                    if value.is_empty() {
                        0
                    } else {
                        let body = unsafe { Self::encoded_len_impl_raw(value) };
                        key_len(tag) + encoded_len_varint(body as u64) + body
                    }
                }
                ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                    let n = value.len();
                    if n == 0 {
                        0
                    } else {
                        key_len(tag) * n + unsafe { Self::encoded_len_impl_raw(value) }
                    }
                }
                ProtoKind::Repeated(_) => {
                    unreachable!()
                }
            }
        }

        #[inline]
        unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
            match T::KIND {
                ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => value
                    .iter()
                    .map(|v| {
                        let input = T::encode_input_from_ref(v);
                        unsafe { T::encoded_len_impl_raw(&input) }
                    })
                    .sum(),

                ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => value
                    .iter()
                    .map(|m| {
                        let input = T::encode_input_from_ref(m);
                        let len = unsafe { T::encoded_len_impl_raw(&input) };
                        encoded_len_varint(len as u64) + len
                    })
                    .sum(),
                ProtoKind::Repeated(_) => {
                    unreachable!()
                }
            }
        }

        #[inline]
        fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {
            panic!("Do not call encode_raw_unchecked on HashSet<T,S>");
        }

        #[inline]
        fn encode_with_tag(tag: u32, value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
            match T::KIND {
                ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                    if value.is_empty() {
                        return;
                    }
                    encode_key(tag, WireType::LengthDelimited, buf);
                    let body_len = value
                        .iter()
                        .map(|v| {
                            let input = T::encode_input_from_ref(v);
                            T::encoded_len_impl(&input)
                        })
                        .sum::<usize>();
                    encode_varint(body_len as u64, buf);
                    for v in value {
                        let input = T::encode_input_from_ref(v);
                        T::encode_raw_unchecked(input, buf);
                    }
                }
                ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                    for m in value {
                        let input = T::encode_input_from_ref(m);
                        let len = unsafe { T::encoded_len_impl_raw(&input) };
                        encode_key(tag, WireType::LengthDelimited, buf);
                        encode_varint(len as u64, buf);
                        T::encode_raw_unchecked(input, buf);
                    }
                }
                ProtoKind::Repeated(_) => {
                    unreachable!()
                }
            }
        }

        #[inline]
        fn decode_into(wire_type: WireType, set: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
            match T::KIND {
                ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                    if wire_type == WireType::LengthDelimited {
                        let len = decode_varint(buf)? as usize;
                        let mut slice = buf.take(len);
                        while slice.has_remaining() {
                            let mut v = T::proto_default();
                            T::decode_into(T::WIRE_TYPE, &mut v, &mut slice, ctx)?;
                            set.insert(v);
                        }
                        debug_assert!(!slice.has_remaining());
                    } else {
                        let mut v = T::proto_default();
                        T::decode_into(wire_type, &mut v, buf, ctx)?;
                        set.insert(v);
                    }
                    Ok(())
                }
                ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                    let mut v = T::proto_default();
                    T::decode_into(wire_type, &mut v, buf, ctx)?;
                    set.insert(v);
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
            HashSet::default()
        }
        #[inline]
        fn clear(&mut self) {
            HashSet::clear(self);
        }
    }
}

/// Implements `ProtoWire` for `BTreeSet<$ty>` for Prost-compatible primitive types.
/// - Uses packed (LengthDelimited) encoding for numeric fields.
/// - Mirrors Prost's packed repeated field logic.
/// - Excludes `f32` and `f64` because they don't implement `Ord`.
macro_rules! impl_proto_wire_btreeset_for_copy {
    ($($ty:ty => $kind:expr),* $(,)?) => {
        $(
            impl crate::ProtoWire for alloc::collections::BTreeSet<$ty> {
                type EncodeInput<'a> = &'a alloc::collections::BTreeSet<$ty>;
                const KIND: crate::traits::ProtoKind = $kind;
                const _REPEATED_SUPPORT: Option<&'static str> = Some("BTreeSet");

                #[inline(always)]
                fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
                    unsafe { Self::encoded_len_impl_raw(value) }
                }

                #[inline(always)]
                fn encoded_len_tagged(&self, tag: u32) -> usize
                where for<'b> Self: crate::ProtoWire<EncodeInput<'b> = &'b Self> {
                    Self::encoded_len_tagged_impl(&self, tag)
                }

                #[inline(always)]
                fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
                    if value.is_empty() { 0 } else {
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

                #[inline(always)]
                fn encode_raw_unchecked(_: Self::EncodeInput<'_>, _: &mut impl bytes::BufMut) {
                    panic!("Do not call encode_raw_unchecked on BTreeSet<$ty>");
                }

                #[inline(always)]
                fn encode_with_tag(
                    tag: u32,
                    value: Self::EncodeInput<'_>,
                    buf: &mut impl bytes::BufMut,
                ) {
                    use crate::encoding::{encode_key, encode_varint, WireType};
                    use crate::ProtoWire;

                    if value.is_empty() {
                        return;
                    }

                    encode_key(tag, WireType::LengthDelimited, buf);
                    let body_len = value.iter()
                        .map(|v| <$ty as ProtoWire>::encoded_len_impl(&v))
                        .sum::<usize>();
                    encode_varint(body_len as u64, buf);

                    for v in value {
                        <$ty as ProtoWire>::encode_raw_unchecked(*v, buf);
                    }

                }

                #[inline(always)]
                fn decode_into(
                    wire_type: crate::encoding::WireType,
                    set: &mut Self,
                    buf: &mut impl bytes::Buf,
                    ctx: crate::encoding::DecodeContext,
                ) -> Result<(), crate::DecodeError> {
                    use crate::encoding::{WireType, decode_varint};
                    use bytes::Buf;

                    match wire_type {
                        WireType::LengthDelimited => {
                            let len = decode_varint(buf)? as usize;
                        let mut slice = buf.take(len);
                        while slice.has_remaining() {
                            let mut v = <$ty>::default();
                            <$ty as crate::ProtoWire>::decode_into(
                                <$ty as crate::ProtoWire>::WIRE_TYPE,
                                &mut v,
                                &mut slice,
                                ctx.clone(),
                            )?;
                            set.insert(v);
                        }
                        debug_assert!(!slice.has_remaining());
                        Ok(())
                    }
                        other => {
                            let mut v = <$ty>::default();
                            <$ty as crate::ProtoWire>::decode_into(other, &mut v, buf, ctx)?;
                            set.insert(v);
                            Ok(())
                        }
                    }
                }

                #[inline(always)]
                fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                    value.is_empty()
                }

                #[inline(always)]
                fn proto_default() -> Self {
                    alloc::collections::BTreeSet::new()
                }

                #[inline(always)]
                fn clear(&mut self) {
                    self.clear();
                }
            }
        )*
    };
}

// Instantiate only for Ord-compatible primitive types
impl_proto_wire_btreeset_for_copy! {
    bool  => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::Bool),
    i8    => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I8),
    u16   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::U16),
    i16   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I16),
    u32   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::U32),
    i32   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I32),
    u64   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::U64),
    i64   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I64),
}

/// Implements `ProtoWire` for `HashSet<$ty, S>` where `S: BuildHasher + Default`.
/// Uses packed encoding for numeric fields, same as Prost.
/// Excludes `f32`/`f64` (no Eq + Hash).
#[cfg(feature = "std")]
macro_rules! impl_proto_wire_hashset_for_copy {
    ($($ty:ty => $kind:expr),* $(,)?) => {
        $(
            impl<S> crate::ProtoWire for std::collections::HashSet<$ty, S>
            where
                for <'a> S: core::hash::BuildHasher + Default + 'a,
            {
                type EncodeInput<'a> = &'a std::collections::HashSet<$ty, S>;
                const KIND: crate::traits::ProtoKind = $kind;
                 const _REPEATED_SUPPORT: Option<&'static str> = Some("HashSet");

                #[inline(always)]
                fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
                    unsafe { Self::encoded_len_impl_raw(value) }
                }

                #[inline(always)]
                fn encoded_len_tagged(&self, tag: u32) -> usize
                where for<'b> Self: crate::ProtoWire<EncodeInput<'b> = &'b Self> {
                    Self::encoded_len_tagged_impl(&self, tag)
                }

                #[inline(always)]
                fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
                    if value.is_empty() { 0 } else {
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

                #[inline(always)]
                fn encode_raw_unchecked(_: Self::EncodeInput<'_>, _: &mut impl bytes::BufMut) {
                    panic!("Do not call encode_raw_unchecked on HashSet<$ty, S>");
                }

                #[inline(always)]
                fn encode_with_tag(
                    tag: u32,
                    value: Self::EncodeInput<'_>,
                    buf: &mut impl bytes::BufMut,
                ) {
                    use crate::encoding::{encode_key, encode_varint, WireType};
                    use crate::ProtoWire;

                    if value.is_empty() {
                        return;
                    }

                    encode_key(tag, WireType::LengthDelimited, buf);
                    let body_len = value.iter()
                        .map(|v| <$ty as ProtoWire>::encoded_len_impl(&v))
                        .sum::<usize>();
                    encode_varint(body_len as u64, buf);

                    for v in value {
                        <$ty as ProtoWire>::encode_raw_unchecked(*v, buf);
                    }

                }

                #[inline(always)]
                fn decode_into(
                    wire_type: crate::encoding::WireType,
                    set: &mut Self,
                    buf: &mut impl bytes::Buf,
                    ctx: crate::encoding::DecodeContext,
                ) -> Result<(), crate::DecodeError> {
                    use crate::encoding::{WireType, decode_varint};
                    use bytes::Buf;

                    match wire_type {
                        WireType::LengthDelimited => {
                            let len = decode_varint(buf)? as usize;
                        let mut slice = buf.take(len);
                        while slice.has_remaining() {
                            let mut v = <$ty>::default();
                            <$ty as crate::ProtoWire>::decode_into(
                                <$ty as crate::ProtoWire>::WIRE_TYPE,
                                &mut v,
                                &mut slice,
                                ctx.clone(),
                            )?;
                            set.insert(v);
                        }
                        debug_assert!(!slice.has_remaining());
                        Ok(())
                    }
                        other => {
                            let mut v = <$ty>::default();
                            <$ty as crate::ProtoWire>::decode_into(other, &mut v, buf, ctx)?;
                            set.insert(v);
                            Ok(())
                        }
                    }
                }

                #[inline(always)]
                fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                    value.is_empty()
                }

                #[inline(always)]
                fn proto_default() -> Self {
                    std::collections::HashSet::with_hasher(S::default())
                }

                #[inline(always)]
                fn clear(&mut self) {
                    self.clear();
                }
            }
        )*
    };
}

#[cfg(feature = "std")]
impl_proto_wire_hashset_for_copy! {
    bool  => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::Bool),
    i8    => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I8),
    u16   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::U16),
    i16   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I16),
    u32   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::U32),
    i32   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I32),
    u64   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::U64),
    i64   => crate::traits::ProtoKind::Primitive(crate::traits::PrimitiveKind::I64),
}
