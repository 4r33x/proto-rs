#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::hint::black_box;
use std::io::Write;
use std::io::{self};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;

use bytes::BufMut;
use bytes::Bytes;
use chrono::Local;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use prost::Message as ProstMessage;
use proto_rs::ProtoExt;
use proto_rs::ToZeroCopyResponse;
use proto_rs::proto_message;

static BENCH_RECORDER: OnceLock<BenchRecorder> = OnceLock::new();

fn bench_recorder() -> &'static BenchRecorder {
    BENCH_RECORDER.get_or_init(BenchRecorder::new)
}

#[derive(Clone, Default)]
struct BenchAggregate {
    total_duration: Duration,
    iterations: u64,
    bytes: Option<u64>,
}

struct BenchRecorder {
    groups: Mutex<BTreeMap<String, BTreeMap<String, BenchAggregate>>>,
}

impl BenchRecorder {
    fn new() -> Self {
        Self { groups: Mutex::new(BTreeMap::new()) }
    }

    fn record(&self, group: &str, bench: &str, duration: Duration, iterations: u64, bytes: Option<u64>) {
        let mut groups = self.groups.lock().expect("bench recorder poisoned");
        let benchmarks = groups.entry(group.to_string()).or_default();
        let entry = benchmarks.entry(bench.to_string()).or_default();
        entry.total_duration += duration;
        entry.iterations += iterations;
        if let Some(bytes) = bytes {
            entry.bytes = Some(bytes);
        }
    }

    fn write_markdown(&self) -> io::Result<()> {
        const MIB: f64 = 1024.0 * 1024.0;

        let groups = self.groups.lock().map_err(|_| io::Error::other("bench recorder poisoned"))?;

        let base: PathBuf = env!("CARGO_MANIFEST_DIR").into();
        let path = base.join("benches").join("bench.md");

        // Open file in append mode (preserve history)
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;

        let now = Local::now();
        writeln!(file, "\n# Benchmark Run — {}\n", now.format("%Y-%m-%d %H:%M:%S"))?;
        writeln!(file, "| Group | Benchmark | Avg ns/op | Avg µs/op | MiB/s |")?;
        writeln!(file, "| --- | --- | ---: | ---: | ---: |")?;

        for (group_name, benchmarks) in groups.iter() {
            for (bench_name, aggregate) in benchmarks {
                if aggregate.iterations == 0 {
                    continue;
                }

                let avg_ns = aggregate.total_duration.as_nanos() as f64 / aggregate.iterations as f64;
                let avg_usecs = avg_ns / 1_000.0;
                let avg_sec = avg_ns / 1_000_000_000.0;

                let throughput = aggregate.bytes.map(|bytes| {
                    let mib = bytes as f64 / MIB;
                    if avg_sec > 0.0 { mib / avg_sec } else { 0.0 }
                });

                let throughput_display = throughput.map_or_else(|| "-".to_string(), |v| format!("{v:.2}"));

                writeln!(file, "| {group_name} | {bench_name} | {avg_ns:.2} | {avg_usecs:.2} | {throughput_display} |")?;
            }
        }

        writeln!(file)?;
        Ok(())
    }
}

impl Drop for BenchRecorder {
    fn drop(&mut self) {
        let _ = self.write_markdown();
    }
}

// ============================================================================
// Benchmarks
// ============================================================================

