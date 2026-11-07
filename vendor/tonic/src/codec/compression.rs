use crate::{Status, metadata::MetadataValue};
use bytes::{Buf, BufMut, BytesMut};
#[cfg(feature = "gzip")]
use flate2::read::GzDecoder;
#[cfg(feature = "deflate")]
use flate2::{bufread::ZlibEncoder, read::ZlibDecoder};
use std::{borrow::Cow, fmt};
#[cfg(feature = "zstd")]
use zstd::stream::read::{Decoder, Encoder};

pub(crate) const ENCODING_HEADER: &str = "grpc-encoding";
pub(crate) const ACCEPT_ENCODING_HEADER: &str = "grpc-accept-encoding";

/// Struct used to configure which encodings are enabled on a server or channel.
///
/// Represents an ordered list of compression encodings that are enabled.
#[derive(Debug, Default, Clone, Copy)]
pub struct EnabledCompressionEncodings {
    inner: [Option<CompressionEncoding>; 3],
}

impl EnabledCompressionEncodings {
    /// Enable a [`CompressionEncoding`].
    ///
    /// Adds the new encoding to the end of the encoding list.
    pub fn enable(&mut self, encoding: CompressionEncoding) {
        for e in self.inner.iter_mut() {
            match e {
                Some(e) if *e == encoding => return,
                None => {
                    *e = Some(encoding);
                    return;
                }
                _ => continue,
            }
        }
    }

    /// Remove the last [`CompressionEncoding`].
    pub fn pop(&mut self) -> Option<CompressionEncoding> {
        self.inner
            .iter_mut()
            .rev()
            .find(|entry| entry.is_some())?
            .take()
    }

    pub(crate) fn into_accept_encoding_header_value(self) -> Option<http::HeaderValue> {
        let mut value = BytesMut::new();
        for encoding in self.inner.into_iter().flatten() {
            value.put_slice(encoding.as_str().as_bytes());
            value.put_u8(b',');
        }

        if value.is_empty() {
            return None;
        }

        value.put_slice(b"identity");
        Some(http::HeaderValue::from_maybe_shared(value).unwrap())
    }

    /// Check if a [`CompressionEncoding`] is enabled.
    pub fn is_enabled(&self, encoding: CompressionEncoding) -> bool {
        self.inner.contains(&Some(encoding))
    }

    /// Check if any [`CompressionEncoding`]s are enabled.
    pub fn is_empty(&self) -> bool {
        self.inner.iter().all(|e| e.is_none())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CompressionSettings {
    pub(crate) encoding: CompressionEncoding,
    /// buffer_growth_interval controls memory growth for internal buffers to balance resizing cost against memory waste.
    /// The default buffer growth interval is 8 kilobytes.
    pub(crate) buffer_growth_interval: usize,
}

/// The compression encodings Tonic supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompressionEncoding {
    #[allow(missing_docs)]
    #[cfg(feature = "gzip")]
    Gzip,
    #[allow(missing_docs)]
    #[cfg(feature = "deflate")]
    Deflate,
    #[allow(missing_docs)]
    #[cfg(feature = "zstd")]
    Zstd,
}

impl CompressionEncoding {
    pub(crate) const ENCODINGS: &'static [CompressionEncoding] = &[
        #[cfg(feature = "gzip")]
        CompressionEncoding::Gzip,
        #[cfg(feature = "deflate")]
        CompressionEncoding::Deflate,
        #[cfg(feature = "zstd")]
        CompressionEncoding::Zstd,
    ];

    /// Based on the `grpc-accept-encoding` header, pick an encoding to use.
    pub(crate) fn from_accept_encoding_header(
        map: &http::HeaderMap,
        enabled_encodings: EnabledCompressionEncodings,
    ) -> Option<Self> {
        if enabled_encodings.is_empty() {
            return None;
        }

        let header_value = map.get(ACCEPT_ENCODING_HEADER)?;
        let header_value_str = header_value.to_str().ok()?;

        split_by_comma(header_value_str).find_map(|value| match value {
            #[cfg(feature = "gzip")]
            "gzip" => Some(CompressionEncoding::Gzip),
            #[cfg(feature = "deflate")]
            "deflate" => Some(CompressionEncoding::Deflate),
            #[cfg(feature = "zstd")]
            "zstd" => Some(CompressionEncoding::Zstd),
            _ => None,
        })
    }

