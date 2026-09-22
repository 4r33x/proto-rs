//! Generic encoding and decoding.
//!
//! This module contains the generic `Codec`, `Encoder` and `Decoder` traits.

mod buffer;
pub(crate) mod compression;
mod decode;
mod encode;
use std::io;

pub use self::buffer::DecodeBuf;
pub use self::buffer::EncodeBuf;
pub use self::compression::CompressionEncoding;
pub use self::compression::EnabledCompressionEncodings;
// Doc hidden since this is used in a test in another crate, we can expose this publically later
// if we need it.
#[doc(hidden)]
pub use self::compression::SingleMessageCompressionOverride;
pub use self::decode::Streaming;
pub use self::encode::EncodeBody;
use crate::Status;

/// Unless overridden, this is the buffer size used for encoding requests.
/// This is spent per-rpc, so you may wish to adjust it. The default is
/// pretty good for most uses, but if you have a ton of concurrent rpcs
/// you may find it too expensive.
const DEFAULT_CODEC_BUFFER_SIZE: usize = 8 * 1024;
const DEFAULT_YIELD_THRESHOLD: usize = 32 * 1024;

/// Settings for how tonic allocates and grows buffers.
///
/// Buffered encoders eagerly allocate buffer_size per RPC; encoders opting into
/// owned handoff defer allocation until they actually need a destination. Tonic grows
/// the buffer by buffer_size increments to handle larger messages.
/// Buffer size defaults to 8KiB.
///
/// Example:
/// ```ignore
/// Buffer start:       | 8kb |
/// Message received:   |   24612 bytes    |
/// Buffer grows:       | 8kb | 8kb | 8kb | 8kb |
/// ```
///
/// The buffer grows to the next largest buffer_size increment of
/// 32768 to hold 24612 bytes, which is just slightly too large for
/// the previous buffer increment of 24576.
///
/// If you use a smaller buffer size you will waste less memory, but
/// you will allocate more frequently. If one way or the other matters
/// more to you, you may wish to customize your tonic Codec (see
/// codec_buffers example).
///
/// Yield threshold is an optimization for streaming rpcs. Sometimes
/// you may have many small messages ready to send. When they are ready,
/// it is a much more efficient use of system resources to batch them
/// together into one larger send(). The yield threshold controls how
/// much you want to bulk up such a batch of ready-to-send messages.
/// The larger your yield threshold the more you will batch - and
/// consequently allocate contiguous memory, which might be relevant
/// if you're considering large numbers here.
/// If your server streaming rpc does not reach the yield threshold
/// before it reaches Poll::Pending (meaning, it's waiting for more
/// data from wherever you're streaming from) then Tonic will just send
/// along a smaller batch. Yield threshold is an upper-bound, it will
/// not affect the responsiveness of your streaming rpc (for reasonable
/// sizes of yield threshold).
/// Yield threshold defaults to 32 KiB.
#[derive(Clone, Copy, Debug)]
pub struct BufferSettings {
    buffer_size: usize,
    yield_threshold: usize,
}

impl BufferSettings {
    /// Create a new `BufferSettings`
    pub fn new(buffer_size: usize, yield_threshold: usize) -> Self {
        Self {
            buffer_size,
            yield_threshold,
        }
    }
}

impl Default for BufferSettings {
    fn default() -> Self {
        Self {
            buffer_size: DEFAULT_CODEC_BUFFER_SIZE,
            yield_threshold: DEFAULT_YIELD_THRESHOLD,
        }
    }
}

// Doc hidden because it's used in tests in another crate but not part of the
// public api.
#[doc(hidden)]
pub const HEADER_SIZE: usize =
    // compression flag
    std::mem::size_of::<u8>() +
    // data length
    std::mem::size_of::<u32>();

// The default maximum uncompressed size in bytes for a message. Defaults to 4MB.
const DEFAULT_MAX_RECV_MESSAGE_SIZE: usize = 4 * 1024 * 1024;
const DEFAULT_MAX_SEND_MESSAGE_SIZE: usize = usize::MAX;

/// Trait that knows how to encode and decode gRPC messages.
pub trait Codec {
    /// The encodable message.
    type Encode: Send + 'static;
    /// The decodable message.
    type Decode: Send + 'static;

