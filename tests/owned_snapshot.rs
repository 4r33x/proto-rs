#![cfg(feature = "tonic-owned")]
#![cfg_attr(all(not(feature = "stable"), feature = "tonic-transport"), feature(impl_trait_in_assoc_type))]

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use bytes::BufMut;
use bytes::Bytes;
use proto_rs::BytesMode;
use proto_rs::EncodedSnapshot;
use proto_rs::ProtoCodec;
use proto_rs::ProtoEncode;
use proto_rs::ProtoEncoder;
use proto_rs::proto_message;
use tonic::codec::Codec;
use tonic::codec::CompressionEncoding;
use tonic::codec::EncodeBody;
use tonic::codec::EncodeBuf;
use tonic::codec::EncodeResult;
use tonic::codec::Encoder;
use tonic::codec::OwnedMessage;
use tonic::codec::Streaming;
use tonic::codegen::Body;

#[proto_message]
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    id: u64,
    data: Vec<u8>,
}

#[cfg(feature = "tonic-transport")]
#[proto_rs::proto_rpc(rpc_package = "owned_snapshot", rpc_client = true, rpc_server = true)]
pub trait SnapshotEcho {
    async fn echo(
        &self,
        request: proto_rs::grpc::Request<Message>,
    ) -> Result<proto_rs::grpc::Response<proto_rs::EncodedSnapshot<Message>>, proto_rs::grpc::Status>;
}

#[cfg(feature = "tonic-transport")]
struct Echo;

#[cfg(feature = "tonic-transport")]
#[test]
fn automatic_transport_preserves_explicit_tls_and_endpoint_policies() {
    use tonic::transport::ClientTlsConfig;
    use tonic::transport::Endpoint;
    let ordinary = Endpoint::from_static("http://localhost:1234");
    assert!(ordinary.owned_transport_fallback_reason().is_none());
    assert!(ordinary.clone().timeout(std::time::Duration::from_secs(1)).owned_transport_fallback_reason().is_none());
    // Explicit TLS configuration is authoritative, even with an http URI.
    let encrypted = ordinary.tls_config(ClientTlsConfig::new()).unwrap();
    assert!(encrypted.owned_transport_fallback_reason().unwrap().contains("TLS"));
}
#[cfg(feature = "tonic-transport")]
impl SnapshotEcho for Echo {
    async fn echo(
        &self,
        request: proto_rs::grpc::Request<Message>,
    ) -> Result<proto_rs::grpc::Response<proto_rs::EncodedSnapshot<Message>>, proto_rs::grpc::Status> {
        Ok(proto_rs::grpc::Response::new(request.into_inner().to_encoded_snapshot()))
    }
}

