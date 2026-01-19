#![allow(clippy::inline_always)]
#![allow(clippy::wrong_self_convention)]

use bytes::Buf;
use bytes::BufMut;
pub use decode::ProtoDecode;
pub use encode::ArchivedProtoInner;
pub use encode::ProtoArchive;
pub use utils::ProtoKind;
pub use utils::const_test_validate_with_ext;
pub use utils::const_unreachable;

use crate::alloc::vec::Vec;
use crate::encoding::DecodeContext;
use crate::encoding::WireType;
use crate::encoding::decode_key;
use crate::encoding::decode_varint;
use crate::encoding::encode_varint;
use crate::encoding::encoded_len_varint;
use crate::error::DecodeError;
use crate::error::EncodeError;
use crate::traits::decode::ProtoShadowDecode;
use crate::traits::encode::ProtoEncode;
use crate::traits::utils::VarintConst;
use crate::traits::utils::encode_varint_const;

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
