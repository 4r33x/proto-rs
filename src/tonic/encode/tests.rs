use core::cell::Cell;
use core::mem::MaybeUninit;

use bytes::BytesMut;

use super::*;
use crate::EncodeSizeHint;

struct Raw {
    bytes: Vec<u8>,
    hint: EncodeSizeHint,
    calls: Cell<usize>,
    panic: bool,
}

struct Shadow<'a>(&'a Raw);

impl ProtoExt for Raw {
    const KIND: ProtoKind = ProtoKind::Message;
}
impl ProtoExt for Shadow<'_> {
    const KIND: ProtoKind = ProtoKind::Message;
}
impl ProtoEncode for Raw {
    type Shadow<'a> = Shadow<'a>;
}
impl<'a> ProtoShadowEncode<'a, Raw> for Shadow<'a> {
    fn from_sun(value: &'a Raw) -> Self {
        Self(value)
    }
}
impl ProtoArchive for Shadow<'_> {
    fn is_default(&self) -> bool {
        self.0.bytes.is_empty()
    }
    fn encoded_size_hint<const TAG: u32>(&self) -> EncodeSizeHint {
        self.0.hint
    }
    fn archive<const TAG: u32>(&self, writer: &mut impl RevWriter) {
        self.0.calls.set(self.0.calls.get() + 1);
        for chunk in self.0.bytes.rchunks(13) {
            writer.put_slice(chunk);
        }
        assert!(!self.0.panic, "injected archive panic");
    }
}

#[test]
fn cloning_encoder_does_not_copy_retained_scratch() {
    let mut encoder = ProtoEncoder::<(), crate::SunByVal>::default();
    encoder.scratch = RevVec::with_capacity(8192);
    encoder.last_size = 123;
    let cloned = encoder.clone();
    assert_eq!(cloned.last_size, 123);
    assert_eq!(
        cloned.scratch.as_written_slice().as_ptr(),
        RevVec::empty().as_written_slice().as_ptr()
    );
}

#[test]
fn direct_output_matches_owned_encoding_and_preserves_prefix() {
    let mut output = BytesMut::with_capacity(8192);
    output.extend_from_slice(b"previous frame");
    let mut expected = output.to_vec();
    let mut scratch = RevVec::empty();
    let mut last_size = 0;
    for value in [0, 1, 127, 128, u64::MAX] {
        encode_into(&value, &mut output, &mut scratch, &mut last_size);
        expected.extend(value.encode_to_vec());
    }
    for value in [vec![], vec![42u8], vec![23; 130], vec![99; 4096]] {
        encode_into(&value, &mut output, &mut scratch, &mut last_size);
        expected.extend(value.encode_to_vec());
    }
    for value in [String::new(), "protobuf".into(), "x".repeat(130)] {
        encode_into(&value, &mut output, &mut scratch, &mut last_size);
        expected.extend(value.encode_to_vec());
    }
    assert_eq!(output.as_ref(), expected);
    assert!(scratch.is_empty());
    assert_eq!(scratch.as_written_slice().as_ptr(), RevVec::empty().as_written_slice().as_ptr());
}

#[test]
fn inaccurate_hints_spill_once_and_reuse_scratch_even_after_empty_messages() {
    let mut scratch = RevVec::with_capacity(8192);
    let scratch_end = scratch.as_written_slice().as_ptr();
    let mut last_size = 0;
    for size in [4000, 0, 1, 4096] {
        for hint in [
            EncodeSizeHint::EMPTY,
            EncodeSizeHint::new(3, true),
            EncodeSizeHint::UNKNOWN,
            EncodeSizeHint::new(9000, true),
        ] {
            let value = Raw {
                bytes: vec![17; size],
                hint,
                calls: Cell::new(0),
                panic: false,
            };
            let mut output = BytesMut::from(&b"frame"[..]);
            encode_into(&value, &mut output, &mut scratch, &mut last_size);
            assert_eq!(&output[..5], b"frame");
            assert_eq!(&output[5..], value.bytes);
            assert_eq!(value.calls.get(), 1, "archive must not be retried on spill");
            assert!(scratch.is_empty());
            assert_eq!(scratch.as_written_slice().as_ptr(), scratch_end);
        }
    }
}

#[test]
fn large_spill_capacity_is_not_retained() {
    let value = Raw {
        bytes: vec![7; MAX_RETAINED_CAPACITY + 1],
        hint: EncodeSizeHint::EMPTY,
        calls: Cell::new(0),
        panic: false,
    };
    let mut output = BytesMut::new();
    let mut scratch = RevVec::empty();
    let mut last_size = 0;
    encode_into(&value, &mut output, &mut scratch, &mut last_size);
    assert_eq!(output.as_ref(), value.bytes);
    assert_eq!(last_size, MAX_RETAINED_CAPACITY);
    assert_eq!(scratch.as_written_slice().as_ptr(), RevVec::empty().as_written_slice().as_ptr());
}

#[test]
fn panic_does_not_commit_uninitialized_output_and_encoder_can_be_reused() {
    for size in [8, 4000] {
        let mut value = Raw {
            bytes: vec![9; size],
            hint: EncodeSizeHint::UNKNOWN,
            calls: Cell::new(0),
            panic: true,
        };
        let mut output = BytesMut::from(&b"prefix"[..]);
        let mut scratch = RevVec::empty();
        let mut last_size = 0;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            encode_into(&value, &mut output, &mut scratch, &mut last_size);
        }));
        assert!(result.is_err());
        assert_eq!(output.as_ref(), b"prefix");
        value.panic = false;
        encode_into(&value, &mut output, &mut scratch, &mut last_size);
        assert_eq!(&output[6..], value.bytes);
    }
}

#[test]
fn writer_operations_match_revvec_at_every_spill_boundary() {
    for capacity in 0..96 {
        let mut memory = [MaybeUninit::uninit(); 96];
        let mut writer = OutputWriter {
            buf: UninitSlice::uninit(&mut memory[..capacity]),
            pos: capacity,
            scratch: RevVec::empty(),
            spilled: false,
        };
        let mut reference = RevVec::empty();
        let mark = writer.mark();
        for value in [
            0,
            127,
            128,
            16383,
            16384,
            1 << 21,
            1 << 28,
            1 << 35,
            1 << 42,
            1 << 49,
            1 << 56,
            1 << 63,
            u64::MAX,
        ] {
            writer.put_varint(value);
            reference.put_varint(value);
            writer.put_bytes::<1>(b"hello");
            reference.put_bytes::<1>(b"hello");
            writer.put_bytes::<16>(b"world");
            reference.put_bytes::<16>(b"world");
            writer.put_slice(&[]);
            assert_eq!(writer.as_written_slice(), reference.as_written_slice());
            assert_eq!(writer.written_since(mark), reference.len());
        }
        let (len, spilled, scratch) = writer.finish();
        if spilled {
            assert_eq!(scratch.as_written_slice(), reference.as_written_slice());
        } else {
            // SAFETY: finish initializes the first len bytes.
            let bytes = unsafe { core::slice::from_raw_parts(memory.as_ptr().cast::<u8>(), len) };
            assert_eq!(bytes, reference.as_written_slice());
        }
    }
}
