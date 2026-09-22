use proto_rs::DecodeContext;
use proto_rs::ProtoArchive;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::ProtoExt;
use proto_rs::RevVec;
use proto_rs::RevWriter;
use proto_rs::bytes::Buf;
use proto_rs::bytes::Bytes;
use proto_rs::proto_message;

struct NoTemporaryBytes<'a>(&'a [u8]);

impl Buf for NoTemporaryBytes<'_> {
    fn remaining(&self) -> usize {
        self.0.len()
    }
    fn chunk(&self) -> &[u8] {
        self.0
    }
    fn advance(&mut self, count: usize) {
        self.0.advance(count);
    }
    fn copy_to_bytes(&mut self, _len: usize) -> Bytes {
        panic!("owned byte containers must not allocate an intermediate Bytes");
    }
}

#[test]
fn owned_bytes_decode_without_a_temporary_bytes_buffer() {
    let wire = [10, 3, 0, 128, 255];
    assert_eq!(
        Vec::<u8>::decode(NoTemporaryBytes(&wire), DecodeContext::default()).unwrap(),
        [0, 128, 255]
    );
    let deque = std::collections::VecDeque::<u8>::decode(NoTemporaryBytes(&wire), DecodeContext::default()).unwrap();
    assert_eq!(deque, [0, 128, 255]);
    let fragmented = wire[..3].chain(&wire[3..]);
    assert_eq!(Vec::<u8>::decode(fragmented, DecodeContext::default()).unwrap(), [0, 128, 255]);
    assert!(Vec::<u8>::decode(&wire[..4], DecodeContext::default()).is_err());
}

// Exercise the provided method too: third-party writers need not implement the
// optimized operation to remain source- and wire-compatible.
struct DefaultByteWriter(RevVec);

impl RevWriter for DefaultByteWriter {
    type TightBuf = Vec<u8>;
    type Mark = usize;
    fn with_capacity(cap: usize) -> Self {
        Self(RevVec::with_capacity(cap))
    }
    fn empty() -> Self {
        Self(RevVec::empty())
    }
    fn mark(&self) -> usize {
        self.0.mark()
    }
    fn written_since(&self, mark: usize) -> usize {
        self.0.written_since(mark)
    }
    fn as_written_slice(&self) -> &[u8] {
        self.0.as_written_slice()
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    fn put_u8(&mut self, byte: u8) {
        self.0.put_u8(byte);
    }
    fn put_slice(&mut self, bytes: &[u8]) {
        self.0.put_slice(bytes);
    }
    fn put_varint(&mut self, value: u64) {
        self.0.put_varint(value);
    }
    fn finish_tight(self) -> Vec<u8> {
        self.0.finish_tight()
    }
}

fn check_byte_field<W: RevWriter<TightBuf = Vec<u8>>, const TAG: u32>(payload: &[u8], capacity: usize) {
    let mut expected = Vec::new();
    if TAG != 0 {
        proto_rs::encoding::encode_key(TAG, proto_rs::encoding::WireType::LengthDelimited, &mut expected);
        proto_rs::encoding::encode_varint(payload.len() as u64, &mut expected);
    }
    expected.extend_from_slice(payload);
    expected.extend_from_slice(&[7, 8, 9]);
    let mut writer = W::with_capacity(capacity);
    writer.put_slice(&[7, 8, 9]);
    writer.put_bytes::<TAG>(payload);
    assert_eq!(writer.as_written_slice(), expected);
    assert_eq!(writer.finish_tight(), expected);
}

#[test]
fn fused_byte_fields_handle_headers_growth_and_empty_payloads() {
    for len in [0, 1, 7, 127, 128, 129, 16383, 16384] {
        let payload = vec![0xa5; len];
        for capacity in [0, 1, len + 16] {
            check_byte_field::<RevVec, 0>(&payload, capacity);
            check_byte_field::<RevVec, 1>(&payload, capacity);
            check_byte_field::<RevVec, 15>(&payload, capacity);
            check_byte_field::<RevVec, 16>(&payload, capacity);
            check_byte_field::<RevVec, 536_870_911>(&payload, capacity);
            check_byte_field::<DefaultByteWriter, 0>(&payload, capacity);
            check_byte_field::<DefaultByteWriter, 1>(&payload, capacity);
            check_byte_field::<DefaultByteWriter, 15>(&payload, capacity);
            check_byte_field::<DefaultByteWriter, 16>(&payload, capacity);
            check_byte_field::<DefaultByteWriter, 536_870_911>(&payload, capacity);
        }
    }
}

#[proto_message]
#[derive(Clone, Copy)]
enum SmallEnum {
    Zero,
    One,
    Two,
}

#[proto_message]
#[derive(Clone, Copy)]
enum WideEnum {
    Negative = -1,
    Zero = 0,
    Large = 128,
}

#[test]
fn single_packed_elements_preserve_validation_and_boundaries() {
    let values = Vec::<SmallEnum>::decode(&[10, 1, 1, 8, 2][..], DecodeContext::default()).unwrap();
    assert!(matches!(values.as_slice(), [SmallEnum::One, SmallEnum::Two]));
    assert!(Vec::<SmallEnum>::decode(&[10, 1, 3][..], DecodeContext::default()).is_err());
    assert!(Vec::<SmallEnum>::decode(&[10, 1, 128][..], DecodeContext::default()).is_err());
    // A valid integer must still not consume bytes beyond the packed payload.
    assert!(Vec::<i32>::decode(&[10, 1, 128, 1][..], DecodeContext::default()).is_err());
}

fn check_exact_hint<T: ProtoArchive + ProtoEncode + ProtoExt>(value: &T) {
    let hint = value.encoded_size_hint::<1>();
    let encoded = value.encode_to_vec();
    assert!(hint.exact, "{}: {hint:?}", core::any::type_name::<T>());
    assert_eq!(hint.size, encoded.len());
    assert_eq!(encoded.capacity(), encoded.len());
}

#[test]
fn bounded_collection_hints_preserve_empty_and_default_elements() {
    const {
        assert!(SmallEnum::ENCODED_SIZE_HINT.exact);
        assert!(!WideEnum::ENCODED_SIZE_HINT.exact);
    }
    check_exact_hint(&vec![String::new(), "hello".into(), "x".repeat(128)]);
    check_exact_hint(&vec![Vec::<u8>::new(), vec![0, 255], vec![1; 128]]);
    check_exact_hint(&vec![0i32, -1, 127, 128]);
    check_exact_hint(&vec![SmallEnum::Zero, SmallEnum::One, SmallEnum::Two]);
    // Larger dynamic collections deliberately keep a constant-time estimate.
    assert!(!vec!["hello".to_string(); 17].encoded_size_hint::<1>().exact);
}
