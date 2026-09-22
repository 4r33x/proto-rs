#![cfg(feature = "tonic-owned")]
#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::Weak;

use bytes::Bytes;
use proto_rs::ProtoDecode;
use proto_rs::grpc::Code;
use proto_rs::grpc::MessageStream;
use proto_rs::grpc::Request;
use proto_rs::grpc::Response;
use proto_rs::grpc::Status;
use proto_rs::proto_message;
use proto_rs::proto_rpc;
use tokio_stream::Stream;
use tokio_stream::StreamExt;

struct CountingAllocator;

thread_local! {
    static ALLOCATIONS: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

// SAFETY: operations are delegated unchanged to System. The thread-local
// counter cannot allocate and tolerates TLS teardown.
unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get().map(|n| n + 1)));
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(ptr, layout) };
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get().map(|n| n + 1)));
        unsafe { std::alloc::System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn count_allocations(action: impl FnOnce()) -> usize {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ALLOCATIONS.set(None);
        }
    }
    ALLOCATIONS.set(Some(0));
    let _reset = Reset;
    action();
    ALLOCATIONS.get().unwrap()
}

#[test]
fn fresh_encoders_share_the_warmed_tls_pool() {
    use proto_rs::EncodedSnapshot;
    use proto_rs::ProtoEncoder;
    use proto_rs::SunByRef;
    use tonic::codec::Encoder;
    for _ in 0..proto_rs::EncodePoolConfig::default().max_buffers {
        drop(EncodedSnapshot::new(&42u64));
    }
    let allocations = count_allocations(|| {
        for _ in 0..1000 {
            let mut encoder = ProtoEncoder::<u64, SunByRef>::default();
            drop(encoder.encode_owned(42).unwrap());
        }
    });
    assert_eq!(
        allocations, 1000,
        "fresh encoders share payload storage; only Bytes metadata allocates"
    );
}

#[tokio::test]
async fn multiple_generated_clients_and_servers_need_no_pool_handles() {
    for _ in 0..6 {
        let implementation = Arc::new(Implementation::default());
        implementation.this.set(Arc::downgrade(&implementation)).unwrap();
        let server = allocation_service_server::AllocationServiceServer::from_arc(implementation);
        let client = allocation_service_client::AllocationServiceClient::new(server);
        let mut cloned = client.clone();
        drop(client);
        for _ in 0..2 {
            let message = Message { value: 7 };
            let mut request = tonic::Request::new(&message);
            request.metadata_mut().insert("x-request", "preserved".parse().unwrap());
            assert_eq!(cloned.echo(request).await.unwrap().into_inner(), message);
        }
    }
}

#[cfg(feature = "chrono")]
#[test]
fn successful_chrono_decode_does_not_allocate_errors() {
    let bytes = &[8, 1][..];
    assert_eq!(
        count_allocations(|| {
            for _ in 0..1000 {
                std::hint::black_box(chrono::DateTime::<chrono::Utc>::decode(bytes, proto_rs::DecodeContext::default()).unwrap());
            }
        }),
        0
    );
}

#[proto_message]
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    value: u64,
}

#[proto_rpc(rpc_package = "allocation_tests", rpc_server = true, rpc_client = true)]
pub trait AllocationService {
    type ValuesStream: Stream<Item = Result<Message, Status>> + Send;

    fn echo(&self, request: Request<Message>) -> Result<Response<Message>, Status>;
    fn values(&self, request: Request<Message>) -> Result<Response<Self::ValuesStream>, Status>;
    async fn echo_async(&self, request: Request<Message>) -> Result<Response<Message>, Status>;
}

#[derive(Default)]
struct Implementation {
    this: OnceLock<Weak<Self>>,
}

impl AllocationService for Implementation {
    type ValuesStream = tokio_stream::Iter<std::vec::IntoIter<Result<Message, Status>>>;

    fn echo(&self, request: Request<Message>) -> Result<Response<Message>, Status> {
        // Only the server and the active call own handles. The method adapter
        // must move the latter into its future, not clone it again.
        assert_eq!(self.this.get().unwrap().strong_count(), 2);
        if request.get_ref().value == 0 {
            let mut metadata = proto_rs::grpc::MetadataMap::new();
            metadata.insert("x-error", "preserved".parse().unwrap());
            return Err(Status::with_details_and_metadata(
                Code::InvalidArgument,
                "zero",
                Bytes::from_static(b"details"),
                metadata,
            ));
        }
        assert_eq!(request.metadata()["x-request"], "preserved");
        let mut response = Response::new(request.into_inner());
        response.metadata_mut().insert("x-response", "preserved".parse().unwrap());
        Ok(response)
    }

