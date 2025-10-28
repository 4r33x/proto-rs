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
            K::encode_raw_unchecked(k, buf);
            // Value (field 2)
            encode_key(2, V::WIRE_TYPE, buf);
            V::encode_raw_unchecked(v, buf);
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

        buf.advance(len);
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
                K::encode_raw_unchecked(k, buf);
                encode_key(2, V::WIRE_TYPE, buf);
                V::encode_raw_unchecked(v, buf);
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

            buf.advance(len);
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