#[cfg(feature = "tonic-transport")]
#[tokio::test]
async fn eager_snapshots_interoperate_over_plain_and_tls_with_gzip() {
    use tonic::transport::Certificate;
    use tonic::transport::ClientTlsConfig;
    use tonic::transport::Endpoint;
    use tonic::transport::Identity;
    use tonic::transport::Server;
    use tonic::transport::ServerTlsConfig;
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        for tls in [false, true] {
            for gzip in [false, true] {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                let address = listener.local_addr().unwrap();
                let mut server = Server::builder();
                let mut endpoint = Endpoint::from_shared(format!("{}://{address}", if tls { "https" } else { "http" })).unwrap();
                if tls {
                    let rcgen::CertifiedKey { cert, signing_key } = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
                    let certificate = cert.pem();
                    server = server
                        .tls_config(ServerTlsConfig::new().identity(Identity::from_pem(&certificate, signing_key.serialize_pem())))
                        .unwrap();
                    endpoint = endpoint
                        .tls_config(ClientTlsConfig::new().domain_name("localhost").ca_certificate(Certificate::from_pem(certificate)))
                        .unwrap();
                }
                let mut service = snapshot_echo_server::SnapshotEchoServer::new(Echo);
                if gzip {
                    service = service.accept_compressed(CompressionEncoding::Gzip).send_compressed(CompressionEncoding::Gzip);
                }
                let (shutdown, stopped) = tokio::sync::oneshot::channel();
                let task = tokio::spawn(async move {
                    server
                        .add_service(service)
                        .serve_with_incoming_shutdown(tokio_stream::wrappers::TcpListenerStream::new(listener), async {
                            let _ = stopped.await;
                        })
                        .await
                        .unwrap();
                });
                let channel = proto_rs::grpc::AutoChannel::connect(
                    endpoint,
                    proto_rs::grpc::ChannelOptions {
                        kernel_zero_copy: true,
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
                if tls {
                    assert!(channel.fallback_reason().unwrap().contains("TLS"));
                }
                let observer = channel.clone();
                let mut client = snapshot_echo_client::SnapshotEchoClient::new(channel);
                if gzip {
                    client = client.send_compressed(CompressionEncoding::Gzip).accept_compressed(CompressionEncoding::Gzip);
                }
                let message = Message {
                    id: 128,
                    data: vec![42; 256 * 1024],
                };
                assert_eq!(client.echo(message.to_encoded_snapshot()).await.unwrap().into_inner(), message);
                let expected = message.clone();
                let pending = client.echo(&message);
                drop(message);
                assert_eq!(pending.await.unwrap().into_inner(), expected);
                // Two client handles concurrently send the same immutable
                // snapshot over plaintext/TLS, with and without compression.
                let shared = expected.to_encoded_snapshot();
                let mut other_client = client.clone();
                let (first, second) = tokio::join!(client.echo(shared.clone()), other_client.echo(shared));
                assert_eq!(first.unwrap().into_inner(), expected);
                assert_eq!(second.unwrap().into_inner(), expected);
                drop(other_client);
                if tls {
                    assert_eq!(observer.fallback_reason(), Some("TLS requires the normal encrypted transport"));
                }
                drop(client);
                shutdown.send(()).unwrap();
                task.await.unwrap();
            }
        }
    })
    .await
    .unwrap();
}

fn data_frame<B: Body<Data = Bytes> + Unpin>(body: &mut B) -> Bytes
where
    B::Error: std::fmt::Debug,
{
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(result) = Pin::new(&mut *body).poll_frame(&mut cx) {
            return result.unwrap().unwrap().into_data().unwrap();
        }
    }
}

#[test]
fn snapshot_is_eager_and_tonic_preserves_its_allocation() {
    for size in [0, 1, 63, 64, 8192, 65536] {
        let mut value = Message {
            id: 7,
            data: vec![42; size],
        };
        let expected = value.encode_to_vec();
        let snapshot = value.to_encoded_snapshot();
        let pointer = snapshot.as_bytes().as_ptr();
        value.data.fill(99);
        assert_eq!(snapshot.as_bytes(), expected);
        let mut body = EncodeBody::new_client(
            ProtoEncoder::<EncodedSnapshot<Message>, BytesMode>::default(),
            tokio_stream::iter([Ok(snapshot)]),
            None,
            None,
        );
        let frame = data_frame(&mut body);
        assert_eq!(frame.as_ptr().wrapping_add(5), pointer);
        assert_eq!(&frame[5..], expected);
        assert_eq!(frame[0], 0);
        assert_eq!(u32::from_be_bytes(frame[1..5].try_into().unwrap()) as usize, expected.len());
    }
}

