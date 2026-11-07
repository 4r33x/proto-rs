/*
 *
 * Copyright 2025 gRPC authors.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to
 * deal in the Software without restriction, including without limitation the
 * rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
 * sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
 * IN THE SOFTWARE.
 *
 */

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::ready;

use bytes::Buf;
use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
#[cfg(feature = "h2")]
use h2::{Error as H2Error, Reason as H2Reason};
use http::HeaderMap;
use http_body::Body;
use http_body::Frame;
use pin_project::pin_project;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tokio_stream::adapters::Fuse;

use super::BufferSettings;
use super::DEFAULT_MAX_SEND_MESSAGE_SIZE;
use super::EncodeBuf;
use super::EncodeResult;
use super::Encoder;
use super::HEADER_SIZE;
use super::compression::CompressionEncoding;
use super::compression::CompressionSettings;
use super::compression::SingleMessageCompressionOverride;
use super::compression::compress;
use super::compression::compress_buf;
use crate::Status;
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

/// Combinator for efficient encoding of messages into reasonably sized buffers.
/// EncodedBytes encodes ready messages from its delegate stream into a BytesMut,
/// splitting off and yielding a buffer when either:
///  * The delegate stream polls as not ready, or
///  * The encoded buffer surpasses YIELD_THRESHOLD.
#[pin_project(project = EncodedBytesProj)]
#[derive(Debug)]
struct EncodedBytes<T, U> {
    #[pin]
    source: Fuse<U>,
    encoder: T,
    compression_encoding: Option<CompressionEncoding>,
    max_message_size: Option<usize>,
    buf: BytesMut,
    uncompression_buf: BytesMut,
    error: Option<Status>,
    pending_owned: std::array::IntoIter<Bytes, 3>,
    pending_compression: Option<super::OwnedMessage>,
}

impl<T: Encoder, U: Stream> EncodedBytes<T, U> {
    fn new(
        encoder: T,
        source: U,
        compression_encoding: Option<CompressionEncoding>,
        compression_override: SingleMessageCompressionOverride,
        max_message_size: Option<usize>,
    ) -> Self {
        // Preserve eager allocation for ordinary encoders. An owned plain
        // frame must not allocate an unused 8 KiB destination for each RPC.
        let buffer_settings = encoder.buffer_settings();
        let buf = if encoder.supports_owned() {
            BytesMut::new()
        } else {
            BytesMut::with_capacity(buffer_settings.buffer_size)
        };

        let compression_encoding =
            if compression_override == SingleMessageCompressionOverride::Disable {
                None
            } else {
                compression_encoding
            };

        let uncompression_buf = if !encoder.supports_owned() && compression_encoding.is_some() {
            BytesMut::with_capacity(buffer_settings.buffer_size)
        } else {
            BytesMut::new()
        };

        Self {
            source: source.fuse(),
            encoder,
            compression_encoding,
            max_message_size,
            buf,
            uncompression_buf,
            error: None,
            pending_owned: [Bytes::new(), Bytes::new(), Bytes::new()].into_iter(),
            pending_compression: None,
        }
    }
}