#[allow(clippy::too_many_lines)]
fn bench_encode_decode(c: &mut Criterion) {
    let message = sample_complex_root();
    let prost_message = &ComplexRootProst::from(&message);

    let proto_bytes = Bytes::from(ComplexRoot::encode_to_vec(&message));
    let proto_bytes_ref = ComplexRoot::encoded_len(&&message);
    assert_eq!(proto_bytes.len(), proto_bytes_ref);
    let prost_bytes = Bytes::from({
        let mut buf = Vec::with_capacity(prost_message.encoded_len());
        prost_message.encode(&mut buf).unwrap();
        buf
    });

    let mut group = c.benchmark_group("complex_root_encode_decode");
    group.throughput(Throughput::Bytes(proto_bytes.len() as u64));

    group.bench_function("proto_rs encode_to_vec", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                let encoded = ComplexRoot::encode_to_vec(black_box(&message));
                black_box(&encoded);
                total += start.elapsed();
            }
            bench_recorder().record("complex_root_encode", "proto_rs encode_to_vec", total, iters, Some(proto_bytes.len() as u64));

            total
        });
    });

    group.bench_function("prost encode_to_vec", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                let buf = prost_message.encode_to_vec();
                black_box(&buf);
                total += start.elapsed();
            }
            bench_recorder().record("complex_root_encode", "prost encode_to_vec", total, iters, Some(prost_bytes.len() as u64));

            total
        });
    });

    group.bench_function("proto_rs decode proto_rs input", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                let data = proto_bytes.clone();
                let decoded = ComplexRoot::decode(black_box(data)).expect("proto decode failed");
                black_box(decoded);
                total += start.elapsed();
            }
            bench_recorder().record("complex_root_decode", "proto_rs decode proto_rs input", total, iters, Some(proto_bytes.len() as u64));

            total
        });
    });

    group.bench_function("prost decode proto_rs input", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                let data = proto_bytes.clone();
                let decoded = ComplexRoot::decode(black_box(data)).expect("proto decode failed");
                black_box(decoded);
                total += start.elapsed();
            }
            bench_recorder().record("complex_root_decode", "prost decode proto_rs input", total, iters, Some(proto_bytes.len() as u64));

            total
        });
    });

    group.bench_function("proto_rs decode prost input", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                let data = prost_bytes.clone();
                let decoded = ComplexRoot::decode(black_box(data)).expect("proto decode failed");
                black_box(decoded);
                total += start.elapsed();
            }
            bench_recorder().record("complex_root_decode", "proto_rs decode prost input", total, iters, Some(prost_bytes.len() as u64));

            total
        });
    });

    group.bench_function("prost decode prost input", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                let data = prost_bytes.clone();
                let decoded = ComplexRoot::decode(black_box(data)).expect("proto decode failed");
                black_box(decoded);
                total += start.elapsed();
            }
            bench_recorder().record("complex_root_decode", "prost decode prost input", total, iters, Some(prost_bytes.len() as u64));

            total
        });
    });

    group.finish();
}

fn bench_zero_copy_vs_prost(c: &mut Criterion) {
    let message = sample_complex_root();
    let prost_message: ComplexRootProst = ComplexRootProst::from(&message);
    let prost_len = prost_message.encode_to_vec().len();
    let proto_len = ComplexRoot::encode_to_vec(&message).len();

    let mut group = c.benchmark_group("zero_copy_vs_prost");
    group.throughput(Throughput::Bytes(proto_len as u64));

    group.bench_function("prost clone + encode", |b| {
        let mut buf = Vec::with_capacity(prost_len);
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                buf.clear();
                let start = Instant::now();
                let cloned = prost_message.clone();
                cloned.encode(&mut buf).unwrap();
                black_box(&buf);
                total += start.elapsed();
            }
            bench_recorder().record("bench_zero_copy_vs_prost", "prost clone + encode", total, iters, Some(prost_len as u64));

            total
        });
    });

    group.bench_function("proto_rs zero_copy response", |b| {
        let mut buf = Vec::with_capacity(proto_len);
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                let zero_copy = (&message).to_zero_copy();
                buf.put_slice(zero_copy.into_response().get_ref().as_slice());
                black_box(&buf);
                total += start.elapsed();
            }
            bench_recorder().record("bench_zero_copy_vs_prost", "proto_rs zero_copy response", total, iters, Some(proto_len as u64));

            total
        });
    });

    group.finish();
}