#[test]
fn fanout_shares_one_allocation_until_last_transport_slice_is_released() {
    let mut value = Message {
        id: 7,
        data: vec![42; 1024],
    };
    let expected = value.encode_to_vec();
    let snapshot = EncodedSnapshot::new(&value);
    let pointer = snapshot.as_bytes().as_ptr();
    value.data.fill(1);
    let frames: Vec<_> = (0..8)
        .map(|_| {
            let mut body = EncodeBody::new_client(
                ProtoEncoder::<proto_rs::EncodedSnapshot<Message>, BytesMode>::default(),
                tokio_stream::iter([Ok(snapshot.clone())]),
                None,
                None,
            );
            let frame = data_frame(&mut body);
            assert_eq!(frame.as_ptr().wrapping_add(5), pointer);
            assert_eq!(&frame[5..], expected);
            frame
        })
        .collect();
    let last_slice = frames[0].slice(7..);
    drop(snapshot);
    drop(frames);
    let other = EncodedSnapshot::new(&value);
    assert_ne!(other.as_bytes().as_ptr(), pointer);
    assert_eq!(last_slice.as_ref(), &expected[2..]);
    drop(other);
    std::thread::spawn(move || drop(last_slice)).join().unwrap();
    assert_tls_reuses(&value, pointer);
}

fn batch_values(mut data: &[u8]) -> Vec<u64> {
    use proto_rs::ProtoDecode;
    let mut values = Vec::new();
    while !data.is_empty() {
        assert_eq!(data[0], 0);
        let len = u32::from_be_bytes(data[1..5].try_into().unwrap()) as usize;
        values.push(u64::decode(&data[5..5 + len], proto_rs::DecodeContext::default()).unwrap());
        data = &data[5 + len..];
    }
    values
}

#[test]
fn ordinary_batches_flush_on_pending_and_preserve_source_errors() {
    use tokio_stream::Stream;
    struct Source(usize);
    impl Stream for Source {
        type Item = Result<u64, tonic::Status>;
        fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let step = self.0;
            self.0 += 1;
            match step {
                0 | 1 => Poll::Ready(Some(Ok(step as u64))),
                2 => Poll::Pending,
                3 => Poll::Ready(Some(Ok(128))),
                4 => Poll::Ready(Some(Err(tonic::Status::aborted("source stopped")))),
                _ => Poll::Ready(None),
            }
        }
    }
    let mut body = EncodeBody::new_client(ProtoEncoder::<u64, proto_rs::SunByRef>::default(), Source(0), None, None);
    assert_eq!(batch_values(&data_frame(&mut body)), [0, 1]);
    assert_eq!(batch_values(&data_frame(&mut body)), [128]);
    let Poll::Ready(Some(Err(error))) = Pin::new(&mut body).poll_frame(&mut Context::from_waker(Waker::noop())) else {
        panic!("missing source error")
    };
    assert_eq!(error.code(), tonic::Code::Aborted);
}

#[test]
fn ordinary_batches_bound_ready_empty_streams_and_check_individual_limits() {
    struct ReadyZeros(usize);
    impl tokio_stream::Stream for ReadyZeros {
        type Item = Result<u64, tonic::Status>;
        fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if self.0 == 0 {
                return Poll::Ready(None);
            }
            self.0 -= 1;
            Poll::Ready(Some(Ok(0)))
        }
    }
    let mut body = EncodeBody::new_client(ProtoEncoder::<u64, proto_rs::SunByRef>::default(), ReadyZeros(300), None, Some(0));
    for count in [128, 128, 44] {
        assert_eq!(batch_values(&data_frame(&mut body)), vec![0; count]);
    }
    assert!(matches!(
        Pin::new(&mut body).poll_frame(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(None)
    ));
    let mut body = EncodeBody::new_client(
        ProtoEncoder::<u64, proto_rs::SunByRef>::default(),
        tokio_stream::iter([Ok(0), Ok(u64::MAX)]),
        None,
        Some(1),
    );
    let Poll::Ready(Some(Err(error))) = Pin::new(&mut body).poll_frame(&mut Context::from_waker(Waker::noop())) else {
        panic!("batch bypassed individual message limit")
    };
    assert_eq!(error.code(), tonic::Code::OutOfRange);
}

#[test]
fn ordinary_batch_flushes_at_size_hint_threshold() {
    let source = tokio_stream::iter((0..3).map(|id| {
        Ok(Message {
            id,
            data: vec![42; 20_000],
        })
    }));
    let mut body = EncodeBody::new_client(ProtoEncoder::<Message, proto_rs::SunByRef>::default(), source, None, None);
    let first = data_frame(&mut body);
    let second = data_frame(&mut body);
    assert!(first.len() > 40_000 && first.len() < 41_000);
    assert!(second.len() > 20_000 && second.len() < 21_000);
}

