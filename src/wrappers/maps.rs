use alloc::collections::BTreeMap;

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

impl<K, V> ProtoShadow for BTreeMap<K, V>
where
    for<'a> K: ProtoShadow + ProtoWire<EncodeInput<'a> = &'a K> + 'a,
    for<'a> V: ProtoShadow + ProtoWire<EncodeInput<'a> = &'a V> + 'a,
{
    type Sun<'a> = &'a BTreeMap<K, V>;
    type OwnedSun = BTreeMap<K, V>;
    type View<'a> = &'a BTreeMap<K, V>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }
    #[inline]
    fn from_sun(v: Self::Sun<'_>) -> Self::View<'_> {
        v
    }
}

impl<K, V> ProtoWire for BTreeMap<K, V>
where
    for<'a> K: ProtoWire<EncodeInput<'a> = &'a K> + Ord + 'a,
    for<'a> V: ProtoWire<EncodeInput<'a> = &'a V> + 'a,
{
    type EncodeInput<'a> = &'a BTreeMap<K, V>;
    const KIND: ProtoKind = ProtoKind::Repeated(&V::KIND); // map = repeated entry messages

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
        if value.is_empty() {
            0
        } else {
            value
                .iter()
                .map(|(k, v)| {
                    let entry_len = key_len(1) + K::encoded_len_impl(&k) + key_len(2) + V::encoded_len_impl(&v);
                    key_len(tag) + encoded_len_varint(entry_len as u64) + entry_len
                })
                .sum()
        }
    }

    #[inline]
    unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
        value
            .iter()
            .map(|(k, v)| {
                let entry_len = key_len(1) + unsafe { K::encoded_len_impl_raw(&k) } + key_len(2) + unsafe { V::encoded_len_impl_raw(&v) };
                encoded_len_varint(entry_len as u64) + entry_len
            })
            .sum()
    }

    #[inline]
    fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {
        panic!("Do not call encode_raw_unchecked on BTreeMap<K,V>");
    }

    #[inline]
    fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        for (k, v) in map {
            let entry_len = key_len(1) + K::encoded_len_impl(&k) + key_len(2) + V::encoded_len_impl(&v);
            encode_key(tag, WireType::LengthDelimited, buf);
            encode_varint(entry_len as u64, buf);

            // Key (field 1)
            encode_key(1, K::WIRE_TYPE, buf);
            K::encode_entrypoint(k, buf)?;
            // Value (field 2)
            encode_key(2, V::WIRE_TYPE, buf);
            V::encode_entrypoint(v, buf)?;
        }
        Ok(())
    }

    #[inline]
    fn decode_into(_wire_type: WireType, map: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        // Each entry is a length-delimited submessage
        let len = decode_varint(buf)? as usize;
        let mut slice = buf.take(len);
        let mut key = K::proto_default();
        let mut value = V::proto_default();

        while slice.has_remaining() {
            let (tag, wire) = crate::encoding::decode_key(&mut slice)?;
            match tag {
                1 => K::decode_into(wire, &mut key, &mut slice, ctx)?,
                2 => V::decode_into(wire, &mut value, &mut slice, ctx)?,
                _ => crate::encoding::skip_field(wire, tag, &mut slice, ctx)?,
            }
        }

        debug_assert!(!slice.has_remaining());
        map.insert(key, value);
        Ok(())
    }

    #[inline]
    fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
        value.is_empty()
    }
    #[inline]
    fn proto_default() -> Self {
        BTreeMap::new()
    }
    #[inline]
    fn clear(&mut self) {
        BTreeMap::clear(self);
    }
}

#[cfg(feature = "std")]
mod hashmap_impl {
    use std::collections::HashMap;
    use std::hash::BuildHasher;
    use std::hash::Hash;

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

    impl<K, V, S> ProtoShadow for HashMap<K, V, S>
    where
        for<'a> K: ProtoShadow + ProtoWire<EncodeInput<'a> = &'a K> + 'a,
        for<'a> V: ProtoShadow + ProtoWire<EncodeInput<'a> = &'a V> + 'a,
        for<'a> S: BuildHasher + 'a,
    {
        type Sun<'a> = &'a HashMap<K, V, S>;
        type OwnedSun = HashMap<K, V, S>;
        type View<'a> = &'a HashMap<K, V, S>;

        #[inline]
        fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
            Ok(self)
        }
        #[inline]
        fn from_sun(v: Self::Sun<'_>) -> Self::View<'_> {
            v
        }
    }

