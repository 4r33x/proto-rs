use super::EncodeSizeHint;
use super::PhantomData;
use super::ProtoArchive;
use super::ProtoEncode;
use super::ProtoExt;
use super::ProtoKind;
use super::ProtoShadowEncode;
use super::RevVec;
use super::RevWriter;
use super::pool::Buffer as SnapshotBuffer;
use crate::coders::AsBytes;

const HEADER: usize = 5;

/// Encode once, then clone to fan out the same immutable payload to many peers.
/// Clones share the allocation through Bytes' atomic reference count, without
/// copying or re-encoding. Public bytes exclude transport framing.
/// Allocations automatically reuse the encoding thread's pool.
pub struct EncodedSnapshot<T: ProtoEncode> {
    frame: bytes::Bytes,
    _pd: PhantomData<fn() -> T>,
}

impl<T: ProtoEncode> Clone for EncodedSnapshot<T> {
    fn clone(&self) -> Self {
        Self {
            frame: self.frame.clone(),
            _pd: PhantomData,
        }
    }
}

impl SnapshotBuffer {
    fn into_framed_bytes(mut self) -> bytes::Bytes {
        // gRPC's size limit is enforced on handoff; protobuf-only snapshots may
        // still expose larger payloads.
        let len = u32::try_from(self.buffer.len()).unwrap_or(0);
        self.buffer.put_slice(&len.to_be_bytes());
        self.buffer.put_u8(0);
        bytes::Bytes::from_owner(self)
    }
}

impl AsRef<[u8]> for SnapshotBuffer {
    fn as_ref(&self) -> &[u8] {
        self.buffer.as_written_slice()
    }
}

impl<T: ProtoEncode + ProtoExt> EncodedSnapshot<T> {
    pub fn new(value: &T) -> Self {
        Self::with_max_preallocation(value, crate::DEFAULT_MAX_ENCODE_PREALLOCATION)
    }

    /// Encode once using bounded transport hints. The cap excludes five bytes
    /// of framing headroom and is not a message-size or total memory limit.
    pub fn with_max_preallocation(value: &T, limit: usize) -> Self {
        Self::build(value, limit)
    }

    fn build(value: &T, limit: usize) -> Self {
        Self {
            frame: encode_buffer(value, limit).into_framed_bytes(),
            _pd: PhantomData,
        }
    }
}

impl<T: ProtoEncode> EncodedSnapshot<T> {
    #[cfg(feature = "tonic")]
    pub(crate) fn into_owned_frame(self) -> Result<crate::PreparedMessage, tonic::Status> {
        owned_frame(self.frame)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.frame[HEADER..]
    }

    /// Share the protobuf-only slice without copying. Its owner keeps the full
    /// framed allocation alive until the final slice/clone is dropped.
    pub fn into_bytes(mut self) -> bytes::Bytes {
        // Moving the view avoids a clone/drop pair on the atomic owner.
        bytes::Buf::advance(&mut self.frame, HEADER);
        self.frame
    }
}

fn root_hint<T: ProtoEncode + ProtoExt>(shadow: &T::Shadow<'_>) -> EncodeSizeHint {
    if !matches!(T::KIND, ProtoKind::Message) && shadow.is_default() {
        EncodeSizeHint::EMPTY
    } else if T::WRAP_ROOT {
        shadow.output_size_hint::<1>()
    } else {
        shadow.output_size_hint::<0>()
    }
}

fn archive_root<T: ProtoEncode + ProtoExt>(shadow: &T::Shadow<'_>, writer: &mut SnapshotWriter) {
    if matches!(T::KIND, ProtoKind::Message) || !shadow.is_default() {
        if T::WRAP_ROOT {
            shadow.archive::<1>(writer);
        } else {
            shadow.archive::<0>(writer);
        }
    }
}

fn encode_shadow<T: ProtoEncode + ProtoExt>(shadow: &T::Shadow<'_>, hint: EncodeSizeHint, limit: usize) -> SnapshotBuffer {
    let capacity = hint.size.clamp(64, limit.max(64));
    let needed = capacity.checked_add(HEADER).expect("snapshot capacity overflow");
    let mut writer = SnapshotWriter(SnapshotBuffer::checkout(needed));
    archive_root::<T>(shadow, &mut writer);
    writer.0
}

fn encode_buffer<T: ProtoEncode + ProtoExt>(value: &T, limit: usize) -> SnapshotBuffer {
    let shadow = T::Shadow::from_sun(value);
    encode_shadow::<T>(&shadow, root_hint::<T>(&shadow), limit)
}

#[cfg(feature = "tonic")]
pub(crate) fn prepare_owned<T: ProtoEncode + ProtoExt>(value: &T, limit: usize) -> Result<crate::PreparedMessage, tonic::Status> {
    let owner = encode_buffer(value, limit);
    owned_frame(owner.into_framed_bytes())
}