#[test]
fn owned_batch_hook_rejects_malformed_frames() {
    struct BatchEncoder;
    impl Encoder for BatchEncoder {
        type Item = Bytes;
        type Error = tonic::Status;
        fn supports_owned(&self) -> bool {
            true
        }
        fn supports_owned_batch(&self) -> bool {
            true
        }
        fn encode(&mut self, _: Bytes, _: &mut EncodeBuf<'_>) -> Result<(), tonic::Status> {
            panic!("buffered hook used")
        }
        fn encode_owned_batch(&mut self, first: Bytes, _: &mut dyn FnMut() -> Option<Bytes>, _: usize) -> Result<Bytes, tonic::Status> {
            Ok(first)
        }
    }
    for malformed in [&[][..], &[0], &[1, 0, 0, 0, 0], &[0, 0, 0, 0, 1], &[0, 0, 0, 0, 0, 0]] {
        let mut body = EncodeBody::new_client(
            BatchEncoder,
            tokio_stream::iter([Ok(Bytes::copy_from_slice(malformed))]),
            None,
            None,
        );
        let Poll::Ready(Some(Err(error))) = Pin::new(&mut body).poll_frame(&mut Context::from_waker(Waker::noop())) else {
            panic!("malformed batch accepted")
        };
        assert_eq!(error.code(), tonic::Code::Internal);
    }
}

#[test]
fn switching_from_buffered_to_owned_batches_preserves_order() {
    struct Switching(bool);
    impl Encoder for Switching {
        type Item = Bytes;
        type Error = tonic::Status;
        fn supports_owned(&self) -> bool {
            true
        }
        fn supports_owned_batch(&self) -> bool {
            self.0
        }
        fn encode(&mut self, item: Bytes, dst: &mut EncodeBuf<'_>) -> Result<(), tonic::Status> {
            dst.put_slice(&item);
            self.0 = true;
            Ok(())
        }
        fn encode_owned_batch(&mut self, first: Bytes, next: &mut dyn FnMut() -> Option<Bytes>, _: usize) -> Result<Bytes, tonic::Status> {
            assert!(next().is_none());
            Ok(first)
        }
    }
    let batch = Bytes::from_static(&[0, 0, 0, 0, 1, 2]);
    let pointer = batch.as_ptr();
    let mut body = EncodeBody::new_client(
        Switching(false),
        tokio_stream::iter([Ok(Bytes::from_static(&[1])), Ok(batch), Err(tonic::Status::aborted("end"))]),
        None,
        None,
    );
    assert_eq!(&data_frame(&mut body)[..], &[0, 0, 0, 0, 1, 1]);
    assert_eq!(data_frame(&mut body).as_ptr(), pointer);
    let Poll::Ready(Some(Err(error))) = Pin::new(&mut body).poll_frame(&mut Context::from_waker(Waker::noop())) else {
        panic!("missing source error")
    };
    assert_eq!(error.code(), tonic::Code::Aborted);
}

#[test]
fn tls_pool_waits_for_the_final_frame_owner() {
    let value = Message {
        id: 1,
        data: vec![7; 1024],
    };
    let snapshot = EncodedSnapshot::new(&value);
    let pointer = snapshot.as_bytes().as_ptr();
    let mut body = EncodeBody::new_client(
        ProtoEncoder::<EncodedSnapshot<Message>, BytesMode>::default(),
        tokio_stream::iter([Ok(snapshot)]),
        None,
        None,
    );
    let frame = data_frame(&mut body);
    let retained = frame.slice(5..);
    drop(frame);
    drop(body);
    assert_eq!(retained.as_ref(), value.encode_to_vec());
    drop(retained);
    assert_tls_reuses(&value, pointer);
    let second = EncodedSnapshot::new(&value);
    drop(second);
    drop(EncodedSnapshot::new(&Message {
        id: 1,
        data: vec![9; 70000],
    }));
}