    impl<K, V, S> ProtoWire for HashMap<K, V, S>
    where
        for<'a> K: ProtoWire<EncodeInput<'a> = &'a K> + Eq + Hash + 'a,
        for<'a> V: ProtoWire<EncodeInput<'a> = &'a V> + 'a,
        for<'a> S: BuildHasher + Default + 'a,
    {
        type EncodeInput<'a> = &'a HashMap<K, V, S>;
        const KIND: ProtoKind = ProtoKind::Repeated(&V::KIND);

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
            if value.is_empty() {
                0
            } else {
                value
                    .iter()
                    .map(|(k, v)| {
                        let entry_len = key_len(1) + K::encoded_len_impl(&k) + key_len(2) + V::encoded_len_impl(&v);
                        key_len(tag) + encoded_len_varint(entry_len as u64) + entry_len
                    })
                    .sum()
            }
        }

        #[inline]
        unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
            value
                .iter()
                .map(|(k, v)| {
                    let entry_len = key_len(1) + unsafe { K::encoded_len_impl_raw(&k) } + key_len(2) + unsafe { V::encoded_len_impl_raw(&v) };
                    encoded_len_varint(entry_len as u64) + entry_len
                })
                .sum()
        }

        #[inline]
        fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {
            panic!("Do not call encode_raw_unchecked on HashMap<K,V,S>");
        }

        #[inline]
        fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
            for (k, v) in map {
                let entry_len = key_len(1) + K::encoded_len_impl(&k) + key_len(2) + V::encoded_len_impl(&v);
                encode_key(tag, WireType::LengthDelimited, buf);
                encode_varint(entry_len as u64, buf);

                encode_key(1, K::WIRE_TYPE, buf);
                K::encode_entrypoint(k, buf)?;
                encode_key(2, V::WIRE_TYPE, buf);
                V::encode_entrypoint(v, buf)?;
            }
            Ok(())
        }

        #[inline]
        fn decode_into(_wire_type: WireType, map: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
            let len = decode_varint(buf)? as usize;
            let mut slice = buf.take(len);
            let mut key = K::proto_default();
            let mut value = V::proto_default();

            while slice.has_remaining() {
                let (tag, wire) = crate::encoding::decode_key(&mut slice)?;
                match tag {
                    1 => K::decode_into(wire, &mut key, &mut slice, ctx)?,
                    2 => V::decode_into(wire, &mut value, &mut slice, ctx)?,
                    _ => crate::encoding::skip_field(wire, tag, &mut slice, ctx)?,
                }
            }

            debug_assert!(!slice.has_remaining());
            map.insert(key, value);
            Ok(())
        }

        #[inline]
        fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
            value.is_empty()
        }
        #[inline]
        fn proto_default() -> Self {
            HashMap::default()
        }
        #[inline]
        fn clear(&mut self) {
            HashMap::clear(self);
        }
    }
}

