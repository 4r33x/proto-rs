//! Exercise the actual emitted schemas using protoc's independent C++ codec.
#![cfg(feature = "build-schemas")]
#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;

use proto_rs::DecodeContext;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::ProtoExt;
use proto_rs::grpc::Response;
use proto_rs::proto_message;
use proto_rs::proto_rpc;

#[proto_message(proto_path = "interop.proto")]
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Item {
    pub id: u64,
    pub label: String,
}

#[proto_message(proto_path = "interop.proto")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Off,
    On,
}

type Maybe<T> = Option<T>;
type List<T> = Vec<T>;

#[proto_message(proto_path = "interop.proto")]
pub struct Aliases {
    pub maybe: Maybe<Item>,
    pub list: List<u32>,
    pub bytes: List<u8>,
}

#[proto_rpc(rpc_package = "interop", proto_path = "interop.proto", rpc_server = true, rpc_client = true)]
pub trait Interop {
    async fn scalar(&self, request: Request<u32>) -> Result<Response<u32>, Status>;
    async fn integer(&self, request: Request<i64>) -> Result<Response<i64>, Status>;
    async fn boolean(&self, request: Request<bool>) -> Result<Response<bool>, Status>;
    async fn double(&self, request: Request<f64>) -> Result<Response<f64>, Status>;
    async fn buffer(&self, request: Request<proto_rs::bytes::Bytes>) -> Result<Response<proto_rs::bytes::Bytes>, Status>;
    async fn text(&self, request: Request<String>) -> Result<Response<String>, Status>;
    async fn mode(&self, request: Request<Mode>) -> Result<Response<Mode>, Status>;
    async fn boxed(&self, request: Request<Box<Item>>) -> Result<Response<Box<Item>>, Status>;
    async fn shared(&self, request: Request<Arc<Item>>) -> Result<Response<Arc<Item>>, Status>;
    async fn optional(&self, request: Request<Option<Item>>) -> Result<Response<Option<Item>>, Status>;
    async fn optional_scalar(&self, request: Request<Option<u32>>) -> Result<Response<Option<u32>>, Status>;
    async fn locked(&self, request: Request<Mutex<Item>>) -> Result<Response<Mutex<Item>>, Status>;
    async fn list(&self, request: Request<Vec<Item>>) -> Result<Response<Vec<Item>>, Status>;
    async fn numbers(&self, request: Request<Vec<u32>>) -> Result<Response<Vec<u32>>, Status>;
    async fn bytes(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Status>;
    async fn deque_bytes(&self, request: Request<VecDeque<u8>>) -> Result<Response<VecDeque<u8>>, Status>;
    async fn array_bytes(&self, request: Request<[u8; 3]>) -> Result<Response<[u8; 3]>, Status>;
    async fn array_numbers(&self, request: Request<[u32; 3]>) -> Result<Response<[u32; 3]>, Status>;
    async fn tree_set(&self, request: Request<BTreeSet<u8>>) -> Result<Response<BTreeSet<u8>>, Status>;
    async fn hash_set(&self, request: Request<HashSet<u8>>) -> Result<Response<HashSet<u8>>, Status>;
    async fn tree_map(&self, request: Request<BTreeMap<u32, Item>>) -> Result<Response<BTreeMap<u32, Item>>, Status>;
    async fn hash_map(&self, request: Request<HashMap<u32, Item>>) -> Result<Response<HashMap<u32, Item>>, Status>;
}

struct Schema {
    dir: PathBuf,
}

#[cfg(feature = "arc_swap")]
#[proto_rpc(rpc_package = "interop", proto_path = "interop.proto", rpc_server = true, rpc_client = true)]
pub trait AtomicInterop {
    async fn shared(&self, request: Request<arc_swap::ArcSwap<Item>>) -> Result<Response<arc_swap::ArcSwap<Item>>, Status>;
    async fn optional(&self, request: Request<arc_swap::ArcSwapOption<Item>>) -> Result<Response<arc_swap::ArcSwapOption<Item>>, Status>;
    async fn scalar(&self, request: Request<arc_swap::ArcSwapOption<u32>>) -> Result<Response<arc_swap::ArcSwapOption<u32>>, Status>;
}

#[cfg(feature = "cache_padded")]
#[proto_rpc(rpc_package = "interop", proto_path = "interop.proto", rpc_server = true, rpc_client = true)]
pub trait PaddedInterop {
    async fn padded(
        &self,
        request: Request<crossbeam_utils::CachePadded<Item>>,
    ) -> Result<Response<crossbeam_utils::CachePadded<Item>>, Status>;
}

impl Schema {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("proto-rs-protoc-{}-{serial}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let schema = Self { dir };
        let path = schema.dir.join("interop.proto");
        proto_rs::schemas::write_only_these(
            &[("interop.proto", path.to_str().unwrap())],
            &proto_rs::schemas::RustClientCtx::disabled(),
        )
        .unwrap();
        let output = schema
            .command()
            .arg(format!("--descriptor_set_out={}", schema.dir.join("schema.pb").display()))
            .output()
            .expect("protoc is required for schema interoperability tests");
        assert!(
            output.status.success(),
            "protoc rejected emitted schema:\n{}\n{}",
            String::from_utf8_lossy(&output.stderr),
            std::fs::read_to_string(path).unwrap()
        );
        schema
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(std::env::var_os("PROTOC").unwrap_or_else(|| "protoc".into()));
        cmd.arg(format!("--proto_path={}", self.dir.display())).arg("interop.proto");
        cmd
    }