/// Copy synchronously using the same TLS slot pool as owned output. The lease
/// returns before this function exits, including when encoding unwinds.
#[cfg(feature = "tonic")]
pub(crate) fn encode_into<T: ProtoEncode + ProtoExt>(value: &T, limit: usize, dst: &mut impl bytes::BufMut) -> Result<(), tonic::Status> {
    let owner = encode_buffer(value, limit);
    if owner.buffer.len() > u32::MAX as usize {
        return Err(tonic::Status::resource_exhausted("message exceeds gRPC's 4 GiB frame limit"));
    }
    dst.put_slice(owner.as_ref());
    Ok(())
}

#[cfg(feature = "tonic")]
fn owned_frame(frame: bytes::Bytes) -> Result<crate::PreparedMessage, tonic::Status> {
    if frame.len() - HEADER > u32::MAX as usize {
        return Err(tonic::Status::resource_exhausted("snapshot exceeds gRPC's 4 GiB frame limit"));
    }
    crate::PreparedMessage::from_uncompressed_frame(frame)
}

/// Stage only ready input handles, then prepend messages in reverse source
/// order. Each archive runs once; no per-message output or coalescing copy.
/// Hints bound speculative reservation, not actual output: dishonest hints
/// still grow safely. The count limit also bounds work for empty messages.
#[cfg(feature = "tonic-owned")]
pub(crate) fn encode_batch<I, T: ProtoEncode + ProtoExt>(
    first: &I,
    next: &mut dyn FnMut() -> Option<I>,
    items: &mut Vec<I>,
    value: impl Fn(&I) -> &T,
    threshold: usize,
    limit: usize,
) -> Result<bytes::Bytes, tonic::Status> {
    const MAX_MESSAGES: usize = 128;
    fn hint<T: ProtoEncode + ProtoExt>(value: &T) -> usize {
        let hint = if T::WRAP_ROOT {
            value.size_hint::<1>()
        } else {
            value.size_hint::<0>()
        };
        hint.size.saturating_add(HEADER)
    }
    fn prepend<T: ProtoEncode + ProtoExt>(value: &T, writer: &mut SnapshotWriter) -> Result<(), tonic::Status> {
        let shadow = T::Shadow::from_sun(value);
        let mark = writer.mark();
        archive_root::<T>(&shadow, writer);
        let len = u32::try_from(writer.written_since(mark))
            .map_err(|_| tonic::Status::resource_exhausted("message exceeds gRPC's 4 GiB frame limit"))?;
        writer.0.buffer.put_slice(&len.to_be_bytes());
        writer.0.buffer.put_u8(0);
        Ok(())
    }
    // Also clears retained inputs if a caller catches an earlier encoding panic.
    items.clear();
    let mut estimated = hint(value(first));
    while estimated < threshold && items.len() < MAX_MESSAGES - 1 {
        let Some(item) = next() else { break };
        estimated = estimated.saturating_add(hint(value(&item)));
        items.push(item);
    }
    if items.is_empty() {
        // A unary body can refine its output hint without constructing another
        // shadow. This also honors the reservation cap for large flat batches
        // whose cheap staging hint deliberately does not inspect every element.
        let owner = encode_buffer(value(first), limit);
        if owner.buffer.len() > u32::MAX as usize {
            return Err(tonic::Status::resource_exhausted("message exceeds gRPC's 4 GiB frame limit"));
        }
        return Ok(owner.into_framed_bytes());
    }
    let capacity = estimated.clamp(64, limit.max(64));
    let owner = SnapshotBuffer::checkout(capacity);
    let mut writer = SnapshotWriter(owner);
    for item in items.drain(..).rev() {
        prepend(value(&item), &mut writer)?;
    }
    prepend(value(first), &mut writer)?;
    Ok(bytes::Bytes::from_owner(writer.0))
}

impl<T: ProtoEncode> AsBytes for EncodedSnapshot<T> {
    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }

    #[cfg(feature = "tonic-owned")]
    fn into_owned_message(self) -> Result<tonic::codec::EncodeResult<Self>, tonic::Status> {
        self.into_owned_frame().map(tonic::codec::EncodeResult::Owned)
    }
}

// Keep framing headroom without adding fields/branches to the ordinary RevVec
// encoder. Length/mark semantics always exclude unused framing headroom.
struct SnapshotWriter(SnapshotBuffer);

impl SnapshotWriter {
    fn reserve(&mut self, len: usize) {
        self.0.buffer.ensure_space(len.checked_add(HEADER).expect("snapshot write overflow"));
    }
}