macro_rules! impl_primitive_map_btreemap {
    ($K:ty, $V:ty) => {
        impl $crate::ProtoWire for alloc::collections::BTreeMap<$K, $V> {
            type EncodeInput<'a> = &'a alloc::collections::BTreeMap<$K, $V>;
            const KIND: $crate::traits::ProtoKind = $crate::traits::ProtoKind::Repeated(&<$V as $crate::ProtoWire>::KIND);

            #[inline(always)]
            fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
                unsafe { Self::encoded_len_impl_raw(value) }
            }

            #[inline(always)]
            fn encoded_len_tagged(&self, tag: u32) -> usize
            where
                for<'b> Self: $crate::ProtoWire<EncodeInput<'b> = &'b Self>,
            {
                let input: Self::EncodeInput<'_> = self;
                Self::encoded_len_tagged_impl(&input, tag)
            }

            #[inline(always)]
            fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
                if value.is_empty() {
                    0
                } else {
                    value
                        .iter()
                        .map(|(k, v)| {
                            let entry_len =
                                $crate::encoding::key_len(1) + <$K as $crate::ProtoWire>::encoded_len_impl(&k) + $crate::encoding::key_len(2) + <$V as $crate::ProtoWire>::encoded_len_impl(&v);
                            $crate::encoding::key_len(tag) + $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                        })
                        .sum()
                }
            }

            #[inline]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                value
                    .iter()
                    .map(|(k, v)| {
                        let entry_len = $crate::encoding::key_len(1)
                            + unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(&k) }
                            + $crate::encoding::key_len(2)
                            + unsafe { <$V as $crate::ProtoWire>::encoded_len_impl_raw(&v) };
                        $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                    })
                    .sum()
            }

            #[inline]
            fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl bytes::BufMut) {
                // Same as your hand-written impl: never called for maps.
                panic!("Do not call encode_raw_unchecked on BTreeMap<K,V>");
            }

            #[inline]
            fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl bytes::BufMut) -> Result<(), $crate::EncodeError> {
                for (k, v) in map {
                    let entry_len = $crate::encoding::key_len(1) + <$K as $crate::ProtoWire>::encoded_len_impl(&k) + $crate::encoding::key_len(2) + <$V as $crate::ProtoWire>::encoded_len_impl(&v);

                    $crate::encoding::encode_key(tag, $crate::encoding::WireType::LengthDelimited, buf);
                    $crate::encoding::encode_varint(entry_len as u64, buf);

                    // key = 1
                    $crate::encoding::encode_key(1, <$K as $crate::ProtoWire>::WIRE_TYPE, buf);
                    <$K as $crate::ProtoWire>::encode_entrypoint(*k, buf)?;

                    // value = 2
                    $crate::encoding::encode_key(2, <$V as $crate::ProtoWire>::WIRE_TYPE, buf);
                    <$V as $crate::ProtoWire>::encode_entrypoint(*v, buf)?;
                }
                Ok(())
            }

            #[inline]
            fn decode_into(_wire_type: $crate::encoding::WireType, map: &mut Self, buf: &mut impl bytes::Buf, ctx: $crate::encoding::DecodeContext) -> Result<(), $crate::DecodeError> {
                // submessage per entry
                let len = $crate::encoding::decode_varint(buf)? as usize;
                let mut slice = buf.take(len);
                let mut key = <$K as $crate::ProtoWire>::proto_default();
                let mut value = <$V as $crate::ProtoWire>::proto_default();

                while slice.has_remaining() {
                    let (tag, wire) = $crate::encoding::decode_key(&mut slice)?;
                    match tag {
                        1 => <$K as $crate::ProtoWire>::decode_into(wire, &mut key, &mut slice, ctx)?,
                        2 => <$V as $crate::ProtoWire>::decode_into(wire, &mut value, &mut slice, ctx)?,
                        _ => $crate::encoding::skip_field(wire, tag, &mut slice, ctx)?,
                    }
                }

                debug_assert!(!slice.has_remaining());
                map.insert(key, value);
                Ok(())
            }

            #[inline]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                value.is_empty()
            }

            #[inline]
            fn proto_default() -> Self {
                alloc::collections::BTreeMap::new()
            }

            #[inline]
            fn clear(&mut self) {
                alloc::collections::BTreeMap::clear(self);
            }
        }
    };
}

macro_rules! impl_primitive_map_hashmap {
    ($K:ty, $V:ty) => {
        impl<S> $crate::ProtoWire for std::collections::HashMap<$K, $V, S>
        where
            for<'a> S: std::hash::BuildHasher + Default + 'a,
            $K: Eq + std::hash::Hash,
        {
            type EncodeInput<'a> = &'a std::collections::HashMap<$K, $V, S>;
            const KIND: $crate::traits::ProtoKind = $crate::traits::ProtoKind::Repeated(&<$V as $crate::ProtoWire>::KIND);

            #[inline(always)]
            fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
                unsafe { Self::encoded_len_impl_raw(value) }
            }

            #[inline(always)]
            fn encoded_len_tagged(&self, tag: u32) -> usize
            where
                for<'b> Self: $crate::ProtoWire<EncodeInput<'b> = &'b Self>,
            {
                let input: Self::EncodeInput<'_> = self;
                Self::encoded_len_tagged_impl(&input, tag)
            }

            #[inline(always)]
            fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
                if value.is_empty() {
                    0
                } else {
                    value
                        .iter()
                        .map(|(k, v)| {
                            let entry_len =
                                $crate::encoding::key_len(1) + <$K as $crate::ProtoWire>::encoded_len_impl(&k) + $crate::encoding::key_len(2) + <$V as $crate::ProtoWire>::encoded_len_impl(&v);
                            $crate::encoding::key_len(tag) + $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                        })
                        .sum()
                }
            }

            #[inline]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                value
                    .iter()
                    .map(|(k, v)| {
                        let entry_len = $crate::encoding::key_len(1)
                            + unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(&k) }
                            + $crate::encoding::key_len(2)
                            + unsafe { <$V as $crate::ProtoWire>::encoded_len_impl_raw(&v) };
                        $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                    })
                    .sum()
            }

            #[inline]
            fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl bytes::BufMut) {
                // Same invariant as your code: do not call for maps.
                panic!("Do not call encode_raw_unchecked on HashMap<K,V,S>");
            }

            #[inline]
            fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl bytes::BufMut) -> Result<(), $crate::EncodeError> {
                for (k, v) in map {
                    let entry_len = $crate::encoding::key_len(1) + <$K as $crate::ProtoWire>::encoded_len_impl(&k) + $crate::encoding::key_len(2) + <$V as $crate::ProtoWire>::encoded_len_impl(&v);

                    $crate::encoding::encode_key(tag, $crate::encoding::WireType::LengthDelimited, buf);
                    $crate::encoding::encode_varint(entry_len as u64, buf);

                    $crate::encoding::encode_key(1, <$K as $crate::ProtoWire>::WIRE_TYPE, buf);
                    <$K as $crate::ProtoWire>::encode_entrypoint(*k, buf)?;

                    $crate::encoding::encode_key(2, <$V as $crate::ProtoWire>::WIRE_TYPE, buf);
                    <$V as $crate::ProtoWire>::encode_entrypoint(*v, buf)?;
                }
                Ok(())
            }

            #[inline]
            fn decode_into(_wire_type: $crate::encoding::WireType, map: &mut Self, buf: &mut impl bytes::Buf, ctx: $crate::encoding::DecodeContext) -> Result<(), $crate::DecodeError> {
                let len = $crate::encoding::decode_varint(buf)? as usize;
                let mut slice = buf.take(len);
                let mut key = <$K as $crate::ProtoWire>::proto_default();
                let mut value = <$V as $crate::ProtoWire>::proto_default();

                while slice.has_remaining() {
                    let (tag, wire) = $crate::encoding::decode_key(&mut slice)?;
                    match tag {
                        1 => <$K as $crate::ProtoWire>::decode_into(wire, &mut key, &mut slice, ctx)?,
                        2 => <$V as $crate::ProtoWire>::decode_into(wire, &mut value, &mut slice, ctx)?,
                        _ => $crate::encoding::skip_field(wire, tag, &mut slice, ctx)?,
                    }
                }

                debug_assert!(!slice.has_remaining());
                map.insert(key, value);
                Ok(())
            }

            #[inline]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                value.is_empty()
            }

            #[inline]
            fn proto_default() -> Self {
                std::collections::HashMap::default()
            }

            #[inline]
            fn clear(&mut self) {
                std::collections::HashMap::clear(self);
            }
        }
    };
}

