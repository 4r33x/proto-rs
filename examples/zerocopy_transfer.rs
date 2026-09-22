//! Two-host validation, not a claim that localhost sends avoid copying.
#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]

#[cfg(target_os = "linux")]
mod app {
    use std::sync::Arc;
    use std::time::Duration;
    use std::time::Instant;

    use proto_rs::grpc::Request;
    use proto_rs::grpc::Response;
    use proto_rs::grpc::Status;
    use proto_rs::grpc::zerocopy::ZeroCopyChannel;
    use proto_rs::grpc::zerocopy::ZeroCopyConfig;
    use proto_rs::grpc::zerocopy::ZeroCopyIo;
    use proto_rs::grpc::zerocopy::serve_connection;
    use proto_rs::proto_message;
    use proto_rs::proto_rpc;

    const SIZE: usize = 64 * 1024 * 1024;
    const LIMIT: usize = 128 * 1024 * 1024;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    #[proto_message]
    #[derive(Clone, Debug, PartialEq)]
    pub struct BytesBench {
        a: u64,
        b: u64,
        c: Option<u64>,
        d: Arc<Vec<u8>>,
    }

    #[proto_rpc(rpc_package = "zerocopy_transfer", rpc_client = true, rpc_server = true)]
    pub trait Transfer {
        async fn echo(&self, request: Request<Vec<BytesBench>>) -> Result<Response<Vec<BytesBench>>, Status>;
    }
    struct Echo;
    impl Transfer for Echo {
        async fn echo(&self, request: Request<Vec<BytesBench>>) -> Result<Response<Vec<BytesBench>>, Status> {
            Ok(Response::new(request.into_inner()))
        }
    }

    fn fixture() -> Arc<Vec<BytesBench>> {
        let mut state = 0x1234_5678_9abc_def0u64;
        Arc::new(
            (0..20)
                .map(|i| {
                    let len = SIZE / 20 + usize::from(i < SIZE % 20);
                    let mut bytes = vec![0; len];
                    for chunk in bytes.chunks_mut(8) {
                        state ^= state << 13;
                        state ^= state >> 7;
                        state ^= state << 17;
                        chunk.copy_from_slice(&state.to_le_bytes()[..chunk.len()]);
                    }
                    BytesBench {
                        a: i as u64,
                        b: u64::MAX - i as u64,
                        c: if i % 2 == 0 { None } else { Some(0) },
                        d: Arc::new(bytes),
                    }
                })
                .collect(),
        )
    }

    pub async fn run() -> Result<(), Error> {
        let args: Vec<_> = std::env::args().skip(1).collect();
        let Some(mode) = args.first() else {
            return Err("usage: zerocopy_transfer server IP:PORT | client http://IP:PORT [--gzip] [--force]".into());
        };
        let address = args.get(1).ok_or("missing address")?;
        let gzip = args.iter().any(|arg| arg == "--gzip");
        let force = args.iter().any(|arg| arg == "--force");
        let config = ZeroCopyConfig {
            allow_fallback: !force,
            adaptive_fallback: !force,
            ..Default::default()
        };
        if mode == "server" {
            let listener = tokio::net::TcpListener::bind(address).await?;
            println!("plain HTTP/2 listening on {}; no TLS", listener.local_addr()?);
            loop {
                let (io, peer) = ZeroCopyIo::accept(&listener, config).await?;
                let metrics = io.metrics();
                let mut service = transfer_server::TransferServer::new(Echo)
                    .max_decoding_message_size(LIMIT)
                    .max_encoding_message_size(LIMIT)
                    .with_max_encode_preallocation(LIMIT);
                if gzip {
                    service = service
                        .accept_compressed(tonic::codec::CompressionEncoding::Gzip)
                        .send_compressed(tonic::codec::CompressionEncoding::Gzip);
                }
                let result = serve_connection(io, service).await;
                let drained = tokio::time::timeout(Duration::from_secs(30), metrics.wait_for_idle()).await;
                println!("peer={peer} result={result:?} drained={drained:?} stats={:?}", metrics.snapshot());
            }
        } else if mode == "client" {
            let channel = ZeroCopyChannel::connect(address.parse()?, config).await?;
            let metrics = channel.metrics();
            let mut client = transfer_client::TransferClient::new(channel)
                .max_decoding_message_size(LIMIT)
                .max_encoding_message_size(LIMIT)
                .with_max_encode_preallocation(LIMIT);
            if gzip {
                client = client
                    .send_compressed(tonic::codec::CompressionEncoding::Gzip)
                    .accept_compressed(tonic::codec::CompressionEncoding::Gzip);
            }
            let batch = fixture();
            for _ in 0..3 {
                // Clone only the 20 records/Arc handles, never their byte payloads.
                let request = batch.as_ref().clone();
                let start = Instant::now();
                let echoed = tokio::time::timeout(Duration::from_secs(120), client.echo(request)).await??.into_inner();
                let elapsed = start.elapsed();
                assert_eq!(echoed, *batch);
                tokio::time::timeout(Duration::from_secs(30), metrics.wait_for_idle()).await??;
                println!("64 MiB echo verified, RPC={elapsed:?}, stats={:?}", metrics.snapshot());
            }
            Ok(())
        } else {
            Err("expected server or client".into())
        }
    }
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    app::run().await
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("linux-zerocopy requires Linux");
}
