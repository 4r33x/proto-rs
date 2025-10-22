use bytes::Buf;
use bytes::BufMut;
use bytes::Bytes;

pub trait BytesAdapterEncode {
    fn len(&self) -> usize;
    fn append_to(&self, buf: &mut impl BufMut);
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub trait BytesAdapterDecode: BytesAdapterEncode + Default {
    fn replace_with(&mut self, buf: impl Buf);
}

impl BytesAdapterEncode for Bytes {
    fn len(&self) -> usize {
        Buf::remaining(self)
    }

    fn append_to(&self, buf: &mut impl BufMut) {
        buf.put(self.clone());
    }
}

impl BytesAdapterDecode for Bytes {
    fn replace_with(&mut self, mut buf: impl Buf) {
        *self = buf.copy_to_bytes(buf.remaining());
    }
}

impl BytesAdapterEncode for Vec<u8> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn append_to(&self, buf: &mut impl BufMut) {
        buf.put(self.as_slice());
    }
}

impl BytesAdapterDecode for Vec<u8> {
    fn replace_with(&mut self, buf: impl Buf) {
        self.clear();
        self.reserve(buf.remaining());
        self.put(buf);
    }
}

impl BytesAdapterEncode for &Vec<u8> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn append_to(&self, buf: &mut impl BufMut) {
        buf.put(self.as_slice());
    }
}

impl BytesAdapterEncode for &Bytes {
    fn len(&self) -> usize {
        Buf::remaining(*self)
    }

    fn append_to(&self, buf: &mut impl BufMut) {
        buf.put((*self).clone());
    }
}

impl BytesAdapterEncode for &[u8] {
    fn len(&self) -> usize {
        (*self).len()
    }

    fn append_to(&self, buf: &mut impl BufMut) {
        buf.put(*self);
    }
}

impl<const N: usize> BytesAdapterEncode for &[u8; N] {
    fn len(&self) -> usize {
        N
    }

    fn append_to(&self, buf: &mut impl BufMut) {
        buf.put(&self[..]);
    }
}

impl<const N: usize> BytesAdapterEncode for [u8; N] {
    fn len(&self) -> usize {
        N
    }

    fn append_to(&self, buf: &mut impl BufMut) {
        buf.put(&self[..]);
    }
}
