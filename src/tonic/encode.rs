//! Reverse encoding into Tonic's spare output capacity, without private layout access.
//!
//! The destination is not advanced until its prefix is fully initialized. If the
//! spare slice is too small, encoding continues in reusable scratch storage; it
//! never repeats the archive operation (which may read locks or mutable state).

use bytes::BufMut;
use bytes::buf::UninitSlice;
use tonic::codec::EncodeBuf;

use crate::ProtoArchive;
use crate::ProtoEncode;
use crate::ProtoExt;
use crate::ProtoShadowEncode;
use crate::RevVec;
use crate::RevWriter;
use crate::coders::ProtoEncoder;
use crate::traits::ProtoKind;

const MAX_RETAINED_CAPACITY: usize = 64 * 1024;
// This is output capacity, not retained scratch. A larger but still bounded
// reservation avoids an intermediate 64 KiB allocation for large unary bodies.
const MAX_OUTPUT_PREALLOCATION: usize = 1024 * 1024;

impl<T, Mode> ProtoEncoder<T, Mode> {
    #[inline(always)]
    pub(super) fn encode_direct<P: ProtoEncode + ProtoExt>(&mut self, item: &P, dst: &mut EncodeBuf<'_>) {
        encode_into(item, dst, &mut self.scratch, &mut self.last_size);
    }
}

// Kept generic over the buffer only to exercise the same code with BytesMut in
// tests. EncodeBuf's public reserve/chunk_mut API suffices; no vendoring needed.
trait OutputBuffer: BufMut {
    fn reserve_output(&mut self, additional: usize);
}

impl OutputBuffer for EncodeBuf<'_> {
    fn reserve_output(&mut self, additional: usize) {
        self.reserve(additional);
    }
}

#[cfg(test)]
impl OutputBuffer for bytes::BytesMut {
    fn reserve_output(&mut self, additional: usize) {
        self.reserve(additional);
    }
}

#[inline(always)]
fn encode_into<P: ProtoEncode + ProtoExt>(item: &P, dst: &mut impl OutputBuffer, scratch: &mut RevVec, last_size: &mut usize) {
    let shadow = P::Shadow::from_sun(item);
    if !matches!(P::KIND, ProtoKind::Message) && shadow.is_default() {
        return;
    }
    let hint = if P::WRAP_ROOT {
        shadow.encoded_size_hint::<1>()
    } else {
        shadow.encoded_size_hint::<0>()
    };
    dst.reserve_output(hint.size.clamp(64, MAX_OUTPUT_PREALLOCATION).max(*last_size));
    let chunk = dst.chunk_mut();
    // An exact hint can avoid compaction, but is only a hint: the writer still
    // checks every write and spills safely if a custom implementation lies.
    let capacity = if hint.exact && hint.size <= chunk.len() {
        hint.size
    } else {
        chunk.len()
    };
    let mut writer = OutputWriter {
        buf: &mut chunk[..capacity],
        pos: capacity,
        scratch: core::mem::replace(scratch, RevVec::empty()),
        spilled: false,
    };
    if P::WRAP_ROOT {
        shadow.archive::<1>(&mut writer);
    } else {
        shadow.archive::<0>(&mut writer);
    }
    let (len, spilled, retained) = writer.finish();
    *scratch = retained;
    if spilled {
        dst.put_slice(scratch.as_written_slice());
    } else {
        // SAFETY: finish initialized exactly the first `len` bytes of the
        // chunk returned by dst. No destination operation occurred meanwhile.
        unsafe { dst.advance_mut(len) };
    }
    *last_size = len.min(MAX_RETAINED_CAPACITY);
    scratch.reset_for_reuse(MAX_RETAINED_CAPACITY);
}

struct OutputWriter<'a> {
    buf: &'a mut UninitSlice,
    pos: usize,
    scratch: RevVec,
    spilled: bool,
}

impl OutputWriter<'_> {
    #[cold]
    fn spill(&mut self) {
        // SAFETY: only [pos..len) has been initialized by this writer. The
        // exclusive buffer borrow remains live while scratch copies the bytes.
        let bytes = unsafe { core::slice::from_raw_parts(self.buf.as_mut_ptr().add(self.pos), self.buf.len() - self.pos) };
        self.scratch.put_slice(bytes);
        self.spilled = true;
        self.pos = 0;
    }

    #[cold]
    fn put_slice_slow(&mut self, bytes: &[u8]) {
        if !self.spilled {
            self.spill();
        }
        self.scratch.put_slice(bytes);
    }

    #[cold]
    fn put_u8_slow(&mut self, byte: u8) {
        if !self.spilled {
            self.spill();
        }
        self.scratch.put_u8(byte);
    }

    #[cold]
    fn put_varint_slow(&mut self, value: u64) {
        if !self.spilled {
            self.spill();
        }
        self.scratch.put_varint(value);
    }

    #[inline]
    fn finish(self) -> (usize, bool, RevVec) {
        let len = self.len();
        if !self.spilled && self.pos != 0 && len != 0 {
            // SAFETY: source is the initialized suffix; both ranges lie within
            // the exclusively borrowed chunk. copy supports overlapping ranges.
            let ptr = self.buf.as_mut_ptr();
            unsafe { core::ptr::copy(ptr.add(self.pos), ptr, len) };
        }
        (len, self.spilled, self.scratch)
    }
}

