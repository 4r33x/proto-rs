/// Low-level storage backend for reverse writing.
pub trait ProtoBufferMut: Sized {
    fn empty() -> Self;
    fn with_capacity(cap: usize) -> Self;
    fn capacity(&self) -> usize;

    /// # Safety
    /// Must set the length to exactly `len`. Caller guarantees bytes will be initialized
    /// before they are read. Reverse writer uses this to treat the full capacity as writable.
    unsafe fn set_len(&mut self, len: usize);

    fn as_ptr(&self) -> *const u8;
    fn as_mut_ptr(&mut self) -> *mut u8;

    fn replace_with(&mut self, other: Self);
}

impl ProtoBufferMut for Vec<u8> {
    #[inline(always)]
    fn empty() -> Self {
        vec![]
    }
    #[inline(always)]
    fn with_capacity(cap: usize) -> Self {
        Vec::with_capacity(cap)
    }

    #[inline(always)]
    fn capacity(&self) -> usize {
        Vec::capacity(self)
    }

    #[inline(always)]
    unsafe fn set_len(&mut self, len: usize) {
        unsafe { Vec::set_len(self, len) }
    }

    #[inline(always)]
    fn as_ptr(&self) -> *const u8 {
        Vec::as_ptr(self)
    }

    #[inline(always)]
    fn as_mut_ptr(&mut self) -> *mut u8 {
        Vec::as_mut_ptr(self)
    }

    #[inline(always)]
    fn replace_with(&mut self, other: Self) {
        *self = other;
    }
}

pub trait ProtoAsSlice {
    fn as_slice(&self) -> &[u8];
}
impl ProtoAsSlice for Vec<u8> {
    #[inline(always)]
    fn as_slice(&self) -> &[u8] {
        Vec::as_slice(&self)
    }
}

/// Reverse writer trait.
///
/// Requirements:
/// - Must support writing bytes "backwards" (prepend semantics).
/// - Must support growth without invalidating already written bytes (upb strategy: grow + shift tail).
/// - `finish(self)` returns the underlying buffer with NO COPY.
pub trait RevWriter {
    type Buf: ProtoAsSlice;

    /// A "mark" used to compute written lengths (for length-delimited payload length).
    type Mark: Copy;

    fn with_capacity(cap: usize) -> Self;

    fn empty() -> Self;

    fn mark(&self) -> Self::Mark;
    fn written_since(&self, mark: Self::Mark) -> usize;
    fn put_u8(&mut self, b: u8);
    fn put_slice(&mut self, s: &[u8]);

    /// Varint bytes MUST be emitted in normal forward varint byte order.
    fn put_varint(&mut self, v: u64);

    fn put_fixed32(&mut self, v: u32) {
        self.put_slice(&v.to_le_bytes());
    }
    fn put_fixed64(&mut self, v: u64) {
        self.put_slice(&v.to_le_bytes());
    }

    /// Return the underlying buffer with NO COPY.
    fn finish(self) -> Self::Buf;
}

// Concrete reverse writer backed by a single contiguous buffer (Vec<u8> by default).
///
/// Invariant:
/// - We treat the entire capacity as initialized backing store via `set_len(cap)`.
/// - `pos` moves backward; valid bytes are in `buf[pos..cap)`.
///
/// IMPORTANT:
/// - `finish()` returns the *buffer as-is*, i.e. still containing prefix slack in [0..pos).
/// - If you want a tight Vec with len == encoded bytes and data at offset 0,
///   you can add a separate `finish_tight()` that memmoves once.
///   You explicitly asked for "no copy" in finish; memmove counts as a copy, so not done here.
///
/// Integration:
/// - If tonic requires a tight slice at offset 0, you can do the final "tightening"
///   exactly once right before copying to tonic (or accept offset+len metadata in your pipeline).
pub struct RevVec<B: ProtoBufferMut = Vec<u8>> {
    buf: B,
    pos: usize,
}

impl<B: ProtoBufferMut> RevVec<B> {
    const MIN_GROW: usize = 64;

    #[inline(always)]
    fn cap(&self) -> usize {
        self.buf.capacity()
    }

    #[inline(always)]
    fn ensure_space(&mut self, need: usize) {
        if self.pos >= need {
            return;
        }

        let old_cap = self.cap();
        let used = old_cap - self.pos;

        // Exponential growth (upb-style). Ensure new_cap >= used + need.
        let mut new_cap = (old_cap * 2).next_power_of_two().max(Self::MIN_GROW);
        while new_cap < used + need {
            new_cap *= 2;
        }

        let mut new_buf = B::with_capacity(new_cap);
        unsafe { new_buf.set_len(new_cap) };

        // Copy existing payload [pos..old_cap) to the end of the new buffer.
        unsafe {
            core::ptr::copy_nonoverlapping(self.buf.as_ptr().add(self.pos), new_buf.as_mut_ptr().add(new_cap - used), used);
        }

        self.buf.replace_with(new_buf);
        self.pos = new_cap - used;
    }

    /// (Optional helper) Return where the valid bytes start (no copy).
    #[inline(always)]
    pub fn start(&self) -> usize {
        self.pos
    }

    /// (Optional helper) Return encoded length (no copy).
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.cap() - self.pos
    }

    /// (Optional helper) Borrow the encoded bytes slice (no copy), as `[start..cap)`.
    ///
    /// Only usable while you still have access to the writer.
    #[inline(always)]
    pub fn as_written_slice(&self) -> &[u8] {
        let cap = self.cap();
        // SAFETY: buf length is cap (we set_len(cap)); bytes in [pos..cap) are initialized by us.
        unsafe { core::slice::from_raw_parts(self.buf.as_ptr().add(self.pos), cap - self.pos) }
    }
}

impl<B: ProtoBufferMut + ProtoAsSlice> RevWriter for RevVec<B> {
    type Buf = B;
    type Mark = usize;

    #[inline(always)]
    fn mark(&self) -> Self::Mark {
        self.pos
    }

    #[inline(always)]
    fn written_since(&self, mark: Self::Mark) -> usize {
        mark - self.pos
    }

    #[inline(always)]
    fn put_u8(&mut self, b: u8) {
        self.ensure_space(1);
        self.pos -= 1;
        unsafe {
            *self.buf.as_mut_ptr().add(self.pos) = b;
        }
    }

    #[inline(always)]
    fn put_slice(&mut self, s: &[u8]) {
        let n = s.len();
        if n == 0 {
            return;
        }
        self.ensure_space(n);
        self.pos -= n;
        unsafe {
            core::ptr::copy_nonoverlapping(s.as_ptr(), self.buf.as_mut_ptr().add(self.pos), n);
        }
    }

    #[inline(always)]
    fn put_varint(&mut self, mut v: u64) {
        // Emit standard varint bytes; reverse writer places them at descending addresses.
        loop {
            let byte = (v as u8) & 0x7F;
            v >>= 7;
            if v == 0 {
                self.put_u8(byte);
                break;
            }
            self.put_u8(byte | 0x80);
        }
    }

    #[inline(always)]
    fn finish(self) -> Self::Buf {
        self.buf
    }
    #[inline(always)]
    fn with_capacity(cap: usize) -> Self {
        let mut buf = B::with_capacity(cap);
        unsafe { buf.set_len(cap) };
        Self { buf, pos: cap }
    }

    fn empty() -> Self {
        Self { buf: B::empty(), pos: 0 }
    }
}