    /// Get the value of `grpc-encoding` header. Returns an error if the encoding isn't supported.
    pub(crate) fn from_encoding_header(
        map: &http::HeaderMap,
        enabled_encodings: EnabledCompressionEncodings,
    ) -> Result<Option<Self>, Status> {
        let Some(header_value) = map.get(ENCODING_HEADER) else {
            return Ok(None);
        };

        match header_value.as_bytes() {
            #[cfg(feature = "gzip")]
            b"gzip" if enabled_encodings.is_enabled(CompressionEncoding::Gzip) => {
                Ok(Some(CompressionEncoding::Gzip))
            }
            #[cfg(feature = "deflate")]
            b"deflate" if enabled_encodings.is_enabled(CompressionEncoding::Deflate) => {
                Ok(Some(CompressionEncoding::Deflate))
            }
            #[cfg(feature = "zstd")]
            b"zstd" if enabled_encodings.is_enabled(CompressionEncoding::Zstd) => {
                Ok(Some(CompressionEncoding::Zstd))
            }
            b"identity" => Ok(None),
            other => {
                let other = match std::str::from_utf8(other) {
                    Ok(s) => Cow::Borrowed(s),
                    Err(_) => Cow::Owned(format!("{other:?}")),
                };

                let mut status = Status::unimplemented(format!(
                    "Content is compressed with `{other}` which isn't supported"
                ));

                let header_value = enabled_encodings
                    .into_accept_encoding_header_value()
                    .map(MetadataValue::unchecked_from_header_value)
                    .unwrap_or_else(|| MetadataValue::from_static("identity"));
                status
                    .metadata_mut()
                    .insert(ACCEPT_ENCODING_HEADER, header_value);

                Err(status)
            }
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            #[cfg(feature = "gzip")]
            CompressionEncoding::Gzip => "gzip",
            #[cfg(feature = "deflate")]
            CompressionEncoding::Deflate => "deflate",
            #[cfg(feature = "zstd")]
            CompressionEncoding::Zstd => "zstd",
        }
    }

    #[cfg(any(feature = "gzip", feature = "deflate", feature = "zstd"))]
    pub(crate) fn into_header_value(self) -> http::HeaderValue {
        http::HeaderValue::from_static(self.as_str())
    }
}

impl fmt::Display for CompressionEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn split_by_comma(s: &str) -> impl Iterator<Item = &str> {
    s.split(',').map(|s| s.trim())
}

/// Compress `len` bytes from `decompressed_buf` into `out_buf`.
/// buffer_size_increment is a hint to control the growth of out_buf versus the cost of resizing it.
#[allow(unused_variables, unreachable_code)]
pub(crate) fn compress(
    settings: CompressionSettings,
    decompressed_buf: &mut BytesMut,
    out_buf: &mut BytesMut,
    len: usize,
) -> Result<(), std::io::Error> {
    compress_buf(settings, &decompressed_buf[..len], out_buf)?;
    decompressed_buf.advance(len);
    Ok(())
}

/// Compress an immutable owned-message payload. Gzip joins segmented input
/// for its whole-message compressor; other encodings read the segments directly.
#[allow(unused_variables, unreachable_code)]
pub(crate) fn compress_buf(
    settings: CompressionSettings,
    decompressed_buf: impl Buf,
    out_buf: &mut BytesMut,
) -> Result<(), std::io::Error> {
    let growth = settings
        .buffer_growth_interval
        .max(1)
        .min(decompressed_buf.remaining().saturating_add(128));

    match settings.encoding {
        #[cfg(feature = "gzip")]
        CompressionEncoding::Gzip => {
            compress_gzip(decompressed_buf, out_buf)?;
        }
        #[cfg(feature = "deflate")]
        CompressionEncoding::Deflate => {
            let mut deflate_encoder = ZlibEncoder::new(
                decompressed_buf.reader(),
                // FIXME: support customizing the compression level
                flate2::Compression::new(6),
            );
            read_compressed(&mut deflate_encoder, out_buf, growth)?;
        }
        #[cfg(feature = "zstd")]
        CompressionEncoding::Zstd => {
            let mut zstd_encoder = Encoder::with_buffer(
                decompressed_buf.reader(),
                // FIXME: support customizing the compression level
                zstd::DEFAULT_COMPRESSION_LEVEL,
            )?;
            read_compressed(&mut zstd_encoder, out_buf, growth)?;
        }
    }

    Ok(())
}

