#![allow(clippy::inline_always)]
#![allow(clippy::wrong_self_convention)]

pub use decode::ProtoDecode;
pub use decode::ProtoDecoder;
pub use decode::ProtoDefault;
pub use decode::ProtoFieldMerge;
pub use decode::ProtoShadowDecode;
pub use encode::ArchivedProtoField;
pub use encode::ArchivedProtoMessage;
pub use encode::ArchivedProtoMessageWriter;
pub use encode::ProtoArchive;
pub use encode::ProtoEncode;
pub use encode::ProtoShadowEncode;
pub use encode::ZeroCopy;
pub use utils::PrimitiveKind;
pub use utils::ProtoKind;
pub use utils::const_test_validate_with_ext;
pub use utils::const_unreachable;

use crate::encoding::WireType;

pub mod buffer;
mod decode;
mod encode;
mod example_impl;
mod utils;

pub trait ProtoExt: Sized {
    const KIND: ProtoKind;
    const WIRE_TYPE: WireType = Self::KIND.wire_type();
    const _REPEATED_SUPPORT: Option<&'static str> = None;

    const _TEST_REPEATED: () = {
        if let Some(name) = Self::_REPEATED_SUPPORT
            && let ProtoKind::Repeated(_) = Self::KIND
        {
            const_unreachable::<Self>(name);
        }
    };
}
impl<T: ProtoExt> ProtoExt for &T {
    const KIND: ProtoKind = T::KIND;
}
