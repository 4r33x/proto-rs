use crate::impl_proto_ident;

#[cfg(not(feature = "build-schemas"))]
impl_proto_ident!(core::hash::BuildHasherDefault<T>);
#[cfg(not(feature = "build-schemas"))]
impl_proto_ident!(std::hash::RandomState);

#[cfg(feature = "solana_address_hash")]
impl_proto_ident!(solana_address::AddressHasherBuilder);

#[cfg(feature = "ahash")]
impl_proto_ident!(ahash::RandomState);