impl RevWriter for SnapshotWriter {
    type TightBuf = Vec<u8>;
    type Mark = usize;
    fn with_capacity(cap: usize) -> Self {
        Self(SnapshotBuffer::unpooled(RevVec::with_capacity(
            cap.checked_add(HEADER).expect("snapshot capacity overflow"),
        )))
    }
    fn empty() -> Self {
        Self::with_capacity(0)
    }
    fn mark(&self) -> usize {
        self.0.buffer.mark()
    }
    fn written_since(&self, mark: usize) -> usize {
        self.0.buffer.written_since(mark)
    }
    fn as_written_slice(&self) -> &[u8] {
        self.0.as_ref()
    }
    fn len(&self) -> usize {
        self.0.buffer.len()
    }
    fn is_empty(&self) -> bool {
        self.0.buffer.is_empty()
    }
    fn put_u8(&mut self, b: u8) {
        self.reserve(1);
        self.0.buffer.put_u8(b);
    }
    fn put_slice(&mut self, bytes: &[u8]) {
        self.reserve(bytes.len());
        self.0.buffer.put_slice(bytes);
    }
    fn put_varint(&mut self, v: u64) {
        self.reserve(crate::encoding::encoded_len_varint(v));
        self.0.buffer.put_varint(v);
    }
    fn put_bytes<const TAG: u32>(&mut self, bytes: &[u8]) {
        let header = if TAG == 0 {
            0
        } else {
            crate::encoding::key_len(TAG) + crate::encoding::encoded_len_varint(bytes.len() as u64)
        };
        self.reserve(bytes.len().checked_add(header).expect("snapshot field overflow"));
        self.0.buffer.put_bytes::<TAG>(bytes);
    }
    fn finish_tight(mut self) -> Vec<u8> {
        core::mem::replace(&mut self.0.buffer, RevVec::empty()).finish_tight()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    struct Raw {
        bytes: Vec<u8>,
        hint: EncodeSizeHint,
        calls: Cell<usize>,
        shadows: Cell<usize>,
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
        fn size_hint<const TAG: u32>(&self) -> EncodeSizeHint {
            self.hint.for_field::<TAG>(Self::WIRE_TYPE)
        }

        type Shadow<'a> = Shadow<'a>;
    }
    impl<'a> ProtoShadowEncode<'a, Raw> for Shadow<'a> {
        fn from_sun(value: &'a Raw) -> Self {
            value.shadows.set(value.shadows.get() + 1);
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
            // Reentrant encoding must not hold a TLS borrow or alias the
            // outer writer's leased storage, even if every slot is occupied.
            let nested = EncodedSnapshot::new(&123u64);
            for bytes in self.0.bytes.rchunks(17) {
                writer.put_slice(bytes);
            }
            assert_eq!(nested.as_bytes(), 123u64.encode_to_vec());
            assert!(!self.0.panic, "injected snapshot panic");
        }
    }

    #[test]
    fn snapshot_headroom_survives_growth_and_dishonest_hints_without_reencoding() {
        for (hint, cap) in [(0, 64), (1, 8192), (4096, 8192), (100000, 8192)] {
            let raw = Raw {
                bytes: vec![42; 4096],
                hint: EncodeSizeHint::new(hint, true),
                calls: Cell::new(0),
                shadows: Cell::new(0),
                panic: false,
            };
            let snapshot = EncodedSnapshot::with_max_preallocation(&raw, cap);
            assert_eq!(raw.calls.get(), 1);
            assert_eq!(snapshot.as_bytes(), raw.bytes);
            assert_eq!(snapshot.frame.as_ptr().wrapping_add(HEADER), snapshot.as_bytes().as_ptr());
            let pointer = snapshot.as_bytes().as_ptr();
            let bytes = snapshot.into_bytes();
            assert_eq!(bytes.as_ptr(), pointer);
            assert_eq!(&bytes[..], raw.bytes);
        }
    }

    #[cfg(feature = "tonic-owned")]
    #[test]
    fn batch_archives_once_in_wire_order_even_with_dishonest_hints() {
        for hint in [0, 1, 1024] {
            let inputs: Vec<_> = (1..=8)
                .map(|n| Raw {
                    bytes: vec![n; n as usize * 100],
                    hint: EncodeSizeHint::new(hint, true),
                    calls: Cell::new(0),
                    shadows: Cell::new(0),
                    panic: false,
                })
                .collect();
            let mut rest = inputs.iter().skip(1);
            let mut staging = Vec::new();
            let frame = encode_batch(&&inputs[0], &mut || rest.next(), &mut staging, |v| *v, 32768, 64).unwrap();
            let mut data = &frame[..];
            for input in &inputs {
                assert_eq!(input.calls.get(), 1);
                assert_eq!(data[0], 0);
                let len = u32::from_be_bytes(data[1..5].try_into().unwrap()) as usize;
                assert_eq!(&data[5..5 + len], &input.bytes);
                data = &data[5 + len..];
            }
            assert!(data.is_empty());
            assert!(staging.is_empty());
        }
    }

