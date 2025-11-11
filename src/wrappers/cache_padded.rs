// use bytes::Buf;
// use bytes::BufMut;
// use crossbeam_utils::CachePadded;

// use crate::DecodeError;
// use crate::ProtoExt;
// use crate::ProtoShadow;
// use crate::ProtoWire;
// use crate::encoding::DecodeContext;
// use crate::encoding::WireType;
// use crate::traits::ProtoKind;

// impl<T> ProtoShadow<CachePadded<T>> for T::Shadow<'_>
// where
//     T: ProtoExt,
// {
//     type Sun<'a> = <T::Shadow<'a> as ProtoShadow<T>>::Sun<'a>;
//     type OwnedSun = CachePadded<T>;
//     type View<'a> = <T::Shadow<'a> as ProtoShadow<T>>::View<'a>;

//     #[inline(always)]
//     fn to_sun(self) -> Result<Self::OwnedSun, DecodeError> {
//         Ok(CachePadded::new(ProtoShadow::to_sun(self)?))
//     }

//     #[inline(always)]
//     fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
//         <T::Shadow<'_> as ProtoShadow<T>>::from_sun(value)
//     }
// }

// // -----------------------------------------------------------------------------
// // CachePadded<T>: ProtoWire
// // -----------------------------------------------------------------------------
// //
// // CachePadded<T> encodes exactly as T; no layout differences on wire.
// impl<T> ProtoWire for CachePadded<T>
// where
//     T: ProtoWire,
// {
//     // EncodeInput stays same as T’s
//     type EncodeInput<'a> = T::EncodeInput<'a>;
//     const KIND: ProtoKind = T::KIND;

//     #[inline(always)]
//     fn proto_default() -> Self {
//         CachePadded::new(T::proto_default())
//     }

//     #[inline(always)]
//     fn clear(&mut self) {
//         T::clear(&mut **self);
//     }

//     #[inline(always)]
//     fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
//         T::is_default_impl(value)
//     }

//     #[inline(always)]
//     unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
//         unsafe { T::encoded_len_impl_raw(value) }
//     }

//     #[inline(always)]
//     fn encoded_len_impl(value: &Self::EncodeInput<'_>) -> usize {
//         T::encoded_len_impl(value)
//     }

//     #[inline(always)]
//     fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl BufMut) {
//         T::encode_raw_unchecked(value, buf);
//     }

//     #[inline(always)]
//     fn decode_into(wire: WireType, value: &mut Self, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
//         T::decode_into(wire, &mut **value, buf, ctx)
//     }
// }

// // -----------------------------------------------------------------------------
// // CachePadded<T>: ProtoExt
// // -----------------------------------------------------------------------------
// //
// // Shadow<'a> is just T::Shadow<'a>, not padded.
// impl<T> ProtoExt for CachePadded<T>
// where
//     T: ProtoExt,
// {
//     type Shadow<'a> = T::Shadow<'a>;

//     #[inline(always)]
//     fn merge_field(value: &mut Self::Shadow<'_>, tag: u32, wire: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
//         T::merge_field(value, tag, wire, buf, ctx)
//     }
// }