#[test]
fn cancelling_unpolled_body_returns_snapshot_to_pool() {
    let snapshot = EncodedSnapshot::new(&Message { id: 1, data: vec![1; 100] });
    let pointer = snapshot.as_bytes().as_ptr();
    let body = EncodeBody::new_client(
        ProtoEncoder::<EncodedSnapshot<Message>, BytesMode>::default(),
        tokio_stream::iter([Ok::<_, tonic::Status>(snapshot)]),
        None,
        None,
    );
    drop(body);
    assert_tls_reuses(&Message { id: 1, data: vec![1; 100] }, pointer);
}

#[test]
fn reused_buffer_grows_without_retaining_stale_payload() {
    drop(EncodedSnapshot::new(&Message { id: 1, data: vec![1; 100] }));
    drop(EncodedSnapshot::new(&Message {
        id: 2,
        data: vec![2; 32768],
    }));
    let expected = Message { id: 3, data: vec![3; 10] };
    let reused = EncodedSnapshot::new(&expected);
    assert_eq!(reused.as_bytes(), expected.encode_to_vec());
}

#[test]
fn compressed_snapshot_send_limit_matches_buffered_policy() {
    let value = Message {
        id: 1,
        data: vec![42; 10000],
    };
    let mut body = EncodeBody::new_client(
        ProtoEncoder::<EncodedSnapshot<Message>, BytesMode>::default(),
        tokio_stream::iter([Ok(value.to_encoded_snapshot())]),
        Some(CompressionEncoding::Gzip),
        Some(1),
    );
    let Poll::Ready(Some(Err(error))) = Pin::new(&mut body).poll_frame(&mut Context::from_waker(Waker::noop())) else {
        panic!("compressed limit ignored")
    };
    assert_eq!(error.code(), tonic::Code::OutOfRange);
    // Tonic's limit applies to the encoded (compressed) message size.
    let mut body = EncodeBody::new_client(
        ProtoEncoder::<EncodedSnapshot<Message>, BytesMode>::default(),
        tokio_stream::iter([Ok(value.to_encoded_snapshot())]),
        Some(CompressionEncoding::Gzip),
        Some(1024),
    );
    assert_eq!(data_frame(&mut body)[0], 1);
}

#[tokio::test]
async fn snapshots_roundtrip_with_gzip_and_without_compression() {
    for compression in [None, Some(CompressionEncoding::Gzip)] {
        let values: Vec<_> = [0, 1, 8192, 200_000, 7]
            .into_iter()
            .enumerate()
            .map(|(i, n)| Message {
                id: i as u64,
                data: vec![42; n],
            })
            .collect();
        let snapshots = values.iter().map(ProtoEncode::to_encoded_snapshot).map(Ok).collect::<Vec<_>>();
        let body = EncodeBody::new_client(
            ProtoEncoder::<EncodedSnapshot<Message>, BytesMode>::default(),
            tokio_stream::iter(snapshots),
            compression,
            None,
        );
        let mut decoded = Streaming::new_request(ProtoCodec::<(), Message>::default().decoder(), body, compression, None);
        for value in values {
            assert_eq!(decoded.message().await.unwrap(), Some(value));
        }
        assert!(decoded.message().await.unwrap().is_none());
    }
}

enum Mixed {
    Buffered(Vec<u8>),
    Owned(Bytes),
}
struct MixedEncoder;
impl Encoder for MixedEncoder {
    fn supports_owned(&self) -> bool {
        true
    }
    type Item = Mixed;
    type Error = tonic::Status;
    fn encode(&mut self, item: Mixed, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        match item {
            Mixed::Buffered(bytes) => dst.put_slice(&bytes),
            Mixed::Owned(_) => panic!("owned payload copied"),
        }
        Ok(())
    }
    fn encode_owned(&mut self, item: Mixed) -> Result<EncodeResult<Mixed>, Self::Error> {
        match item {
            Mixed::Owned(frame) => OwnedMessage::from_uncompressed_frame(frame).map(EncodeResult::Owned),
            item @ Mixed::Buffered(_) => Ok(EncodeResult::Buffered(item)),
        }
    }
}

