use core::hash::BuildHasher;
use core::hash::Hash;
use core::ops::Deref;

use bytes::Buf;
use bytes::BufMut;
use papaya::HashMap;

use super::maps::encode_map_entry_component;
use super::maps::map_entry_field_len;
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
        Self {
            map,
            guard: Some(map.pin()),
        }
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
    for<'a> K: ProtoShadow<K> + ProtoWire + Eq + Hash + 'a,
    for<'a> V: ProtoShadow<V> + ProtoWire + 'a,
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

impl<'a, K, V, S> EncodeInputFromRef<'a> for HashMap<K, V, S>
where
    for<'b> K: ProtoWire + EncodeInputFromRef<'b> + Eq + Hash + 'b,
    for<'b> V: ProtoWire + EncodeInputFromRef<'b> + 'b,
    for<'b> S: BuildHasher + Default + 'b,
{
    #[inline]
    fn encode_input_from_ref(value: &'a Self) -> Self::EncodeInput<'a> {
        PapayaMapShadow::new(value)
    }
}

impl<K, V, S> ProtoWire for HashMap<K, V, S>
where
    for<'a> K: ProtoWire + EncodeInputFromRef<'a> + Eq + Hash + 'a,
    for<'a> V: ProtoWire + EncodeInputFromRef<'a> + 'a,
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
                        map_entry_field_len(K::WIRE_TYPE, 1, key_body)
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
                        map_entry_field_len(V::WIRE_TYPE, 2, value_body)
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
                    map_entry_field_len(K::WIRE_TYPE, 1, key_body)
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
                    map_entry_field_len(V::WIRE_TYPE, 2, value_body)
                };
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
                map_entry_field_len(K::WIRE_TYPE, 1, key_body)
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
                map_entry_field_len(V::WIRE_TYPE, 2, value_body)
            };
            let entry_len = key_len_total + value_len_total;
            encode_key(tag, WireType::LengthDelimited, buf);
            encode_varint(entry_len as u64, buf);

            if !key_default {
                encode_map_entry_component::<K>(1, key_body, key_input, buf);
            }
            if !value_default {
                encode_map_entry_component::<V>(2, value_body, value_input, buf);
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