#[cfg(test)]
mod tests;

impl RevWriter for OutputWriter<'_> {
    type TightBuf = Vec<u8>;
    type Mark = usize;

    fn with_capacity(cap: usize) -> Self {
        Self {
            buf: UninitSlice::new(&mut []),
            pos: 0,
            scratch: RevVec::with_capacity(cap),
            spilled: true,
        }
    }

    fn empty() -> Self {
        Self::with_capacity(0)
    }

    #[inline]
    fn mark(&self) -> usize {
        self.len()
    }

    #[inline]
    fn written_since(&self, mark: usize) -> usize {
        self.len() - mark
    }

    #[inline]
    fn as_written_slice(&self) -> &[u8] {
        if self.spilled {
            self.scratch.as_written_slice()
        } else {
            // SAFETY: writes initialize the entire suffix, and `pos` is always
            // in bounds. UninitSlice has the same layout as a byte slice.
            unsafe { core::slice::from_raw_parts(core::ptr::from_ref(self.buf).cast::<u8>().add(self.pos), self.buf.len() - self.pos) }
        }
    }

    #[inline]
    fn len(&self) -> usize {
        if self.spilled {
            self.scratch.len()
        } else {
            self.buf.len() - self.pos
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    fn put_u8(&mut self, byte: u8) {
        if self.pos != 0 {
            self.pos -= 1;
            // SAFETY: pos started at buf.len() and only decreases. A spilled
            // writer always has pos=0, so this writes only into the direct chunk.
            unsafe { self.buf.as_mut_ptr().add(self.pos).write(byte) };
        } else {
            self.put_u8_slow(byte);
        }
    }

    #[inline]
    fn put_slice(&mut self, bytes: &[u8]) {
        if self.pos >= bytes.len() {
            self.pos -= bytes.len();
            // SAFETY: the checked subtraction reserves bytes.len() bytes in
            // the exclusive chunk. Safe inputs cannot alias that mutable borrow.
            unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), self.buf.as_mut_ptr().add(self.pos), bytes.len()) };
        } else {
            self.put_slice_slow(bytes);
        }
    }

    #[inline(always)]
    fn put_varint(&mut self, mut value: u64) {
        if value < 128 {
            self.put_u8(value as u8);
            return;
        }
        let len = crate::encoding::encoded_len_varint(value);
        if self.pos >= len {
            self.pos -= len;
            // SAFETY: the checked subtraction reserves exactly the varint's
            // length, which is 2..=10 here. Every reserved byte is initialized.
            unsafe {
                let mut ptr = self.buf.as_mut_ptr().add(self.pos);
                while value >= 128 {
                    ptr.write(value as u8 | 0x80);
                    ptr = ptr.add(1);
                    value >>= 7;
                }
                ptr.write(value as u8);
            }
        } else {
            self.put_varint_slow(value);
        }
    }

    #[inline]
    fn put_bytes<const TAG: u32>(&mut self, bytes: &[u8]) {
        if TAG != 0 && TAG < 16 && bytes.len() < 128 && self.pos >= bytes.len() + 2 {
            self.pos -= bytes.len() + 2;
            // SAFETY: the checked subtraction reserves the complete field, and
            // a spilled writer has pos=0. All writes are within the direct chunk.
            unsafe {
                let ptr = self.buf.as_mut_ptr().add(self.pos);
                ptr.write(((TAG << 3) | 2) as u8);
                ptr.add(1).write(bytes.len() as u8);
                core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(2), bytes.len());
            }
        } else {
            self.put_slice(bytes);
            if TAG != 0 {
                self.put_varint(bytes.len() as u64);
                self.put_varint(((TAG << 3) | 2) as u64);
            }
        }
    }

    fn finish_tight(self) -> Vec<u8> {
        if self.spilled {
            self.scratch.finish_tight()
        } else {
            self.as_written_slice().to_vec()
        }
    }
}