#[cfg(feature = "gzip")]
fn compress_gzip(mut input: impl Buf, output: &mut BytesMut) -> std::io::Result<()> {
    use libdeflater::{CompressionLvl, Compressor};
    thread_local! {
        static COMPRESSOR: std::cell::RefCell<Option<Compressor>> = const { std::cell::RefCell::new(None) };
    }
    let mut compress = |compressor: &mut Compressor, output: &mut BytesMut| {
        let length = input.remaining();
        let bound = compressor.gzip_compress_bound(length);
        let start = output.len();
        let end = start
            .checked_add(bound)
            .ok_or_else(|| std::io::Error::other("gzip output size overflow"))?;
        output.resize(end, 0);
        let result = if input.chunk().len() == length {
            compressor.gzip_compress(input.chunk(), &mut output[start..])
        } else {
            // The bulk body and small filter suffix must form one gzip member.
            let joined = input.copy_to_bytes(length);
            compressor.gzip_compress(&joined, &mut output[start..])
        };
        match result {
            Ok(written) => {
                output.truncate(start + written);
                Ok(())
            }
            Err(error) => {
                output.truncate(start);
                Err(std::io::Error::other(error))
            }
        }
    };
    COMPRESSOR.with(|slot| {
        // Custom Buf implementations may reenter compression.
        let Ok(mut slot) = slot.try_borrow_mut() else {
            return compress(&mut Compressor::new(CompressionLvl::new(1).expect("libdeflate supports level 1")), output);
        };
        compress(
            slot.get_or_insert_with(|| Compressor::new(CompressionLvl::new(1).expect("libdeflate supports level 1"))),
            output,
        )
    })
}