macro_rules! impl_all_primitive_maps {
    () => {
        // values may be any primitive (your list)
        macro_rules! __for_each_val {
            ($mac:ident, $K:ty) => {
                $mac!($K, bool);
                $mac!($K, i8);
                $mac!($K, i16);
                $mac!($K, i32);
                $mac!($K, i64);
                $mac!($K, u8);
                $mac!($K, u16);
                $mac!($K, u32);
                $mac!($K, u64);
                $mac!($K, f32);
                $mac!($K, f64);
            };
        }

        // emit both container impls for one (K,V)
        macro_rules! __emit_both {
            ($K:ty, $V:ty) => {
                impl_primitive_map_btreemap!($K, $V);
                impl_primitive_map_hashmap!($K, $V);
            };
        }

        // keys: restrict to protobuf-valid numeric keys that satisfy Eq/Hash/Ord
        __for_each_val!(__emit_both, bool);
        __for_each_val!(__emit_both, i8);
        __for_each_val!(__emit_both, i16);
        __for_each_val!(__emit_both, i32);
        __for_each_val!(__emit_both, i64);
        __for_each_val!(__emit_both, u8);
        __for_each_val!(__emit_both, u16);
        __for_each_val!(__emit_both, u32);
        __for_each_val!(__emit_both, u64);
    };
}

impl_all_primitive_maps!();