criterion_group!(benches, bench_encode_decode, bench_zero_copy_vs_prost);
fn main() {
    benches();
    criterion::Criterion::default().configure_from_args().final_summary();
    let _ = bench_recorder().write_markdown();
}

// ============================================================================
// Types
// ============================================================================

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SimpleEnum {
    #[default]
    Alpha,
    Beta,
    Gamma,
    Delta,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct NestedLeaf {
    pub id: u64,
    pub name: String,
    pub active: bool,
    pub scores: Vec<i32>,
    pub payload: Bytes,
    pub attachments: Vec<Bytes>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ExtraDetails {
    pub description: String,
    pub counters: HashMap<String, u32>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DeepMessage {
    pub label: String,
    pub blob: Bytes,
    pub leaves: Vec<NestedLeaf>,
    pub leaf_lookup: HashMap<String, NestedLeaf>,
    pub simple_codes: Vec<SimpleEnum>,
    pub simple_lookup: HashMap<String, SimpleEnum>,
    pub focus: Option<Box<NestedLeaf>>,
    pub details: Option<Box<ExtraDetails>>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ComplexEnumEmpty;

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq)]
pub enum ComplexEnum {
    #[proto(tag = 1)]
    Leaf(NestedLeaf),
    #[proto(tag = 2)]
    Deep(DeepMessage),
    #[proto(tag = 3)]
    Details(ExtraDetails),
    #[proto(tag = 4)]
    Empty(ComplexEnumEmpty),
}

impl Default for ComplexEnum {
    fn default() -> Self {
        Self::Empty(ComplexEnumEmpty {})
    }
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ComplexRoot {
    pub id: String,
    pub payload: Bytes,
    pub leaves: Vec<NestedLeaf>,
    pub deep_list: Vec<DeepMessage>,
    pub leaf_lookup: HashMap<String, NestedLeaf>,
    pub deep_lookup: HashMap<String, DeepMessage>,
    pub status: ComplexEnum,
    pub status_history: Vec<ComplexEnum>,
    pub status_lookup: HashMap<String, ComplexEnum>,
    pub codes: Vec<SimpleEnum>,
    pub code_lookup: HashMap<String, SimpleEnum>,
    pub attachments: Vec<Bytes>,
    pub tags: Vec<String>,
    pub count: i64,
    pub ratio: f64,
    pub active: bool,
    pub big_numbers: Vec<u64>,
    pub audit_log: HashMap<String, DeepMessage>,
    pub primary_focus: Option<Box<NestedLeaf>>,
    pub secondary_focus: Option<Box<DeepMessage>>,
}

// ============================================================================
// Prost equivalents
// ============================================================================

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct NestedLeafProst {
    #[prost(uint64, tag = "1")]
    pub id: u64,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(bool, tag = "3")]
    pub active: bool,
    #[prost(int32, repeated, tag = "4")]
    pub scores: Vec<i32>,
    #[prost(bytes, tag = "5")]
    pub payload: Vec<u8>,
    #[prost(bytes, repeated, tag = "6")]
    pub attachments: Vec<Vec<u8>>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct ExtraDetailsProst {
    #[prost(string, tag = "1")]
    pub description: String,
    #[prost(map = "string, uint32", tag = "2")]
    pub counters: HashMap<String, u32>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct DeepMessageProst {
    #[prost(string, tag = "1")]
    pub label: String,
    #[prost(bytes, tag = "2")]
    pub blob: Vec<u8>,
    #[prost(message, repeated, tag = "3")]
    pub leaves: Vec<NestedLeafProst>,
    #[prost(map = "string, message", tag = "4")]
    pub leaf_lookup: HashMap<String, NestedLeafProst>,
    #[prost(enumeration = "SimpleEnumProst", repeated, tag = "5")]
    pub simple_codes: Vec<i32>,
    #[prost(map = "string, enumeration(SimpleEnumProst)", tag = "6")]
    pub simple_lookup: HashMap<String, i32>,
    #[prost(message, optional, boxed, tag = "7")]
    pub focus: Option<Box<NestedLeafProst>>,
    #[prost(message, optional, boxed, tag = "8")]
    pub details: Option<Box<ExtraDetailsProst>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum SimpleEnumProst {
    Alpha = 0,
    Beta = 1,
    Gamma = 2,
    Delta = 3,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct ComplexEnumEmptyProst {}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct ComplexEnumProst {
    #[prost(oneof = "complex_enum_prost::Kind", tags = "1, 2, 3, 4")]
    pub kind: Option<complex_enum_prost::Kind>,
}

pub mod complex_enum_prost {
    #[derive(Clone, PartialEq, prost::Oneof)]
    pub enum Kind {
        #[prost(message, tag = "1")]
        Leaf(super::NestedLeafProst),
        #[prost(message, tag = "2")]
        Deep(super::DeepMessageProst),
        #[prost(message, tag = "3")]
        Details(super::ExtraDetailsProst),
        #[prost(message, tag = "4")]
        Empty(super::ComplexEnumEmptyProst),
    }
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct ComplexRootProst {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(bytes, tag = "2")]
    pub payload: Vec<u8>,
    #[prost(message, repeated, tag = "3")]
    pub leaves: Vec<NestedLeafProst>,
    #[prost(message, repeated, tag = "4")]
    pub deep_list: Vec<DeepMessageProst>,
    #[prost(map = "string, message", tag = "5")]
    pub leaf_lookup: HashMap<String, NestedLeafProst>,
    #[prost(map = "string, message", tag = "6")]
    pub deep_lookup: HashMap<String, DeepMessageProst>,
    #[prost(message, optional, tag = "7")]
    pub status: Option<ComplexEnumProst>,
    #[prost(message, repeated, tag = "8")]
    pub status_history: Vec<ComplexEnumProst>,
    #[prost(map = "string, message", tag = "9")]
    pub status_lookup: HashMap<String, ComplexEnumProst>,
    #[prost(enumeration = "SimpleEnumProst", repeated, tag = "10")]
    pub codes: Vec<i32>,
    #[prost(map = "string, enumeration(SimpleEnumProst)", tag = "11")]
    pub code_lookup: HashMap<String, i32>,
    #[prost(bytes, repeated, tag = "12")]
    pub attachments: Vec<Vec<u8>>,
    #[prost(string, repeated, tag = "13")]
    pub tags: Vec<String>,
    #[prost(int64, tag = "14")]
    pub count: i64,
    #[prost(double, tag = "15")]
    pub ratio: f64,
    #[prost(bool, tag = "16")]
    pub active: bool,
    #[prost(uint64, repeated, tag = "17")]
    pub big_numbers: Vec<u64>,
    #[prost(map = "string, message", tag = "18")]
    pub audit_log: HashMap<String, DeepMessageProst>,
    #[prost(message, optional, boxed, tag = "19")]
    pub primary_focus: Option<Box<NestedLeafProst>>,
    #[prost(message, optional, boxed, tag = "20")]
    pub secondary_focus: Option<Box<DeepMessageProst>>,
}

// ============================================================================
// Sample data
// ============================================================================

fn sample_nested_leaf(id: u64, name: &str) -> NestedLeaf {
    NestedLeaf {
        id,
        name: name.to_string(),
        active: id.is_multiple_of(2),
        scores: vec![id as i32, (id * 2) as i32, (id * 3) as i32],
        payload: Bytes::from(vec![id as u8, (id + 1) as u8, (id + 2) as u8]),
        attachments: vec![Bytes::from(vec![1, 2, 3, id as u8]), Bytes::from(vec![4, 5, 6, (id + 1) as u8])],
    }
}

fn sample_deep_message(label: &str, base: u64) -> DeepMessage {
    let leaf_a = sample_nested_leaf(base, &format!("{label}-leaf-a"));
    let leaf_b = sample_nested_leaf(base + 1, &format!("{label}-leaf-b"));

    let mut leaf_lookup = HashMap::new();
    leaf_lookup.insert("primary".into(), leaf_a.clone());
    leaf_lookup.insert("secondary".into(), leaf_b.clone());

    let mut simple_lookup = HashMap::new();
    simple_lookup.insert("alpha".into(), SimpleEnum::Alpha);
    simple_lookup.insert("beta".into(), SimpleEnum::Beta);

    let mut counters = HashMap::new();
    counters.insert("observations".into(), (base * 3) as u32);
    counters.insert("warnings".into(), base as u32);

    DeepMessage {
        label: label.to_string(),
        blob: Bytes::from(vec![7, 8, 9, base as u8]),
        leaves: vec![leaf_a.clone(), leaf_b.clone()],
        leaf_lookup,
        simple_codes: vec![SimpleEnum::Alpha, SimpleEnum::Gamma, SimpleEnum::Delta],
        simple_lookup,
        focus: Some(Box::new(leaf_a)),
        details: Some(Box::new(ExtraDetails {
            description: format!("details for {label}"),
            counters,
        })),
    }
}

fn sample_complex_root() -> ComplexRoot {
    let main_leaf = sample_nested_leaf(42, "main");
    let aux_leaf = sample_nested_leaf(7, "aux");

    let deep_primary = sample_deep_message("primary", 100);
    let deep_secondary = sample_deep_message("secondary", 200);

    ComplexRoot {
        id: "complex-root".into(),
        payload: Bytes::from_static(b"complex-payload"),
        leaves: vec![main_leaf.clone(), aux_leaf.clone()],
        deep_list: vec![deep_primary.clone(), deep_secondary.clone()],
        leaf_lookup: HashMap::from([("main".into(), main_leaf.clone()), ("aux".into(), aux_leaf.clone())]),
        deep_lookup: HashMap::from([("primary".into(), deep_primary.clone()), ("secondary".into(), deep_secondary.clone())]),
        status: ComplexEnum::Details(ExtraDetails {
            description: "aggregated".into(),
            counters: HashMap::from([("total".into(), 5u32), ("errors".into(), 1u32)]),
        }),
        status_history: vec![ComplexEnum::Leaf(main_leaf.clone()), ComplexEnum::Deep(deep_secondary.clone()), ComplexEnum::Empty(ComplexEnumEmpty {})],
        status_lookup: HashMap::from([("ready".into(), ComplexEnum::Leaf(main_leaf.clone())), ("processing".into(), ComplexEnum::Deep(deep_primary.clone()))]),
        codes: vec![SimpleEnum::Alpha, SimpleEnum::Beta, SimpleEnum::Delta],
        code_lookup: HashMap::from([("alpha".into(), SimpleEnum::Alpha), ("gamma".into(), SimpleEnum::Gamma)]),
        attachments: vec![Bytes::from_static(b"attachment-a"), Bytes::from_static(b"attachment-b"), Bytes::from(vec![0, 1, 2, 3])],
        tags: vec!["primary".into(), "urgent".into(), "external".into()],
        count: 99,
        ratio: 0.875,
        active: true,
        big_numbers: vec![1_000_000, 2_000_000, 3_500_000],
        audit_log: HashMap::from([("initial".into(), deep_primary.clone()), ("update".into(), deep_secondary.clone())]),
        primary_focus: Some(Box::new(main_leaf)),
        secondary_focus: Some(Box::new(deep_secondary)),
    }
}

impl From<&SimpleEnum> for SimpleEnumProst {
    fn from(value: &SimpleEnum) -> Self {
        match value {
            SimpleEnum::Alpha => SimpleEnumProst::Alpha,
            SimpleEnum::Beta => SimpleEnumProst::Beta,
            SimpleEnum::Gamma => SimpleEnumProst::Gamma,
            SimpleEnum::Delta => SimpleEnumProst::Delta,
        }
    }
}

impl From<&NestedLeaf> for NestedLeafProst {
    fn from(value: &NestedLeaf) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            active: value.active,
            scores: value.scores.clone(),
            payload: value.payload.clone().to_vec(),
            attachments: value.attachments.iter().map(|b| b.clone().to_vec()).collect(),
        }
    }
}

impl From<&ExtraDetails> for ExtraDetailsProst {
    fn from(value: &ExtraDetails) -> Self {
        Self {
            description: value.description.clone(),
            counters: value.counters.clone(),
        }
    }
}

impl From<&DeepMessage> for DeepMessageProst {
    fn from(value: &DeepMessage) -> Self {
        Self {
            label: value.label.clone(),
            blob: value.blob.clone().to_vec(),
            leaves: value.leaves.iter().map(NestedLeafProst::from).collect(),
            leaf_lookup: value.leaf_lookup.iter().map(|(k, v)| (k.clone(), NestedLeafProst::from(v))).collect(),
            simple_codes: value.simple_codes.iter().map(|code| SimpleEnumProst::from(code) as i32).collect(),
            simple_lookup: value.simple_lookup.iter().map(|(k, v)| (k.clone(), SimpleEnumProst::from(v) as i32)).collect(),
            focus: value.focus.as_ref().map(|leaf| Box::new(NestedLeafProst::from(leaf.as_ref()))),
            details: value.details.as_ref().map(|details| Box::new(ExtraDetailsProst::from(details.as_ref()))),
        }
    }
}

impl From<&ComplexEnum> for ComplexEnumProst {
    fn from(value: &ComplexEnum) -> Self {
        use complex_enum_prost::Kind;

        let kind = match value {
            ComplexEnum::Leaf(leaf) => Some(Kind::Leaf(NestedLeafProst::from(leaf))),
            ComplexEnum::Deep(deep) => Some(Kind::Deep(DeepMessageProst::from(deep))),
            ComplexEnum::Details(details) => Some(Kind::Details(ExtraDetailsProst::from(details))),
            ComplexEnum::Empty(_) => Some(Kind::Empty(ComplexEnumEmptyProst {})),
        };

        Self { kind }
    }
}

impl From<&ComplexRoot> for ComplexRootProst {
    fn from(value: &ComplexRoot) -> Self {
        Self {
            id: value.id.clone(),
            payload: value.payload.clone().to_vec(),
            leaves: value.leaves.iter().map(NestedLeafProst::from).collect(),
            deep_list: value.deep_list.iter().map(DeepMessageProst::from).collect(),
            leaf_lookup: value.leaf_lookup.iter().map(|(k, v)| (k.clone(), NestedLeafProst::from(v))).collect(),
            deep_lookup: value.deep_lookup.iter().map(|(k, v)| (k.clone(), DeepMessageProst::from(v))).collect(),
            status: Some(ComplexEnumProst::from(&value.status)),
            status_history: value.status_history.iter().map(ComplexEnumProst::from).collect(),
            status_lookup: value.status_lookup.iter().map(|(k, v)| (k.clone(), ComplexEnumProst::from(v))).collect(),
            codes: value.codes.iter().map(|code| SimpleEnumProst::from(code) as i32).collect(),
            code_lookup: value.code_lookup.iter().map(|(k, v)| (k.clone(), SimpleEnumProst::from(v) as i32)).collect(),
            attachments: value.attachments.iter().map(|bytes| bytes.clone().to_vec()).collect(),
            tags: value.tags.clone(),
            count: value.count,
            ratio: value.ratio,
            active: value.active,
            big_numbers: value.big_numbers.clone(),
            audit_log: value.audit_log.iter().map(|(k, v)| (k.clone(), DeepMessageProst::from(v))).collect(),
            primary_focus: value.primary_focus.as_ref().map(|leaf| Box::new(NestedLeafProst::from(leaf.as_ref()))),
            secondary_focus: value.secondary_focus.as_ref().map(|deep| Box::new(DeepMessageProst::from(deep.as_ref()))),
        }
    }
}
