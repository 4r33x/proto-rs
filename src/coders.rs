#![allow(dead_code)]

use core::marker::PhantomData;

use crate::alloc::vec::Vec;

pub trait AsBytes {
    fn as_bytes(&self) -> &[u8];
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

// impl AsBytes for ZeroCopyBufferInner {
//     #[inline]
//     fn as_bytes(&self) -> &[u8] {
//         self.as_slice()
//     }
// }

#[derive(Clone, Copy, Default)]
pub struct BytesMode;
#[derive(Clone, Copy, Default)]
pub struct SunByVal; // Sun<'a> = T
#[derive(Clone, Copy, Default)]
pub struct SunByRef; // Sun<'a> = &'a T
#[derive(Clone, Copy, Default)]
pub struct SunByRefDeref; // Sun<'a> = &'a T::Target

#[derive(Debug, Clone)]
pub struct ProtoCodec<Encode = (), Decode = (), Mode = SunByRef> {
    _marker: PhantomData<(Encode, Decode, Mode)>,
}

impl<Encode, Decode, Mode> Default for ProtoCodec<Encode, Decode, Mode> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<Encode, Decode, Mode> ProtoCodec<Encode, Decode, Mode> {
    pub const fn new() -> Self {
        Self { _marker: PhantomData }
    }
}

#[derive(Debug)]
pub struct ProtoEncoder<T, Mode> {
    _marker: core::marker::PhantomData<(T, Mode)>,
    #[cfg(feature = "tonic")]
    pub(crate) scratch: crate::RevVec,
    #[cfg(feature = "tonic")]
    pub(crate) last_size: usize,
}

impl<T, Mode> Clone for ProtoEncoder<T, Mode> {
    fn clone(&self) -> Self {
        // Scratch contains no message state between calls. Do not allocate and
        // copy an empty retained buffer just to clone an encoder.
        Self {
            _marker: PhantomData,
            #[cfg(feature = "tonic")]
            scratch: <crate::RevVec as crate::RevWriter>::empty(),
            #[cfg(feature = "tonic")]
            last_size: self.last_size,
        }
    }
}

impl<T, Mode> Default for ProtoEncoder<T, Mode> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
            #[cfg(feature = "tonic")]
            scratch: <crate::RevVec as crate::RevWriter>::empty(),
            #[cfg(feature = "tonic")]
            last_size: 0,
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