macro_rules! impl_string_map_btreemap {
    ($V:ty) => {
        impl $crate::ProtoWire for alloc::collections::BTreeMap<String, $V> {
            type EncodeInput<'a> = &'a alloc::collections::BTreeMap<String, $V>;
            const KIND: $crate::traits::ProtoKind = $crate::traits::ProtoKind::Repeated(&<$V as $crate::ProtoWire>::KIND);

            #[inline(always)]
            fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
                unsafe { Self::encoded_len_impl_raw(value) }
            }

            #[inline(always)]
            fn encoded_len_tagged(&self, tag: u32) -> usize
            where
                for<'b> Self: $crate::ProtoWire<EncodeInput<'b> = &'b Self>,
            {
                let input: Self::EncodeInput<'_> = self;
                Self::encoded_len_tagged_impl(&input, tag)
            }

            #[inline(always)]
            fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
                if value.is_empty() {
                    0
                } else {
                    value
                        .iter()
                        .map(|(k, v)| {
                            let key_len = $crate::encoding::key_len(1) + $crate::encoding::encoded_len_varint(k.len() as u64) + k.len();
                            let val_len = $crate::encoding::key_len(2) + <$V as $crate::ProtoWire>::encoded_len_impl(&v);
                            let entry_len = key_len + val_len;
                            $crate::encoding::key_len(tag) + $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                        })
                        .sum()
                }
            }

            #[inline]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                value
                    .iter()
                    .map(|(k, v)| {
                        let key_len = $crate::encoding::key_len(1) + $crate::encoding::encoded_len_varint(k.len() as u64) + k.len();
                        let val_len = $crate::encoding::key_len(2) + unsafe { <$V as $crate::ProtoWire>::encoded_len_impl_raw(&v) };
                        let entry_len = key_len + val_len;
                        $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                    })
                    .sum()
            }

            #[inline]
            fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl bytes::BufMut) {
                panic!("Do not call encode_raw_unchecked on BTreeMap<String,V>");
            }

            #[inline]
            fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl bytes::BufMut) -> Result<(), $crate::EncodeError> {
                for (k, v) in map {
                    let key_len = $crate::encoding::key_len(1) + $crate::encoding::encoded_len_varint(k.len() as u64) + k.len();
                    let val_len = $crate::encoding::key_len(2) + <$V as $crate::ProtoWire>::encoded_len_impl(&v);
                    let entry_len = key_len + val_len;

                    $crate::encoding::encode_key(tag, $crate::encoding::WireType::LengthDelimited, buf);
                    $crate::encoding::encode_varint(entry_len as u64, buf);

                    // Key = 1 (string)
                    $crate::encoding::encode_key(1, $crate::encoding::WireType::LengthDelimited, buf);
                    $crate::encoding::encode_varint(k.len() as u64, buf);
                    buf.put_slice(k.as_bytes());

                    // Value = 2
                    $crate::encoding::encode_key(2, <$V as $crate::ProtoWire>::WIRE_TYPE, buf);
                    <$V as $crate::ProtoWire>::encode_entrypoint(*v, buf)?;
                }
                Ok(())
            }

            #[inline]
            fn decode_into(_wire_type: $crate::encoding::WireType, map: &mut Self, buf: &mut impl bytes::Buf, ctx: $crate::encoding::DecodeContext) -> Result<(), $crate::DecodeError> {
                let len = $crate::encoding::decode_varint(buf)? as usize;
                let mut slice = buf.take(len);
                let mut key = String::new();
                let mut value = <$V as $crate::ProtoWire>::proto_default();

                while slice.has_remaining() {
                    let (tag, wire) = $crate::encoding::decode_key(&mut slice)?;
                    match tag {
                        1 => {
                            let slen = $crate::encoding::decode_varint(&mut slice)? as usize;
                            let mut bytes = vec![0u8; slen];
                            slice.copy_to_slice(&mut bytes);
                            key = String::from_utf8(bytes).map_err(|_| $crate::DecodeError::new("invalid UTF-8 string key"))?;
                        }
                        2 => <$V as $crate::ProtoWire>::decode_into(wire, &mut value, &mut slice, ctx)?,
                        _ => $crate::encoding::skip_field(wire, tag, &mut slice, ctx)?,
                    }
                }

                debug_assert!(!slice.has_remaining());
                map.insert(key, value);
                Ok(())
            }

            #[inline]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                value.is_empty()
            }

            #[inline]
            fn proto_default() -> Self {
                alloc::collections::BTreeMap::new()
            }

            #[inline]
            fn clear(&mut self) {
                alloc::collections::BTreeMap::clear(self);
            }
        }
    };
}

