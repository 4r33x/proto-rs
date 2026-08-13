#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

use std::collections::HashMap;
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use encoding_messages::ZeroCopyContainer;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::grpc::GrpcService;
use proto_rs::grpc::GrpcTransport;
use proto_rs::grpc::Request;
use proto_rs::grpc::Response;
use proto_rs::grpc::Status;
use proto_rs::proto_rpc;
use tokio_stream::Stream;
use tokio_stream::StreamExt;

mod encoding_messages;

use encoding_messages::CollectionsMessage;
use encoding_messages::CollectionsMessageProst;
use encoding_messages::NestedMessage;
use encoding_messages::NestedMessageProst;
use encoding_messages::SampleEnum;
use encoding_messages::SampleMessage;
use encoding_messages::SampleMessageProst;
use encoding_messages::ZeroCopyContainerProst;
use encoding_messages::sample_collections_messages;
use encoding_messages::sample_message;
use encoding_messages::zero_copy_fixture;

#[proto_rpc(
    rpc_package = "complex_rpc",
    rpc_server = true,
    rpc_client = true,
    proto_path = "protos/tests/complex_rpc.proto"
)]
#[proto_imports(encoding = ["SampleMessage", "CollectionsMessage", "NestedMessage", "ZeroCopyContainer"])]
pub trait ComplexService {
    type StreamCollectionsStream: Stream<Item = Result<CollectionsMessage, Status>> + Send;

    async fn echo_sample(&self, request: Request<SampleMessage>) -> Result<Response<SampleMessage>, Status>;

    async fn echo_sample_arc(&self, request: Request<SampleMessage>) -> Result<Response<Arc<SampleMessage>>, Status>;

    async fn echo_sample_box(&self, request: Request<SampleMessage>) -> Result<Response<Box<SampleMessage>>, Status>;

    async fn stream_collections(&self, request: Request<SampleMessage>) -> Result<Response<Self::StreamCollectionsStream>, Status>;

    async fn echo_container(&self, request: Request<ZeroCopyContainer>) -> Result<Response<ZeroCopyContainer>, Status>;
}

fn request_message() -> SampleMessage {
    sample_message()
}

fn response_message() -> SampleMessage {
    let mut msg = sample_message();
    msg.id = 1337;
    msg.flag = false;
    msg.name = "complex-response".into();
    msg.data = vec![9, 8, 7, 6, 5];
    msg.nested = Some(NestedMessage { value: 2048 });
    msg.nested_list.push(NestedMessage { value: -128 });
    msg.values = vec![5, 10, -15];
    msg.mode = SampleEnum::One;
    msg.optional_mode = Some(SampleEnum::Two);
    msg
}

fn request_container() -> ZeroCopyContainer {
    zero_copy_fixture()
}

fn response_container() -> ZeroCopyContainer {
    let mut container = zero_copy_fixture();
    container.bytes32[0] = 0xAA;
    container.smalls[0] = 512;
    container.enum_lookup.insert("response".into(), SampleEnum::Zero);
    container.boxed = Some(Box::new(NestedMessage { value: 2048 }));
    container.shared = Some(Arc::new(NestedMessage { value: -999 }));
    container
}

fn response_collections() -> Vec<CollectionsMessage> {
    let mut messages = sample_collections_messages();
    if let Some(first) = messages.get_mut(0) {
        first.hash_scores.insert(99, -99);
        first.hash_tags.insert("omega".into());
    }
    if let Some(second) = messages.get_mut(1) {
        second.tree_messages.insert("delta".into(), NestedMessage { value: -256 });
    }
    messages
}

const fn nested_to_tonic(nested: &NestedMessageProst) -> tonic_prost_test::encoding::NestedMessage {
    tonic_prost_test::encoding::NestedMessage { value: nested.value }
}

const fn nested_from_tonic(nested: tonic_prost_test::encoding::NestedMessage) -> NestedMessageProst {
    NestedMessageProst { value: nested.value }
}

fn sample_to_tonic(msg: &SampleMessage) -> tonic_prost_test::encoding::SampleMessage {
    let prost = SampleMessageProst::from(msg);
    tonic_prost_test::encoding::SampleMessage {
        id: prost.id,
        flag: prost.flag,
        name: prost.name,
        data: prost.data,
        nested: prost.nested.map(|nested| nested_to_tonic(&nested)),
        nested_list: prost.nested_list.into_iter().map(|nested| nested_to_tonic(&nested)).collect(),
        values: prost.values,
        mode: prost.mode,
        optional_mode: prost.optional_mode,
    }
}

