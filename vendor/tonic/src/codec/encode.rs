use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::ready;

use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
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
use super::compression::compress_slice;
use crate::Status;

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
    pending_owned: Option<Bytes>,
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
            pending_owned: None,
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
        } = self.project();
        let buffer_settings = encoder.buffer_settings();

        if let Some(frame) = pending_owned.take() {
            return Poll::Ready(Some(Ok(frame)));
        }

        if let Some(status) = error.take() {
            return Poll::Ready(Some(Err(status)));
        }

        loop {
            match source.as_mut().poll_next(cx) {
                Poll::Pending if buf.is_empty() => {
                    return Poll::Pending;
                }
                Poll::Ready(None) if buf.is_empty() => {
                    return Poll::Ready(None);
                }
                Poll::Pending | Poll::Ready(None) => {
                    return Poll::Ready(Some(Ok(buf.split_to(buf.len()).freeze())));
                }
                Poll::Ready(Some(Ok(item))) => {
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
                                *pending_owned = Some(frames);
                                Poll::Ready(Some(Ok(buf.split_to(buf.len()).freeze())))
                            }
                            frames => Poll::Ready(Some(frames)),
                        };
                    }
                    let owned = if encoder.supports_owned() {
                        encode_item_owned(
                            encoder,
                            buf,
                            uncompression_buf,
                            *compression_encoding,
                            *max_message_size,
                            buffer_settings,
                            item,
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
                        Err(status) => return Poll::Ready(Some(Err(status))),
                    };
                    if let Some(frame) = owned {
                        if buf.is_empty() {
                            return Poll::Ready(Some(Ok(frame)));
                        }
                        // Preserve stream order when an encoder mixes owned
                        // frames with its ordinary coalesced buffered output.
                        *pending_owned = Some(frame);
                        return Poll::Ready(Some(Ok(buf.split_to(buf.len()).freeze())));
                    }

                    if buf.len() >= buffer_settings.yield_threshold {
                        return Poll::Ready(Some(Ok(buf.split_to(buf.len()).freeze())));
                    }
                }
                Poll::Ready(Some(Err(status))) => {
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
) -> Result<Option<Bytes>, Status>
where
    T: Encoder<Error = Status>,
{
    let encoded = encoder.encode_owned(item);
    let item = match encoded.map_err(|err| Status::internal(format!("Error encoding: {err}")))? {
        EncodeResult::Buffered(item) => item,
        EncodeResult::Owned(message) => {
            if let Some(encoding) = compression_encoding {
                if buf.capacity() == 0 {
                    buf.reserve(buffer_settings.buffer_size);
                }
                let offset = buf.len();
                buf.extend_from_slice(&[0; HEADER_SIZE]);
                compress_slice(
                    CompressionSettings {
                        encoding,
                        buffer_growth_interval: buffer_settings.buffer_size,
                    },
                    message.payload(),
                    buf,
                )
                .map_err(|err| Status::internal(format!("Error compressing: {err}")))?;
                finish_encoding(compression_encoding, max_message_size, &mut buf[offset..])?;
                return Ok(None);
            }
            check_message_size(message.payload().len(), max_message_size)?;
            return Ok(Some(message.into_frame()));
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
    unsafe {
        buf.advance_mut(HEADER_SIZE);
    }

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