impl<T, U> Stream for EncodedBytes<T, U>
where
    T: Encoder<Error = Status>,
    U: Stream<Item = Result<T::Item, Status>>,
{
    type Item = Result<Bytes, Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let EncodedBytesProj {
            mut source,
            encoder,
            compression_encoding,
            max_message_size,
            buf,
            uncompression_buf,
            error,
            pending_owned,
            pending_compression,
        } = self.project();
        let buffer_settings = encoder.buffer_settings();
        let small_threshold = buffer_settings.yield_threshold.min(32 * 1024);

        if let Some(frame) = pending_owned.find(|frame| !frame.is_empty()) {
            return Poll::Ready(Some(Ok(frame)));
        }

        if let Some(status) = error.take() {
            return Poll::Ready(Some(Err(status)));
        }

        let mut first_small: Option<[Bytes; 3]> = None;
        if let Some(message) = pending_compression.take() {
            match encode_owned_message(
                &message,
                cx,
                compression_encoding.unwrap(),
                *max_message_size,
                buffer_settings,
            ) {
                Poll::Ready(Ok(frame)) if frame.len() < small_threshold => {
                    // Resumed cache waiters still participate in batching.
                    first_small = Some([frame, Bytes::new(), Bytes::new()]);
                }
                Poll::Ready(frame) => return Poll::Ready(Some(frame)),
                Poll::Pending => {
                    *pending_compression = Some(message);
                    return Poll::Pending;
                }
            }
        }

        loop {
            match source.as_mut().poll_next(cx) {
                exhausted @ (Poll::Pending | Poll::Ready(None)) => {
                    if let Some(frames) = first_small.take() {
                        *pending_owned = frames.into_iter();
                        return Poll::Ready(pending_owned.find(|frame| !frame.is_empty()).map(Ok));
                    }
                    if buf.is_empty() {
                        return exhausted.map(|_| None);
                    }
                    return Poll::Ready(Some(Ok(buf.split_to(buf.len()).freeze())));
                }
                Poll::Ready(Some(Ok(item))) => {
                    if let Some(frames) = first_small.take() {
                        buf.reserve(buffer_settings.yield_threshold);
                        for frame in frames {
                            buf.extend_from_slice(&frame);
                        }
                    }
                    if compression_encoding.is_none() && encoder.supports_owned_batch() {
                        let frames = encoder.encode_owned_batch(
                            item,
                            &mut || match source.as_mut().poll_next(cx) {
                                Poll::Ready(Some(Ok(item))) => Some(item),
                                Poll::Ready(Some(Err(status))) => {
                                    *error = Some(status);
                                    None
                                }
                                Poll::Pending | Poll::Ready(None) => None,
                            },
                            buffer_settings.yield_threshold,
                        );
                        let frames = frames.and_then(|frames| {
                            check_batch(&frames, *max_message_size)?;
                            Ok(frames)
                        });
                        return match frames {
                            Ok(frames) if !buf.is_empty() => {
                                *pending_owned = [frames, Bytes::new(), Bytes::new()].into_iter();
                                Poll::Ready(Some(Ok(buf.split_to(buf.len()).freeze())))
                            }
                            frames => Poll::Ready(Some(frames)),
                        };
                    }
                    let offset = buf.len();
                    let owned = if encoder.supports_owned() {
                        encode_item_owned(
                            encoder,
                            buf,
                            uncompression_buf,
                            *compression_encoding,
                            *max_message_size,
                            buffer_settings,
                            item,
                            cx,
                            pending_compression,
                        )
                    } else {
                        encode_item(
                            encoder,
                            buf,
                            uncompression_buf,
                            *compression_encoding,
                            *max_message_size,
                            buffer_settings,
                            item,
                        )
                        .map(|()| None)
                    };
                    let owned = match owned {
                        Ok(owned) => owned,
                        Err(status) => {
                            // A buffered encoder may leave a partial failed message.
                            buf.truncate(offset);
                            if buf.is_empty() {
                                return Poll::Ready(Some(Err(status)));
                            }
                            *error = Some(status);
                            return Poll::Ready(Some(Ok(buf.split_to(buf.len()).freeze())));
                        }
                    };
                    if pending_compression.is_some() {
                        if buf.is_empty() {
                            return Poll::Pending;
                        }
                        return Poll::Ready(Some(Ok(buf.split_to(buf.len()).freeze())));
                    }
                    if let Some(frames) = owned {
                        let size = frames.iter().map(Bytes::len).sum::<usize>();
                        // Batch ready small messages, including cached compressed frames.
                        // Large payloads retain their shared owned segments.
                        if size < small_threshold {
                            if buf.is_empty() {
                                // Keep singleton messages owned. Only copy once a
                                // second message is already available for batching.
                                first_small = Some(frames);
                                continue;
                            }
                            for frame in frames {
                                buf.extend_from_slice(&frame);
                            }
                            if buf.len() >= buffer_settings.yield_threshold {
                                return Poll::Ready(Some(Ok(buf.split_to(buf.len()).freeze())));
                            }
                            continue;
                        }
                        *pending_owned = frames.into_iter();
                        if buf.is_empty() {
                            return Poll::Ready(
                                pending_owned.find(|frame| !frame.is_empty()).map(Ok),
                            );
                        }
                        // Preserve stream order when an encoder mixes owned
                        // frames with its ordinary coalesced buffered output.
                        return Poll::Ready(Some(Ok(buf.split_to(buf.len()).freeze())));
                    }

                    if buf.len() >= buffer_settings.yield_threshold {
                        return Poll::Ready(Some(Ok(buf.split_to(buf.len()).freeze())));
                    }
                }
                Poll::Ready(Some(Err(status))) => {
                    if let Some(frames) = first_small.take() {
                        *error = Some(status);
                        *pending_owned = frames.into_iter();
                        return Poll::Ready(pending_owned.find(|frame| !frame.is_empty()).map(Ok));
                    }
                    if buf.is_empty() {
                        return Poll::Ready(Some(Err(status)));
                    }
                    *error = Some(status);
                    return Poll::Ready(Some(Ok(buf.split_to(buf.len()).freeze())));
                }
            }
        }
    }
}