    /// The encoder that can encode a message.
    type Encoder: Encoder<Item = Self::Encode, Error = Status> + Send + 'static;
    /// The decoder that can decode a message.
    type Decoder: Decoder<Item = Self::Decode, Error = Status> + Send + 'static;

    /// Fetch the encoder.
    fn encoder(&mut self) -> Self::Encoder;
    /// Fetch the decoder.
    fn decoder(&mut self) -> Self::Decoder;
}

/// Encodes gRPC message types
pub trait Encoder {
    /// Opt into draining ready, uncompressed messages into one owned batch.
    /// This is separate from already-encoded message ownership transfer.
    fn supports_owned_batch(&self) -> bool {
        false
    }

    /// Encode `first` and a bounded number of immediately available messages.
    /// `next` stops on Pending, end, or a source error; never call it again
    /// after None. Return concatenated, uncompressed gRPC frames in source
    /// order. Tonic validates framing and each message's size before handoff.
    /// This hook is never used when compression is enabled.
    fn encode_owned_batch(
        &mut self,
        _first: Self::Item,
        _next: &mut dyn FnMut() -> Option<Self::Item>,
        _yield_threshold: usize,
    ) -> Result<bytes::Bytes, Self::Error> {
        Err(io::Error::other("owned batching is not supported").into())
    }

    /// Opt into the owned hook and lazy destination allocation. False preserves
    /// the ordinary encoder's eager allocation and compile-time buffered path.
    #[inline]
    fn supports_owned(&self) -> bool {
        false
    }
    /// The type that is encoded.
    type Item;

    /// The type of encoding errors.
    ///
    /// The type of unrecoverable frame encoding errors.
    type Error: From<io::Error>;

    /// Encodes a message into the provided buffer.
    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error>;

    /// Optional owned, uncompressed gRPC frame handoff (requires supports_owned).
    /// The default returns the
    /// item for ordinary buffered encoding; it must not encode it first.
    fn encode_owned(&mut self, item: Self::Item) -> Result<EncodeResult<Self::Item>, Self::Error> {
        Ok(EncodeResult::Buffered(item))
    }

    /// Controls how tonic creates and expands encode buffers.
    fn buffer_settings(&self) -> BufferSettings {
        BufferSettings::default()
    }
}

/// Result of the optional owned encoder hook.
#[derive(Debug)]
pub enum EncodeResult<T> {
    /// Encode this item through the ordinary destination-buffer interface.
    Buffered(T),
    /// An already encoded uncompressed message, including its gRPC header.
    Owned(OwnedMessage),
}

/// Validated immutable uncompressed gRPC frame. Its payload allocation can be
/// handed directly to the HTTP body, or read directly by a compressor.
#[derive(Debug)]
pub struct OwnedMessage {
    frame: bytes::Bytes,
}

impl OwnedMessage {
    /// Adopt a frame without copying. Requires a zero compression flag and a
    /// four-byte big-endian length matching the remaining payload. Message size
    /// policy is still enforced by EncodeBody, just as for buffered encoders.
    pub fn from_uncompressed_frame(frame: bytes::Bytes) -> Result<Self, crate::Status> {
        if frame.len() < HEADER_SIZE || frame[0] != 0 {
            return Err(crate::Status::internal("invalid owned gRPC frame header"));
        }
        let len = u32::from_be_bytes(frame[1..HEADER_SIZE].try_into().unwrap()) as usize;
        if len != frame.len() - HEADER_SIZE {
            return Err(crate::Status::internal("invalid owned gRPC frame length"));
        }
        Ok(Self { frame })
    }

    /// Borrow the protobuf payload, excluding its gRPC frame header.
    pub fn payload(&self) -> &[u8] {
        &self.frame[HEADER_SIZE..]
    }

    pub(crate) fn into_frame(self) -> bytes::Bytes {
        self.frame
    }
}

/// Decodes gRPC message types
pub trait Decoder {
    /// The type that is decoded.
    type Item;

    /// The type of unrecoverable frame decoding errors.
    type Error: From<io::Error>;

    /// Decode a message from the buffer.
    ///
    /// The buffer will contain exactly the bytes of a full message. There
    /// is no need to get the length from the bytes, gRPC framing is handled
    /// for you.
    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error>;

    /// Controls how tonic creates and expands decode buffers.
    fn buffer_settings(&self) -> BufferSettings {
        BufferSettings::default()
    }
}
