use crate::impl_proto_ident;

impl_proto_ident!(core::hash::BuildHasherDefault<T>);
impl_proto_ident!(std::hash::RandomState);

#[cfg(feature = "solana_address_hash")]
impl_proto_ident!(solana_address::AddressHasherBuilder);