fn sample_from_tonic(msg: tonic_prost_test::encoding::SampleMessage) -> SampleMessage {
    let tonic_prost_test::encoding::SampleMessage {
        id,
        flag,
        name,
        data,
        nested,
        nested_list,
        values,
        mode,
        optional_mode,
    } = msg;

    let nested = nested.map(nested_from_tonic);
    let nested_list = nested_list.into_iter().map(nested_from_tonic).collect();

    let prost = SampleMessageProst {
        id,
        flag,
        name,
        data,
        nested,
        nested_list,
        values,
        mode,
        optional_mode,
    };

    SampleMessage::from(&prost)
}

fn collections_to_tonic(msg: &CollectionsMessage) -> tonic_prost_test::encoding::CollectionsMessage {
    let prost = CollectionsMessageProst::from(msg);
    tonic_prost_test::encoding::CollectionsMessage {
        hash_scores: prost.hash_scores,
        tree_messages: prost.tree_messages.into_iter().map(|(key, value)| (key, nested_to_tonic(&value))).collect(),
        hash_tags: prost.hash_tags,
        tree_ids: prost.tree_ids,
    }
}

fn collections_from_tonic(msg: tonic_prost_test::encoding::CollectionsMessage) -> CollectionsMessage {
    let tonic_prost_test::encoding::CollectionsMessage {
        hash_scores,
        tree_messages,
        hash_tags,
        tree_ids,
    } = msg;

    let tree_messages = tree_messages.into_iter().map(|(key, value)| (key, nested_from_tonic(value))).collect::<HashMap<_, _>>();

    let prost = CollectionsMessageProst {
        hash_scores,
        tree_messages,
        hash_tags,
        tree_ids,
    };

    CollectionsMessage::from(&prost)
}

fn container_to_tonic(msg: &ZeroCopyContainer) -> tonic_prost_test::encoding::ZeroCopyContainer {
    let prost = ZeroCopyContainerProst::from(msg);
    tonic_prost_test::encoding::ZeroCopyContainer {
        bytes32: prost.bytes32,
        smalls: prost.smalls,
        nested_items: prost.nested_items.into_iter().map(|nested| nested_to_tonic(&nested)).collect(),
        boxed: prost.boxed.map(|nested| nested_to_tonic(&nested)),
        shared: prost.shared.map(|nested| nested_to_tonic(&nested)),
        enum_lookup: prost.enum_lookup,
    }
}

fn container_from_tonic(msg: tonic_prost_test::encoding::ZeroCopyContainer) -> ZeroCopyContainer {
    let tonic_prost_test::encoding::ZeroCopyContainer {
        bytes32,
        smalls,
        nested_items,
        boxed,
        shared,
        enum_lookup,
    } = msg;

    let prost = ZeroCopyContainerProst {
        bytes32,
        smalls,
        nested_items: nested_items.into_iter().map(nested_from_tonic).collect(),
        boxed: boxed.map(nested_from_tonic),
        shared: shared.map(nested_from_tonic),
        enum_lookup,
    };

    ZeroCopyContainer::from(&prost)
}

struct OurService;

struct CustomRuntime {
    unary_response: Vec<u8>,
    stream_responses: Vec<Vec<u8>>,
}

struct CustomResponseStream<T> {
    messages: VecDeque<Result<T, Status>>,
}

impl<T> Unpin for CustomResponseStream<T> {}

impl<T> Stream for CustomResponseStream<T> {
    type Item = Result<T, Status>;

    fn poll_next(mut self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.messages.pop_front())
    }
}

impl GrpcTransport for CustomRuntime {
    type Error = Status;
    type ResponseStream<T>
        = CustomResponseStream<T>
    where
        T: Send + 'static;