macro_rules! impl_string_map_hashmap {
    ($V:ty) => {
        impl<S> $crate::ProtoWire for std::collections::HashMap<String, $V, S>
        where
            for<'a> S: std::hash::BuildHasher + Default + 'a,
        {
            type EncodeInput<'a> = &'a std::collections::HashMap<String, $V, S>;
            const KIND: $crate::traits::ProtoKind = $crate::traits::ProtoKind::Repeated(&<$V as $crate::ProtoWire>::KIND);

            #[inline(always)]
            fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
                unsafe { Self::encoded_len_impl_raw(value) }
            }

            #[inline(always)]
            fn encoded_len_tagged(&self, tag: u32) -> usize
            where
                for<'b> Self: $crate::ProtoWire<EncodeInput<'b> = &'b Self>,
            {
                let input: Self::EncodeInput<'_> = self;
                Self::encoded_len_tagged_impl(&input, tag)
            }

            #[inline(always)]
            fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
                if value.is_empty() {
                    0
                } else {
                    value
                        .iter()
                        .map(|(k, v)| {
                            let key_len = $crate::encoding::key_len(1) + $crate::encoding::encoded_len_varint(k.len() as u64) + k.len();
                            let val_len = $crate::encoding::key_len(2) + <$V as $crate::ProtoWire>::encoded_len_impl(&v);
                            let entry_len = key_len + val_len;
                            $crate::encoding::key_len(tag) + $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                        })
                        .sum()
                }
            }

            #[inline]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                value
                    .iter()
                    .map(|(k, v)| {
                        let key_len = $crate::encoding::key_len(1) + $crate::encoding::encoded_len_varint(k.len() as u64) + k.len();
                        let val_len = $crate::encoding::key_len(2) + unsafe { <$V as $crate::ProtoWire>::encoded_len_impl_raw(&v) };
                        let entry_len = key_len + val_len;
                        $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                    })
                    .sum()
            }

            #[inline]
            fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl bytes::BufMut) {
                panic!("Do not call encode_raw_unchecked on HashMap<String,V,S>");
            }

            #[inline]
            fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl bytes::BufMut) -> Result<(), $crate::EncodeError> {
                for (k, v) in map {
                    let key_len = $crate::encoding::key_len(1) + $crate::encoding::encoded_len_varint(k.len() as u64) + k.len();
                    let val_len = $crate::encoding::key_len(2) + <$V as $crate::ProtoWire>::encoded_len_impl(&v);
                    let entry_len = key_len + val_len;

                    $crate::encoding::encode_key(tag, $crate::encoding::WireType::LengthDelimited, buf);
                    $crate::encoding::encode_varint(entry_len as u64, buf);

                    // Key = 1 (string)
                    $crate::encoding::encode_key(1, $crate::encoding::WireType::LengthDelimited, buf);
                    $crate::encoding::encode_varint(k.len() as u64, buf);
                    buf.put_slice(k.as_bytes());

                    // Value = 2
                    $crate::encoding::encode_key(2, <$V as $crate::ProtoWire>::WIRE_TYPE, buf);
                    <$V as $crate::ProtoWire>::encode_entrypoint(*v, buf)?;
                }
                Ok(())
            }

            #[inline]
            fn decode_into(_wire_type: $crate::encoding::WireType, map: &mut Self, buf: &mut impl bytes::Buf, ctx: $crate::encoding::DecodeContext) -> Result<(), $crate::DecodeError> {
                let len = $crate::encoding::decode_varint(buf)? as usize;
                let mut slice = buf.take(len);
                let mut key = String::new();
                let mut value = <$V as $crate::ProtoWire>::proto_default();

                while slice.has_remaining() {
                    let (tag, wire) = $crate::encoding::decode_key(&mut slice)?;
                    match tag {
                        1 => {
                            let slen = $crate::encoding::decode_varint(&mut slice)? as usize;
                            let mut bytes = vec![0u8; slen];
                            slice.copy_to_slice(&mut bytes);
                            key = String::from_utf8(bytes).map_err(|_| $crate::DecodeError::new("invalid UTF-8 string key"))?;
                        }
                        2 => <$V as $crate::ProtoWire>::decode_into(wire, &mut value, &mut slice, ctx)?,
                        _ => $crate::encoding::skip_field(wire, tag, &mut slice, ctx)?,
                    }
                }

                debug_assert!(!slice.has_remaining());
                map.insert(key, value);
                Ok(())
            }

            #[inline]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                value.is_empty()
            }

            #[inline]
            fn proto_default() -> Self {
                std::collections::HashMap::default()
            }

            #[inline]
            fn clear(&mut self) {
                std::collections::HashMap::clear(self);
            }
        }
    };
}

impl_string_map_btreemap!(bool);
impl_string_map_btreemap!(i8);
impl_string_map_btreemap!(i16);
impl_string_map_btreemap!(i32);
impl_string_map_btreemap!(i64);
impl_string_map_btreemap!(u8);
impl_string_map_btreemap!(u16);
impl_string_map_btreemap!(u32);
impl_string_map_btreemap!(u64);
impl_string_map_btreemap!(f32);
impl_string_map_btreemap!(f64);

impl_string_map_hashmap!(bool);
impl_string_map_hashmap!(i8);
impl_string_map_hashmap!(i16);
impl_string_map_hashmap!(i32);
impl_string_map_hashmap!(i64);
impl_string_map_hashmap!(u8);
impl_string_map_hashmap!(u16);
impl_string_map_hashmap!(u32);
impl_string_map_hashmap!(u64);
impl_string_map_hashmap!(f32);
impl_string_map_hashmap!(f64);

