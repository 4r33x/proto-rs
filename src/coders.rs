#![allow(dead_code)]

use core::marker::PhantomData;

use crate::alloc::vec::Vec;
use crate::zero_copy::ZeroCopyBuffer;

pub trait AsBytes {
    fn as_bytes(&self) -> &[u8];
}

impl AsBytes for Vec<u8> {
    #[inline(always)]
    fn as_bytes(&self) -> &[u8] {
        self
    }
}
impl AsBytes for ZeroCopyBuffer {
    #[inline(always)]
    fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}
impl<const N: usize> AsBytes for [u8; N] {
    #[inline(always)]
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

// impl AsBytes for ZeroCopyBufferInner {
//     #[inline(always)]
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
    pub fn new() -> Self {
        Self { _marker: PhantomData }
    }
}

#[derive(Debug, Clone)]
pub struct ProtoEncoder<T, Mode> {
    _marker: core::marker::PhantomData<(T, Mode)>,
}

impl<T, Mode> Default for ProtoEncoder<T, Mode> {
    fn default() -> Self {
        Self { _marker: PhantomData }
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