    async fn unary<Req, Res>(&mut self, _route: &'static str, _request: Request<Req>) -> Result<Response<Res>, Self::Error>
    where
        Req: proto_rs::ProtoEncode + proto_rs::ProtoExt + Send + Sync + 'static,
        Res: ProtoDecode + Send + Sync + 'static,
    {
        let response = Res::decode(self.unary_response.as_slice(), proto_rs::DecodeContext::default())
            .map_err(|error| Status::internal(error.to_string()))?;
        Ok(Response::new(response))
    }

    async fn client_streaming<Req, Res, S>(&mut self, _route: &'static str, _request: Request<S>) -> Result<Response<Res>, Self::Error>
    where
        Req: proto_rs::ProtoEncode + proto_rs::ProtoExt + Send + Sync + 'static,
        Res: ProtoDecode + Send + Sync + 'static,
        S: Stream<Item = Req> + Send + 'static,
    {
        let response = Res::decode(self.unary_response.as_slice(), proto_rs::DecodeContext::default())
            .map_err(|error| Status::internal(error.to_string()))?;
        Ok(Response::new(response))
    }

    async fn server_streaming<Req, Res>(
        &mut self,
        _route: &'static str,
        _request: Request<Req>,
    ) -> Result<Response<Self::ResponseStream<Res>>, Self::Error>
    where
        Req: proto_rs::ProtoEncode + proto_rs::ProtoExt + Send + Sync + 'static,
        Res: ProtoDecode + Send + Sync + 'static,
    {
        let messages = self
            .stream_responses
            .iter()
            .map(|bytes| {
                Res::decode(bytes.as_slice(), proto_rs::DecodeContext::default()).map_err(|error| Status::internal(error.to_string()))
            })
            .collect();
        Ok(Response::new(CustomResponseStream { messages }))
    }

    async fn bidirectional_streaming<Req, Res, S>(
        &mut self,
        _route: &'static str,
        _request: Request<S>,
    ) -> Result<Response<Self::ResponseStream<Res>>, Self::Error>
    where
        Req: proto_rs::ProtoEncode + proto_rs::ProtoExt + Send + Sync + 'static,
        Res: ProtoDecode + Send + Sync + 'static,
        S: Stream<Item = Req> + Send + 'static,
    {
        let messages = self
            .stream_responses
            .iter()
            .map(|bytes| {
                Res::decode(bytes.as_slice(), proto_rs::DecodeContext::default()).map_err(|error| Status::internal(error.to_string()))
            })
            .collect();
        Ok(Response::new(CustomResponseStream { messages }))
    }
}

impl ComplexService for OurService {
    type StreamCollectionsStream = Pin<Box<dyn Stream<Item = Result<CollectionsMessage, Status>> + Send>>;

    async fn echo_sample(&self, _request: Request<SampleMessage>) -> Result<Response<SampleMessage>, Status> {
        Ok(Response::new(response_message()))
    }

    async fn echo_sample_arc(&self, _request: Request<SampleMessage>) -> Result<Response<Arc<SampleMessage>>, Status> {
        Ok(Response::new(Arc::new(response_message())))
    }

    async fn echo_sample_box(&self, _request: Request<SampleMessage>) -> Result<Response<Box<SampleMessage>>, Status> {
        Ok(Response::new(Box::new(response_message())))
    }

    async fn stream_collections(&self, _request: Request<SampleMessage>) -> Result<Response<Self::StreamCollectionsStream>, Status> {
        let stream = tokio_stream::iter(response_collections().into_iter().map(Ok));
        let boxed_stream: Self::StreamCollectionsStream = Box::pin(stream);
        Ok(Response::new(boxed_stream))
    }

    async fn echo_container(&self, _request: Request<ZeroCopyContainer>) -> Result<Response<ZeroCopyContainer>, Status> {
        Ok(Response::new(response_container()))
    }
}

struct ProstService;

#[tonic::async_trait]
impl tonic_prost_test::complex_rpc::complex_service_server::ComplexService for ProstService {
    type StreamCollectionsStream =
        Pin<Box<dyn Stream<Item = Result<tonic_prost_test::encoding::CollectionsMessage, tonic::Status>> + Send>>;

    async fn echo_sample(
        &self,
        _request: tonic::Request<tonic_prost_test::encoding::SampleMessage>,
    ) -> Result<tonic::Response<tonic_prost_test::encoding::SampleMessage>, tonic::Status> {
        Ok(tonic::Response::new(sample_to_tonic(&response_message())))
    }

    async fn echo_sample_arc(
        &self,
        _request: tonic::Request<tonic_prost_test::encoding::SampleMessage>,
    ) -> Result<tonic::Response<tonic_prost_test::encoding::SampleMessage>, tonic::Status> {
        Ok(tonic::Response::new(sample_to_tonic(&response_message())))
    }

    async fn echo_sample_box(
        &self,
        _request: tonic::Request<tonic_prost_test::encoding::SampleMessage>,
    ) -> Result<tonic::Response<tonic_prost_test::encoding::SampleMessage>, tonic::Status> {
        Ok(tonic::Response::new(sample_to_tonic(&response_message())))
    }

    async fn stream_collections(
        &self,
        _request: tonic::Request<tonic_prost_test::encoding::SampleMessage>,
    ) -> Result<tonic::Response<Self::StreamCollectionsStream>, tonic::Status> {
        let items = response_collections().into_iter().map(|msg| Ok(collections_to_tonic(&msg)));
        Ok(tonic::Response::new(Box::pin(tokio_stream::iter(items))))
    }

    async fn echo_container(
        &self,
        _request: tonic::Request<tonic_prost_test::encoding::ZeroCopyContainer>,
    ) -> Result<tonic::Response<tonic_prost_test::encoding::ZeroCopyContainer>, tonic::Status> {
        Ok(tonic::Response::new(container_to_tonic(&response_container())))
    }
}

#[cfg(feature = "tonic")]
async fn spawn_our_server() -> (
    std::net::SocketAddr,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
) {
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::transport::Server;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let incoming = TcpListenerStream::new(listener);

    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(complex_service_server::ComplexServiceServer::new(OurService))
            .serve_with_incoming_shutdown(incoming, async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    (addr, shutdown_tx, handle)
}

#[cfg(feature = "tonic")]
async fn spawn_prost_server() -> (
    std::net::SocketAddr,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
) {
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::transport::Server;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let incoming = TcpListenerStream::new(listener);

    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(tonic_prost_test::complex_rpc::complex_service_server::ComplexServiceServer::new(
                ProstService,
            ))
            .serve_with_incoming_shutdown(incoming, async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    (addr, shutdown_tx, handle)
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "tonic")]
async fn tonic_client_roundtrip_against_proto_server() {
    let (addr, shutdown, handle) = spawn_our_server().await;

    let mut client =
        tonic_prost_test::complex_rpc::complex_service_client::ComplexServiceClient::connect(format!("http://{addr}")).await.unwrap();

    let request = sample_to_tonic(&request_message());
    let response = client.echo_sample(request.clone()).await.unwrap().into_inner();
    assert_eq!(sample_from_tonic(response), response_message());

    let arced = client.echo_sample_arc(request.clone()).await.unwrap().into_inner();
    assert_eq!(sample_from_tonic(arced), response_message());

    let boxed = client.echo_sample_box(request.clone()).await.unwrap().into_inner();
    assert_eq!(sample_from_tonic(boxed), response_message());

    let mut stream = client.stream_collections(request).await.unwrap().into_inner();

    let mut received = Vec::new();
    while let Some(item) = stream.message().await.unwrap() {
        received.push(collections_from_tonic(item));
    }

    assert_eq!(received, response_collections());

    drop(stream);

    let container_request = container_to_tonic(&request_container());
    let container_response = client.echo_container(container_request).await.unwrap().into_inner();
    assert_eq!(container_from_tonic(container_response), response_container());

    shutdown.send(()).unwrap();
    handle.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "tonic")]
async fn proto_client_roundtrip_against_prost_server() {
    let (addr, shutdown, handle) = spawn_prost_server().await;

    let mut client = complex_service_client::ComplexServiceClient::connect(format!("http://{addr}")).await.unwrap();

    let response = client.echo_sample(tonic::Request::new(request_message())).await.unwrap().into_inner();
    assert_eq!(response, response_message());

    let arced = client.echo_sample_arc(tonic::Request::new(request_message())).await.unwrap().into_inner();
    assert_eq!(arced, response_message());

    let boxed = client.echo_sample_box(tonic::Request::new(request_message())).await.unwrap().into_inner();
    assert_eq!(boxed, response_message());

    let mut stream = client.stream_collections(tonic::Request::new(request_message())).await.unwrap().into_inner();

    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.unwrap());
    }