/// Implements `ProtoWire` for `BTreeMap<$K, V>`
/// where `$K` is a copy-primitive key (`EncodeInput<'a> = $K`)
/// and `V` is any type implementing `ProtoWire`.
macro_rules! impl_copykey_map_btreemap {
    ($K:ty) => {
        impl<V> $crate::ProtoWire for alloc::collections::BTreeMap<$K, V>
        where
            for<'a> $K: $crate::ProtoWire<EncodeInput<'a> = $K> + Ord + 'a,
            for<'a> V: $crate::ProtoWire<EncodeInput<'a> = &'a V> + 'a,
        {
            type EncodeInput<'a> = &'a alloc::collections::BTreeMap<$K, V>;
            const KIND: $crate::traits::ProtoKind = $crate::traits::ProtoKind::Repeated(&<V as $crate::ProtoWire>::KIND);

            #[inline(always)]
            fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
                unsafe { Self::encoded_len_impl_raw(value) }
            }

            #[inline(always)]
            fn encoded_len_tagged(&self, tag: u32) -> usize
            where
                for<'b> Self: $crate::ProtoWire<EncodeInput<'b> = &'b Self>,
            {
                let input: Self::EncodeInput<'_> = self;
                Self::encoded_len_tagged_impl(&input, tag)
            }

            #[inline(always)]
            fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
                if value.is_empty() {
                    0
                } else {
                    value
                        .iter()
                        .map(|(k, v)| {
                            let entry_len =
                                $crate::encoding::key_len(1) + <$K as $crate::ProtoWire>::encoded_len_impl(k) + $crate::encoding::key_len(2) + <V as $crate::ProtoWire>::encoded_len_impl(&v);
                            $crate::encoding::key_len(tag) + $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                        })
                        .sum()
                }
            }

            #[inline]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                value
                    .iter()
                    .map(|(k, v)| {
                        let entry_len = $crate::encoding::key_len(1)
                            + unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(k) }
                            + $crate::encoding::key_len(2)
                            + unsafe { <V as $crate::ProtoWire>::encoded_len_impl_raw(&v) };
                        $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                    })
                    .sum()
            }

            #[inline]
            fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl bytes::BufMut) {
                panic!("Do not call encode_raw_unchecked on BTreeMap<K,V>");
            }

            #[inline]
            fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl bytes::BufMut) -> Result<(), $crate::EncodeError> {
                for (k, v) in map {
                    let entry_len = $crate::encoding::key_len(1) + <$K as $crate::ProtoWire>::encoded_len_impl(k) + $crate::encoding::key_len(2) + <V as $crate::ProtoWire>::encoded_len_impl(&v);
                    $crate::encoding::encode_key(tag, $crate::encoding::WireType::LengthDelimited, buf);
                    $crate::encoding::encode_varint(entry_len as u64, buf);

                    // Key (by value)
                    $crate::encoding::encode_key(1, <$K as $crate::ProtoWire>::WIRE_TYPE, buf);
                    <$K as $crate::ProtoWire>::encode_entrypoint(*k, buf)?;

                    // Value (by ref)
                    $crate::encoding::encode_key(2, <V as $crate::ProtoWire>::WIRE_TYPE, buf);
                    <V as $crate::ProtoWire>::encode_entrypoint(v, buf)?;
                }
                Ok(())
            }

            #[inline]
            fn decode_into(_wire_type: $crate::encoding::WireType, map: &mut Self, buf: &mut impl bytes::Buf, ctx: $crate::encoding::DecodeContext) -> Result<(), $crate::DecodeError> {
                let len = $crate::encoding::decode_varint(buf)? as usize;
                let mut slice = buf.take(len);
                let mut key = <$K as $crate::ProtoWire>::proto_default();
                let mut value = <V as $crate::ProtoWire>::proto_default();

                while slice.has_remaining() {
                    let (tag, wire) = $crate::encoding::decode_key(&mut slice)?;
                    match tag {
                        1 => <$K as $crate::ProtoWire>::decode_into(wire, &mut key, &mut slice, ctx)?,
                        2 => <V as $crate::ProtoWire>::decode_into(wire, &mut value, &mut slice, ctx)?,
                        _ => $crate::encoding::skip_field(wire, tag, &mut slice, ctx)?,
                    }
                }

                debug_assert!(!slice.has_remaining());
                map.insert(key, value);
                Ok(())
            }

            #[inline(always)]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                value.is_empty()
            }

            #[inline(always)]
            fn proto_default() -> Self {
                alloc::collections::BTreeMap::new()
            }

            #[inline(always)]
            fn clear(&mut self) {
                alloc::collections::BTreeMap::clear(self);
            }
        }
    };
}

