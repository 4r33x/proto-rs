#![allow(clippy::inline_always)]
#[cfg(feature = "std")]
use std::collections::HashSet;
#[cfg(feature = "std")]
use std::hash::Hash;

use bytes::Buf;
use bytes::BufMut;

use crate::DecodeError;
use crate::EncodeError;
use crate::alloc::collections::BTreeSet;
use crate::alloc::vec::Vec;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_key;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;

// ---------- conversion trait users implement ----------
pub trait ProtoShadow: Sized {
    /// Borrowed or owned form used during encoding.
    type Sun<'a>: 'a;

    /// The value returned after decoding — can be fully owned
    /// (e.g. `D128`, `String`) or a zero-copy wrapper `ZeroCopyAccess<T>`.
    type OwnedSun: Sized;

    /// The *resulting* shadow type when constructed from a given Sun<'b>, it could be just zero-copy view so we can encode it to buffer
    type View<'a>: 'a;

    /// Decoder to owned value
    fn to_sun(self) -> Result<Self::OwnedSun, DecodeError>;

    /// Build a shadow from an existing Sun (borrowed or owned).
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_>;
}

// Helper alias to shorten signatures:
pub type Shadow<'a, T> = <T as ProtoExt>::Shadow<'a>;
pub type SunOf<'a, T> = <Shadow<'a, T> as ProtoShadow>::Sun<'a>;
pub type OwnedSunOf<'a, T> = <Shadow<'a, T> as ProtoShadow>::OwnedSun;
pub type ViewOf<'a, T> = <Shadow<'a, T> as ProtoShadow>::View<'a>;

pub trait ProtoExt: Sized {
    type Shadow<'a>: ProtoShadow<OwnedSun = Self>
    where
        Self: 'a;