    assert_eq!(received, response_collections());

    drop(stream);

    let container_response = client.echo_container(tonic::Request::new(request_container())).await.unwrap().into_inner();
    assert_eq!(container_response, response_container());

    shutdown.send(()).unwrap();
    handle.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "tonic")]
async fn proto_client_roundtrip_against_proto_server() {
    let (addr, shutdown, handle) = spawn_our_server().await;

    let mut client = complex_service_client::ComplexServiceClient::connect(format!("http://{addr}")).await.unwrap();

    let response = client.echo_sample(tonic::Request::new(request_message())).await.unwrap().into_inner();
    assert_eq!(response, response_message());

    let arced = client.echo_sample_arc(tonic::Request::new(request_message())).await.unwrap().into_inner();
    assert_eq!(arced, response_message());

    let boxed = client.echo_sample_box(tonic::Request::new(request_message())).await.unwrap().into_inner();
    assert_eq!(boxed, response_message());

    let mut stream = client.stream_collections(tonic::Request::new(request_message())).await.unwrap().into_inner();

    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.unwrap());
    }

    assert_eq!(received, response_collections());

    drop(stream);

    let container_response = client.echo_container(tonic::Request::new(request_container())).await.unwrap().into_inner();
    assert_eq!(container_response, response_container());

    shutdown.send(()).unwrap();
    handle.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "tonic")]