    fn values(&self, request: Request<Message>) -> Result<Response<Self::ValuesStream>, Status> {
        let response = self.echo(request)?;
        Ok(Response::new(tokio_stream::iter(vec![
            Ok(response.into_inner()),
            Err(Status::aborted("stream error")),
        ])))
    }

    async fn echo_async(&self, request: Request<Message>) -> Result<Response<Message>, Status> {
        tokio::task::yield_now().await;
        self.echo(request)
    }
}

#[tokio::test]
async fn generated_sync_async_and_streaming_adapters_preserve_ownership_and_metadata() {
    let implementation = Arc::new(Implementation::default());
    implementation.this.set(Arc::downgrade(&implementation)).unwrap();
    let observer = Arc::downgrade(&implementation);
    let server = allocation_service_server::AllocationServiceServer::from_arc(implementation).with_max_encode_preallocation(128);
    let mut client = allocation_service_client::AllocationServiceClient::new(server).with_max_encode_preallocation(128);
    let request = || {
        let mut request = tonic::Request::new(Message { value: 7 });
        request.metadata_mut().insert("x-request", "preserved".parse().unwrap());
        request
    };
    for asynchronous in [false, true] {
        let response = if asynchronous {
            client.echo_async(request()).await
        } else {
            client.echo(request()).await
        }
        .unwrap();
        assert_eq!(response.metadata().get("x-response").unwrap(), "preserved");
        assert_eq!(response.into_inner(), Message { value: 7 });
        assert_eq!(observer.strong_count(), 1);
    }
    {
        let future = client.echo_async(request());
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(std::future::Future::poll(future.as_mut(), &mut context).is_pending());
        assert_eq!(observer.strong_count(), 2);
    }
    assert_eq!(
        observer.strong_count(),
        1,
        "cancelling a pending handler releases its service handle"
    );
    let status = client.echo(Message { value: 0 }).await.unwrap_err();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert_eq!(status.details(), b"details");
    assert_eq!(status.metadata().get("x-error").unwrap(), "preserved");
    // The synchronous streaming handler's `?` must return a ready Err future.
    let status = client.values(Message { value: 0 }).await.err().unwrap();
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    {
        let stream = client.values(request()).await.unwrap().into_inner();
        let mut stream = std::pin::pin!(stream);
        assert_eq!(stream.next().await.unwrap().unwrap(), Message { value: 7 });
        assert_eq!(stream.next().await.unwrap().unwrap_err().code(), tonic::Code::Aborted);
    }
    assert_eq!(observer.strong_count(), 1);
}

#[test]
fn owned_status_conversion_preserves_all_fields() {
    let mut metadata = tonic::metadata::MetadataMap::new();
    metadata.insert("x-ascii", "value".parse().unwrap());
    metadata.insert_bin("x-bytes-bin", tonic::metadata::MetadataValue::from_bytes(b"\0\xff"));
    let original = tonic::Status::with_details_and_metadata(tonic::Code::DataLoss, "message", Bytes::from_static(b"details"), metadata);
    let borrowed = proto_rs::grpc::status_from_tonic(&original);
    let moved = proto_rs::grpc::status_from_tonic_owned(original);
    assert_eq!(moved.code(), borrowed.code());
    assert_eq!(moved.message(), borrowed.message());
    assert_eq!(moved.details(), borrowed.details());
    assert_eq!(moved.metadata(), borrowed.metadata());
}

#[test]
fn neutral_once_and_empty_do_not_allocate() {
    assert_eq!(
        count_allocations(|| {
            std::hint::black_box(MessageStream::once(Bytes::from_static(b"message")));
            std::hint::black_box(MessageStream::empty());
        }),
        0
    );
}