/// Implements `ProtoWire` for `HashMap<$K, V, S>`
/// where `$K` is a copy-primitive key (`EncodeInput<'a> = $K`)
/// and `V` is any `ProtoWire`.
macro_rules! impl_copykey_map_hashmap {
    ($K:ty) => {
        impl<V, S> $crate::ProtoWire for std::collections::HashMap<$K, V, S>
        where
            for<'a> S: std::hash::BuildHasher + Default + 'a,
            for<'a> $K: $crate::ProtoWire<EncodeInput<'a> = $K> + Eq + std::hash::Hash + 'a,
            for<'a> V: $crate::ProtoWire<EncodeInput<'a> = &'a V> + 'a,
        {
            type EncodeInput<'a> = &'a std::collections::HashMap<$K, V, S>;
            const KIND: $crate::traits::ProtoKind = $crate::traits::ProtoKind::Repeated(&<V as $crate::ProtoWire>::KIND);

            #[inline(always)]
            fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
                unsafe { Self::encoded_len_impl_raw(value) }
            }

            #[inline(always)]
            fn encoded_len_tagged(&self, tag: u32) -> usize
            where
                for<'b> Self: $crate::ProtoWire<EncodeInput<'b> = &'b Self>,
            {
                let input: Self::EncodeInput<'_> = self;
                Self::encoded_len_tagged_impl(&input, tag)
            }

            #[inline(always)]
            fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
                if value.is_empty() {
                    0
                } else {
                    value
                        .iter()
                        .map(|(k, v)| {
                            let entry_len =
                                $crate::encoding::key_len(1) + <$K as $crate::ProtoWire>::encoded_len_impl(k) + $crate::encoding::key_len(2) + <V as $crate::ProtoWire>::encoded_len_impl(&v);
                            $crate::encoding::key_len(tag) + $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                        })
                        .sum()
                }
            }

            #[inline]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                value
                    .iter()
                    .map(|(k, v)| {
                        let entry_len = $crate::encoding::key_len(1)
                            + unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(k) }
                            + $crate::encoding::key_len(2)
                            + unsafe { <V as $crate::ProtoWire>::encoded_len_impl_raw(&v) };
                        $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                    })
                    .sum()
            }

            #[inline]
            fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl bytes::BufMut) {
                panic!("Do not call encode_raw_unchecked on HashMap<K,V,S>");
            }

            #[inline]
            fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl bytes::BufMut) -> Result<(), $crate::EncodeError> {
                for (k, v) in map {
                    let entry_len = $crate::encoding::key_len(1) + <$K as $crate::ProtoWire>::encoded_len_impl(k) + $crate::encoding::key_len(2) + <V as $crate::ProtoWire>::encoded_len_impl(&v);
                    $crate::encoding::encode_key(tag, $crate::encoding::WireType::LengthDelimited, buf);
                    $crate::encoding::encode_varint(entry_len as u64, buf);

                    // Key (by value)
                    $crate::encoding::encode_key(1, <$K as $crate::ProtoWire>::WIRE_TYPE, buf);
                    <$K as $crate::ProtoWire>::encode_entrypoint(*k, buf)?;

                    // Value (by ref)
                    $crate::encoding::encode_key(2, <V as $crate::ProtoWire>::WIRE_TYPE, buf);
                    <V as $crate::ProtoWire>::encode_entrypoint(v, buf)?;
                }
                Ok(())
            }

            #[inline]
            fn decode_into(_wire_type: $crate::encoding::WireType, map: &mut Self, buf: &mut impl bytes::Buf, ctx: $crate::encoding::DecodeContext) -> Result<(), $crate::DecodeError> {
                let len = $crate::encoding::decode_varint(buf)? as usize;
                let mut slice = buf.take(len);
                let mut key = <$K as $crate::ProtoWire>::proto_default();
                let mut value = <V as $crate::ProtoWire>::proto_default();

                while slice.has_remaining() {
                    let (tag, wire) = $crate::encoding::decode_key(&mut slice)?;
                    match tag {
                        1 => <$K as $crate::ProtoWire>::decode_into(wire, &mut key, &mut slice, ctx)?,
                        2 => <V as $crate::ProtoWire>::decode_into(wire, &mut value, &mut slice, ctx)?,
                        _ => $crate::encoding::skip_field(wire, tag, &mut slice, ctx)?,
                    }
                }

                debug_assert!(!slice.has_remaining());
                map.insert(key, value);
                Ok(())
            }

            #[inline(always)]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                value.is_empty()
            }

            #[inline(always)]
            fn proto_default() -> Self {
                std::collections::HashMap::default()
            }

            #[inline(always)]
            fn clear(&mut self) {
                std::collections::HashMap::clear(self);
            }
        }
    };
}

impl_copykey_map_btreemap!(u8);
impl_copykey_map_btreemap!(u16);
impl_copykey_map_btreemap!(u32);
impl_copykey_map_btreemap!(u64);
impl_copykey_map_btreemap!(i8);
impl_copykey_map_btreemap!(i16);
impl_copykey_map_btreemap!(i32);
impl_copykey_map_btreemap!(i64);
impl_copykey_map_btreemap!(bool);

impl_copykey_map_hashmap!(u8);
impl_copykey_map_hashmap!(u16);
impl_copykey_map_hashmap!(u32);
impl_copykey_map_hashmap!(u64);
impl_copykey_map_hashmap!(i8);
impl_copykey_map_hashmap!(i16);
impl_copykey_map_hashmap!(i32);
impl_copykey_map_hashmap!(i64);
impl_copykey_map_hashmap!(bool);