    fn codec(&self, operation: &str, name: &str, input: &[u8]) -> Vec<u8> {
        let mut child = self
            .command()
            .arg(format!("--{operation}=interop.{name}"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "protoc {operation} {name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    fn check<T: ProtoEncode + ProtoDecode + ProtoExt>(&self, name: &str, value: &T, text: &str) {
        let expected = self.codec("encode", name, text.as_bytes());
        let actual = value.encode_to_vec();
        assert_eq!(actual, expected, "encoding {name}");
        let decoded = T::decode(expected.as_slice(), DecodeContext::default()).unwrap();
        assert_eq!(decoded.encode_to_vec(), expected, "decoding {name}");
        assert_eq!(self.codec("decode", name, &actual), self.codec("decode", name, &expected));
    }
}

impl Drop for Schema {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}

fn item() -> Item {
    Item {
        id: 150,
        label: "hello".into(),
    }
}

#[test]
fn emitted_schema_and_wire_format_agree_with_protoc() {
    let schema = Schema::new();
    schema.check("Item", &item(), "id: 150 label: 'hello'");
    schema.check("Uint32Value", &150u32, "value: 150");
    schema.check("Uint32Value", &0u32, "");
    schema.check("Int64Value", &-150i64, "value: -150");
    schema.check("BoolValue", &true, "value: true");
    schema.check("DoubleValue", &1.5f64, "value: 1.5");
    schema.check(
        "BytesValue",
        &proto_rs::bytes::Bytes::from_static(&[0, 128, 255]),
        "value: '\\000\\200\\377'",
    );
    schema.check("StringValue", &"hello".to_string(), "value: 'hello'");
    schema.check("ModeValue", &Mode::On, "value: MODE_ON");
    schema.check("BoxItem", &Box::new(item()), "value { id: 150 label: 'hello' }");
    schema.check("ArcItem", &Arc::new(item()), "value { id: 150 label: 'hello' }");
    schema.check("MutexItem", &Mutex::new(item()), "value { id: 150 label: 'hello' }");
    schema.check("OptionItem", &Some(item()), "value { id: 150 label: 'hello' }");
    schema.check(
        "OptionItem",
        &Some(Item {
            id: 0,
            label: String::new(),
        }),
        "value {}",
    );
    schema.check("OptionItem", &None::<Item>, "");
    schema.check("OptionU32", &Some(0u32), "value: 0");
    schema.check("OptionU32", &None::<u32>, "");
    schema.check("VecItem", &vec![item()], "value { id: 150 label: 'hello' }");
    schema.check("VecU32", &vec![0u32, 128, 255], "value: [0, 128, 255]");
    schema.check("VecU32", &[0u32, 128, 255], "value: [0, 128, 255]");
    schema.check("VecBytes", &[0u8, 128, 255], "value: '\\000\\200\\377'");
    schema.check("VecBytes", &vec![0u8, 128, 255], "value: '\\000\\200\\377'");
    schema.check("VecDequeBytes", &VecDeque::from([0u8, 128, 255]), "value: '\\000\\200\\377'");
    schema.check("BTreeSetU32", &BTreeSet::from([0u8, 128, 255]), "value: [0, 128, 255]");
    schema.check("HashSetU32", &HashSet::from([255u8]), "value: 255");
    schema.check(
        "BTreeMapU32Item",
        &BTreeMap::from([(1u32, item())]),
        "value { key: 1 value { id: 150 label: 'hello' } }",
    );
    schema.check(
        "HashMapU32Item",
        &HashMap::from([(1u32, item())]),
        "value { key: 1 value { id: 150 label: 'hello' } }",
    );
    schema.check(
        "Aliases",
        &Aliases {
            maybe: Some(item()),
            list: vec![128, 255],
            bytes: vec![128, 255],
        },
        "maybe { id: 150 label: 'hello' } list: [128, 255] bytes: '\\200\\377'",
    );
}

#[test]
fn feature_wrappers_match_protoc() {
    let schema = Schema::new();
    #[cfg(feature = "parking_lot")]
    schema.check("MutexItem", &parking_lot::Mutex::new(item()), "value { id: 150 label: 'hello' }");
    #[cfg(feature = "cache_padded")]
    schema.check(
        "CachePaddedItem",
        &crossbeam_utils::CachePadded::new(item()),
        "value { id: 150 label: 'hello' }",
    );
    #[cfg(feature = "arc_swap")]
    {
        schema.check(
            "ArcSwapItem",
            &arc_swap::ArcSwap::from_pointee(item()),
            "value { id: 150 label: 'hello' }",
        );
        schema.check(
            "ArcSwapOptionItem",
            &arc_swap::ArcSwapOption::<Item>::from_pointee(Some(item())),
            "value { id: 150 label: 'hello' }",
        );
        schema.check("ArcSwapOptionItem", &arc_swap::ArcSwapOption::<Item>::from_pointee(None), "");
        schema.check(
            "ArcSwapOptionU32",
            &arc_swap::ArcSwapOption::<u32>::from_pointee(Some(0)),
            "value: 0",
        );
        // Multiple occurrences of the message-valued wrapper must merge.
        let first = schema.codec("encode", "ArcSwapItem", b"value { id: 150 }");
        let second = schema.codec("encode", "ArcSwapItem", b"value { label: 'hello' }");
        let merged = arc_swap::ArcSwap::<Item>::decode([first, second].concat().as_slice(), DecodeContext::default()).unwrap();
        assert_eq!(**merged.load(), item());
    }
    let _ = schema;
}

#[tokio::test]
async fn grpc_response_dereferencing_matches_the_schema() {
    let schema = Schema::new();
    let response = proto_rs::grpc::encode_unary_response::<_, Item>(Response::new(Box::new(item()))).unwrap();
    let bytes = response.into_inner().next().await.unwrap().unwrap();
    assert_eq!(bytes.as_ref(), schema.codec("encode", "Item", b"id: 150 label: 'hello'"));
    let response = proto_rs::grpc::encode_unary_response::<_, Item>(Response::new(Arc::new(item()))).unwrap();
    let bytes = response.into_inner().next().await.unwrap().unwrap();
    assert_eq!(bytes.as_ref(), schema.codec("encode", "Item", b"id: 150 label: 'hello'"));
}