    #[cfg(feature = "tonic")]
    #[test]
    fn prepared_borrowed_message_archives_once_even_when_growing() {
        let raw = Raw {
            bytes: vec![42; 16384],
            hint: EncodeSizeHint::new(1, true),
            calls: Cell::new(0),
            shadows: Cell::new(0),
            panic: false,
        };
        let request = <&Raw as crate::PrepareRequest<Raw>>::prepare_request(&raw, 64).unwrap();
        assert_eq!(raw.calls.get(), 1);
        drop(raw);
        assert_eq!(request.into_inner().payload(), vec![42; 16384]);
    }

    #[test]
    fn tls_pool_recovers_after_archive_panic() {
        let mut raw = Raw {
            bytes: vec![7; 256],
            hint: EncodeSizeHint::new(256, true),
            calls: Cell::new(0),
            shadows: Cell::new(0),
            panic: true,
        };
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| EncodedSnapshot::new(&raw)));
        assert!(failed.is_err());
        raw.panic = false;
        assert_eq!(EncodedSnapshot::new(&raw).as_bytes(), raw.bytes);
        assert_eq!(raw.calls.get(), 2);
    }

    #[cfg(feature = "tonic")]
    #[test]
    fn synchronous_copy_shares_tls_and_recovers_after_encoding_panics() {
        let held: Vec<_> = (0..crate::EncodePoolConfig::default().max_buffers).map(|_| EncodedSnapshot::new(&vec![3u8; 4096])).collect();
        let mut raw = Raw {
            bytes: vec![7; 16384],
            hint: EncodeSizeHint::new(1, true),
            calls: Cell::new(0),
            shadows: Cell::new(0),
            panic: true,
        };
        let mut destination = bytes::BytesMut::new();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| encode_into(&raw, 64, &mut destination))).is_err());
        assert!(destination.is_empty());
        raw.panic = false;
        encode_into(&raw, 64, &mut destination).unwrap();
        assert_eq!(&destination[..], raw.bytes);
        for snapshot in held {
            assert_eq!(snapshot.as_bytes(), vec![3u8; 4096].encode_to_vec());
        }
        destination.clear();
        encode_into(&raw, 64, &mut destination).unwrap();
        assert_eq!(&destination[..], raw.bytes);
        assert_eq!(raw.calls.get(), 3);
    }

    #[test]
    fn snapshot_root_envelopes_are_unchanged() {
        for value in [0u64, 1, 128, u64::MAX] {
            assert_eq!(value.to_encoded_snapshot().as_bytes(), value.encode_to_vec());
            assert_eq!(value.to_encoded_snapshot().into_bytes(), value.encode_to_vec());
        }
        let value = vec![7u8; 256];
        let snapshot = value.to_encoded_snapshot();
        assert_eq!(snapshot.as_bytes(), value.encode_to_vec());
        assert_eq!(snapshot.frame.len(), snapshot.as_bytes().len() + HEADER);
    }

    #[cfg(feature = "tonic-owned")]
    #[test]
    fn threshold_sized_message_constructs_one_shadow_and_never_polls_a_second_input() {
        let raw = Raw {
            bytes: vec![42; 40_000],
            hint: EncodeSizeHint::new(40_000, true),
            calls: Cell::new(0),
            shadows: Cell::new(0),
            panic: false,
        };
        let frame = encode_batch(
            &&raw,
            &mut || panic!("threshold exceeded: must not poll ahead"),
            &mut Vec::new(),
            |v| *v,
            32768,
            65536,
        )
        .unwrap();
        assert_eq!(raw.shadows.get(), 1);
        assert_eq!(raw.calls.get(), 1);
        assert_eq!(&frame[HEADER..], raw.bytes);
    }

    #[test]
    fn shared_snapshot_clones_do_not_reencode_and_outlive_non_sync_input() {
        let raw = Raw {
            bytes: vec![42; 256],
            hint: EncodeSizeHint::new(256, true),
            calls: Cell::new(0),
            shadows: Cell::new(0),
            panic: false,
        };
        let snapshot = EncodedSnapshot::new(&raw);
        let pointer = snapshot.as_bytes().as_ptr();
        let copy = snapshot.clone(); // Raw is not Clone or Sync.
        let last = snapshot.clone();
        let payload = std::thread::spawn(move || copy.into_bytes()).join().unwrap();
        assert_eq!(payload.as_ptr(), pointer);
        assert_eq!(raw.calls.get(), 1);
        assert_eq!(raw.shadows.get(), 1);
        assert_eq!(last.into_bytes(), raw.bytes);
        drop(raw);
        drop(snapshot);
        drop(payload);
    }
}
