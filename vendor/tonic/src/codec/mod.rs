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
#[derive(Debug, Clone)]
pub struct OwnedMessage {
    frames: [bytes::Bytes; 3],
    compressed: Option<std::sync::Arc<[CompressionCache; 3]>>,
}

#[derive(Debug, Default)]
struct CompressionCache {
    frame: std::sync::OnceLock<Result<bytes::Bytes, Status>>,
    waiters: std::sync::Mutex<Option<Vec<std::task::Waker>>>,
}

struct CompressionCompletion<'a>(&'a CompressionCache);

impl Drop for CompressionCompletion<'_> {
    fn drop(&mut self) {
        let waiters = self.0.waiters.lock().unwrap().take().unwrap();
        for waiter in waiters {
            waiter.wake();
        }
    }
}

impl CompressionCache {
    fn poll_frame(
        &self,
        cx: &mut std::task::Context<'_>,
        compress: impl FnOnce() -> Result<bytes::Bytes, Status>,
    ) -> std::task::Poll<Result<bytes::Bytes, Status>> {
        use std::task::Poll;
        if let Some(frame) = self.frame.get() {
            return Poll::Ready(frame.clone());
        }
        let mut state = self.waiters.lock().unwrap();
        // Completion can race with acquiring the waiter registration lock.
        if let Some(frame) = self.frame.get() {
            return Poll::Ready(frame.clone());
        }
        if let Some(waiters) = state.as_mut() {
            if !waiters.iter().any(|waker| waker.will_wake(cx.waker())) {
                waiters.push(cx.waker().clone());
            }
            return Poll::Pending;
        }
        *state = Some(Vec::new());
        drop(state);
        // Release and wake waiters even if compression unwinds.
        let _completion = CompressionCompletion(self);
        Poll::Ready(self.frame.get_or_init(compress).clone())
    }
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
        Ok(Self {
            frames: [frame, bytes::Bytes::new(), bytes::Bytes::new()],
            compressed: None,
        })
    }

    /// Frame a shared protobuf body and trailing fields without joining them.
    pub fn from_segments(body: bytes::Bytes, suffix: bytes::Bytes) -> Result<Self, crate::Status> {
        let len = body
            .len()
            .checked_add(suffix.len())
            .and_then(|len| u32::try_from(len).ok())
            .ok_or_else(|| {
                crate::Status::resource_exhausted("message exceeds gRPC's 4 GiB limit")
            })?;
        let mut header = [0; HEADER_SIZE];
        header[1..].copy_from_slice(&len.to_be_bytes());
        Ok(Self {
            frames: [bytes::Bytes::copy_from_slice(&header), body, suffix],
            compressed: None,
        })
    }

    /// Cache compression once per encoding across clones of this message.
    /// Entries live only as long as the shared message; compression settings are fixed.
    pub fn with_compression_cache(mut self) -> Self {
        self.compressed.get_or_insert_with(|| {
            std::sync::Arc::new(std::array::from_fn(|_| CompressionCache::default()))
        });
        self
    }

    /// Borrow the protobuf segments, excluding the gRPC header.
    pub fn payload(&self) -> impl bytes::Buf + '_ {
        use bytes::Buf;
        self.frames[0][HEADER_SIZE..]
            .chain(&self.frames[1][..])
            .chain(&self.frames[2][..])
    }

    pub(crate) fn into_frames(self) -> [bytes::Bytes; 3] {
        self.frames
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

#[cfg(test)]
mod compression_cache_tests {
    use super::CompressionCache;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
            mpsc,
        },
        task::{Context, Wake, Waker},
    };

    #[derive(Default)]
    struct WakeCount(AtomicUsize);

    impl Wake for WakeCount {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn compression_cache_recovers_after_producer_panics() {
        let cache = Arc::new(CompressionCache::default());
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let producer_cache = cache.clone();
        let producer = std::thread::spawn(move || {
            let waker = Waker::from(Arc::new(WakeCount::default()));
            producer_cache.poll_frame(&mut Context::from_waker(&waker), || {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                panic!("compression failed");
            })
        });
        started_rx.recv().unwrap();
        let wakes = Arc::new(WakeCount::default());
        let waker = Waker::from(wakes.clone());
        let mut cx = Context::from_waker(&waker);
        for _ in 0..2 {
            assert!(
                cache
                    .poll_frame(&mut cx, || panic!("another producer started"))
                    .is_pending()
            );
        }
        release_tx.send(()).unwrap();
        assert!(producer.join().is_err());
        assert_eq!(wakes.0.load(Ordering::Relaxed), 1);
        let frame = bytes::Bytes::from_static(b"recovered");
        assert!(matches!(
            cache.poll_frame(&mut cx, || Ok(frame.clone())),
            std::task::Poll::Ready(Ok(ref value)) if value == &frame
        ));
        assert!(matches!(
            cache.poll_frame(&mut cx, || panic!("cached frame recompressed")),
            std::task::Poll::Ready(Ok(ref value)) if value == &frame
        ));
    }
}
