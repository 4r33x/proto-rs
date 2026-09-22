#![allow(dead_code)]

use core::marker::PhantomData;

use crate::alloc::vec::Vec;

pub trait AsBytes {
    fn as_bytes(&self) -> &[u8];

    /// Optional ownership transfer; ordinary byte containers keep their copy fallback.
    #[cfg(feature = "tonic-owned")]
    fn into_owned_message(self) -> Result<tonic::codec::EncodeResult<Self>, tonic::Status>
    where
        Self: Sized,
    {
        Ok(tonic::codec::EncodeResult::Buffered(self))
    }
}

impl AsBytes for Vec<u8> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self
    }
}
impl<const N: usize> AsBytes for [u8; N] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

#[derive(Clone, Copy, Default)]
pub struct BytesMode;
#[derive(Clone, Copy, Default)]
pub struct SunByRef; // Sun<'a> = &'a T
#[derive(Clone, Copy, Default)]
pub struct SunByRefDeref; // Sun<'a> = &'a T::Target

#[derive(Debug, Clone)]
pub struct ProtoCodec<Encode = (), Decode = (), Mode = SunByRef> {
    _marker: PhantomData<(Encode, Decode, Mode)>,
    pub(crate) max_encode_preallocation: usize,
}

/// Default upper bound on speculative reservation in Tonic's output buffer.
pub const DEFAULT_MAX_ENCODE_PREALLOCATION: usize = 1024 * 1024;

impl<Encode, Decode, Mode> Default for ProtoCodec<Encode, Decode, Mode> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Encode, Decode, Mode> ProtoCodec<Encode, Decode, Mode> {
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
            max_encode_preallocation: DEFAULT_MAX_ENCODE_PREALLOCATION,
        }
    }

    /// Limit hint-based reservation in Tonic's userspace output buffer.
    /// Defaults to 1 MiB; values below 64 are treated as 64. Raise this for
    /// trusted large messages to avoid initial buffer growth. This is NOT a message
    /// size or total memory limit; configure Tonic's message limits separately.
    #[must_use]
    pub const fn with_max_encode_preallocation(mut self, limit: usize) -> Self {
        self.max_encode_preallocation = limit;
        self
    }
}

#[derive(Debug)]
pub struct ProtoEncoder<T, Mode> {
    _marker: core::marker::PhantomData<(T, Mode)>,
    pub(crate) max_encode_preallocation: usize,
    // Reuse item staging within a streaming RPC. Payloads are never staged here.
    #[cfg(feature = "tonic")]
    pub(crate) batch_items: Vec<T>,
}

impl<T, Mode> ProtoEncoder<T, Mode> {
    /// See [`ProtoCodec::with_max_encode_preallocation`].
    #[must_use]
    pub const fn with_max_encode_preallocation(mut self, limit: usize) -> Self {
        self.max_encode_preallocation = limit;
        self
    }
}

impl<T, Mode> Clone for ProtoEncoder<T, Mode> {
    fn clone(&self) -> Self {
        Self {
            _marker: PhantomData,
            max_encode_preallocation: self.max_encode_preallocation,
            #[cfg(feature = "tonic")]
            batch_items: Vec::new(),
        }
    }
}

impl<T, Mode> Default for ProtoEncoder<T, Mode> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
            max_encode_preallocation: DEFAULT_MAX_ENCODE_PREALLOCATION,
            #[cfg(feature = "tonic")]
            batch_items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProtoDecoder<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for ProtoDecoder<T> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}
