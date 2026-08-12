use core::mem::ManuallyDrop;
use core::mem::MaybeUninit;

/// Reverse writer trait (keeps your existing API shape).
pub trait RevWriter {
    type TightBuf;
    type Mark: Copy;

    fn with_capacity(cap: usize) -> Self;
    fn empty() -> Self;

    fn mark(&self) -> Self::Mark;
    fn written_since(&self, mark: Self::Mark) -> usize;
    fn as_written_slice(&self) -> &[u8];
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;

    fn put_u8(&mut self, b: u8);
    fn put_slice(&mut self, s: &[u8]);
    fn put_varint(&mut self, v: u64);

    #[inline]
    fn put_fixed32(&mut self, v: u32) {
        self.put_slice(&v.to_le_bytes());
    }

    #[inline]
    fn put_fixed64(&mut self, v: u64) {
        self.put_slice(&v.to_le_bytes());
    }

    fn finish_tight(self) -> Self::TightBuf;
}

pub struct RevVec {
    buf: Vec<MaybeUninit<u8>>,
    pos: usize, // valid bytes are in [pos..cap)
}

impl RevVec {
    const MIN_GROW: usize = 64;

    #[inline]
    const fn cap(&self) -> usize {
        self.buf.capacity()
    }

    #[inline]
    fn ensure_space(&mut self, need: usize) {
        if self.pos >= need {
            return;
        }

        let old_cap = self.cap();
        let used = old_cap - self.pos;

        let mut new_cap = (old_cap * 2).next_power_of_two().max(Self::MIN_GROW);
        while new_cap < used + need {
            new_cap *= 2;
        }

        let mut new_buf: Vec<MaybeUninit<u8>> = Vec::with_capacity(new_cap);
        let new_cap = new_buf.capacity();
        unsafe { new_buf.set_len(new_cap) };

        unsafe {
            core::ptr::copy_nonoverlapping(self.buf.as_ptr().add(self.pos), new_buf.as_mut_ptr().add(new_cap - used), used);
        }

        self.buf = new_buf;
        self.pos = new_cap - used;
    }
}

impl RevWriter for RevVec {
    type TightBuf = Vec<u8>;
    type Mark = usize;

    #[inline]
    fn with_capacity(cap: usize) -> Self {
        let mut buf = Vec::<MaybeUninit<u8>>::with_capacity(cap);
        let cap = buf.capacity();
        unsafe { buf.set_len(cap) }; // invariant: len == cap
        Self { buf, pos: cap }
    }

    #[inline]
    fn empty() -> Self {
        Self { buf: Vec::new(), pos: 0 }
    }

    #[inline]
    fn mark(&self) -> Self::Mark {
        self.cap() - self.pos
    }

    #[inline]
    fn len(&self) -> usize {
        self.cap() - self.pos
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    fn written_since(&self, mark: Self::Mark) -> usize {
        (self.cap() - self.pos) - mark
    }

    #[inline]
    fn put_u8(&mut self, b: u8) {
        self.ensure_space(1);
        self.pos -= 1;
        unsafe {
            self.buf.as_mut_ptr().add(self.pos).write(MaybeUninit::new(b));
        }
    }

    #[inline]
    fn put_slice(&mut self, s: &[u8]) {
        let n = s.len();
        if n == 0 {
            return;
        }
        self.ensure_space(n);
        self.pos -= n;
        unsafe {
            core::ptr::copy_nonoverlapping(s.as_ptr(), self.buf.as_mut_ptr().add(self.pos).cast::<u8>(), n);
        }
    }

    #[inline]
    fn put_varint(&mut self, mut v: u64) {
        let mut tmp = [0u8; 10];
        let mut i = 0usize;
        loop {
            let byte = (v as u8) & 0x7f;
            v >>= 7;
            if v == 0 {
                tmp[i] = byte;
                i += 1;
                break;
            }
            tmp[i] = byte | 0x80;
            i += 1;
        }
        self.put_slice(&tmp[..i]);
    }

    /// Optional helper for viewing while still writing (no copy).
    #[inline]
    fn as_written_slice(&self) -> &[u8] {
        let cap = self.cap();
        unsafe { core::slice::from_raw_parts(self.buf.as_ptr().add(self.pos).cast::<u8>(), cap - self.pos) }
    }

    #[inline]
    fn finish_tight(mut self) -> Self::TightBuf {
        let cap = self.cap();
        let pos = self.pos;
        let len = cap - pos;

        if len == 0 {
            return Vec::new();
        }
        if pos != 0 {
            unsafe {
                core::ptr::copy(self.buf.as_ptr().add(pos), self.buf.as_mut_ptr(), len);
            }
        }
        self.buf.truncate(len);

        let mut buf = ManuallyDrop::new(self.buf);
        unsafe { Vec::from_raw_parts(buf.as_mut_ptr().cast::<u8>(), len, buf.capacity()) }
    }
}

#[cfg(test)]
mod tests {
    use super::RevVec;
    use super::RevWriter;

    #[test]
    fn reverse_writer_grows_and_returns_only_initialized_bytes() {
        let mut writer = RevVec::with_capacity(1);
        writer.put_slice(&[3, 4, 5]);
        writer.put_u8(2);
        writer.put_u8(1);

        assert_eq!(writer.as_written_slice(), &[1, 2, 3, 4, 5]);
        assert_eq!(writer.finish_tight(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn empty_reverse_writer_finishes_as_empty_vec() {
        assert_eq!(RevVec::with_capacity(64).finish_tight(), Vec::<u8>::new());
        assert_eq!(RevVec::empty().finish_tight(), Vec::<u8>::new());
    }
}
