use core::hash::BuildHasher;
use core::hash::Hash;
use core::ops::Deref;

use bytes::Buf;
use bytes::BufMut;
use papaya::HashSet;

use crate::EncodeInputFromRef;

#[cfg(feature = "std")]
pub type PapayaSetGuard<'a, T, S> = papaya::HashSetRef<'a, T, S, papaya::LocalGuard<'a>>;

#[cfg(feature = "std")]
pub struct PapayaSetShadow<'a, T, S>
where
    T: Eq + Hash,
    S: BuildHasher + Default + 'a,
{
    set: &'a papaya::HashSet<T, S>,
    guard: Option<PapayaSetGuard<'a, T, S>>,
}

#[cfg(feature = "std")]
impl<'a, T, S> PapayaSetShadow<'a, T, S>
where
    T: Eq + Hash,
    S: BuildHasher + Default + 'a,
{
    #[inline]
    pub fn new(set: &'a papaya::HashSet<T, S>) -> Self {
        Self {
            set,
            guard: Some(set.pin()),
        }
    }

    #[inline]
    fn guard(&self) -> &PapayaSetGuard<'a, T, S> {
        self.guard.as_ref().expect("papaya set guard initialized")
    }

    #[inline]
    pub fn into_guard(self) -> PapayaSetGuard<'a, T, S> {
        let PapayaSetShadow { set, guard } = self;
        guard.unwrap_or_else(|| set.pin())
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.guard().is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.guard().len()
    }
}

#[cfg(feature = "std")]
impl<'a, T, S> Deref for PapayaSetShadow<'a, T, S>
where
    T: Eq + Hash,
    S: BuildHasher + Default + 'a,
{
    type Target = PapayaSetGuard<'a, T, S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.guard()
    }
}

#[cfg(feature = "std")]
#[inline]
#[allow(dead_code)]
pub fn papaya_set_encode_input<'a, T, S>(set: &'a papaya::HashSet<T, S>) -> PapayaSetShadow<'a, T, S>
where
    T: Eq + Hash,
    S: BuildHasher + Default + 'a,
{
    PapayaSetShadow::new(set)
}

use crate::DecodeError;
use crate::ProtoExt;
use crate::ProtoShadow;
use crate::ProtoWire;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_varint;
use crate::encoding::encode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
use crate::encoding::key_len;
use crate::encoding::skip_field;
use crate::traits::ProtoKind;

impl<T, S> ProtoShadow<Self> for HashSet<T, S>
where
    for<'a> T: ProtoShadow<T> + ProtoWire + EncodeInputFromRef<'a> + Eq + Hash + 'a,
    for<'a> S: BuildHasher + Default + 'a,
{
    type Sun<'a> = &'a HashSet<T, S>;
    type OwnedSun = HashSet<T, S>;
    type View<'a> = PapayaSetShadow<'a, T, S>;

    #[inline]
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
        Ok(self)
    }

    #[inline]
    fn from_sun(v: Self::Sun<'_>) -> Self::View<'_> {
        PapayaSetShadow::new(v)
    }
}

impl<'a, T, S> EncodeInputFromRef<'a> for HashSet<T, S>
where
    for<'b> T: ProtoWire + EncodeInputFromRef<'b> + Eq + Hash + 'b,
    for<'b> S: BuildHasher + Default + 'b,
{
    #[inline]
    fn encode_input_from_ref(value: &'a Self) -> Self::EncodeInput<'a> {
        PapayaSetShadow::new(value)
    }
}

impl<T, S> ProtoWire for HashSet<T, S>
where
    for<'a> T: ProtoWire + EncodeInputFromRef<'a> + Eq + Hash + 'a,
    for<'a> S: BuildHasher + Default + 'a,
{
    type EncodeInput<'a> = PapayaSetShadow<'a, T, S>;
    const KIND: ProtoKind = ProtoKind::for_vec(&T::KIND);
    const _REPEATED_SUPPORT: Option<&'static str> = Some("papaya::HashSet");

    #[inline(always)]
    fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
        unsafe { Self::encoded_len_impl_raw(value) }
    }

    #[inline(always)]
    fn encoded_len_tagged(&self, tag: u32) -> usize {
        let shadow = PapayaSetShadow::new(self);
        Self::encoded_len_tagged_impl(&shadow, tag)
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
                    let guard = &**value;
                    let body: usize = guard
                        .iter()
                        .map(|m| {
                            let input = T::encode_input_from_ref(m);
                            let len = unsafe { T::encoded_len_impl_raw(&input) };
                            encoded_len_varint(len as u64) + len
                        })
                        .sum();
                    key_len(tag) * n + body
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
        panic!("Do not call encode_raw_unchecked on papaya::HashSet<T,S>");
    }

    #[inline]
    fn encode_with_tag(tag: u32, value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if value.is_empty() {
                    return;
                }
                let guard = value.into_guard();
                encode_key(tag, WireType::LengthDelimited, buf);
                let body_len = guard
                    .iter()
                    .map(|v| {
                        let input = T::encode_input_from_ref(v);
                        unsafe { T::encoded_len_impl_raw(&input) }
                    })
                    .sum::<usize>();
                encode_varint(body_len as u64, buf);
                for v in &guard {
                    let input = T::encode_input_from_ref(v);
                    T::encode_raw_unchecked(input, buf);
                }
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                let guard = value.into_guard();
                for m in &guard {
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
        let guard = set.pin();
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if wire_type == WireType::LengthDelimited {
                    let len = decode_varint(buf)? as usize;
                    let mut slice = buf.take(len);
                    while slice.has_remaining() {
                        let mut v = T::proto_default();
                        T::decode_into(T::WIRE_TYPE, &mut v, &mut slice, ctx)?;
                        guard.insert(v);
                    }
                    debug_assert!(!slice.has_remaining());
                } else {
                    let mut v = T::proto_default();
                    T::decode_into(wire_type, &mut v, buf, ctx)?;
                    guard.insert(v);
                }
                Ok(())
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                let mut v = T::proto_default();
                T::decode_into(wire_type, &mut v, buf, ctx)?;
                guard.insert(v);
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
        let guard = self.pin();
        guard.clear();
    }
}

impl<T, S> ProtoExt for HashSet<T, S>
where
    for<'a> T: ProtoShadow<T> + ProtoWire + EncodeInputFromRef<'a> + Eq + Hash + 'a,
    for<'a> S: BuildHasher + Default + 'a,
{
    type Shadow<'b> = HashSet<T, S>;

    #[inline(always)]
    fn merge_field(
        value: &mut Self::Shadow<'_>,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        if tag == 1 {
            <HashSet<T, S> as ProtoWire>::decode_into(wire_type, value, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }
}
