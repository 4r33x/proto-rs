use alloc::collections::BTreeMap;

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

#[inline(always)]
pub(crate) fn encode_map_entry_component<T>(field_tag: u32, body_len: usize, value: T::EncodeInput<'_>, buf: &mut impl BufMut)
where
    T: ProtoWire,
{
    encode_key(field_tag, T::WIRE_TYPE, buf);
    if T::WIRE_TYPE == WireType::LengthDelimited {
        encode_varint(body_len as u64, buf);
        T::encode_raw_unchecked(value, buf);
    } else {
        T::encode_raw_unchecked(value, buf);
    }
}

#[inline(always)]
pub(crate) fn map_entry_field_len(wire: WireType, tag: u32, body_len: usize) -> usize {
    let base = key_len(tag);
    base + match wire {
        WireType::LengthDelimited => encoded_len_varint(body_len as u64) + body_len,
        WireType::StartGroup | WireType::EndGroup => body_len + base,
        _ => body_len,
    }
}

impl<K, V> ProtoShadow<Self> for BTreeMap<K, V>
where
    for<'a> K: ProtoShadow<K> + ProtoWire + 'a,
    for<'a> V: ProtoShadow<V> + ProtoWire + 'a,
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
    for<'a> V: ProtoWire + EncodeInputFromRef<'a> + 'a,
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
                    let key_input = K::encode_input_from_ref(k);
                    let key_default = K::is_default_impl(&key_input);
                    let key_body = if key_default {
                        0
                    } else {
                        unsafe { K::encoded_len_impl_raw(&key_input) }
                    };
                    let key_len_total = if key_default {
                        0
                    } else {
                        crate::wrappers::maps::map_entry_field_len(K::WIRE_TYPE, 1, key_body)
                    };
                    let value_input = V::encode_input_from_ref(v);
                    let value_default = V::is_default_impl(&value_input);
                    let value_body = if value_default {
                        0
                    } else {
                        unsafe { V::encoded_len_impl_raw(&value_input) }
                    };
                    let value_len_total = if value_default {
                        0
                    } else {
                        crate::wrappers::maps::map_entry_field_len(V::WIRE_TYPE, 2, value_body)
                    };
                    let entry_len = key_len_total + value_len_total;
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
                let key_input = K::encode_input_from_ref(k);
                let key_default = K::is_default_impl(&key_input);
                let key_body = if key_default {
                    0
                } else {
                    unsafe { K::encoded_len_impl_raw(&key_input) }
                };
                let key_len_total = if key_default {
                    0
                } else {
                    crate::wrappers::maps::map_entry_field_len(K::WIRE_TYPE, 1, key_body)
                };
                let value_input = V::encode_input_from_ref(v);
                let value_default = V::is_default_impl(&value_input);
                let value_body = if value_default {
                    0
                } else {
                    unsafe { V::encoded_len_impl_raw(&value_input) }
                };
                let value_len_total = if value_default {
                    0
                } else {
                    crate::wrappers::maps::map_entry_field_len(V::WIRE_TYPE, 2, value_body)
                };
                let entry_len = key_len_total + value_len_total;
                encoded_len_varint(entry_len as u64) + entry_len
            })
            .sum()
    }

    #[inline]
    fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {
        panic!("Do not call encode_raw_unchecked on BTreeMap<K,V>");
    }

    #[inline]
    fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        for (k, v) in map {
            let key_input = K::encode_input_from_ref(k);
            let key_default = K::is_default_impl(&key_input);
            let key_body = if key_default {
                0
            } else {
                unsafe { K::encoded_len_impl_raw(&key_input) }
            };
            let key_len_total = if key_default {
                0
            } else {
                crate::wrappers::maps::map_entry_field_len(K::WIRE_TYPE, 1, key_body)
            };
            let value_input = V::encode_input_from_ref(v);
            let value_default = V::is_default_impl(&value_input);
            let value_body = if value_default {
                0
            } else {
                unsafe { V::encoded_len_impl_raw(&value_input) }
            };
            let value_len_total = if value_default {
                0
            } else {
                crate::wrappers::maps::map_entry_field_len(V::WIRE_TYPE, 2, value_body)
            };
            let entry_len = key_len_total + value_len_total;
            encode_key(tag, WireType::LengthDelimited, buf);
            encode_varint(entry_len as u64, buf);

            if !key_default {
                crate::wrappers::maps::encode_map_entry_component::<K>(1, key_body, key_input, buf);
            }
            if !value_default {
                crate::wrappers::maps::encode_map_entry_component::<V>(2, value_body, value_input, buf);
            }
        }
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
    use crate::wrappers::maps::EncodeInputFromRef;

    impl<K, V, S> ProtoShadow<Self> for HashMap<K, V, S>
    where
        for<'a> K: ProtoShadow<K> + ProtoWire + 'a,
        for<'a> V: ProtoShadow<V> + ProtoWire + 'a,
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
        for<'a> V: ProtoWire + EncodeInputFromRef<'a> + 'a,
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
                        let key_input = K::encode_input_from_ref(k);
                        let key_default = K::is_default_impl(&key_input);
                        let key_body = if key_default {
                            0
                        } else {
                            unsafe { K::encoded_len_impl_raw(&key_input) }
                        };
                        let key_len_total = if key_default {
                            0
                        } else {
                            crate::wrappers::maps::map_entry_field_len(K::WIRE_TYPE, 1, key_body)
                        };
                        let value_input = V::encode_input_from_ref(v);
                        let value_default = V::is_default_impl(&value_input);
                        let value_body = if value_default {
                            0
                        } else {
                            unsafe { V::encoded_len_impl_raw(&value_input) }
                        };
                        let value_len_total = if value_default {
                            0
                        } else {
                            crate::wrappers::maps::map_entry_field_len(V::WIRE_TYPE, 2, value_body)
                        };
                        let entry_len = key_len_total + value_len_total;
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
                    let key_input = K::encode_input_from_ref(k);
                    let key_default = K::is_default_impl(&key_input);
                    let key_body = if key_default {
                        0
                    } else {
                        unsafe { K::encoded_len_impl_raw(&key_input) }
                    };
                    let key_len_total = if key_default {
                        0
                    } else {
                        crate::wrappers::maps::map_entry_field_len(K::WIRE_TYPE, 1, key_body)
                    };
                    let value_input = V::encode_input_from_ref(v);
                    let value_default = V::is_default_impl(&value_input);
                    let value_body = if value_default {
                        0
                    } else {
                        unsafe { V::encoded_len_impl_raw(&value_input) }
                    };
                    let value_len_total = if value_default {
                        0
                    } else {
                        crate::wrappers::maps::map_entry_field_len(V::WIRE_TYPE, 2, value_body)
                    };
                    let entry_len = key_len_total + value_len_total;
                    encoded_len_varint(entry_len as u64) + entry_len
                })
                .sum()
        }

        #[inline]
        fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {
            panic!("Do not call encode_raw_unchecked on HashMap<K,V,S>");
        }

        #[inline]
        fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
            for (k, v) in map {
                let key_input = K::encode_input_from_ref(k);
                let key_default = K::is_default_impl(&key_input);
                let key_body = if key_default {
                    0
                } else {
                    unsafe { K::encoded_len_impl_raw(&key_input) }
                };
                let key_len_total = if key_default {
                    0
                } else {
                    crate::wrappers::maps::map_entry_field_len(K::WIRE_TYPE, 1, key_body)
                };
                let value_input = V::encode_input_from_ref(v);
                let value_default = V::is_default_impl(&value_input);
                let value_body = if value_default {
                    0
                } else {
                    unsafe { V::encoded_len_impl_raw(&value_input) }
                };
                let value_len_total = if value_default {
                    0
                } else {
                    crate::wrappers::maps::map_entry_field_len(V::WIRE_TYPE, 2, value_body)
                };
                let entry_len = key_len_total + value_len_total;
                encode_key(tag, WireType::LengthDelimited, buf);
                encode_varint(entry_len as u64, buf);

                if !key_default {
                    crate::wrappers::maps::encode_map_entry_component::<K>(1, key_body, key_input, buf);
                }
                if !value_default {
                    crate::wrappers::maps::encode_map_entry_component::<V>(2, value_body, value_input, buf);
                }
            }
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
                            let key_default = <$K as $crate::ProtoWire>::is_default_impl(&k);
                            let key_body = if key_default {
                                0
                            } else {
                                unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(&k) }
                            };
                            let key_len_total = if key_default {
                                0
                            } else {
                                $crate::wrappers::maps::map_entry_field_len(<$K as $crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                            };
                            let value_default = <$V as $crate::ProtoWire>::is_default_impl(&v);
                            let value_body = if value_default {
                                0
                            } else {
                                unsafe { <$V as $crate::ProtoWire>::encoded_len_impl_raw(&v) }
                            };
                            let value_len_total = if value_default {
                                0
                            } else {
                                $crate::wrappers::maps::map_entry_field_len(<$V as $crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                            };
                            let entry_len = key_len_total + value_len_total;
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
                        let key_default = <$K as $crate::ProtoWire>::is_default_impl(&k);
                        let key_body = if key_default {
                            0
                        } else {
                            unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(&k) }
                        };
                        let key_len_total = if key_default {
                            0
                        } else {
                            $crate::wrappers::maps::map_entry_field_len(<$K as $crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                        };
                        let value_default = <$V as $crate::ProtoWire>::is_default_impl(&v);
                        let value_body = if value_default {
                            0
                        } else {
                            unsafe { <$V as $crate::ProtoWire>::encoded_len_impl_raw(&v) }
                        };
                        let value_len_total = if value_default {
                            0
                        } else {
                            $crate::wrappers::maps::map_entry_field_len(<$V as $crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                        };
                        let entry_len = key_len_total + value_len_total;
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
            fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl bytes::BufMut) {
                for (k, v) in map {
                    let key_default = <$K as $crate::ProtoWire>::is_default_impl(&k);
                    let key_body = if key_default {
                        0
                    } else {
                        unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(&k) }
                    };
                    let key_len_total = if key_default {
                        0
                    } else {
                        $crate::wrappers::maps::map_entry_field_len(<$K as $crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                    };
                    let value_default = <$V as $crate::ProtoWire>::is_default_impl(&v);
                    let value_body = if value_default {
                        0
                    } else {
                        unsafe { <$V as $crate::ProtoWire>::encoded_len_impl_raw(&v) }
                    };
                    let value_len_total = if value_default {
                        0
                    } else {
                        $crate::wrappers::maps::map_entry_field_len(<$V as $crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                    };
                    let entry_len = key_len_total + value_len_total;

                    $crate::encoding::encode_key(tag, $crate::encoding::WireType::LengthDelimited, buf);
                    $crate::encoding::encode_varint(entry_len as u64, buf);

                    if !key_default {
                        $crate::wrappers::maps::encode_map_entry_component::<$K>(1, key_body, *k, buf);
                    }
                    if !value_default {
                        $crate::wrappers::maps::encode_map_entry_component::<$V>(2, value_body, *v, buf);
                    }
                }
            }

            #[inline]
            fn decode_into(
                _wire_type: $crate::encoding::WireType,
                map: &mut Self,
                buf: &mut impl bytes::Buf,
                ctx: $crate::encoding::DecodeContext,
            ) -> Result<(), $crate::DecodeError> {
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
                            let key_default = <$K as $crate::ProtoWire>::is_default_impl(&k);
                            let key_body = if key_default {
                                0
                            } else {
                                unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(&k) }
                            };
                            let key_len_total = if key_default {
                                0
                            } else {
                                $crate::wrappers::maps::map_entry_field_len(<$K as $crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                            };
                            let value_default = <$V as $crate::ProtoWire>::is_default_impl(&v);
                            let value_body = if value_default {
                                0
                            } else {
                                unsafe { <$V as $crate::ProtoWire>::encoded_len_impl_raw(&v) }
                            };
                            let value_len_total = if value_default {
                                0
                            } else {
                                $crate::wrappers::maps::map_entry_field_len(<$V as $crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                            };
                            let entry_len = key_len_total + value_len_total;
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
                        let key_default = <$K as $crate::ProtoWire>::is_default_impl(&k);
                        let key_body = if key_default {
                            0
                        } else {
                            unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(&k) }
                        };
                        let key_len_total = if key_default {
                            0
                        } else {
                            $crate::wrappers::maps::map_entry_field_len(<$K as $crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                        };
                        let value_default = <$V as $crate::ProtoWire>::is_default_impl(&v);
                        let value_body = if value_default {
                            0
                        } else {
                            unsafe { <$V as $crate::ProtoWire>::encoded_len_impl_raw(&v) }
                        };
                        let value_len_total = if value_default {
                            0
                        } else {
                            $crate::wrappers::maps::map_entry_field_len(<$V as $crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                        };
                        let entry_len = key_len_total + value_len_total;
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
            fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl bytes::BufMut) {
                for (k, v) in map {
                    let key_default = <$K as $crate::ProtoWire>::is_default_impl(&k);
                    let key_body = if key_default {
                        0
                    } else {
                        unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(&k) }
                    };
                    let key_len_total = if key_default {
                        0
                    } else {
                        $crate::wrappers::maps::map_entry_field_len(<$K as $crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                    };
                    let value_default = <$V as $crate::ProtoWire>::is_default_impl(&v);
                    let value_body = if value_default {
                        0
                    } else {
                        unsafe { <$V as $crate::ProtoWire>::encoded_len_impl_raw(&v) }
                    };
                    let value_len_total = if value_default {
                        0
                    } else {
                        $crate::wrappers::maps::map_entry_field_len(<$V as $crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                    };
                    let entry_len = key_len_total + value_len_total;

                    $crate::encoding::encode_key(tag, $crate::encoding::WireType::LengthDelimited, buf);
                    $crate::encoding::encode_varint(entry_len as u64, buf);

                    if !key_default {
                        $crate::wrappers::maps::encode_map_entry_component::<$K>(1, key_body, *k, buf);
                    }
                    if !value_default {
                        $crate::wrappers::maps::encode_map_entry_component::<$V>(2, value_body, *v, buf);
                    }
                }
            }

            #[inline]
            fn decode_into(
                _wire_type: $crate::encoding::WireType,
                map: &mut Self,
                buf: &mut impl bytes::Buf,
                ctx: $crate::encoding::DecodeContext,
            ) -> Result<(), $crate::DecodeError> {
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
                            let key_default = <$K as $crate::ProtoWire>::is_default_impl(&k);
                            let key_body = if key_default {
                                0
                            } else {
                                unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(k) }
                            };
                            let key_len_total = if key_default {
                                0
                            } else {
                                $crate::wrappers::maps::map_entry_field_len(<$K as $crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                            };
                            let value_default = <V as $crate::ProtoWire>::is_default_impl(&v);
                            let value_body = if value_default {
                                0
                            } else {
                                unsafe { <V as $crate::ProtoWire>::encoded_len_impl_raw(&v) }
                            };
                            let value_len_total = if value_default {
                                0
                            } else {
                                $crate::wrappers::maps::map_entry_field_len(<V as $crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                            };
                            let entry_len = key_len_total + value_len_total;
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
                        let key_default = <$K as $crate::ProtoWire>::is_default_impl(&k);
                        let key_body = if key_default {
                            0
                        } else {
                            unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(k) }
                        };
                        let key_len_total = if key_default {
                            0
                        } else {
                            $crate::wrappers::maps::map_entry_field_len(<$K as $crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                        };
                        let value_default = <V as $crate::ProtoWire>::is_default_impl(&v);
                        let value_body = if value_default {
                            0
                        } else {
                            unsafe { <V as $crate::ProtoWire>::encoded_len_impl_raw(&v) }
                        };
                        let value_len_total = if value_default {
                            0
                        } else {
                            $crate::wrappers::maps::map_entry_field_len(<V as $crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                        };
                        let entry_len = key_len_total + value_len_total;
                        $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                    })
                    .sum()
            }

            #[inline]
            fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl bytes::BufMut) {
                panic!("Do not call encode_raw_unchecked on BTreeMap<K,V>");
            }

            #[inline]
            fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl bytes::BufMut) {
                for (k, v) in map {
                    let key_default = <$K as $crate::ProtoWire>::is_default_impl(&k);
                    let key_body = if key_default {
                        0
                    } else {
                        unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(k) }
                    };
                    let key_len_total = if key_default {
                        0
                    } else {
                        $crate::wrappers::maps::map_entry_field_len(<$K as $crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                    };
                    let value_default = <V as $crate::ProtoWire>::is_default_impl(&v);
                    let value_body = if value_default {
                        0
                    } else {
                        unsafe { <V as $crate::ProtoWire>::encoded_len_impl_raw(&v) }
                    };
                    let value_len_total = if value_default {
                        0
                    } else {
                        $crate::wrappers::maps::map_entry_field_len(<V as $crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                    };
                    let entry_len = key_len_total + value_len_total;
                    $crate::encoding::encode_key(tag, $crate::encoding::WireType::LengthDelimited, buf);
                    $crate::encoding::encode_varint(entry_len as u64, buf);

                    if !key_default {
                        $crate::wrappers::maps::encode_map_entry_component::<$K>(1, key_body, *k, buf);
                    }
                    if !value_default {
                        $crate::wrappers::maps::encode_map_entry_component::<V>(2, value_body, v, buf);
                    }
                }
            }

            #[inline]
            fn decode_into(
                _wire_type: $crate::encoding::WireType,
                map: &mut Self,
                buf: &mut impl bytes::Buf,
                ctx: $crate::encoding::DecodeContext,
            ) -> Result<(), $crate::DecodeError> {
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
                            let key_default = <$K as $crate::ProtoWire>::is_default_impl(&k);
                            let key_body = if key_default {
                                0
                            } else {
                                unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(k) }
                            };
                            let key_len_total = if key_default {
                                0
                            } else {
                                $crate::wrappers::maps::map_entry_field_len(<$K as $crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                            };
                            let value_default = <V as $crate::ProtoWire>::is_default_impl(&v);
                            let value_body = if value_default {
                                0
                            } else {
                                unsafe { <V as $crate::ProtoWire>::encoded_len_impl_raw(&v) }
                            };
                            let value_len_total = if value_default {
                                0
                            } else {
                                $crate::wrappers::maps::map_entry_field_len(<V as $crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                            };
                            let entry_len = key_len_total + value_len_total;
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
                        let key_default = <$K as $crate::ProtoWire>::is_default_impl(&k);
                        let key_body = if key_default {
                            0
                        } else {
                            unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(k) }
                        };
                        let key_len_total = if key_default {
                            0
                        } else {
                            $crate::wrappers::maps::map_entry_field_len(<$K as $crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                        };
                        let value_default = <V as $crate::ProtoWire>::is_default_impl(&v);
                        let value_body = if value_default {
                            0
                        } else {
                            unsafe { <V as $crate::ProtoWire>::encoded_len_impl_raw(&v) }
                        };
                        let value_len_total = if value_default {
                            0
                        } else {
                            $crate::wrappers::maps::map_entry_field_len(<V as $crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                        };
                        let entry_len = key_len_total + value_len_total;
                        $crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                    })
                    .sum()
            }

            #[inline]
            fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl bytes::BufMut) {
                panic!("Do not call encode_raw_unchecked on HashMap<K,V,S>");
            }

            #[inline]
            fn encode_with_tag(tag: u32, map: Self::EncodeInput<'_>, buf: &mut impl bytes::BufMut) {
                for (k, v) in map {
                    let key_default = <$K as $crate::ProtoWire>::is_default_impl(&k);
                    let key_body = if key_default {
                        0
                    } else {
                        unsafe { <$K as $crate::ProtoWire>::encoded_len_impl_raw(k) }
                    };
                    let key_len_total = if key_default {
                        0
                    } else {
                        $crate::wrappers::maps::map_entry_field_len(<$K as $crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                    };
                    let value_default = <V as $crate::ProtoWire>::is_default_impl(&v);
                    let value_body = if value_default {
                        0
                    } else {
                        unsafe { <V as $crate::ProtoWire>::encoded_len_impl_raw(&v) }
                    };
                    let value_len_total = if value_default {
                        0
                    } else {
                        $crate::wrappers::maps::map_entry_field_len(<V as $crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                    };
                    let entry_len = key_len_total + value_len_total;
                    $crate::encoding::encode_key(tag, $crate::encoding::WireType::LengthDelimited, buf);
                    $crate::encoding::encode_varint(entry_len as u64, buf);

                    if !key_default {
                        $crate::wrappers::maps::encode_map_entry_component::<$K>(1, key_body, *k, buf);
                    }
                    if !value_default {
                        $crate::wrappers::maps::encode_map_entry_component::<V>(2, value_body, v, buf);
                    }
                }
            }

            #[inline]
            fn decode_into(
                _wire_type: $crate::encoding::WireType,
                map: &mut Self,
                buf: &mut impl bytes::Buf,
                ctx: $crate::encoding::DecodeContext,
            ) -> Result<(), $crate::DecodeError> {
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

#[cfg(test)]
mod tests {
    use bytes::Buf;
    use bytes::BufMut;

    use super::*;

    #[derive(Clone, Copy, Default)]
    struct TestBytes;

    impl ProtoWire for TestBytes {
        type EncodeInput<'a> = &'a [u8];
        const KIND: ProtoKind = ProtoKind::Bytes;

        #[inline(always)]
        fn proto_default() -> Self {
            Self
        }

        #[inline(always)]
        fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
            value.is_empty()
        }

        #[inline(always)]
        fn clear(&mut self) {}

        #[inline(always)]
        unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
            value.len()
        }

        #[inline(always)]
        fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
            buf.put_slice(value);
        }

        #[inline(always)]
        fn decode_into(_wire_type: WireType, _value: &mut Self, _buf: &mut impl Buf, _ctx: DecodeContext) -> Result<(), DecodeError> {
            Err(DecodeError::new("not implemented"))
        }
    }

    #[test]
    fn length_delimited_component_writes_empty_prefix() {
        let mut storage = [0u8; 2];
        let remaining = {
            let mut slice: &mut [u8] = &mut storage;
            encode_map_entry_component::<TestBytes>(1, 0, &[], &mut slice);
            slice.remaining_mut()
        };
        assert_eq!(storage, [0x0A, 0x00]);
        assert_eq!(remaining, 0);
    }
}