fn check_batch(mut frames: &[u8], max_message_size: Option<usize>) -> Result<(), Status> {
    if frames.is_empty() {
        return Err(Status::internal("empty owned gRPC batch"));
    }
    while !frames.is_empty() {
        if frames.len() < HEADER_SIZE || frames[0] != 0 {
            return Err(Status::internal("invalid owned gRPC batch header"));
        }
        let len = u32::from_be_bytes(frames[1..HEADER_SIZE].try_into().unwrap()) as usize;
        let payload = &frames[HEADER_SIZE..];
        if len > payload.len() {
            return Err(Status::internal("invalid owned gRPC batch length"));
        }
        check_message_size(len, max_message_size)?;
        frames = &payload[len..];
    }
    Ok(())
}

fn encode_item_owned<T>(
    encoder: &mut T,
    buf: &mut BytesMut,
    uncompression_buf: &mut BytesMut,
    compression_encoding: Option<CompressionEncoding>,
    max_message_size: Option<usize>,
    buffer_settings: BufferSettings,
    item: T::Item,
    cx: &mut Context<'_>,
    pending_compression: &mut Option<super::OwnedMessage>,
) -> Result<Option<[Bytes; 3]>, Status>
where
    T: Encoder<Error = Status>,
{
    let encoded = encoder.encode_owned(item);
    let item = match encoded.map_err(|err| Status::internal(format!("Error encoding: {err}")))? {
        EncodeResult::Buffered(item) => item,
        EncodeResult::Owned(message) => {
            if let Some(encoding) = compression_encoding {
                match encode_owned_message(
                    &message,
                    cx,
                    encoding,
                    max_message_size,
                    buffer_settings,
                ) {
                    Poll::Ready(frame) => {
                        return frame.map(|frame| Some([frame, Bytes::new(), Bytes::new()]));
                    }
                    Poll::Pending => {
                        *pending_compression = Some(message);
                        return Ok(None);
                    }
                }
            }
            check_message_size(message.payload().remaining(), max_message_size)?;
            return Ok(Some(message.into_frames()));
        }
    };
    if buf.capacity() == 0 {
        buf.reserve(buffer_settings.buffer_size);
    }
    if compression_encoding.is_some() && uncompression_buf.capacity() == 0 {
        uncompression_buf.reserve(buffer_settings.buffer_size);
    }
    encode_item(
        encoder,
        buf,
        uncompression_buf,
        compression_encoding,
        max_message_size,
        buffer_settings,
        item,
    )?;
    Ok(None)
}

fn encode_owned_message(
    message: &super::OwnedMessage,
    cx: &mut Context<'_>,
    encoding: CompressionEncoding,
    max_message_size: Option<usize>,
    buffer_settings: BufferSettings,
) -> Poll<Result<Bytes, Status>> {
    let compress = || {
        let mut frame = BytesMut::new();
        frame.extend_from_slice(&[0; HEADER_SIZE]);
        compress_buf(
            CompressionSettings {
                encoding,
                buffer_growth_interval: buffer_settings.buffer_size,
            },
            message.payload(),
            &mut frame,
        )
        .map_err(|err| Status::internal(format!("Error compressing: {err}")))?;
        finish_encoding(Some(encoding), None, &mut frame)?;
        Ok(frame.freeze())
    };
    let frame = match &message.compressed {
        Some(cache) => std::task::ready!(cache[encoding as usize].poll_frame(cx, compress)),
        None => compress(),
    };
    Poll::Ready(frame.and_then(|frame| {
        check_message_size(frame.len() - HEADER_SIZE, max_message_size)?;
        Ok(frame)
    }))
}

