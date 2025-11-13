#![cfg(feature = "papaya")]

use alloc::string::String;
use alloc::vec::Vec;
use core::hash::BuildHasher;
use core::hash::Hash;
use core::ops::Deref;

use bytes::Buf;
use bytes::BufMut;
use papaya::HashMap;

use super::maps::encode_map_entry_component;
use super::maps::map_entry_field_len;
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

#[cfg(feature = "std")]
pub type PapayaMapGuard<'a, K, V, S> = papaya::HashMapRef<'a, K, V, S, papaya::LocalGuard<'a>>;

#[cfg(feature = "std")]
pub struct PapayaMapShadow<'a, K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Default + 'a,
{
    map: &'a papaya::HashMap<K, V, S>,
    guard: Option<PapayaMapGuard<'a, K, V, S>>,
}

#[cfg(feature = "std")]
impl<'a, K, V, S> PapayaMapShadow<'a, K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Default + 'a,
{
    #[inline]
    pub fn new(map: &'a papaya::HashMap<K, V, S>) -> Self {
        Self { map, guard: Some(map.pin()) }
    }

    #[inline]
    fn guard(&self) -> &PapayaMapGuard<'a, K, V, S> {
        self.guard.as_ref().expect("papaya map guard initialized")
    }

    #[inline]
    pub fn into_guard(self) -> PapayaMapGuard<'a, K, V, S> {
        let PapayaMapShadow { map, guard } = self;
        guard.unwrap_or_else(|| map.pin())
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.guard().is_empty()
    }
}

#[cfg(feature = "std")]
impl<'a, K, V, S> Deref for PapayaMapShadow<'a, K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Default + 'a,
{
    type Target = PapayaMapGuard<'a, K, V, S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.guard()
    }
}

#[cfg(feature = "std")]
#[inline]
#[allow(dead_code)]
pub fn papaya_map_encode_input<'a, K, V, S>(map: &'a papaya::HashMap<K, V, S>) -> PapayaMapShadow<'a, K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Default + 'a,
{
    PapayaMapShadow::new(map)
}

impl<K, V, S> ProtoShadow<Self> for HashMap<K, V, S>
where
    for<'a> K: ProtoShadow<K> + ProtoWire<EncodeInput<'a> = &'a K> + Eq + Hash + 'a,
    for<'a> V: ProtoShadow<V> + ProtoWire<EncodeInput<'a> = &'a V> + 'a,
    for<'a> S: BuildHasher + Default + 'a,
{
    type Sun<'a> = &'a HashMap<K, V, S>;
    type OwnedSun = HashMap<K, V, S>;
    type View<'a> = PapayaMapShadow<'a, K, V, S>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline]
    fn from_sun(v: Self::Sun<'_>) -> Self::View<'_> {
        PapayaMapShadow::new(v)
    }
}

