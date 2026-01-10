use core::hash::BuildHasherDefault;
use std::hash::RandomState;

use crate::impl_proto_ident;

impl_proto_ident!(BuildHasherDefault<T>);
impl_proto_ident!(RandomState);