#[test]
fn pooled_encoder_allocates_only_owned_metadata_per_batch_after_warmup() {
    use tonic::codegen::Body;
    // Warm every mask before measuring the body's subsequent batch.
    for _ in 0..proto_rs::EncodePoolConfig::default().max_buffers {
        drop(proto_rs::EncodedSnapshot::new(&vec![0u8; 1024]));
    }
    let encoder = proto_rs::ProtoEncoder::<Message, proto_rs::SunByRef>::default();
    let source = tokio_stream::iter((0..256).map(|_| Ok(Message { value: 7 })));
    let mut body = std::pin::pin!(tonic::codec::EncodeBody::new_client(encoder, source, None, None));
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    let std::task::Poll::Ready(Some(Ok(frame))) = body.as_mut().poll_frame(&mut cx) else {
        panic!("missing warm batch")
    };
    // tokio_stream::iter cooperatively yields after 32 messages.
    assert_eq!(frame.data_ref().unwrap().len(), 32 * 7);
    drop(frame);
    let direct = count_allocations(|| {
        let std::task::Poll::Ready(Some(Ok(frame))) = body.as_mut().poll_frame(&mut cx) else {
            panic!("missing batch")
        };
        assert_eq!(frame.data_ref().unwrap().len(), 32 * 7);
    });
    assert_eq!(
        direct, 1,
        "one Bytes owner record for 32 messages, no payload or staging allocations after warmup"
    );
}

