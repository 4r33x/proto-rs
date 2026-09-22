#![allow(clippy::inline_always)]
#![allow(clippy::wrong_self_convention)]

pub use decode::DecodeIrBuilder;
pub use decode::ProtoDecode;
pub use decode::ProtoDecoder;
pub use decode::ProtoDefault;
pub use decode::ProtoFieldMerge;
pub use decode::ProtoShadowDecode;
pub use encode::ArchivedProtoField;
pub use encode::ArchivedProtoMessage;
pub use encode::ArchivedProtoMessageWriter;
pub use encode::EncodePoolConfig;
pub use encode::EncodeSizeHint;
pub use encode::EncodedSnapshot;
pub use encode::MAX_PREALLOCATED_CAPACITY;
pub use encode::ProtoArchive;
pub use encode::ProtoEncode;
pub use encode::ProtoShadowEncode;
pub use encode::configure_encode_pool;
#[cfg(feature = "tonic-owned")]
pub(crate) use encode::encode_batch;
#[cfg(feature = "tonic")]
pub(crate) use encode::encode_into;
#[cfg(feature = "tonic")]
pub(crate) use encode::prepare_owned;
pub use utils::PrimitiveKind;
pub use utils::ProtoKind;
pub use utils::const_test_validate_with_ext;
pub use utils::const_unreachable;

use crate::encoding::WireType;

pub mod buffer;
mod decode;
mod decode_state;
pub use decode_state::DecodeState;
mod encode;
mod utils;

pub trait ProtoExt: Sized {
    const KIND: ProtoKind;
    /// Encode this standalone value as field 1 of a wrapper message.
    ///
    /// This is independent of its field wire type: e.g. `Option<Message>` and
    /// `Box<Message>` remain message fields, but have a wrapper at the root.
    const WRAP_ROOT: bool = !matches!(Self::KIND, ProtoKind::Message);
    /// Whether sequences of this element use protobuf `bytes`. Wrappers do not inherit this.
    const IS_BYTE: bool = false;

    // Safe specialization hooks: wire metadata never proves memory layout.
    #[doc(hidden)]
    fn byte_slice(_values: &[Self]) -> Option<&[u8]> {
        None
    }
    #[doc(hidden)]
    fn byte_slice_mut(_values: &mut [Self]) -> Option<&mut [u8]> {
        None
    }
    #[doc(hidden)]
    fn byte_vec_mut(_values: &mut Vec<Self>) -> Option<&mut Vec<u8>> {
        None
    }
    #[doc(hidden)]
    fn byte_deque_mut(_values: &mut std::collections::VecDeque<Self>) -> Option<&mut std::collections::VecDeque<u8>> {
        None
    }
    const WIRE_TYPE: WireType = Self::KIND.wire_type();
    const ENCODED_SIZE_HINT: EncodeSizeHint = EncodeSizeHint::from_kind(&Self::KIND);
    const REPEATED_SUPPORT: Option<&'static str> = None;

    const TEST_REPEATED: () = {
        if let Some(name) = Self::REPEATED_SUPPORT
            && let ProtoKind::Repeated(_) = Self::KIND
        {
            const_unreachable::<Self>(name);
        }
    };
}
impl<T: ProtoExt> ProtoExt for &T {
    const KIND: ProtoKind = T::KIND;
    const WRAP_ROOT: bool = T::WRAP_ROOT;
    const ENCODED_SIZE_HINT: EncodeSizeHint = T::ENCODED_SIZE_HINT;
}