// Keep the original buffered entry point and return type. Owned-message
// dispatch stays outside the ordinary encoder's hot call path.
fn encode_item<T>(
    encoder: &mut T,
    buf: &mut BytesMut,
    uncompression_buf: &mut BytesMut,
    compression_encoding: Option<CompressionEncoding>,
    max_message_size: Option<usize>,
    buffer_settings: BufferSettings,
    item: T::Item,
) -> Result<(), Status>
where
    T: Encoder<Error = Status>,
{
    let offset = buf.len();

    buf.reserve(HEADER_SIZE);
    buf.put_slice(&[0u8; HEADER_SIZE]);

    if let Some(encoding) = compression_encoding {
        uncompression_buf.clear();

        encoder
            .encode(item, &mut EncodeBuf::new(uncompression_buf))
            .map_err(|err| Status::internal(format!("Error encoding: {err}")))?;

        let uncompressed_len = uncompression_buf.len();

        compress(
            CompressionSettings {
                encoding,
                buffer_growth_interval: buffer_settings.buffer_size,
            },
            uncompression_buf,
            buf,
            uncompressed_len,
        )
        .map_err(|err| Status::internal(format!("Error compressing: {err}")))?;
    } else {
        encoder
            .encode(item, &mut EncodeBuf::new(buf))
            .map_err(|err| Status::internal(format!("Error encoding: {err}")))?;
    }

    // now that we know length, we can write the header
    finish_encoding(compression_encoding, max_message_size, &mut buf[offset..])
}

fn finish_encoding(
    compression_encoding: Option<CompressionEncoding>,
    max_message_size: Option<usize>,
    buf: &mut [u8],
) -> Result<(), Status> {
    let len = buf.len() - HEADER_SIZE;
    check_message_size(len, max_message_size)?;
    {
        let mut buf = &mut buf[..HEADER_SIZE];
        buf.put_u8(compression_encoding.is_some() as u8);
        buf.put_u32(len as u32);
    }

    Ok(())
}

fn check_message_size(len: usize, max_message_size: Option<usize>) -> Result<(), Status> {
    let limit = max_message_size.unwrap_or(DEFAULT_MAX_SEND_MESSAGE_SIZE);
    if len > limit {
        return Err(Status::out_of_range(format!(
            "Error, encoded message length too large: found {len} bytes, the limit is: {limit} bytes"
        )));
    }

    if len > u32::MAX as usize {
        return Err(Status::resource_exhausted(format!(
            "Cannot return body with more than 4GB of data but got {len} bytes"
        )));
    }
    Ok(())
}

#[derive(Debug)]
enum Role {
    Client,
    Server,
}

/// A specialized implementation of [Body] for encoding [Result<Bytes, Status>].
#[pin_project]
#[derive(Debug)]
pub struct EncodeBody<T, U> {
    #[pin]
    inner: EncodedBytes<T, U>,
    state: EncodeState,
    #[pin]
    cancellation_fut: Option<WaitForCancellationFutureOwned>,
}

#[derive(Debug)]
struct EncodeState {
    error: Option<Status>,
    role: Role,
    is_end_stream: bool,
}

impl<T: Encoder, U: Stream> EncodeBody<T, U> {
    /// Turns a stream of grpc messages into [EncodeBody] which is used by grpc clients for
    /// turning the messages into http frames for sending over the network.
    pub fn new_client(
        encoder: T,
        source: U,
        compression_encoding: Option<CompressionEncoding>,
        max_message_size: Option<usize>,
    ) -> Self {
        Self::new_client_with_cancellation(
            encoder,
            source,
            compression_encoding,
            max_message_size,
            None,
        )
    }

    pub(crate) fn new_client_with_cancellation(
        encoder: T,
        source: U,
        compression_encoding: Option<CompressionEncoding>,
        max_message_size: Option<usize>,
        cancellation_token: Option<CancellationToken>,
    ) -> Self {
        let cancellation_fut = cancellation_token.map(|c| c.cancelled_owned());
        Self {
            inner: EncodedBytes::new(
                encoder,
                source,
                compression_encoding,
                SingleMessageCompressionOverride::default(),
                max_message_size,
            ),
            state: EncodeState {
                error: None,
                role: Role::Client,
                is_end_stream: false,
            },
            cancellation_fut,
        }
    }