#[test]
fn mixed_buffered_owned_and_error_order_is_preserved() {
    let owned = Bytes::from_static(&[0, 0, 0, 0, 1, 2]);
    let pointer = owned.as_ptr();
    let source = tokio_stream::iter([
        Ok(Mixed::Buffered(vec![1])),
        Ok(Mixed::Owned(owned)),
        Ok(Mixed::Buffered(vec![3])),
        Err(tonic::Status::aborted("end")),
    ]);
    let mut body = EncodeBody::new_client(MixedEncoder, source, None, None);
    assert_eq!(&data_frame(&mut body)[..], &[0, 0, 0, 0, 1, 1]);
    let frame = data_frame(&mut body);
    assert_eq!(frame.as_ptr(), pointer);
    assert_eq!(&frame[..], &[0, 0, 0, 0, 1, 2]);
    assert_eq!(&data_frame(&mut body)[..], &[0, 0, 0, 0, 1, 3]);
    let Poll::Ready(Some(Err(error))) = Pin::new(&mut body).poll_frame(&mut Context::from_waker(Waker::noop())) else {
        panic!("missing terminal error")
    };
    assert_eq!(error.code(), tonic::Code::Aborted);
}

#[test]
fn owned_message_limits_and_header_validation_are_enforced() {
    for invalid in [vec![], vec![0; 4], vec![1, 0, 0, 0, 0], vec![0, 0, 0, 0, 1]] {
        assert!(OwnedMessage::from_uncompressed_frame(Bytes::from(invalid)).is_err());
    }
    let snapshot = EncodedSnapshot::new(&Message {
        id: 1,
        data: vec![42; 128],
    });
    let mut body = EncodeBody::new_client(
        ProtoEncoder::<EncodedSnapshot<Message>, BytesMode>::default(),
        tokio_stream::iter([Ok(snapshot)]),
        None,
        Some(8),
    );
    let Poll::Ready(Some(Err(error))) = Pin::new(&mut body).poll_frame(&mut Context::from_waker(Waker::noop())) else {
        panic!("oversized owned message accepted")
    };
    assert_eq!(error.code(), tonic::Code::OutOfRange);
}

#[test]
fn owned_server_trailers_and_compression_override_are_preserved() {
    use tonic::codec::SingleMessageCompressionOverride;
    let value = Message { id: 3, data: vec![8; 100] };
    let snapshot = value.to_encoded_snapshot();
    let pointer = snapshot.as_bytes().as_ptr();
    let mut body = EncodeBody::new_server(
        ProtoEncoder::<EncodedSnapshot<Message>, BytesMode>::default(),
        tokio_stream::iter([Ok(snapshot)]),
        Some(CompressionEncoding::Gzip),
        SingleMessageCompressionOverride::Disable,
        None,
    );
    let frame = data_frame(&mut body);
    assert_eq!(frame[0], 0);
    assert_eq!(frame.as_ptr().wrapping_add(5), pointer);
    let Poll::Ready(Some(Ok(frame))) = Pin::new(&mut body).poll_frame(&mut Context::from_waker(Waker::noop())) else {
        panic!("missing server trailers")
    };
    assert_eq!(frame.into_trailers().unwrap()["grpc-status"], "0");
}

// Keep all candidates alive so every TLS slot is examined, regardless of the
// preferred mask cursor. Overflow allocations cannot replace a leased slot.
fn assert_tls_reuses(value: &Message, pointer: *const u8) {
    let candidates: Vec<_> = (0..proto_rs::EncodePoolConfig::default().max_buffers).map(|_| EncodedSnapshot::new(value)).collect();
    assert!(candidates.iter().any(|s| s.as_bytes().as_ptr() == pointer));
}
