#![cfg(feature = "tonic")]
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
    let server = allocation_service_server::AllocationServiceServer::from_arc(implementation);
    let mut client = allocation_service_client::AllocationServiceClient::new(server);
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

struct PreviousEncoder;

impl tonic::codec::Encoder for PreviousEncoder {
    type Item = Message;
    type Error = tonic::Status;

    fn encode(&mut self, item: Message, dst: &mut tonic::codec::EncodeBuf<'_>) -> Result<(), Self::Error> {
        proto_rs::ProtoEncode::encode(&item, dst).unwrap();
        Ok(())
    }
}

fn drain(encoder: impl tonic::codec::Encoder<Item = Message, Error = tonic::Status>) {
    use tonic::codegen::Body;
    let source = tokio_stream::iter((0..32).map(|value| Ok(Message { value })));
    let mut body = std::pin::pin!(tonic::codec::EncodeBody::new_client(encoder, source, None, None));
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        match body.as_mut().poll_frame(&mut context) {
            std::task::Poll::Ready(Some(Ok(frame))) => {
                std::hint::black_box(frame);
            }
            std::task::Poll::Ready(None) => return,
            std::task::Poll::Ready(Some(Err(status))) => panic!("{status}"),
            std::task::Poll::Pending => {}
        }
    }
}

#[test]
fn direct_encoder_removes_per_message_scratch_allocations() {
    drain(PreviousEncoder);
    drain(proto_rs::ProtoEncoder::<Message, proto_rs::SunByVal>::default());
    let previous = count_allocations(|| drain(PreviousEncoder));
    let direct = count_allocations(|| drain(proto_rs::ProtoEncoder::<Message, proto_rs::SunByVal>::default()));
    assert!(direct <= 2 && previous >= direct + 31, "previous={previous}, direct={direct}");
}

#[proto_message]
#[derive(Clone, Debug, PartialEq)]
struct Payload {
    bytes: Vec<u8>,
    sequence: u64,
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
        let encoder = proto_rs::ProtoEncoder::<Payload, proto_rs::SunByVal>::default();
        let body = EncodeBody::new_client(encoder, tokio_stream::iter(values.clone().into_iter().map(Ok)), compression, None);
        let decoder = proto_rs::ProtoCodec::<Payload, Payload>::default().decoder();
        let mut messages = Streaming::new_request(decoder, body, compression, None);
        for expected in values {
            assert_eq!(messages.message().await.unwrap().unwrap(), expected);
        }
        assert!(messages.message().await.unwrap().is_none());
    }

    let encoder = proto_rs::ProtoEncoder::<Payload, proto_rs::SunByVal>::default();
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