    /// Turns a stream of grpc results (message or error status) into [EncodeBody] which is used by grpc
    /// servers for turning the messages into http frames for sending over the network.
    pub fn new_server(
        encoder: T,
        source: U,
        compression_encoding: Option<CompressionEncoding>,
        compression_override: SingleMessageCompressionOverride,
        max_message_size: Option<usize>,
    ) -> Self {
        Self {
            inner: EncodedBytes::new(
                encoder,
                source,
                compression_encoding,
                compression_override,
                max_message_size,
            ),
            state: EncodeState {
                error: None,
                role: Role::Server,
                is_end_stream: false,
            },
            cancellation_fut: None,
        }
    }
}

impl EncodeState {
    fn trailers(&mut self) -> Option<Result<HeaderMap, Status>> {
        match self.role {
            Role::Client => None,
            Role::Server => {
                if self.is_end_stream {
                    return None;
                }

                self.is_end_stream = true;
                let status = if let Some(status) = self.error.take() {
                    status
                } else {
                    Status::ok("")
                };
                Some(status.to_header_map())
            }
        }
    }
}

impl<T, U> Body for EncodeBody<T, U>
where
    T: Encoder<Error = Status>,
    U: Stream<Item = Result<T::Item, Status>>,
{
    type Data = Bytes;
    type Error = Status;

    fn is_end_stream(&self) -> bool {
        self.state.is_end_stream
    }

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let self_proj = self.project();

        if let Some(cancellation_fut) = self_proj.cancellation_fut.as_pin_mut()
            && let Poll::Ready(()) = cancellation_fut.poll(cx)
        {
            #[cfg(feature = "h2")]
            let status = {
                let mut status = Status::cancelled("client cancelled");
                // h2 inspects the error's source chain to determine the RST
                // code, so we set it here.
                status.set_source(std::sync::Arc::new(H2Error::from(H2Reason::CANCEL)));
                status
            };

            #[cfg(not(feature = "h2"))]
            let status = Status::cancelled("client cancelled");
            return Poll::Ready(Some(Err(status)));
        }

        match ready!(self_proj.inner.poll_next(cx)) {
            Some(Ok(d)) => Some(Ok(Frame::data(d))).into(),
            Some(Err(status)) => match self_proj.state.role {
                Role::Client => Some(Err(status)).into(),
                Role::Server => {
                    self_proj.state.is_end_stream = true;
                    Some(Ok(Frame::trailers(status.to_header_map()?))).into()
                }
            },
            None => self_proj.state.trailers().map(|t| t.map(Frame::trailers)).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OwnedEncoder;

    impl Encoder for OwnedEncoder {
        type Item = super::super::OwnedMessage;
        type Error = Status;

        fn encode(&mut self, _: Self::Item, _: &mut EncodeBuf<'_>) -> Result<(), Status> {
            unreachable!("owned messages bypass buffered encoding")
        }

        fn supports_owned(&self) -> bool {
            true
        }

        fn encode_owned(&mut self, item: Self::Item) -> Result<EncodeResult<Self::Item>, Status> {
            Ok(EncodeResult::Owned(item))
        }
    }

    #[tokio::test]
    async fn segmented_messages_batch_in_order_without_copying_large_payloads() {
        let small = Bytes::from_static(b"small");
        let large = Bytes::from(vec![9; 65536]);
        let suffix = Bytes::from_static(b"suffix");
        let messages = [
            super::super::OwnedMessage::from_segments(small.clone(), Bytes::new()),
            super::super::OwnedMessage::from_segments(small.clone(), Bytes::new()),
            super::super::OwnedMessage::from_segments(large.clone(), suffix.clone()),
            super::super::OwnedMessage::from_segments(small, Bytes::new()),
        ];
        let mut frames = EncodedBytes::new(
            OwnedEncoder,
            tokio_stream::iter(messages),
            None,
            SingleMessageCompressionOverride::Inherit,
            None,
        );
        let mut chunks = Vec::new();
        while let Some(frame) = frames.next().await {
            chunks.push(frame.unwrap());
        }
        assert_eq!(chunks.len(), 6);
        assert_eq!(chunks[2].as_ptr(), large.as_ptr());
        let mut wire = Bytes::from(chunks.concat());
        let mut payloads = Vec::new();
        while wire.has_remaining() {
            assert_eq!(wire.get_u8(), 0);
            let len = wire.get_u32() as usize;
            payloads.push(wire.split_to(len));
        }
        assert_eq!(payloads.len(), 4);
        assert_eq!(payloads[0].as_ref(), b"small");
        assert_eq!(payloads[1].as_ref(), b"small");
        assert_eq!(payloads[2].as_ref(), [large.as_ref(), suffix.as_ref()].concat());
        assert_eq!(payloads[3].as_ref(), b"small");
    }

    #[cfg(feature = "gzip")]
    #[tokio::test]
    async fn cached_segmented_gzip_roundtrips_and_checks_each_subscribers_limit() {
        use std::io::Read;
        let message = super::super::OwnedMessage::from_segments(
            Bytes::from(vec![42; 4096]), Bytes::from_static(b"suffix"),
        ).unwrap().with_compression_cache();
        let mut cached_frame: Option<Bytes> = None;
        for limit in [None, None, Some(1)] {
            let mut frames = EncodedBytes::new(
                OwnedEncoder,
                tokio_stream::iter([Ok(message.clone())]),
                Some(CompressionEncoding::Gzip),
                SingleMessageCompressionOverride::Inherit,
                limit,
            );
            let frame = frames.next().await.unwrap();
            if limit.is_some() {
                assert_eq!(frame.unwrap_err().code(), crate::Code::OutOfRange);
                continue;
            }
            let frame = frame.unwrap();
            assert_eq!(frame[0], 1);
            assert_eq!(u32::from_be_bytes(frame[1..5].try_into().unwrap()) as usize, frame.len() - 5);
            let mut decoded = Vec::new();
            flate2::read::GzDecoder::new(&frame[5..]).read_to_end(&mut decoded).unwrap();
            assert_eq!(decoded, [vec![42; 4096], b"suffix".to_vec()].concat());
            if let Some(previous) = &cached_frame {
                assert_eq!(frame.as_ptr(), previous.as_ptr());
            }
            cached_frame = Some(frame);
            assert!(frames.next().await.is_none());
        }
    }

    struct PartialEncoder;

    impl Encoder for PartialEncoder {
        type Item = &'static [u8];
        type Error = Status;

        fn encode(&mut self, item: Self::Item, buf: &mut EncodeBuf<'_>) -> Result<(), Status> {
            buf.put_slice(item);
            if item == b"bad" {
                return Err(Status::internal("failed after writing"));
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn buffered_errors_flush_only_complete_messages() {
        for invalid in [b"bad".as_slice(), b"too large".as_slice()] {
            let mut frames = EncodedBytes::new(
                PartialEncoder,
                tokio_stream::iter([Ok(b"ok".as_slice()), Ok(invalid)]),
                None,
                SingleMessageCompressionOverride::Inherit,
                Some(2),
            );
            assert_eq!(
                frames.next().await.unwrap().unwrap().as_ref(),
                b"\0\0\0\0\x02ok"
            );
            assert!(frames.next().await.unwrap().is_err());
        }
    }
}

#[cfg(test)]
mod upstream_exception_safety_tests {
    use super::*;
    use std::panic::catch_unwind;

    struct PanickingEncoder;

    impl Encoder for PanickingEncoder {
        type Item = String;
        type Error = Status;

        fn encode(
            &mut self,
            _item: Self::Item,
            _dst: &mut EncodeBuf<'_>,
        ) -> Result<(), Self::Error> {
            panic!("encoder deliberate panic for testing exception safety");
        }
    }

    #[test]
    fn encode_item_exception_safety_on_panic() {
        let mut encoder = PanickingEncoder;
        let mut buf = BytesMut::new();
        let mut uncompression_buf = BytesMut::new();

        let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
            encode_item(
                &mut encoder,
                &mut buf,
                &mut uncompression_buf,
                None,
                None,
                BufferSettings::default(),
                "test".to_string(),
            )
        }));

        assert!(result.is_err(), "Encoder panic should unwind correctly.");
        // Buffer must only contain initialized bytes (5 zero bytes written by put_slice).
        assert_eq!(
            buf.len(),
            HEADER_SIZE,
            "Buffer length should reflect reserved header bytes."
        );
        assert_eq!(
            &buf[..],
            &[0u8; HEADER_SIZE],
            "Buffer must contain only initialized zero bytes."
        );
    }
}