#[tokio::test]
#[allow(clippy::drop_non_drop)] // Moving the input proves it is not captured by the future.
async fn borrowed_requests_release_input_before_polling_and_preserve_metadata() {
    fn is_send(_: &impl Send) {}
    let implementation = Arc::new(Implementation::default());
    implementation.this.set(Arc::downgrade(&implementation)).unwrap();
    let server = allocation_service_server::AllocationServiceServer::from_arc(implementation);
    let mut client = allocation_service_client::AllocationServiceClient::new(server);
    let mut input = Message { value: 7 };
    let mut request = tonic::Request::new(&input);
    request.metadata_mut().insert("x-request", "preserved".parse().unwrap());
    let pending = client.echo(request);
    is_send(&pending);
    input.value = 99;
    drop(input);
    let response = pending.await.unwrap();
    assert_eq!(response.into_inner().value, 7);

    let input = Message { value: 8 };
    let mut request = tonic::Request::new(&input);
    request.metadata_mut().insert("x-request", "preserved".parse().unwrap());
    let pending = client.values(request);
    drop(input);
    let mut stream = pending.await.unwrap().into_inner();
    assert_eq!(stream.next().await.unwrap().unwrap().value, 8);
    assert_eq!(stream.next().await.unwrap().unwrap_err().code(), tonic::Code::Aborted);

    let input = Message { value: 0 };
    let pending = client.echo(&input);
    drop(input);
    assert_eq!(pending.await.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
#[allow(clippy::drop_non_drop)]
async fn neutral_tonic_adapter_prepares_borrowed_input_before_returning() {
    let implementation = Arc::new(Implementation::default());
    implementation.this.set(Arc::downgrade(&implementation)).unwrap();
    let server = allocation_service_server::AllocationServiceServer::from_arc(implementation);
    let mut transport = proto_rs::grpc::TonicTransport::new(server);
    let message = Message { value: 17 };
    let mut request = Request::new(&message);
    request.metadata_mut().insert("x-request", "preserved".parse().unwrap());
    let pending = transport.unary_ref::<_, Message>("/allocation_tests.AllocationService/Echo", request);
    drop(message);
    assert_eq!(pending.await.unwrap().into_inner().value, 17);
}

#[tokio::test]
async fn neutral_owned_calls_prepare_and_release_input_before_returning() {
    use proto_rs::grpc::GrpcTransport;
    let implementation = Arc::new(Implementation::default());
    implementation.this.set(Arc::downgrade(&implementation)).unwrap();
    let server = allocation_service_server::AllocationServiceServer::from_arc(implementation);
    let mut transport = proto_rs::grpc::TonicTransport::new(server);

    // Cancellation before poll must not retain the owned request. No RPC is
    // sent in these two lifetime checks.
    let input = Arc::new(Message { value: 17 });
    let pending = transport.unary::<_, Message>("/allocation_tests.AllocationService/Echo", Request::new(Arc::clone(&input)));
    assert_eq!(Arc::strong_count(&input), 1);
    drop(pending);
    let pending = transport.server_streaming::<_, Message>("/allocation_tests.AllocationService/Values", Request::new(Arc::clone(&input)));
    assert_eq!(Arc::strong_count(&input), 1);
    drop(pending);

    let mut request = Request::new(Message { value: 17 });
    request.metadata_mut().insert("x-request", "preserved".parse().unwrap());
    let response = transport.unary::<_, Message>("/allocation_tests.AllocationService/Echo", request).await.unwrap();
    assert_eq!(response.metadata()["x-response"], "preserved");
    assert_eq!(response.into_inner().value, 17);
    let mut request = Request::new(Message { value: 7 });
    request.metadata_mut().insert("x-request", "preserved".parse().unwrap());
    let response = transport.server_streaming::<_, Message>("/allocation_tests.AllocationService/Values", request).await.unwrap();
    let mut stream = std::pin::pin!(response.into_inner());
    assert_eq!(stream.next().await.unwrap().unwrap().value, 7);
    assert_eq!(stream.next().await.unwrap().unwrap_err().code(), Code::Aborted);
}

#[cfg(feature = "tonic-transport")]
#[tokio::test]
#[allow(clippy::drop_non_drop)]
async fn streaming_sender_borrows_only_during_preparation() {
    use proto_rs::grpc::EncodedSender;
    let (sender, mut stream) = EncodedSender::<Message>::channel(1);
    let mut message = Message { value: 9 };
    let pending = sender.send(&message);
    message.value = 99;
    drop(message);
    pending.await.unwrap();
    let frame = stream.next().await.unwrap();
    assert_eq!(
        Message::decode(frame.payload(), proto_rs::DecodeContext::default()).unwrap().value,
        9
    );

    sender.send(&Message { value: 10 }).await.unwrap();
    let message = Message { value: 11 };
    let pending = sender.send(&message);
    drop(message);
    let mut pending = std::pin::pin!(pending);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(std::future::Future::poll(pending.as_mut(), &mut cx).is_pending());
    assert_eq!(
        Message::decode(stream.next().await.unwrap().payload(), proto_rs::DecodeContext::default()).unwrap().value,
        10
    );
    pending.await.unwrap();
    assert_eq!(
        Message::decode(stream.next().await.unwrap().payload(), proto_rs::DecodeContext::default()).unwrap().value,
        11
    );
    drop(stream);
    assert_eq!(
        sender.send(&Message { value: 12 }).await.unwrap_err().code(),
        tonic::Code::Cancelled
    );
}

#[proto_message]
#[derive(Clone, Debug, PartialEq)]
struct Payload {
    bytes: Vec<u8>,
    sequence: u64,
}

#[proto_message]
#[derive(Clone, Debug, PartialEq)]
struct BatchRecord {
    a: u64,
    b: u64,
    c: Option<u64>,
    d: Arc<Vec<u8>>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct ProstBatchRecord {
    #[prost(uint64, tag = "1")]
    a: u64,
    #[prost(uint64, tag = "2")]
    b: u64,
    #[prost(uint64, optional, tag = "3")]
    c: Option<u64>,
    #[prost(bytes = "vec", tag = "4")]
    d: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct ProstBatch {
    #[prost(message, repeated, tag = "1")]
    items: Vec<ProstBatchRecord>,
}

fn batch(count: usize, size: usize) -> Vec<BatchRecord> {
    (0..count)
        .map(|index| BatchRecord {
            a: index as u64,
            b: u64::MAX - index as u64,
            c: match index % 3 {
                0 => None,
                1 => Some(0),
                _ => Some(u64::MAX),
            },
            d: Arc::new(vec![u8::try_from(index).unwrap(); size]),
        })
        .collect()
}

#[test]
fn batch_output_hint_is_exact_bounded_and_matches_prost() {
    use proto_rs::ProtoArchive;
    use proto_rs::ProtoEncode;
    for count in [0, 1, 20, 32, 33] {
        for size in [0, 128] {
            let values = batch(count, size);
            let bytes = values.encode_to_vec();
            let hint = values.as_slice().output_size_hint::<1>();
            assert_eq!(hint.exact, count <= 32);
            if hint.exact {
                assert_eq!(hint.size, bytes.len());
            }
            let prost = ProstBatch {
                items: values
                    .iter()
                    .map(|value| ProstBatchRecord {
                        a: value.a,
                        b: value.b,
                        c: value.c,
                        d: value.d.as_ref().clone(),
                    })
                    .collect(),
            };
            assert_eq!(bytes, prost::Message::encode_to_vec(&prost));
            if count > 0 {
                assert!(!values.as_slice().encoded_size_hint::<1>().exact);
            }
            let nested = vec![values];
            assert!(!nested.as_slice().output_size_hint::<1>().exact, "must not recurse into lists");
        }
    }
    // Repeated empty messages must retain their key and zero-length delimiter.
    let empty = [BatchRecord {
        a: 0,
        b: 0,
        c: None,
        d: Arc::new(vec![]),
    }];
    assert_eq!(empty.as_slice().output_size_hint::<1>().size, 2);
}

#[test]
fn shared_snapshot_allocates_metadata_once_and_cloning_and_handoff_do_not_allocate() {
    use proto_rs::BytesMode;
    use proto_rs::EncodedSnapshot;
    use proto_rs::ProtoEncoder;
    use tonic::codec::EncodeBody;
    use tonic::codegen::Body;
    type Batch = Vec<BatchRecord>;
    let values = batch(20, 128 * 1024);
    let mut snapshot = None;
    // Lease every TLS slot before counting to force a temporary allocation.
    let held: Vec<_> = (0..proto_rs::EncodePoolConfig::default().max_buffers).map(|_| EncodedSnapshot::new(&42u64)).collect();
    let created = count_allocations(|| snapshot = Some(EncodedSnapshot::with_max_preallocation(&values, 4 * 1024 * 1024)));
    assert_eq!(created, 2, "one payload allocation and one atomic shared-owner record");
    let snapshot = snapshot.unwrap();
    let pointer = snapshot.as_bytes().as_ptr();
    assert_eq!(
        count_allocations(|| {
            let clone = snapshot.clone();
            assert_eq!(clone.as_bytes().as_ptr(), pointer);
            std::hint::black_box(clone);
        }),
        0,
        "cloning shares the existing owner, no new Arc/Bytes allocation"
    );
    let mut frame = None;
    let handed_off = count_allocations(|| {
        let mut body = std::pin::pin!(EncodeBody::new_client(
            ProtoEncoder::<EncodedSnapshot<Batch>, BytesMode>::default(),
            tokio_stream::iter([Ok(snapshot)]),
            None,
            None
        ));
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        loop {
            if let std::task::Poll::Ready(value) = body.as_mut().poll_frame(&mut cx) {
                frame = Some(value.unwrap().unwrap().into_data().unwrap());
                break;
            }
        }
    });
    assert_eq!(handed_off, 0, "handoff reuses the snapshot's existing shared-owner record");
    assert_eq!(frame.unwrap().as_ptr().wrapping_add(5), pointer);
    drop(held);
    for _ in 0..proto_rs::EncodePoolConfig::default().max_buffers {
        drop(EncodedSnapshot::with_max_preallocation(&values, 4 * 1024 * 1024));
    }
    let reused = count_allocations(|| drop(EncodedSnapshot::with_max_preallocation(&values, 4 * 1024 * 1024)));
    assert_eq!(reused, 1, "warm pool needs only one shared-owner record for a new snapshot");
}

#[tokio::test]
async fn large_shared_batch_avoids_scratch_and_roundtrips_with_gzip() {
    use proto_rs::ProtoCodec;
    use proto_rs::ProtoEncoder;
    use proto_rs::ProtoResponse;
    use tonic::codec::Codec;
    use tonic::codec::CompressionEncoding;
    use tonic::codec::EncodeBody;
    use tonic::codec::Streaming;
    use tonic::codegen::Body;
    type Batch = Vec<BatchRecord>;
    type Mode = <Response<Arc<Batch>> as ProtoResponse<Batch>>::Mode;
    let values = Arc::new(batch(20, 128 * 1024));
    // Keep the cold preallocation comparison honest: TLS reuse must not hide
    // an undersized initial reservation on the second iteration.
    let held: Vec<_> = (0..proto_rs::EncodePoolConfig::default().max_buffers).map(|_| proto_rs::EncodedSnapshot::new(&42u64)).collect();
    let mut counts = Vec::new();
    for cap in [proto_rs::DEFAULT_MAX_ENCODE_PREALLOCATION, 4 * 1024 * 1024] {
        let encoder = ProtoEncoder::<Arc<Batch>, Mode>::default().with_max_encode_preallocation(cap);
        let mut body = std::pin::pin!(EncodeBody::new_client(
            encoder,
            tokio_stream::iter([Ok(Arc::clone(&values))]),
            None,
            None
        ));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        let mut frame = None;
        let count = count_allocations(|| {
            loop {
                match body.as_mut().poll_frame(&mut context) {
                    std::task::Poll::Ready(Some(Ok(value))) => {
                        frame = Some(value.into_data().unwrap());
                        break;
                    }
                    std::task::Poll::Pending => {}
                    std::task::Poll::Ready(_) => panic!("expected a data frame"),
                }
            }
        });
        let frame = frame.unwrap();
        assert_eq!(frame[0], 0);
        assert_eq!(u32::from_be_bytes(frame[1..5].try_into().unwrap()) as usize, frame.len() - 5);
        assert_eq!(Batch::decode(&frame[5..], proto_rs::DecodeContext::default()).unwrap(), *values);
        counts.push(count);
    }
    assert!(counts[1] < counts[0], "bounded vs direct allocation calls: {counts:?}");
    eprintln!("large batch allocation calls (bounded, direct): {counts:?}");
    drop(held);
    for compression in [None, Some(CompressionEncoding::Gzip)] {
        let encoder = ProtoEncoder::<Arc<Batch>, Mode>::default().with_max_encode_preallocation(4 * 1024 * 1024);
        let body = EncodeBody::new_client(encoder, tokio_stream::iter([Ok(Arc::clone(&values))]), compression, None);
        let decoder = ProtoCodec::<Batch, Batch>::default().decoder();
        let mut stream = Streaming::new_request(decoder, body, compression, None);
        assert_eq!(stream.message().await.unwrap().unwrap(), *values);
        assert!(stream.message().await.unwrap().is_none());
    }
    assert_eq!(Arc::strong_count(&values), 1);
    assert!(values.iter().all(|value| Arc::strong_count(&value.d) == 1));
}

#[tokio::test]
async fn tonic_framing_compression_and_message_limits_are_preserved() {
    use tonic::codec::Codec;
    use tonic::codec::CompressionEncoding;
    use tonic::codec::EncodeBody;
    use tonic::codec::Streaming;

    for compression in [None, Some(CompressionEncoding::Gzip)] {
        let values: Vec<_> = [0, 1, 8192, 200_000, 7, 0]
            .into_iter()
            .enumerate()
            .map(|(index, len)| Payload {
                bytes: vec![42; len],
                sequence: index as u64,
            })
            .collect();
        let encoder = proto_rs::ProtoEncoder::<Payload, proto_rs::SunByRef>::default();
        let body = EncodeBody::new_client(encoder, tokio_stream::iter(values.clone().into_iter().map(Ok)), compression, None);
        let decoder = proto_rs::ProtoCodec::<Payload, Payload>::default().decoder();
        let mut messages = Streaming::new_request(decoder, body, compression, None);
        for expected in values {
            assert_eq!(messages.message().await.unwrap().unwrap(), expected);
        }
        assert!(messages.message().await.unwrap().is_none());
    }

    let encoder = proto_rs::ProtoEncoder::<Payload, proto_rs::SunByRef>::default();
    let body = EncodeBody::new_client(
        encoder,
        tokio_stream::iter([Ok(Payload {
            bytes: vec![0; 128],
            sequence: 1,
        })]),
        None,
        Some(16),
    );
    let decoder = proto_rs::ProtoCodec::<Payload, Payload>::default().decoder();
    let mut messages = Streaming::new_request(decoder, body, None, None);
    assert_eq!(messages.message().await.unwrap_err().code(), tonic::Code::OutOfRange);
}

// A !Unpin source exercises structural pin projection in the now-inline adapter.
pin_project_lite::pin_project! {
    struct PinnedStream {
        next: Option<Message>,
        #[pin]
        pinned: std::marker::PhantomPinned,
    }
}

impl Stream for PinnedStream {
    type Item = Result<Message, Status>;

    fn poll_next(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        std::task::Poll::Ready(self.project().next.take().map(Ok))
    }
}

#[tokio::test]
async fn neutral_streams_support_inline_singletons_and_pinned_sources() {
    let mut once = MessageStream::once(Bytes::from_static(b"value"));
    assert_eq!(once.next().await.unwrap().unwrap(), b"value"[..]);
    assert!(once.next().await.is_none());
    assert!(MessageStream::empty().next().await.is_none());
    let source = PinnedStream {
        next: Some(Message { value: 42 }),
        pinned: std::marker::PhantomPinned,
    };
    let mut response = proto_rs::grpc::encode_streaming_response::<Message, Message, _>(Response::new(source)).into_inner();
    let bytes = response.next().await.unwrap().unwrap();
    assert_eq!(
        Message::decode(bytes, proto_rs::DecodeContext::default()).unwrap(),
        Message { value: 42 }
    );
    assert!(StreamExt::next(&mut response).await.is_none());
}
