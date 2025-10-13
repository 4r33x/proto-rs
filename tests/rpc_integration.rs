use std::collections::HashMap;
use std::pin::Pin;

use proto_rs::proto_rpc;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tonic::Request;
use tonic::Response;
use tonic::Status;

#[path = "common/encoding_messages.rs"]
mod encoding_messages;

use encoding_messages::{CollectionsMessage, CollectionsMessageProst, NestedMessage, NestedMessageProst, SampleEnum, SampleMessage, SampleMessageProst, sample_collections_messages, sample_message};

#[proto_rpc(rpc_package = "complex_rpc", rpc_server = true, rpc_client = true, proto_path = "protos/tests/complex_rpc.proto")]
#[proto_imports(encoding = ["SampleMessage", "CollectionsMessage", "NestedMessage"])]
pub trait ComplexService {
    type StreamCollectionsStream: Stream<Item = Result<CollectionsMessage, Status>> + Send;

    async fn echo_sample(&self, request: Request<SampleMessage>) -> Result<Response<SampleMessage>, Status>;

    async fn stream_collections(&self, request: Request<SampleMessage>) -> Result<Response<Self::StreamCollectionsStream>, Status>;
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

fn nested_to_tonic(nested: &NestedMessageProst) -> tonic_prost_test::encoding::NestedMessage {
    tonic_prost_test::encoding::NestedMessage { value: nested.value }
}

fn nested_from_tonic(nested: tonic_prost_test::encoding::NestedMessage) -> NestedMessageProst {
    NestedMessageProst { value: nested.value }
}

fn sample_to_tonic(msg: &SampleMessage) -> tonic_prost_test::encoding::SampleMessage {
    let prost = SampleMessageProst::from(msg);
    tonic_prost_test::encoding::SampleMessage {
        id: prost.id,
        flag: prost.flag,
        name: prost.name,
        data: prost.data.into_iter().map(u32::from).collect(),
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

    let data = data.into_iter().map(|value| u8::try_from(value).expect("value must fit in u8")).collect();

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

struct OurService;

#[tonic::async_trait]
impl ComplexService for OurService {
    type StreamCollectionsStream = Pin<Box<dyn Stream<Item = Result<CollectionsMessage, Status>> + Send>>;

    async fn echo_sample(&self, _request: Request<SampleMessage>) -> Result<Response<SampleMessage>, Status> {
        Ok(Response::new(response_message()))
    }

    async fn stream_collections(&self, _request: Request<SampleMessage>) -> Result<Response<Self::StreamCollectionsStream>, Status> {
        let stream = tokio_stream::iter(response_collections().into_iter().map(Ok));
        Ok(Response::new(Box::pin(stream)))
    }
}

struct ProstService;

#[tonic::async_trait]
impl tonic_prost_test::complex_rpc::complex_service_server::ComplexService for ProstService {
    type StreamCollectionsStream = Pin<Box<dyn Stream<Item = Result<tonic_prost_test::encoding::CollectionsMessage, Status>> + Send>>;

    async fn echo_sample(&self, _request: Request<tonic_prost_test::encoding::SampleMessage>) -> Result<Response<tonic_prost_test::encoding::SampleMessage>, Status> {
        Ok(Response::new(sample_to_tonic(&response_message())))
    }

    async fn stream_collections(&self, _request: Request<tonic_prost_test::encoding::SampleMessage>) -> Result<Response<Self::StreamCollectionsStream>, Status> {
        let items = response_collections().into_iter().map(|msg| Ok(collections_to_tonic(&msg)));
        Ok(Response::new(Box::pin(tokio_stream::iter(items))))
    }
}

async fn spawn_our_server() -> (std::net::SocketAddr, tokio::sync::oneshot::Sender<()>, tokio::task::JoinHandle<Result<(), tonic::transport::Error>>) {
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

async fn spawn_prost_server() -> (std::net::SocketAddr, tokio::sync::oneshot::Sender<()>, tokio::task::JoinHandle<Result<(), tonic::transport::Error>>) {
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::transport::Server;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let incoming = TcpListenerStream::new(listener);

    let handle = tokio::spawn(async move {
        Server::builder()
            .add_service(tonic_prost_test::complex_rpc::complex_service_server::ComplexServiceServer::new(ProstService))
            .serve_with_incoming_shutdown(incoming, async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    (addr, shutdown_tx, handle)
}

#[tokio::test(flavor = "multi_thread")]
async fn tonic_client_roundtrip_against_proto_server() {
    let (addr, shutdown, handle) = spawn_our_server().await;

    let mut client = tonic_prost_test::complex_rpc::complex_service_client::ComplexServiceClient::connect(format!("http://{}", addr))
        .await
        .unwrap();

    let request = sample_to_tonic(&request_message());
    let response = client.echo_sample(request.clone()).await.unwrap().into_inner();
    assert_eq!(sample_from_tonic(response), response_message());

    let mut stream = client.stream_collections(request).await.unwrap().into_inner();

    let mut received = Vec::new();
    while let Some(item) = stream.message().await.unwrap() {
        received.push(collections_from_tonic(item));
    }

    assert_eq!(received, response_collections());

    shutdown.send(()).unwrap();
    handle.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn proto_client_roundtrip_against_prost_server() {
    let (addr, shutdown, handle) = spawn_prost_server().await;

    let mut client = complex_service_client::ComplexServiceClient::connect(format!("http://{}", addr)).await.unwrap();

    let response = client.echo_sample(request_message()).await.unwrap().into_inner();
    assert_eq!(response, response_message());

    let mut stream = client.stream_collections(request_message()).await.unwrap().into_inner();

    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.unwrap());
    }

    assert_eq!(received, response_collections());

    shutdown.send(()).unwrap();
    handle.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn proto_client_roundtrip_against_proto_server() {
    let (addr, shutdown, handle) = spawn_our_server().await;

    let mut client = complex_service_client::ComplexServiceClient::connect(format!("http://{}", addr)).await.unwrap();

    let response = client.echo_sample(request_message()).await.unwrap().into_inner();
    assert_eq!(response, response_message());

    let mut stream = client.stream_collections(request_message()).await.unwrap().into_inner();

    let mut received = Vec::new();
    while let Some(item) = stream.next().await {
        received.push(item.unwrap());
    }

    assert_eq!(received, response_collections());

    shutdown.send(()).unwrap();
    handle.await.unwrap().unwrap();
}