async fn transport_client_roundtrip_against_proto_server() {
    let (addr, shutdown, handle) = spawn_our_server().await;

    let channel = tonic::transport::Endpoint::from_shared(format!("http://{addr}")).unwrap().connect().await.unwrap();
    let transport: proto_rs::grpc::TonicTransport<_> = channel.into();
    let mut client = complex_service_transport_client::ComplexServiceTransportClient::new(transport);

    let response = client.echo_sample(Request::new(request_message())).await.unwrap().into_inner();
    assert_eq!(response, response_message());

    let mut stream = client.stream_collections(Request::new(request_message())).await.unwrap().into_inner();
    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.unwrap());
    }
    assert_eq!(received, response_collections());

    drop(stream);
    shutdown.send(()).unwrap();
    handle.await.unwrap().unwrap();
}

#[tokio::test]
async fn generated_router_dispatches_without_tonic() {
    let router = complex_service_service::ComplexServiceRouter::new(OurService);
    assert_eq!(router.methods(), complex_service_service::METHODS);

    let request = Request::new(proto_rs::grpc::MessageStream::once(request_message().encode_to_vec().into()));
    let response = router.call("/complex_rpc.ComplexService/EchoSample", request).await.unwrap();
    let mut messages = response.into_inner();
    let message = messages.next().await.unwrap().unwrap();
    assert_eq!(
        SampleMessage::decode(message, proto_rs::DecodeContext::default()).unwrap(),
        response_message()
    );
    assert!(messages.next().await.is_none());

    let request = Request::new(proto_rs::grpc::MessageStream::once(request_message().encode_to_vec().into()));
    let response = router.call("/complex_rpc.ComplexService/StreamCollections", request).await.unwrap();
    let mut messages = response.into_inner();
    let mut received = Vec::new();
    while let Some(message) = messages.next().await {
        received.push(CollectionsMessage::decode(message.unwrap(), proto_rs::DecodeContext::default()).unwrap());
    }
    assert_eq!(received, response_collections());
}

#[tokio::test]
async fn generated_client_accepts_custom_runtime() {
    let runtime = CustomRuntime {
        unary_response: response_message().encode_to_vec(),
        stream_responses: response_collections().iter().map(ProtoEncode::encode_to_vec).collect(),
    };
    let mut client = complex_service_transport_client::ComplexServiceTransportClient::new(runtime);

    let response = client.echo_sample(Request::new(request_message())).await.unwrap().into_inner();
    assert_eq!(response, response_message());

    let mut stream = client.stream_collections(Request::new(request_message())).await.unwrap().into_inner();
    let mut received = Vec::new();
    while let Some(message) = stream.next().await {
        received.push(message.unwrap());
    }
    assert_eq!(received, response_collections());
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "tonic")]
async fn proto_client_accepts_borrowed_requests() {
    let (addr, shutdown, handle) = spawn_our_server().await;

    let mut client = complex_service_client::ComplexServiceClient::connect(format!("http://{addr}")).await.unwrap();

    let request = request_message();

    let response = client.echo_sample(request_message()).await.unwrap().into_inner();
    assert_eq!(response, response_message());

    let response = client.echo_sample(tonic::Request::new(request.clone())).await.unwrap().into_inner();
    assert_eq!(response, response_message());

    shutdown.send(()).unwrap();
    handle.await.unwrap().unwrap();
}