// Read directly into the final allocation, without io::copy's staging buffer.
#[cfg(any(feature = "deflate", feature = "zstd"))]
fn read_compressed(
    reader: &mut impl std::io::Read,
    out: &mut BytesMut,
    growth: usize,
) -> std::io::Result<()> {
    loop {
        let start = out.len();
        out.resize(start + growth, 0);
        let result = reader.read(&mut out[start..]);
        out.truncate(start + result.as_ref().copied().unwrap_or(0));
        match result {
            Ok(0) => return Ok(()),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

/// Decompress `len` bytes from `compressed_buf` into `out_buf`.
#[allow(unused_variables, unreachable_code)]
pub(crate) fn decompress(
    settings: CompressionSettings,
    compressed_buf: &mut BytesMut,
    mut out_buf: bytes::buf::Limit<&mut BytesMut>,
    len: usize,
) -> Result<(), std::io::Error> {
    let buffer_growth_interval = settings.buffer_growth_interval;
    let estimate_decompressed_len = len * 2;
    let capacity = std::cmp::min(
        bytes::buf::Limit::limit(&out_buf),
        ((estimate_decompressed_len / buffer_growth_interval) + 1) * buffer_growth_interval,
    );

    out_buf.get_mut().reserve(capacity);

    #[cfg(any(feature = "gzip", feature = "deflate", feature = "zstd"))]
    let mut out_writer = out_buf.writer();

    match settings.encoding {
        #[cfg(feature = "gzip")]
        CompressionEncoding::Gzip => {
            let mut gzip_decoder = GzDecoder::new(&compressed_buf[0..len]);
            std::io::copy(&mut gzip_decoder, &mut out_writer)?;
        }
        #[cfg(feature = "deflate")]
        CompressionEncoding::Deflate => {
            let mut deflate_decoder = ZlibDecoder::new(&compressed_buf[0..len]);
            std::io::copy(&mut deflate_decoder, &mut out_writer)?;
        }
        #[cfg(feature = "zstd")]
        CompressionEncoding::Zstd => {
            let mut zstd_decoder = Decoder::new(&compressed_buf[0..len])?;
            std::io::copy(&mut zstd_decoder, &mut out_writer)?;
        }
    }

    compressed_buf.advance(len);

    Ok(())
}

/// Controls compression behavior for individual messages within a stream.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SingleMessageCompressionOverride {
    /// Inherit whatever compression is already configured. If the stream is compressed this
    /// message will also be configured.
    ///
    /// This is the default.
    #[default]
    Inherit,
    /// Don't compress this message, even if compression is enabled on the stream.
    Disable,
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(all(feature = "gzip", feature = "deflate", feature = "zstd"))]
    fn segmented_inputs_roundtrip_with_each_compressor() {
        use std::io::Read;
        for encoding in [
            CompressionEncoding::Gzip,
            CompressionEncoding::Deflate,
            CompressionEncoding::Zstd,
        ] {
            for len in [0, 4096, 17] {
                let body = vec![42; len];
                let suffix = b"suffix";
                let mut compressed = BytesMut::new();
                compress_buf(
                    CompressionSettings { encoding, buffer_growth_interval: 32 },
                    bytes::Buf::chain(body.as_slice(), suffix.as_slice()),
                    &mut compressed,
                ).unwrap();
                let mut decoded = Vec::new();
                match encoding {
                    CompressionEncoding::Gzip => {
                        flate2::read::GzDecoder::new(compressed.as_ref()).read_to_end(&mut decoded).unwrap();
                    }
                    CompressionEncoding::Deflate => {
                        flate2::read::ZlibDecoder::new(compressed.as_ref()).read_to_end(&mut decoded).unwrap();
                    }
                    CompressionEncoding::Zstd => {
                        zstd::stream::read::Decoder::new(compressed.as_ref()).unwrap().read_to_end(&mut decoded).unwrap();
                    }
                }
                assert_eq!(decoded, [body.as_slice(), suffix].concat());
            }
        }
    }

    #[cfg(any(feature = "gzip", feature = "deflate", feature = "zstd"))]
    use http::HeaderValue;

    use super::*;

    #[test]
    fn convert_none_into_header_value() {
        let encodings = EnabledCompressionEncodings::default();

        assert!(encodings.into_accept_encoding_header_value().is_none());
    }

    #[test]
    #[cfg(feature = "gzip")]
    fn convert_gzip_into_header_value() {
        const GZIP: HeaderValue = HeaderValue::from_static("gzip,identity");

        let encodings = EnabledCompressionEncodings {
            inner: [Some(CompressionEncoding::Gzip), None, None],
        };

        assert_eq!(encodings.into_accept_encoding_header_value().unwrap(), GZIP);

        let encodings = EnabledCompressionEncodings {
            inner: [None, None, Some(CompressionEncoding::Gzip)],
        };

        assert_eq!(encodings.into_accept_encoding_header_value().unwrap(), GZIP);
    }

    #[test]
    #[cfg(feature = "zstd")]
    fn convert_zstd_into_header_value() {
        const ZSTD: HeaderValue = HeaderValue::from_static("zstd,identity");

        let encodings = EnabledCompressionEncodings {
            inner: [Some(CompressionEncoding::Zstd), None, None],
        };

        assert_eq!(encodings.into_accept_encoding_header_value().unwrap(), ZSTD);

        let encodings = EnabledCompressionEncodings {
            inner: [None, None, Some(CompressionEncoding::Zstd)],
        };

        assert_eq!(encodings.into_accept_encoding_header_value().unwrap(), ZSTD);
    }

    #[test]
    #[cfg(all(feature = "gzip", feature = "deflate", feature = "zstd"))]
    fn convert_compression_encodings_into_header_value() {
        let encodings = EnabledCompressionEncodings {
            inner: [
                Some(CompressionEncoding::Gzip),
                Some(CompressionEncoding::Deflate),
                Some(CompressionEncoding::Zstd),
            ],
        };

        assert_eq!(
            encodings.into_accept_encoding_header_value().unwrap(),
            HeaderValue::from_static("gzip,deflate,zstd,identity"),
        );

        let encodings = EnabledCompressionEncodings {
            inner: [
                Some(CompressionEncoding::Zstd),
                Some(CompressionEncoding::Deflate),
                Some(CompressionEncoding::Gzip),
            ],
        };

        assert_eq!(
            encodings.into_accept_encoding_header_value().unwrap(),
            HeaderValue::from_static("zstd,deflate,gzip,identity"),
        );
    }
}