impl<K, V, S> ProtoWire for HashMap<K, V, S>
where
    for<'a> K: ProtoWire<EncodeInput<'a> = &'a K> + Eq + Hash + 'a,
    for<'a> V: ProtoWire<EncodeInput<'a> = &'a V> + 'a,
    for<'a> S: BuildHasher + Default + 'a,
{
    type EncodeInput<'a> = crate::wrappers::conc_map::PapayaMapShadow<'a, K, V, S>;
    const KIND: ProtoKind = ProtoKind::Repeated(&V::KIND);

    #[inline(always)]
    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
        unsafe { Self::encoded_len_impl_raw(value) }
    }

    #[inline(always)]
    fn encoded_len_tagged(&self, tag: u32) -> usize {
        let shadow = PapayaMapShadow::new(self);
        Self::encoded_len_tagged_impl(&shadow, tag)
    }

    #[inline(always)]
    fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
        if value.is_empty() {
            0
        } else {
            value
                .iter()
                .map(|(k, v)| {
                    let key_default = K::is_default_impl(&k);
                    let key_body = if key_default { 0 } else { unsafe { K::encoded_len_impl_raw(&k) } };
                    let key_len_total = if key_default { 0 } else { map_entry_field_len(K::WIRE_TYPE, 1, key_body) };
                    let value_default = V::is_default_impl(&v);
                    let value_body = if value_default { 0 } else { unsafe { V::encoded_len_impl_raw(&v) } };
                    let value_len_total = if value_default { 0 } else { map_entry_field_len(V::WIRE_TYPE, 2, value_body) };
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
                let key_default = K::is_default_impl(&k);
                let key_body = if key_default { 0 } else { unsafe { K::encoded_len_impl_raw(&k) } };
                let key_len_total = if key_default { 0 } else { map_entry_field_len(K::WIRE_TYPE, 1, key_body) };
                let value_default = V::is_default_impl(&v);
                let value_body = if value_default { 0 } else { unsafe { V::encoded_len_impl_raw(&v) } };
                let value_len_total = if value_default { 0 } else { map_entry_field_len(V::WIRE_TYPE, 2, value_body) };
                let entry_len = key_len_total + value_len_total;
                encoded_len_varint(entry_len as u64) + entry_len
            })
            .sum()
    }

    #[inline]
    fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {
        panic!("Do not call encode_raw_unchecked on papaya::HashMap<K,V,S>");
    }

    #[inline]
    fn encode_with_tag(tag: u32, shadow: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        let guard = shadow.into_guard();
        for (k, v) in &guard {
            let key_default = K::is_default_impl(&k);
            let key_body = if key_default { 0 } else { unsafe { K::encoded_len_impl_raw(&k) } };
            let key_len_total = if key_default { 0 } else { map_entry_field_len(K::WIRE_TYPE, 1, key_body) };
            let value_default = V::is_default_impl(&v);
            let value_body = if value_default { 0 } else { unsafe { V::encoded_len_impl_raw(&v) } };
            let value_len_total = if value_default { 0 } else { map_entry_field_len(V::WIRE_TYPE, 2, value_body) };
            let entry_len = key_len_total + value_len_total;
            encode_key(tag, WireType::LengthDelimited, buf);
            encode_varint(entry_len as u64, buf);

            if !key_default {
                encode_map_entry_component::<K>(1, key_body, k, buf);
            }
            if !value_default {
                encode_map_entry_component::<V>(2, value_body, v, buf);
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
        let guard = map.pin();
        guard.insert(key, value);
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
        let guard = self.pin();
        guard.clear();
    }
}
#[cfg(feature = "std")]
macro_rules! impl_papaya_primitive_map {
    ($K:ty, $V:ty) => {
        impl<S> crate::ProtoWire for papaya::HashMap<$K, $V, S>
        where
            for<'a> S: core::hash::BuildHasher + Default + 'a,
            $K: Eq + core::hash::Hash,
        {
            type EncodeInput<'a> = crate::wrappers::conc_map::PapayaMapShadow<'a, $K, $V, S>;
            const KIND: crate::traits::ProtoKind = crate::traits::ProtoKind::Repeated(&<$V as crate::ProtoWire>::KIND);

            #[inline(always)]
            fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
                unsafe { Self::encoded_len_impl_raw(value) }
            }

            #[inline(always)]
            fn encoded_len_tagged(&self, tag: u32) -> usize {
                let shadow = crate::wrappers::conc_map::PapayaMapShadow::new(self);
                Self::encoded_len_tagged_impl(&shadow, tag)
            }

            #[inline(always)]
            fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
                if value.is_empty() {
                    0
                } else {
                    value
                        .iter()
                        .map(|(k, v)| {
                            let key_default = <$K as crate::ProtoWire>::is_default_impl(&k);
                            let key_body = if key_default { 0 } else { unsafe { <$K as crate::ProtoWire>::encoded_len_impl_raw(&k) } };
                            let key_len_total = if key_default {
                                0
                            } else {
                                crate::wrappers::maps::map_entry_field_len(<$K as crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                            };
                            let value_default = <$V as crate::ProtoWire>::is_default_impl(&v);
                            let value_body = if value_default { 0 } else { unsafe { <$V as crate::ProtoWire>::encoded_len_impl_raw(&v) } };
                            let value_len_total = if value_default {
                                0
                            } else {
                                crate::wrappers::maps::map_entry_field_len(<$V as crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                            };
                            let entry_len = key_len_total + value_len_total;
                            crate::encoding::key_len(tag) + crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                        })
                        .sum()
                }
            }

            #[inline]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                value
                    .iter()
                    .map(|(k, v)| {
                        let key_default = <$K as crate::ProtoWire>::is_default_impl(&k);
                        let key_body = if key_default { 0 } else { unsafe { <$K as crate::ProtoWire>::encoded_len_impl_raw(&k) } };
                        let key_len_total = if key_default {
                            0
                        } else {
                            crate::wrappers::maps::map_entry_field_len(<$K as crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                        };
                        let value_default = <$V as crate::ProtoWire>::is_default_impl(&v);
                        let value_body = if value_default { 0 } else { unsafe { <$V as crate::ProtoWire>::encoded_len_impl_raw(&v) } };
                        let value_len_total = if value_default {
                            0
                        } else {
                            crate::wrappers::maps::map_entry_field_len(<$V as crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                        };
                        let entry_len = key_len_total + value_len_total;
                        crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                    })
                    .sum()
            }

            #[inline]
            fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {
                panic!("Do not call encode_raw_unchecked on papaya::HashMap<$K,$V,S>");
            }

            #[inline]
            fn encode_with_tag(tag: u32, shadow: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
                let guard = shadow.into_guard();
                for (k, v) in guard.iter() {
                    let key_default = <$K as crate::ProtoWire>::is_default_impl(&k);
                    let key_body = if key_default { 0 } else { unsafe { <$K as crate::ProtoWire>::encoded_len_impl_raw(&k) } };
                    let key_len_total = if key_default {
                        0
                    } else {
                        crate::wrappers::maps::map_entry_field_len(<$K as crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                    };
                    let value_default = <$V as crate::ProtoWire>::is_default_impl(&v);
                    let value_body = if value_default { 0 } else { unsafe { <$V as crate::ProtoWire>::encoded_len_impl_raw(&v) } };
                    let value_len_total = if value_default {
                        0
                    } else {
                        crate::wrappers::maps::map_entry_field_len(<$V as crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                    };
                    let entry_len = key_len_total + value_len_total;
                    crate::encoding::encode_key(tag, crate::encoding::WireType::LengthDelimited, buf);
                    crate::encoding::encode_varint(entry_len as u64, buf);

                    if !key_default {
                        crate::wrappers::maps::encode_map_entry_component::<$K>(1, key_body, *k, buf);
                    }
                    if !value_default {
                        crate::wrappers::maps::encode_map_entry_component::<$V>(2, value_body, *v, buf);
                    }
                }
            }

            #[inline]
            fn decode_into(_wire_type: crate::encoding::WireType, map: &mut Self, buf: &mut impl Buf, ctx: crate::encoding::DecodeContext) -> Result<(), crate::DecodeError> {
                let len = crate::encoding::decode_varint(buf)? as usize;
                let mut slice = buf.take(len);
                let mut key = <$K as crate::ProtoWire>::proto_default();
                let mut value = <$V as crate::ProtoWire>::proto_default();

                while slice.has_remaining() {
                    let (tag, wire) = crate::encoding::decode_key(&mut slice)?;
                    match tag {
                        1 => <$K as crate::ProtoWire>::decode_into(wire, &mut key, &mut slice, ctx)?,
                        2 => <$V as crate::ProtoWire>::decode_into(wire, &mut value, &mut slice, ctx)?,
                        _ => crate::encoding::skip_field(wire, tag, &mut slice, ctx)?,
                    }
                }

                debug_assert!(!slice.has_remaining());
                let guard = map.pin();
                guard.insert(key, value);
                Ok(())
            }

            #[inline]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                value.is_empty()
            }

            #[inline]
            fn proto_default() -> Self {
                papaya::HashMap::default()
            }

            #[inline]
            fn clear(&mut self) {
                let guard = self.pin();
                guard.clear();
            }
        }
    };
}

#[cfg(feature = "std")]
macro_rules! impl_papaya_all_primitive_maps {
    () => {
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

        macro_rules! __emit {
            ($K:ty, $V:ty) => {
                impl_papaya_primitive_map!($K, $V);
            };
        }

        __for_each_val!(__emit, bool);
        __for_each_val!(__emit, i8);
        __for_each_val!(__emit, i16);
        __for_each_val!(__emit, i32);
        __for_each_val!(__emit, i64);
        __for_each_val!(__emit, u8);
        __for_each_val!(__emit, u16);
        __for_each_val!(__emit, u32);
        __for_each_val!(__emit, u64);
    };
}

#[cfg(feature = "std")]
impl_papaya_all_primitive_maps!();

#[cfg(feature = "std")]
macro_rules! impl_papaya_string_map {
    ($V:ty) => {
        impl<S> crate::ProtoWire for papaya::HashMap<String, $V, S>
        where
            for<'a> S: core::hash::BuildHasher + Default + 'a,
        {
            type EncodeInput<'a> = crate::wrappers::conc_map::PapayaMapShadow<'a, String, $V, S>;
            const KIND: crate::traits::ProtoKind = crate::traits::ProtoKind::Repeated(&<$V as crate::ProtoWire>::KIND);

            #[inline(always)]
            fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
                unsafe { Self::encoded_len_impl_raw(value) }
            }

            #[inline(always)]
            fn encoded_len_tagged(&self, tag: u32) -> usize {
                let shadow = crate::wrappers::conc_map::PapayaMapShadow::new(self);
                Self::encoded_len_tagged_impl(&shadow, tag)
            }

            #[inline(always)]
            fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
                if value.is_empty() {
                    0
                } else {
                    value
                        .iter()
                        .map(|(k, v)| {
                            let key_default = k.is_empty();
                            let key_len = if key_default {
                                0
                            } else {
                                crate::encoding::key_len(1) + crate::encoding::encoded_len_varint(k.len() as u64) + k.len()
                            };
                            let value_default = <$V as crate::ProtoWire>::is_default_impl(&v);
                            let value_body = if value_default { 0 } else { unsafe { <$V as crate::ProtoWire>::encoded_len_impl_raw(&v) } };
                            let value_len = if value_default {
                                0
                            } else {
                                crate::wrappers::maps::map_entry_field_len(<$V as crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                            };
                            let entry_len = key_len + value_len;
                            crate::encoding::key_len(tag) + crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                        })
                        .sum()
                }
            }

            #[inline]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                value
                    .iter()
                    .map(|(k, v)| {
                        let key_default = k.is_empty();
                        let key_len = if key_default {
                            0
                        } else {
                            crate::encoding::key_len(1) + crate::encoding::encoded_len_varint(k.len() as u64) + k.len()
                        };
                        let value_default = <$V as crate::ProtoWire>::is_default_impl(&v);
                        let value_body = if value_default { 0 } else { unsafe { <$V as crate::ProtoWire>::encoded_len_impl_raw(&v) } };
                        let value_len = if value_default {
                            0
                        } else {
                            crate::wrappers::maps::map_entry_field_len(<$V as crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                        };
                        let entry_len = key_len + value_len;
                        crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                    })
                    .sum()
            }

            #[inline]
            fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {
                panic!("Do not call encode_raw_unchecked on papaya::HashMap<String,$V,S>");
            }

            #[inline]
            fn encode_with_tag(tag: u32, shadow: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
                let guard = shadow.into_guard();
                for (k, v) in guard.iter() {
                    let key_default = k.is_empty();
                    let key_len = if key_default {
                        0
                    } else {
                        crate::encoding::key_len(1) + crate::encoding::encoded_len_varint(k.len() as u64) + k.len()
                    };
                    let value_default = <$V as crate::ProtoWire>::is_default_impl(&v);
                    let value_body = if value_default { 0 } else { unsafe { <$V as crate::ProtoWire>::encoded_len_impl_raw(&v) } };
                    let value_len = if value_default {
                        0
                    } else {
                        crate::wrappers::maps::map_entry_field_len(<$V as crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                    };
                    let entry_len = key_len + value_len;
                    crate::encoding::encode_key(tag, crate::encoding::WireType::LengthDelimited, buf);
                    crate::encoding::encode_varint(entry_len as u64, buf);

                    if !key_default {
                        crate::encoding::encode_key(1, crate::encoding::WireType::LengthDelimited, buf);
                        crate::encoding::encode_varint(k.len() as u64, buf);
                        buf.put_slice(k.as_bytes());
                    }

                    if !value_default {
                        crate::wrappers::maps::encode_map_entry_component::<$V>(2, value_body, *v, buf);
                    }
                }
            }

            #[inline]
            fn decode_into(_wire_type: crate::encoding::WireType, map: &mut Self, buf: &mut impl Buf, ctx: crate::encoding::DecodeContext) -> Result<(), crate::DecodeError> {
                let len = crate::encoding::decode_varint(buf)? as usize;
                let mut slice = buf.take(len);
                let mut key = String::new();
                let mut value = <$V as crate::ProtoWire>::proto_default();

                while slice.has_remaining() {
                    let (tag, wire) = crate::encoding::decode_key(&mut slice)?;
                    match tag {
                        1 => {
                            let slen = crate::encoding::decode_varint(&mut slice)? as usize;
                            let mut bytes = Vec::with_capacity(slen);
                            bytes.resize(slen, 0);
                            slice.copy_to_slice(&mut bytes);
                            key = String::from_utf8(bytes).map_err(|_| crate::DecodeError::new("invalid UTF-8 string key"))?;
                        }
                        2 => <$V as crate::ProtoWire>::decode_into(wire, &mut value, &mut slice, ctx)?,
                        _ => crate::encoding::skip_field(wire, tag, &mut slice, ctx)?,
                    }
                }

                debug_assert!(!slice.has_remaining());
                let guard = map.pin();
                guard.insert(key, value);
                Ok(())
            }

            #[inline]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                value.is_empty()
            }

            #[inline]
            fn proto_default() -> Self {
                papaya::HashMap::default()
            }

            #[inline]
            fn clear(&mut self) {
                let guard = self.pin();
                guard.clear();
            }
        }
    };
}

#[cfg(feature = "std")]
impl_papaya_string_map!(bool);
#[cfg(feature = "std")]
impl_papaya_string_map!(i8);
#[cfg(feature = "std")]
impl_papaya_string_map!(i16);
#[cfg(feature = "std")]
impl_papaya_string_map!(i32);
#[cfg(feature = "std")]
impl_papaya_string_map!(i64);
#[cfg(feature = "std")]
impl_papaya_string_map!(u8);
#[cfg(feature = "std")]
impl_papaya_string_map!(u16);
#[cfg(feature = "std")]
impl_papaya_string_map!(u32);
#[cfg(feature = "std")]
impl_papaya_string_map!(u64);
#[cfg(feature = "std")]
impl_papaya_string_map!(f32);
#[cfg(feature = "std")]
impl_papaya_string_map!(f64);

#[cfg(feature = "std")]
macro_rules! impl_papaya_copykey_map {
    ($K:ty) => {
        impl<V, S> crate::ProtoWire for papaya::HashMap<$K, V, S>
        where
            for<'a> S: core::hash::BuildHasher + Default + 'a,
            for<'a> $K: crate::ProtoWire<EncodeInput<'a> = $K> + Eq + core::hash::Hash + 'a,
            for<'a> V: crate::ProtoWire<EncodeInput<'a> = &'a V> + 'a,
        {
            type EncodeInput<'a> = crate::wrappers::conc_map::PapayaMapShadow<'a, $K, V, S>;
            const KIND: crate::traits::ProtoKind = crate::traits::ProtoKind::Repeated(&<V as crate::ProtoWire>::KIND);

            #[inline(always)]
            fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
                unsafe { Self::encoded_len_impl_raw(value) }
            }

            #[inline(always)]
            fn encoded_len_tagged(&self, tag: u32) -> usize {
                let shadow = crate::wrappers::conc_map::PapayaMapShadow::new(self);
                Self::encoded_len_tagged_impl(&shadow, tag)
            }

            #[inline(always)]
            fn encoded_len_tagged_impl(value: &Self::EncodeInput<'_>, tag: u32) -> usize {
                if value.is_empty() {
                    0
                } else {
                    value
                        .iter()
                        .map(|(k, v)| {
                            let key_default = <$K as crate::ProtoWire>::is_default_impl(&k);
                            let key_body = if key_default { 0 } else { unsafe { <$K as crate::ProtoWire>::encoded_len_impl_raw(k) } };
                            let key_len_total = if key_default {
                                0
                            } else {
                                crate::wrappers::maps::map_entry_field_len(<$K as crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                            };
                            let value_default = <V as crate::ProtoWire>::is_default_impl(&v);
                            let value_body = if value_default { 0 } else { unsafe { <V as crate::ProtoWire>::encoded_len_impl_raw(&v) } };
                            let value_len_total = if value_default {
                                0
                            } else {
                                crate::wrappers::maps::map_entry_field_len(<V as crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                            };
                            let entry_len = key_len_total + value_len_total;
                            crate::encoding::key_len(tag) + crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                        })
                        .sum()
                }
            }

            #[inline]
            unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
                value
                    .iter()
                    .map(|(k, v)| {
                        let key_default = <$K as crate::ProtoWire>::is_default_impl(&k);
                        let key_body = if key_default { 0 } else { unsafe { <$K as crate::ProtoWire>::encoded_len_impl_raw(k) } };
                        let key_len_total = if key_default {
                            0
                        } else {
                            crate::wrappers::maps::map_entry_field_len(<$K as crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                        };
                        let value_default = <V as crate::ProtoWire>::is_default_impl(&v);
                        let value_body = if value_default { 0 } else { unsafe { <V as crate::ProtoWire>::encoded_len_impl_raw(&v) } };
                        let value_len_total = if value_default {
                            0
                        } else {
                            crate::wrappers::maps::map_entry_field_len(<V as crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                        };
                        let entry_len = key_len_total + value_len_total;
                        crate::encoding::encoded_len_varint(entry_len as u64) + entry_len
                    })
                    .sum()
            }

            #[inline]
            fn encode_raw_unchecked(_value: Self::EncodeInput<'_>, _buf: &mut impl BufMut) {
                panic!("Do not call encode_raw_unchecked on papaya::HashMap<$K,V,S>");
            }

            #[inline]
            fn encode_with_tag(tag: u32, shadow: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
                let guard = shadow.into_guard();
                for (k, v) in guard.iter() {
                    let key_default = <$K as crate::ProtoWire>::is_default_impl(&k);
                    let key_body = if key_default { 0 } else { unsafe { <$K as crate::ProtoWire>::encoded_len_impl_raw(k) } };
                    let key_len_total = if key_default {
                        0
                    } else {
                        crate::wrappers::maps::map_entry_field_len(<$K as crate::ProtoWire>::WIRE_TYPE, 1, key_body)
                    };
                    let value_default = <V as crate::ProtoWire>::is_default_impl(&v);
                    let value_body = if value_default { 0 } else { unsafe { <V as crate::ProtoWire>::encoded_len_impl_raw(&v) } };
                    let value_len_total = if value_default {
                        0
                    } else {
                        crate::wrappers::maps::map_entry_field_len(<V as crate::ProtoWire>::WIRE_TYPE, 2, value_body)
                    };
                    let entry_len = key_len_total + value_len_total;
                    crate::encoding::encode_key(tag, crate::encoding::WireType::LengthDelimited, buf);
                    crate::encoding::encode_varint(entry_len as u64, buf);

                    if !key_default {
                        crate::wrappers::maps::encode_map_entry_component::<$K>(1, key_body, *k, buf);
                    }
                    if !value_default {
                        crate::wrappers::maps::encode_map_entry_component::<V>(2, value_body, v, buf);
                    }
                }
            }

            #[inline]
            fn decode_into(_wire_type: crate::encoding::WireType, map: &mut Self, buf: &mut impl Buf, ctx: crate::encoding::DecodeContext) -> Result<(), crate::DecodeError> {
                let len = crate::encoding::decode_varint(buf)? as usize;
                let mut slice = buf.take(len);
                let mut key = <$K as crate::ProtoWire>::proto_default();
                let mut value = <V as crate::ProtoWire>::proto_default();

                while slice.has_remaining() {
                    let (tag, wire) = crate::encoding::decode_key(&mut slice)?;
                    match tag {
                        1 => <$K as crate::ProtoWire>::decode_into(wire, &mut key, &mut slice, ctx)?,
                        2 => <V as crate::ProtoWire>::decode_into(wire, &mut value, &mut slice, ctx)?,
                        _ => crate::encoding::skip_field(wire, tag, &mut slice, ctx)?,
                    }
                }

                debug_assert!(!slice.has_remaining());
                let guard = map.pin();
                guard.insert(key, value);
                Ok(())
            }

            #[inline]
            fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
                value.is_empty()
            }

            #[inline]
            fn proto_default() -> Self {
                papaya::HashMap::default()
            }

            #[inline]
            fn clear(&mut self) {
                let guard = self.pin();
                guard.clear();
            }
        }
    };
}

#[cfg(feature = "std")]
impl_papaya_copykey_map!(u8);
#[cfg(feature = "std")]
impl_papaya_copykey_map!(u16);
#[cfg(feature = "std")]
impl_papaya_copykey_map!(u32);
#[cfg(feature = "std")]
impl_papaya_copykey_map!(u64);
#[cfg(feature = "std")]
impl_papaya_copykey_map!(i8);
#[cfg(feature = "std")]
impl_papaya_copykey_map!(i16);
#[cfg(feature = "std")]
impl_papaya_copykey_map!(i32);
#[cfg(feature = "std")]
impl_papaya_copykey_map!(i64);
#[cfg(feature = "std")]
impl_papaya_copykey_map!(bool);