    fn proto_default<'a>() -> Self::Shadow<'a>;
    fn encoded_len(value: &ViewOf<'_, Self>) -> usize;
    #[doc(hidden)]
    fn encode_raw(value: ViewOf<'_, Self>, buf: &mut impl BufMut);

    #[doc(hidden)]
    fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    #[inline(always)]
    fn post_decode(value: Self::Shadow<'_>) -> Result<Self, DecodeError> {
        value.to_sun()
    }

    #[inline(always)]
    fn with_shadow<R, F>(value: SunOf<'_, Self>, f: F) -> R
    where
        F: FnOnce(ViewOf<'_, Self>) -> R,
    {
        let shadow = Self::Shadow::from_sun(value);
        f(shadow)
    }

    #[inline(always)]
    fn ensure_capacity(buf: &mut impl BufMut, required: usize) -> Result<(), EncodeError> {
        let remaining = buf.remaining_mut();
        if required > remaining { Err(EncodeError::new(required, remaining)) } else { Ok(()) }
    }

    #[inline(always)]
    fn length_delimited_capacity(len: usize) -> usize {
        len + encoded_len_varint(len as u64)
    }

    // -------- Encoding entry points (Sun -> Shadow -> write)
    #[inline(always)]
    fn encode(value: SunOf<'_, Self>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        Self::with_shadow(value, |shadow| {
            let required = Self::encoded_len(&shadow);
            Self::ensure_capacity(buf, required)?;
            Self::encode_raw(shadow, buf);
            Ok(())
        })
    }
    #[inline(always)]
    fn encode_to_vec(value: SunOf<'_, Self>) -> Vec<u8> {
        Self::with_shadow(value, |shadow| {
            let len = Self::encoded_len(&shadow);
            let mut buf = Vec::with_capacity(len);
            Self::encode_raw(shadow, &mut buf);
            buf
        })
    }
    #[inline(always)]
    fn encode_to_array<const N: usize>(value: SunOf<'_, Self>) -> [u8; N] {
        Self::with_shadow(value, |shadow| {
            let len = Self::encoded_len(&shadow);
            debug_assert!(len <= N, "encode_to_array called with insufficient capacity");
            let mut buf = [0; N];
            Self::encode_raw(shadow, &mut buf.as_mut_slice());
            buf
        })
    }

    #[inline(always)]
    fn encode_length_delimited(value: SunOf<'_, Self>, buf: &mut impl BufMut) -> Result<(), EncodeError> {
        Self::with_shadow(value, |shadow| {
            let len = Self::encoded_len(&shadow);
            let required = Self::length_delimited_capacity(len);
            Self::ensure_capacity(buf, required)?;

            encode_varint(len as u64, buf);
            Self::encode_raw(shadow, buf);
            Ok(())
        })
    }

    #[inline(always)]
    fn encode_length_delimited_to_vec(value: SunOf<'_, Self>) -> Vec<u8> {
        Self::with_shadow(value, |shadow| {
            let len = Self::encoded_len(&shadow);
            let mut buf = Vec::with_capacity(Self::length_delimited_capacity(len));
            encode_varint(len as u64, &mut buf);
            Self::encode_raw(shadow, &mut buf);
            buf
        })
    }
    #[inline]
    ///N should include encoded_len_varint
    fn encode_length_delimited_to_array<const VAR_INT_LEN: usize>(value: SunOf<'_, Self>) -> [u8; VAR_INT_LEN] {
        Self::with_shadow(value, |shadow| {
            let len = Self::encoded_len(&shadow);
            let required = Self::length_delimited_capacity(len);
            debug_assert!(required <= VAR_INT_LEN, "encode_length_delimited_to_array called with insufficient capacity");
            let mut buf = [0; VAR_INT_LEN];
            let mut slice = buf.as_mut_slice();
            encode_varint(len as u64, &mut slice);
            Self::encode_raw(shadow, &mut slice);
            buf
        })
    }

    #[inline(always)]
    fn decode(mut buf: impl Buf) -> Result<Self, DecodeError> {
        let mut shadow = Self::proto_default();
        Self::merge(&mut shadow, &mut buf)?;
        Self::post_decode(shadow)
    }
    #[inline(always)]
    fn decode_length_delimited(buf: impl Buf) -> Result<Self, DecodeError> {
        let mut shadow = Self::proto_default();
        Self::merge_length_delimited(&mut shadow, buf)?;
        Self::post_decode(shadow)
    }
    #[inline(always)]
    fn merge_length_delimited(value: &mut Self::Shadow<'_>, mut buf: impl Buf) -> Result<(), DecodeError> {
        crate::encoding::message::merge::<Self, _>(WireType::LengthDelimited, value, &mut buf, DecodeContext::default())
    }
    #[inline]
    fn merge(value: &mut Self::Shadow<'_>, mut buf: impl Buf) -> Result<(), DecodeError> {
        let ctx = DecodeContext::default();
        while buf.has_remaining() {
            let (tag, wire_type) = decode_key(&mut buf)?;
            Self::merge_field(value, tag, wire_type, &mut buf, ctx)?;
        }
        Ok(())
    }

    fn encode_singular_field(tag: u32, value: ViewOf<'_, Self>, buf: &mut impl BufMut);

    fn merge_singular_field(wire_type: WireType, value: &mut Self::Shadow<'_>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError>;

    fn encoded_len_singular_field(tag: u32, value: &ViewOf<'_, Self>) -> usize;

    #[inline(always)]
    fn encode_option_field(tag: u32, value: Option<ViewOf<'_, Self>>, buf: &mut impl BufMut) {
        if let Some(inner) = value {
            Self::encode_singular_field(tag, inner, buf);
        }
    }

    #[inline(always)]
    fn merge_option_field(wire_type: WireType, target: &mut Option<Self::Shadow<'_>>, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if let Some(value) = target.as_mut() {
            Self::merge_singular_field(wire_type, value, buf, ctx)
        } else {
            let mut value = Self::proto_default();
            Self::merge_singular_field(wire_type, &mut value, buf, ctx)?;
            *target = Some(value);
            Ok(())
        }
    }

    #[inline(always)]
    fn encoded_len_option_field(tag: u32, value: Option<ViewOf<'_, Self>>) -> usize {
        value.as_ref().map_or(0, |inner| Self::encoded_len_singular_field(tag, inner))
    }

    fn clear(&mut self);
}
pub trait RepeatedCollection<T> {
    fn reserve_hint(&mut self, _additional: usize) {}

    fn push(&mut self, value: T);

    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for value in iter {
            self.push(value);
        }
    }
}

impl<T> RepeatedCollection<T> for Vec<T> {
    #[inline]
    fn reserve_hint(&mut self, additional: usize) {
        Vec::reserve(self, additional);
    }

    #[inline]
    fn push(&mut self, value: T) {
        Vec::push(self, value);
    }
}

impl<T: Ord> RepeatedCollection<T> for BTreeSet<T> {
    #[inline]
    fn push(&mut self, value: T) {
        let _ = BTreeSet::insert(self, value);
    }
}

#[cfg(feature = "std")]
impl<T: Eq + Hash, S: std::hash::BuildHasher> RepeatedCollection<T> for HashSet<T, S> {
    #[inline]
    fn reserve_hint(&mut self, additional: usize) {
        HashSet::reserve(self, additional);
    }

    #[inline]
    fn push(&mut self, value: T) {
        let _ = HashSet::insert(self, value);
    }
}
