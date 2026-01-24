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
use chrono::Utc;
use criterion::BenchmarkGroup;
use criterion::Criterion;
use criterion::Throughput;
use criterion::measurement::WallTime;
use prost::Message as ProstMessage;
use proto_rs::DecodeContext;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
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
        Self {
            groups: Mutex::new(BTreeMap::new()),
        }
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
        use std::fmt::Write as _;
        const TOL: f64 = 0.01;
        const MIB: f64 = 1024.0 * 1024.0;

        // ---------------------------------------------------------------------
        // Simplified baseline mapping: proto_rs bench -> prost bench
        // ---------------------------------------------------------------------
        fn baseline_for(bench_name: &str) -> Option<String> {
            // Special case mappings where names don't follow the standard pattern
            const SPECIAL_CASES: &[(&str, &str)] = &[("proto_rs | zero_copy", "prost | clone + encode")];

            for (proto_name, prost_name) in SPECIAL_CASES {
                if bench_name == *proto_name {
                    return Some(prost_name.to_string());
                }
            }
            if bench_name.contains("proto_rs") {
                Some(bench_name.replacen("proto_rs", "prost", 1))
            } else {
                None
            }
        }

        let groups = self.groups.lock().map_err(|_| io::Error::other("bench recorder poisoned"))?;

        let base: PathBuf = env!("CARGO_MANIFEST_DIR").into();
        let path = base.parent().map(|p| p.join("bench.md")).unwrap_or_else(|| PathBuf::from("../bench.md"));

        // ---------------------------------------------------------------------
        // Build markdown section
        // ---------------------------------------------------------------------
        let mut buffer = String::new();
        let now = Utc::now();

        writeln!(&mut buffer, "\n# Benchmark Run — {}\n", now.format("%Y-%m-%d %H:%M:%S")).map_err(io::Error::other)?;
        writeln!(&mut buffer, "| Group | Benchmark | Impl | Ops / s | MiB/s | Speedup vs Prost |").map_err(io::Error::other)?;
        writeln!(&mut buffer, "| --- | --- | --- | ---: | ---: | ---: |").map_err(io::Error::other)?;

        // Index avg_us for all benchmarks
        let mut avg_index: HashMap<(String, String), f64> = HashMap::new();

        for (group_name, benchmarks) in groups.iter() {
            for (bench_name, aggregate) in benchmarks {
                if aggregate.iterations == 0 {
                    continue;
                }
                let avg_us = aggregate.total_duration.as_micros() as f64 / aggregate.iterations as f64;
                avg_index.insert((group_name.clone(), bench_name.clone()), avg_us);
            }
        }

        // ---------------------------------------------------------------------
        // Render rows
        // ---------------------------------------------------------------------
        for (group_name, benchmarks) in groups.iter() {
            for (bench_name, aggregate) in benchmarks {
                if aggregate.iterations == 0 {
                    continue;
                }

                let avg_us = aggregate.total_duration.as_micros() as f64 / aggregate.iterations as f64;
                let avg_sec = avg_us / 1_000_000.0;

                // Ops per second (bigger, more intuitive numbers)
                let ops_per_sec = if avg_sec > 0.0 { 1.0 / avg_sec } else { 0.0 };

                // Throughput in MiB/s
                let throughput_display = aggregate.bytes.map_or_else(
                    || "-".to_string(),
                    |bytes| {
                        if avg_sec > 0.0 {
                            let mib = bytes as f64 / MIB;
                            format!("{:.2}", mib / avg_sec)
                        } else {
                            "-".to_string()
                        }
                    },
                );

                // Find prost baseline
                let baseline_avg = if let Some(baseline_name) = baseline_for(bench_name) {
                    avg_index.get(&(group_name.clone(), baseline_name)).copied().or(Some(avg_us))
                } else {
                    Some(avg_us)
                };

                // Relative speedup
                let rel_display = if let Some(base_avg_us) = baseline_avg {
                    if base_avg_us > 0.0 {
                        let ratio = base_avg_us / avg_us; // prost / ours
                        if ratio > 1.0 + TOL {
                            format!("{ratio:.2}× faster")
                        } else if ratio < 1.0 - TOL {
                            format!("{ratio:.2}× slower")
                        } else {
                            "1.00×".to_string()
                        }
                    } else {
                        "-".to_string()
                    }
                } else {
                    "-".to_string()
                };

                // Split benchmark name to extract implementation and clean name
                let (clean_bench_name, impl_name) = if let Some(idx) = bench_name.find(" | ") {
                    let (name_part, impl_part) = bench_name.split_at(idx);
                    let impl_part = &impl_part[3..]; // Skip " | "
                    (name_part.to_string(), impl_part.to_string())
                } else {
                    (bench_name.clone(), "-".to_string())
                };

                let bench_name_clean = clean_bench_name.replace('|', "\\|");
                let impl_name_clean = impl_name.replace('|', "\\|");

                writeln!(
                    &mut buffer,
                    "| {group_name} | {bench_name_clean} | {impl_name_clean} | {ops_per_sec:.2} | {throughput_display} | {rel_display} |"
                )
                .map_err(io::Error::other)?;
            }
        }

        writeln!(&mut buffer).map_err(io::Error::other)?;

        // ---------------------------------------------------------------------
        // Prepend to existing markdown file
        // ---------------------------------------------------------------------
        let old_content = std::fs::read_to_string(&path).unwrap_or_default();
        let new_content = format!("{buffer}{old_content}");

        let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(&path)?;
        file.write_all(new_content.as_bytes())?;
        Ok(())
    }
}

fn run_component_bench<F>(group_name: &str, group: &mut BenchmarkGroup<'_, WallTime>, bench_name: &str, throughput_bytes: usize, mut f: F)
where
    F: FnMut(),
{
    group.throughput(Throughput::Bytes(throughput_bytes as u64));
    group.bench_function(bench_name, |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                f();
                total += start.elapsed();
            }
            bench_recorder().record(group_name, bench_name, total, iters, Some(throughput_bytes as u64));
            total
        });
    });
}

#[allow(clippy::too_many_lines)]
fn bench_encode_decode(c: &mut Criterion) {
    let message = sample_complex_root();
    let prost_message = &ComplexRootProst::from(&message);

    let proto_bytes = Bytes::from(ComplexRoot::encode_to_vec(&message));

    let prost_bytes = Bytes::from({
        let mut buf = Vec::with_capacity(prost_message.encoded_len());
        prost_message.encode(&mut buf).unwrap();
        buf
    });
    assert_eq!(proto_bytes.len(), prost_bytes.len());

    let mut group = c.benchmark_group("complex_root_encode_decode");

    group.throughput(Throughput::Bytes(prost_bytes.len() as u64));

    group.bench_function("prost encode_to_vec", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                let buf = prost::Message::encode_to_vec(black_box(prost_message));
                black_box(&buf);
                total += start.elapsed();
            }
            bench_recorder().record(
                "complex_root_encode",
                "prost | encode_to_vec",
                total,
                iters,
                Some(prost_bytes.len() as u64),
            );
            total
        });
    });

    group.bench_function("proto_rs encode_to_vec", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let start = Instant::now();
                let buf = ComplexRoot::encode_to_vec(black_box(&message));
                black_box(&buf);
                total += start.elapsed();
            }
            bench_recorder().record(
                "complex_root_encode",
                "proto_rs | encode_to_vec",
                total,
                iters,
                Some(proto_bytes.len() as u64),
            );
            total
        });
    });

    group.bench_function("proto_rs decode proto_rs input", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let data = proto_bytes.clone();
                let start = Instant::now();
                let decoded =
                    ComplexRoot::decode(black_box(data), DecodeContext::default()).expect("proto_rs decode failed for proto_rs input");
                black_box(decoded);
                total += start.elapsed();
            }
            bench_recorder().record(
                "complex_root_decode",
                "proto_rs | decode proto_rs input",
                total,
                iters,
                Some(proto_bytes.len() as u64),
            );
            total
        });
    });

    group.bench_function("prost decode proto_rs input", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let data = proto_bytes.clone();
                let start = Instant::now();
                let decoded = ComplexRootProst::decode(black_box(data)).expect("prost decode failed for proto_rs input");
                black_box(decoded);
                total += start.elapsed();
            }
            bench_recorder().record(
                "complex_root_decode",
                "prost | decode proto_rs input",
                total,
                iters,
                Some(proto_bytes.len() as u64),
            );
            total
        });
    });

    group.bench_function("proto_rs decode prost input", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let data = prost_bytes.clone();
                let start = Instant::now();
                let decoded =
                    ComplexRoot::decode(black_box(data), DecodeContext::default()).expect("proto_rs decode failed for prost input");
                black_box(decoded);
                total += start.elapsed();
            }
            bench_recorder().record(
                "complex_root_decode",
                "proto_rs | decode prost input",
                total,
                iters,
                Some(prost_bytes.len() as u64),
            );
            total
        });
    });

    group.bench_function("prost decode prost input", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let data = prost_bytes.clone();
                let start = Instant::now();
                let decoded = ComplexRootProst::decode(black_box(data)).expect("prost decode failed for prost input");
                black_box(decoded);
                total += start.elapsed();
            }
            bench_recorder().record(
                "complex_root_decode",
                "prost | decode prost input",
                total,
                iters,
                Some(prost_bytes.len() as u64),
            );
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

    let mut group = c.benchmark_group("zero_copy_vs_clone");
    println!("prost len {prost_len} proto_rs len {proto_len}");
    group.throughput(Throughput::Bytes(prost_len as u64));
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
            bench_recorder().record("zero_copy_vs_clone", "prost | clone + encode", total, iters, Some(prost_len as u64));
            total
        });
    });

    group.throughput(Throughput::Bytes(proto_len as u64));
    group.bench_function("proto_rs zero_copy response", |b| {
        let mut buf = Vec::with_capacity(proto_len);
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                buf.clear();
                let start = Instant::now();
                let zero_copy = ComplexRoot::encode_to_zerocopy(&message);
                buf.put_slice(zero_copy.as_slice());
                black_box(&buf);
                total += start.elapsed();
            }
            bench_recorder().record("zero_copy_vs_clone", "proto_rs | zero_copy", total, iters, Some(proto_len as u64));
            total
        });
    });

    group.finish();
}

fn bench_complex_components(c: &mut Criterion) {
    const GROUP: &str = "complex_root_components_encode";

    let root = sample_complex_root();
    let prost_root = ComplexRootProst::from(&root);

    // --- Root-level consistency check
    let proto_root_size = ComplexRoot::encode_to_vec(&root).len();
    let prost_root_size = prost_root.encode_to_vec().len();
    assert_eq!(
        proto_root_size, prost_root_size,
        "ComplexRoot size mismatch: proto_rs = {}, prost = {}",
        proto_root_size, prost_root_size
    );

    // -------------------------------------------------------------------------
    // Individual components
    // -------------------------------------------------------------------------

    // --- NestedLeaf
    let nested_leaf = root.leaves.first().expect("sample leaf").clone();
    let nested_leaf_prost = NestedLeafProst::from(&nested_leaf);
    let nested_leaf_proto_size = NestedLeaf::encode_to_vec(&nested_leaf).len();
    let nested_leaf_prost_size = nested_leaf_prost.encode_to_vec().len();
    assert_eq!(
        nested_leaf_proto_size, nested_leaf_prost_size,
        "NestedLeaf size mismatch: proto_rs = {}, prost = {}",
        nested_leaf_proto_size, nested_leaf_prost_size
    );

    // --- DeepMessage
    let deep_message = root.deep_list.first().expect("sample deep message").clone();
    let deep_message_prost = DeepMessageProst::from(&deep_message);
    let deep_message_proto_size = DeepMessage::encode_to_vec(&deep_message).len();
    let deep_message_prost_size = deep_message_prost.encode_to_vec().len();
    assert_eq!(
        deep_message_proto_size, deep_message_prost_size,
        "DeepMessage size mismatch: proto_rs = {}, prost = {}",
        deep_message_proto_size, deep_message_prost_size
    );

    // --- ComplexEnum
    let complex_enum = root.status.clone();
    let complex_enum_prost = ComplexEnumProst::from(&complex_enum);
    let complex_enum_proto_size = ComplexEnum::encode_to_vec(&complex_enum).len();
    let complex_enum_prost_size = complex_enum_prost.encode_to_vec().len();
    assert_eq!(
        complex_enum_proto_size, complex_enum_prost_size,
        "ComplexEnum size mismatch: proto_rs = {}, prost = {}",
        complex_enum_proto_size, complex_enum_prost_size
    );

    // --- Lists and maps
    let leaves_proto = BenchNestedLeafList {
        items: root.leaves.clone(),
    };
    let leaves_prost = BenchNestedLeafListProst {
        items: prost_root.leaves.clone(),
    };
    let leaves_proto_size = BenchNestedLeafList::encode_to_vec(&leaves_proto).len();
    let leaves_prost_size = leaves_prost.encode_to_vec().len();
    assert_eq!(
        leaves_proto_size, leaves_prost_size,
        "BenchNestedLeafList size mismatch: proto_rs = {}, prost = {}",
        leaves_proto_size, leaves_prost_size
    );

    let deep_list_proto = BenchDeepMessageList {
        items: root.deep_list.clone(),
    };
    let deep_list_prost = BenchDeepMessageListProst {
        items: prost_root.deep_list.clone(),
    };
    let deep_list_proto_size = BenchDeepMessageList::encode_to_vec(&deep_list_proto).len();
    let deep_list_prost_size = deep_list_prost.encode_to_vec().len();
    assert_eq!(
        deep_list_proto_size, deep_list_prost_size,
        "BenchDeepMessageList size mismatch: proto_rs = {}, prost = {}",
        deep_list_proto_size, deep_list_prost_size
    );

    let leaf_lookup_proto = BenchLeafLookup {
        entries: root.leaf_lookup.clone(),
    };
    let leaf_lookup_prost = BenchLeafLookupProst {
        entries: prost_root.leaf_lookup.clone(),
    };
    let leaf_lookup_proto_size = BenchLeafLookup::encode_to_vec(&leaf_lookup_proto).len();
    let leaf_lookup_prost_size = leaf_lookup_prost.encode_to_vec().len();
    assert_eq!(
        leaf_lookup_proto_size, leaf_lookup_prost_size,
        "BenchLeafLookup size mismatch: proto_rs = {}, prost = {}",
        leaf_lookup_proto_size, leaf_lookup_prost_size
    );

    let deep_lookup_proto = BenchDeepLookup {
        entries: root.deep_lookup.clone(),
    };
    let deep_lookup_prost = BenchDeepLookupProst {
        entries: prost_root.deep_lookup.clone(),
    };
    let deep_lookup_proto_size = BenchDeepLookup::encode_to_vec(&deep_lookup_proto).len();
    let deep_lookup_prost_size = deep_lookup_prost.encode_to_vec().len();
    assert_eq!(
        deep_lookup_proto_size, deep_lookup_prost_size,
        "BenchDeepLookup size mismatch: proto_rs = {}, prost = {}",
        deep_lookup_proto_size, deep_lookup_prost_size
    );

    let status_history_proto = BenchStatusHistory {
        items: root.status_history.clone(),
    };
    let status_history_prost = BenchStatusHistoryProst {
        items: prost_root.status_history.clone(),
    };
    let status_history_proto_size = BenchStatusHistory::encode_to_vec(&status_history_proto).len();
    let status_history_prost_size = status_history_prost.encode_to_vec().len();
    assert_eq!(
        status_history_proto_size, status_history_prost_size,
        "BenchStatusHistory size mismatch: proto_rs = {}, prost = {}",
        status_history_proto_size, status_history_prost_size
    );

    let status_lookup_proto = BenchStatusLookup {
        entries: root.status_lookup.clone(),
    };
    let status_lookup_prost = BenchStatusLookupProst {
        entries: prost_root.status_lookup.clone(),
    };
    let status_lookup_proto_size = BenchStatusLookup::encode_to_vec(&status_lookup_proto).len();
    let status_lookup_prost_size = status_lookup_prost.encode_to_vec().len();
    assert_eq!(
        status_lookup_proto_size, status_lookup_prost_size,
        "BenchStatusLookup size mismatch: proto_rs = {}, prost = {}",
        status_lookup_proto_size, status_lookup_prost_size
    );

    let attachments_proto = BenchAttachments {
        items: root.attachments.clone(),
    };
    let attachments_prost = BenchAttachmentsProst {
        items: prost_root.attachments.clone(),
    };
    let attachments_proto_size = BenchAttachments::encode_to_vec(&attachments_proto).len();
    let attachments_prost_size = attachments_prost.encode_to_vec().len();
    assert_eq!(
        attachments_proto_size, attachments_prost_size,
        "BenchAttachments size mismatch: proto_rs = {}, prost = {}",
        attachments_proto_size, attachments_prost_size
    );

    let audit_log_proto = BenchAuditLog {
        entries: root.audit_log.clone(),
    };
    let audit_log_prost = BenchAuditLogProst {
        entries: prost_root.audit_log.clone(),
    };
    let audit_log_proto_size = BenchAuditLog::encode_to_vec(&audit_log_proto).len();
    let audit_log_prost_size = audit_log_prost.encode_to_vec().len();
    assert_eq!(
        audit_log_proto_size, audit_log_prost_size,
        "BenchAuditLog size mismatch: proto_rs = {}, prost = {}",
        audit_log_proto_size, audit_log_prost_size
    );

    let codes_proto = BenchCodes { items: root.codes.clone() };
    let codes_prost = BenchCodesProst {
        items: prost_root.codes.clone(),
    };
    let codes_proto_size = BenchCodes::encode_to_vec(&codes_proto).len();
    let codes_prost_size = codes_prost.encode_to_vec().len();
    assert_eq!(
        codes_proto_size, codes_prost_size,
        "BenchCodes size mismatch: proto_rs = {}, prost = {}",
        codes_proto_size, codes_prost_size
    );

    let tags_proto = BenchTags { items: root.tags.clone() };
    let tags_prost = BenchTagsProst {
        items: prost_root.tags.clone(),
    };
    let tags_proto_size = BenchTags::encode_to_vec(&tags_proto).len();
    let tags_prost_size = tags_prost.encode_to_vec().len();
    assert_eq!(
        tags_proto_size, tags_prost_size,
        "BenchTags size mismatch: proto_rs = {}, prost = {}",
        tags_proto_size, tags_prost_size
    );

    let mut group = c.benchmark_group(GROUP);

    run_component_bench(
        GROUP,
        &mut group,
        "nested_leaf | prost encode_to_vec",
        nested_leaf_prost_size,
        || {
            let buf = nested_leaf_prost.encode_to_vec();
            black_box(&buf);
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "nested_leaf | proto_rs encode_to_vec",
        nested_leaf_proto_size,
        || {
            let buf = NestedLeaf::encode_to_vec(&nested_leaf);
            black_box(&buf);
        },
    );

    run_component_bench(
        GROUP,
        &mut group,
        "deep_message | prost encode_to_vec",
        deep_message_prost_size,
        || {
            let buf = deep_message_prost.encode_to_vec();
            black_box(&buf);
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "deep_message | proto_rs encode_to_vec",
        deep_message_proto_size,
        || {
            let buf = DeepMessage::encode_to_vec(&deep_message);
            black_box(&buf);
        },
    );

    run_component_bench(
        GROUP,
        &mut group,
        "complex_enum | prost encode_to_vec",
        complex_enum_prost_size,
        || {
            let buf = complex_enum_prost.encode_to_vec();
            black_box(&buf);
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "complex_enum | proto_rs encode_to_vec",
        complex_enum_proto_size,
        || {
            let buf = ComplexEnum::encode_to_vec(&complex_enum);
            black_box(&buf);
        },
    );

    run_component_bench(GROUP, &mut group, "leaves list | prost encode_to_vec", leaves_prost_size, || {
        let buf = leaves_prost.encode_to_vec();
        black_box(&buf);
    });
    run_component_bench(GROUP, &mut group, "leaves list | proto_rs encode_to_vec", leaves_proto_size, || {
        let buf = BenchNestedLeafList::encode_to_vec(&leaves_proto);
        black_box(&buf);
    });

    run_component_bench(GROUP, &mut group, "deep list | prost encode_to_vec", deep_list_prost_size, || {
        let buf = deep_list_prost.encode_to_vec();
        black_box(&buf);
    });
    run_component_bench(
        GROUP,
        &mut group,
        "deep list | proto_rs encode_to_vec",
        deep_list_proto_size,
        || {
            let buf = BenchDeepMessageList::encode_to_vec(&deep_list_proto);
            black_box(&buf);
        },
    );

    run_component_bench(
        GROUP,
        &mut group,
        "leaf lookup | prost encode_to_vec",
        leaf_lookup_prost_size,
        || {
            let buf = leaf_lookup_prost.encode_to_vec();
            black_box(&buf);
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "leaf lookup | proto_rs encode_to_vec",
        leaf_lookup_proto_size,
        || {
            let buf = BenchLeafLookup::encode_to_vec(&leaf_lookup_proto);
            black_box(&buf);
        },
    );

    run_component_bench(
        GROUP,
        &mut group,
        "deep lookup | prost encode_to_vec",
        deep_lookup_prost_size,
        || {
            let buf = deep_lookup_prost.encode_to_vec();
            black_box(&buf);
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "deep lookup | proto_rs encode_to_vec",
        deep_lookup_proto_size,
        || {
            let buf = BenchDeepLookup::encode_to_vec(&deep_lookup_proto);
            black_box(&buf);
        },
    );

    run_component_bench(
        GROUP,
        &mut group,
        "status history | prost encode_to_vec",
        status_history_prost_size,
        || {
            let buf = status_history_prost.encode_to_vec();
            black_box(&buf);
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "status history | proto_rs encode_to_vec",
        status_history_proto_size,
        || {
            let buf = BenchStatusHistory::encode_to_vec(&status_history_proto);
            black_box(&buf);
        },
    );

    run_component_bench(
        GROUP,
        &mut group,
        "status lookup | prost encode_to_vec",
        status_lookup_prost_size,
        || {
            let buf = status_lookup_prost.encode_to_vec();
            black_box(&buf);
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "status lookup | proto_rs encode_to_vec",
        status_lookup_proto_size,
        || {
            let buf = BenchStatusLookup::encode_to_vec(&status_lookup_proto);
            black_box(&buf);
        },
    );

    run_component_bench(
        GROUP,
        &mut group,
        "attachments | prost encode_to_vec",
        attachments_prost_size,
        || {
            let buf = attachments_prost.encode_to_vec();
            black_box(&buf);
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "attachments | proto_rs encode_to_vec",
        attachments_proto_size,
        || {
            let buf = BenchAttachments::encode_to_vec(&attachments_proto);
            black_box(&buf);
        },
    );

    run_component_bench(GROUP, &mut group, "audit log | prost encode_to_vec", audit_log_prost_size, || {
        let buf = audit_log_prost.encode_to_vec();
        black_box(&buf);
    });
    run_component_bench(
        GROUP,
        &mut group,
        "audit log | proto_rs encode_to_vec",
        audit_log_proto_size,
        || {
            let buf = BenchAuditLog::encode_to_vec(&audit_log_proto);
            black_box(&buf);
        },
    );

    run_component_bench(GROUP, &mut group, "codes | prost encode_to_vec", codes_prost_size, || {
        let buf = codes_prost.encode_to_vec();
        black_box(&buf);
    });
    run_component_bench(GROUP, &mut group, "codes | proto_rs encode_to_vec", codes_proto_size, || {
        let buf = BenchCodes::encode_to_vec(&codes_proto);
        black_box(&buf);
    });

    run_component_bench(GROUP, &mut group, "tags | prost encode_to_vec", tags_prost_size, || {
        let buf = tags_prost.encode_to_vec();
        black_box(&buf);
    });
    run_component_bench(GROUP, &mut group, "tags | proto_rs encode_to_vec", tags_proto_size, || {
        let buf = BenchTags::encode_to_vec(&tags_proto);
        black_box(&buf);
    });

    group.finish();
}

fn bench_micro_fields_encode(c: &mut Criterion) {
    const GROUP: &str = "micro_fields_encode";

    // Use sample data taken from your root so content matches previous benches
    let root = sample_complex_root();

    // --- String
    let one_string = OneString { v: root.id.clone() };
    let one_string_prost = OneStringProst { v: root.id.clone() };
    let one_string_sz = OneString::encode_to_vec(&one_string).len();
    let one_string_prost_sz = one_string_prost.encode_to_vec().len();
    assert_eq!(one_string_sz, one_string_prost_sz);
    let mut group = c.benchmark_group(GROUP);
    run_component_bench(GROUP, &mut group, "one_string | prost encode_to_vec", one_string_prost_sz, || {
        let _ = one_string_prost.encode_to_vec();
    });
    run_component_bench(GROUP, &mut group, "one_string | proto_rs encode_to_vec", one_string_sz, || {
        let _ = OneString::encode_to_vec(&one_string);
    });
    group.finish();

    // --- Bytes (payload)
    let one_bytes = OneBytes { v: root.payload.clone() };
    let one_bytes_prost = OneBytesProst {
        v: root.payload.clone().to_vec(),
    };
    let one_bytes_sz = OneBytes::encode_to_vec(&one_bytes).len();
    let one_bytes_prost_sz = one_bytes_prost.encode_to_vec().len();
    assert_eq!(one_bytes_sz, one_bytes_prost_sz);
    let mut group = c.benchmark_group(GROUP);
    run_component_bench(GROUP, &mut group, "one_bytes | prost encode_to_vec", one_bytes_prost_sz, || {
        let buf = OneBytesProst::encode_to_vec(&one_bytes_prost);
        black_box(&buf);
    });
    run_component_bench(GROUP, &mut group, "one_bytes | proto_rs encode_to_vec", one_bytes_sz, || {
        let buf = OneBytes::encode_to_vec(&one_bytes);
        black_box(&buf);
    });
    group.finish();

    // --- Enum (SimpleEnum)
    let one_enum = OneEnum { v: root.codes[0] };
    let one_enum_prost = OneEnumProst {
        v: SimpleEnumProst::from(&root.codes[0]) as i32,
    };
    let one_enum_sz = OneEnum::encode_to_vec(&one_enum).len();
    let one_enum_prost_sz = one_enum_prost.encode_to_vec().len();
    assert_eq!(one_enum_sz, one_enum_prost_sz);
    let mut group = c.benchmark_group(GROUP);
    run_component_bench(GROUP, &mut group, "one_enum | prost encode_to_vec", one_enum_prost_sz, || {
        let _ = one_enum_prost.encode_to_vec();
    });
    run_component_bench(GROUP, &mut group, "one_enum | proto_rs encode_to_vec", one_enum_sz, || {
        let _ = OneEnum::encode_to_vec(&one_enum);
    });
    group.finish();

    // --- NestedLeaf
    let leaf = root.leaves[0].clone();
    let one_leaf = OneNestedLeaf { v: leaf.clone() };
    let one_leaf_prost = OneNestedLeafProst {
        v: Some(NestedLeafProst::from(&leaf)),
    };
    let one_leaf_sz = OneNestedLeaf::encode_to_vec(&one_leaf).len();
    let one_leaf_prost_sz = one_leaf_prost.encode_to_vec().len();
    assert_eq!(one_leaf_sz, one_leaf_prost_sz);
    let mut group = c.benchmark_group(GROUP);
    run_component_bench(
        GROUP,
        &mut group,
        "one_nested_leaf | prost encode_to_vec",
        one_leaf_prost_sz,
        || {
            let _ = one_leaf_prost.encode_to_vec();
        },
    );
    run_component_bench(GROUP, &mut group, "one_nested_leaf | proto_rs encode_to_vec", one_leaf_sz, || {
        let _ = OneNestedLeaf::encode_to_vec(&one_leaf);
    });
    group.finish();

    // --- DeepMessage
    let deep = root.deep_list[0].clone();
    let one_deep = OneDeepMessage { v: deep.clone() };
    let one_deep_prost = OneDeepMessageProst {
        v: Some(DeepMessageProst::from(&deep)),
    };
    let one_deep_sz = OneDeepMessage::encode_to_vec(&one_deep).len();
    let one_deep_prost_sz = one_deep_prost.encode_to_vec().len();
    assert_eq!(one_deep_sz, one_deep_prost_sz);
    let mut group = c.benchmark_group(GROUP);
    run_component_bench(
        GROUP,
        &mut group,
        "one_deep_message | prost encode_to_vec",
        one_deep_prost_sz,
        || {
            let _ = one_deep_prost.encode_to_vec();
        },
    );
    run_component_bench(GROUP, &mut group, "one_deep_message | proto_rs encode_to_vec", one_deep_sz, || {
        let _ = OneDeepMessage::encode_to_vec(&one_deep);
    });
    group.finish();

    // --- ComplexEnum variants individually (pick the current status variant)
    let ce = root.status.clone();
    let one_ce = OneComplexEnum { v: ce.clone() };
    let one_ce_prost = OneComplexEnumProst {
        v: Some(ComplexEnumProst::from(&ce)),
    };
    let one_ce_sz = OneComplexEnum::encode_to_vec(&one_ce).len();
    let one_ce_prost_sz = one_ce_prost.encode_to_vec().len();
    assert_eq!(one_ce_sz, one_ce_prost_sz);
    let mut group = c.benchmark_group(GROUP);
    run_component_bench(GROUP, &mut group, "one_complex_enum | prost encode_to_vec", one_ce_prost_sz, || {
        let _ = one_ce_prost.encode_to_vec();
    });
    run_component_bench(GROUP, &mut group, "one_complex_enum | proto_rs encode_to_vec", one_ce_sz, || {
        let _ = OneComplexEnum::encode_to_vec(&one_ce);
    });
    group.finish();
}

fn bench_collection_overhead_encode(c: &mut Criterion) {
    const GROUP: &str = "collection_overhead_encode";

    let root = sample_complex_root();

    // --- Vec<String> (tags) with exactly 1 item vs single-string message
    let one_tag = BenchTags {
        items: vec![root.tags[0].clone()],
    };
    let one_tag_prost = BenchTagsProst {
        items: vec![root.tags[0].clone()],
    };
    let one_tag_sz = BenchTags::encode_to_vec(&one_tag).len();
    let one_tag_prost_sz = one_tag_prost.encode_to_vec().len();
    assert_eq!(one_tag_sz, one_tag_prost_sz);

    let single_str = OneString { v: root.tags[0].clone() };
    let single_str_prost = OneStringProst { v: root.tags[0].clone() };
    let single_str_sz = OneString::encode_to_vec(&single_str).len();
    let single_str_prost_sz = single_str_prost.encode_to_vec().len();
    assert_eq!(single_str_sz, single_str_prost_sz);

    let mut group = c.benchmark_group(GROUP);
    run_component_bench(GROUP, &mut group, "tags_len1 | prost encode_to_vec", one_tag_prost_sz, || {
        let _ = one_tag_prost.encode_to_vec();
    });
    run_component_bench(GROUP, &mut group, "tags_len1 | proto_rs encode_to_vec", one_tag_sz, || {
        let _ = BenchTags::encode_to_vec(&one_tag);
    });
    run_component_bench(GROUP, &mut group, "one_string | prost encode_to_vec", single_str_prost_sz, || {
        let _ = single_str_prost.encode_to_vec();
    });
    run_component_bench(GROUP, &mut group, "one_string | proto_rs encode_to_vec", single_str_sz, || {
        let _ = OneString::encode_to_vec(&single_str);
    });
    group.finish();

    // --- Vec<Bytes> (attachments) len=1 vs one-bytes
    let one_bytes_vec = BenchAttachments {
        items: vec![root.attachments[0].clone()],
    };
    let one_bytes_vec_prost = BenchAttachmentsProst {
        items: vec![root.attachments[0].clone().to_vec()],
    };
    let one_bytes_vec_sz = BenchAttachments::encode_to_vec(&one_bytes_vec).len();
    let one_bytes_vec_prost_sz = one_bytes_vec_prost.encode_to_vec().len();
    assert_eq!(one_bytes_vec_sz, one_bytes_vec_prost_sz);

    let single_bytes = OneBytes {
        v: root.attachments[0].clone(),
    };
    let single_bytes_prost = OneBytesProst {
        v: root.attachments[0].clone().to_vec(),
    };
    let single_bytes_sz = OneBytes::encode_to_vec(&single_bytes).len();
    let single_bytes_prost_sz = single_bytes_prost.encode_to_vec().len();
    assert_eq!(single_bytes_sz, single_bytes_prost_sz);

    let mut group = c.benchmark_group(GROUP);
    run_component_bench(
        GROUP,
        &mut group,
        "attachments_len1 | prost encode_to_vec",
        one_bytes_vec_prost_sz,
        || {
            let _ = one_bytes_vec_prost.encode_to_vec();
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "attachments_len1 | proto_rs encode_to_vec",
        one_bytes_vec_sz,
        || {
            let _ = BenchAttachments::encode_to_vec(&one_bytes_vec);
        },
    );
    run_component_bench(GROUP, &mut group, "one_bytes | prost encode_to_vec", single_bytes_prost_sz, || {
        let _ = single_bytes_prost.encode_to_vec();
    });
    run_component_bench(GROUP, &mut group, "one_bytes | proto_rs encode_to_vec", single_bytes_sz, || {
        let _ = OneBytes::encode_to_vec(&single_bytes);
    });
    group.finish();

    // --- Vec<SimpleEnum> (codes) len=1 vs one-enum
    let one_enum_vec = BenchCodes {
        items: vec![root.codes[0]],
    };
    let one_enum_vec_prost = BenchCodesProst {
        items: vec![SimpleEnumProst::from(&root.codes[0]) as i32],
    };
    let one_enum_vec_sz = BenchCodes::encode_to_vec(&one_enum_vec).len();
    let one_enum_vec_prost_sz = one_enum_vec_prost.encode_to_vec().len();
    assert_eq!(one_enum_vec_sz, one_enum_vec_prost_sz);

    let single_enum = OneEnum { v: root.codes[0] };
    let single_enum_prost = OneEnumProst {
        v: SimpleEnumProst::from(&root.codes[0]) as i32,
    };
    let single_enum_sz = OneEnum::encode_to_vec(&single_enum).len();
    let single_enum_prost_sz = single_enum_prost.encode_to_vec().len();
    assert_eq!(single_enum_sz, single_enum_prost_sz);

    let mut group = c.benchmark_group(GROUP);
    run_component_bench(GROUP, &mut group, "codes_len1 | prost encode_to_vec", one_enum_vec_prost_sz, || {
        let _ = one_enum_vec_prost.encode_to_vec();
    });
    run_component_bench(GROUP, &mut group, "codes_len1 | proto_rs encode_to_vec", one_enum_vec_sz, || {
        let _ = BenchCodes::encode_to_vec(&one_enum_vec);
    });
    run_component_bench(GROUP, &mut group, "one_enum | prost encode_to_vec", single_enum_prost_sz, || {
        let _ = single_enum_prost.encode_to_vec();
    });
    run_component_bench(GROUP, &mut group, "one_enum | proto_rs encode_to_vec", single_enum_sz, || {
        let _ = OneEnum::encode_to_vec(&single_enum);
    });
    group.finish();

    // --- Vec<NestedLeaf> (leaves) len=1 vs one-nested-leaf
    let one_leaf_vec = BenchNestedLeafList {
        items: vec![root.leaves[0].clone()],
    };
    let one_leaf_vec_prost = BenchNestedLeafListProst {
        items: vec![NestedLeafProst::from(&root.leaves[0])],
    };
    let one_leaf_vec_sz = BenchNestedLeafList::encode_to_vec(&one_leaf_vec).len();
    let one_leaf_vec_prost_sz = one_leaf_vec_prost.encode_to_vec().len();
    assert_eq!(one_leaf_vec_sz, one_leaf_vec_prost_sz);

    let single_leaf = OneNestedLeaf { v: root.leaves[0].clone() };
    let single_leaf_prost = OneNestedLeafProst {
        v: Some(NestedLeafProst::from(&root.leaves[0])),
    };
    let single_leaf_sz = OneNestedLeaf::encode_to_vec(&single_leaf).len();
    let single_leaf_prost_sz = single_leaf_prost.encode_to_vec().len();
    assert_eq!(single_leaf_sz, single_leaf_prost_sz);

    let mut group = c.benchmark_group(GROUP);
    run_component_bench(
        GROUP,
        &mut group,
        "leaves_len1 | prost encode_to_vec",
        one_leaf_vec_prost_sz,
        || {
            let _ = one_leaf_vec_prost.encode_to_vec();
        },
    );
    run_component_bench(GROUP, &mut group, "leaves_len1 | proto_rs encode_to_vec", one_leaf_vec_sz, || {
        let _ = BenchNestedLeafList::encode_to_vec(&one_leaf_vec);
    });
    run_component_bench(
        GROUP,
        &mut group,
        "one_nested_leaf | prost encode_to_vec",
        single_leaf_prost_sz,
        || {
            let _ = single_leaf_prost.encode_to_vec();
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "one_nested_leaf | proto_rs encode_to_vec",
        single_leaf_sz,
        || {
            let _ = OneNestedLeaf::encode_to_vec(&single_leaf);
        },
    );
    group.finish();

    // --- Vec<DeepMessage> (deep_list) len=1 vs one-deep-message
    let one_deep_vec = BenchDeepMessageList {
        items: vec![root.deep_list[0].clone()],
    };
    let one_deep_vec_prost = BenchDeepMessageListProst {
        items: vec![DeepMessageProst::from(&root.deep_list[0])],
    };
    let one_deep_vec_sz = BenchDeepMessageList::encode_to_vec(&one_deep_vec).len();
    let one_deep_vec_prost_sz = one_deep_vec_prost.encode_to_vec().len();
    assert_eq!(one_deep_vec_sz, one_deep_vec_prost_sz);

    let single_deep = OneDeepMessage {
        v: root.deep_list[0].clone(),
    };
    let single_deep_prost = OneDeepMessageProst {
        v: Some(DeepMessageProst::from(&root.deep_list[0])),
    };
    let single_deep_sz = OneDeepMessage::encode_to_vec(&single_deep).len();
    let single_deep_prost_sz = single_deep_prost.encode_to_vec().len();
    assert_eq!(single_deep_sz, single_deep_prost_sz);

    let mut group = c.benchmark_group(GROUP);
    run_component_bench(
        GROUP,
        &mut group,
        "deep_list_len1 | prost encode_to_vec",
        one_deep_vec_prost_sz,
        || {
            let _ = one_deep_vec_prost.encode_to_vec();
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "deep_list_len1 | proto_rs encode_to_vec",
        one_deep_vec_sz,
        || {
            let _ = BenchDeepMessageList::encode_to_vec(&one_deep_vec);
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "one_deep_message | prost encode_to_vec",
        single_deep_prost_sz,
        || {
            let _ = single_deep_prost.encode_to_vec();
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "one_deep_message | proto_rs encode_to_vec",
        single_deep_sz,
        || {
            let _ = OneDeepMessage::encode_to_vec(&single_deep);
        },
    );
    group.finish();

    // --- Vec<ComplexEnum> (status_history) len=1 vs one-complex-enum
    let one_ce_vec = BenchStatusHistory {
        items: vec![root.status.clone()],
    };
    let one_ce_vec_prost = BenchStatusHistoryProst {
        items: vec![ComplexEnumProst::from(&root.status)],
    };
    let one_ce_vec_sz = BenchStatusHistory::encode_to_vec(&one_ce_vec).len();
    let one_ce_vec_prost_sz = one_ce_vec_prost.encode_to_vec().len();
    assert_eq!(one_ce_vec_sz, one_ce_vec_prost_sz);

    let single_ce = OneComplexEnum { v: root.status.clone() };
    let single_ce_prost = OneComplexEnumProst {
        v: Some(ComplexEnumProst::from(&root.status)),
    };
    let single_ce_sz = OneComplexEnum::encode_to_vec(&single_ce).len();
    let single_ce_prost_sz = single_ce_prost.encode_to_vec().len();
    assert_eq!(single_ce_sz, single_ce_prost_sz);

    let mut group = c.benchmark_group(GROUP);
    run_component_bench(
        GROUP,
        &mut group,
        "status_history_len1 | prost encode_to_vec",
        one_ce_vec_prost_sz,
        || {
            let _ = one_ce_vec_prost.encode_to_vec();
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "status_history_len1 | proto_rs encode_to_vec",
        one_ce_vec_sz,
        || {
            let _ = BenchStatusHistory::encode_to_vec(&one_ce_vec);
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "one_complex_enum | prost encode_to_vec",
        single_ce_prost_sz,
        || {
            let _ = single_ce_prost.encode_to_vec();
        },
    );
    run_component_bench(GROUP, &mut group, "one_complex_enum | proto_rs encode_to_vec", single_ce_sz, || {
        let _ = OneComplexEnum::encode_to_vec(&single_ce);
    });
    group.finish();

    // --- Maps with exactly one entry (leaf_lookup as example)
    let mut one_leaf_map = HashMap::new();
    one_leaf_map.insert("k".to_string(), root.leaves[0].clone());
    let one_leaf_map_msg = BenchLeafLookup {
        entries: one_leaf_map.clone(),
    };
    let one_leaf_map_msg_prost = BenchLeafLookupProst {
        entries: {
            let mut m = HashMap::new();
            m.insert("k".to_string(), NestedLeafProst::from(&root.leaves[0]));
            m
        },
    };
    let one_leaf_map_sz = BenchLeafLookup::encode_to_vec(&one_leaf_map_msg).len();
    let one_leaf_map_prost_sz = one_leaf_map_msg_prost.encode_to_vec().len();
    assert_eq!(one_leaf_map_sz, one_leaf_map_prost_sz);

    let mut group = c.benchmark_group(GROUP);
    run_component_bench(
        GROUP,
        &mut group,
        "leaf_lookup_len1 | prost encode_to_vec",
        one_leaf_map_prost_sz,
        || {
            let _ = one_leaf_map_msg_prost.encode_to_vec();
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "leaf_lookup_len1 | proto_rs encode_to_vec",
        one_leaf_map_sz,
        || {
            let _ = BenchLeafLookup::encode_to_vec(&one_leaf_map_msg);
        },
    );
    group.finish();
}

// ============================================================================
// Decode benches (mirrors encode ones)
// ============================================================================

fn bench_complex_components_decode(c: &mut Criterion) {
    const GROUP: &str = "complex_root_components_decode";

    let root = sample_complex_root();

    // Pre-encode for consistent decode input
    let nested_leaf_bytes = NestedLeaf::encode_to_vec(&root.leaves[0]);
    let deep_message_bytes = DeepMessage::encode_to_vec(&root.deep_list[0]);
    let complex_enum_bytes = ComplexEnum::encode_to_vec(&root.status);

    let leaves_bytes = BenchNestedLeafList::encode_to_vec(&BenchNestedLeafList {
        items: root.leaves.clone(),
    });
    let deep_list_bytes = BenchDeepMessageList::encode_to_vec(&BenchDeepMessageList {
        items: root.deep_list.clone(),
    });
    let leaf_lookup_bytes = BenchLeafLookup::encode_to_vec(&BenchLeafLookup {
        entries: root.leaf_lookup.clone(),
    });
    let deep_lookup_bytes = BenchDeepLookup::encode_to_vec(&BenchDeepLookup {
        entries: root.deep_lookup.clone(),
    });
    let status_history_bytes = BenchStatusHistory::encode_to_vec(&BenchStatusHistory {
        items: root.status_history.clone(),
    });
    let status_lookup_bytes = BenchStatusLookup::encode_to_vec(&BenchStatusLookup {
        entries: root.status_lookup.clone(),
    });
    let attachments_bytes = BenchAttachments::encode_to_vec(&BenchAttachments {
        items: root.attachments.clone(),
    });
    let audit_log_bytes = BenchAuditLog::encode_to_vec(&BenchAuditLog {
        entries: root.audit_log.clone(),
    });
    let codes_bytes = BenchCodes::encode_to_vec(&BenchCodes { items: root.codes.clone() });
    let tags_bytes = BenchTags::encode_to_vec(&BenchTags { items: root.tags.clone() });

    let mut group = c.benchmark_group(GROUP);

    // -------------------- Single Components --------------------
    run_component_bench(GROUP, &mut group, "nested_leaf | prost decode", nested_leaf_bytes.len(), || {
        let _ = NestedLeafProst::decode(nested_leaf_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "nested_leaf | proto_rs decode", nested_leaf_bytes.len(), || {
        let _ = NestedLeaf::decode(nested_leaf_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    run_component_bench(GROUP, &mut group, "deep_message | prost decode", deep_message_bytes.len(), || {
        let _ = DeepMessageProst::decode(deep_message_bytes.as_slice()).unwrap();
    });
    run_component_bench(
        GROUP,
        &mut group,
        "deep_message | proto_rs decode",
        deep_message_bytes.len(),
        || {
            let _ = DeepMessage::decode(deep_message_bytes.as_slice(), DecodeContext::default()).unwrap();
        },
    );

    run_component_bench(GROUP, &mut group, "complex_enum | prost decode", complex_enum_bytes.len(), || {
        let _ = ComplexEnumProst::decode(complex_enum_bytes.as_slice()).unwrap();
    });
    run_component_bench(
        GROUP,
        &mut group,
        "complex_enum | proto_rs decode",
        complex_enum_bytes.len(),
        || {
            let _ = ComplexEnum::decode(complex_enum_bytes.as_slice(), DecodeContext::default()).unwrap();
        },
    );

    // -------------------- Collections --------------------
    run_component_bench(GROUP, &mut group, "leaves list | prost decode", leaves_bytes.len(), || {
        let _ = BenchNestedLeafListProst::decode(leaves_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "leaves list | proto_rs decode", leaves_bytes.len(), || {
        let _ = BenchNestedLeafList::decode(leaves_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    run_component_bench(GROUP, &mut group, "deep list | prost decode", deep_list_bytes.len(), || {
        let _ = BenchDeepMessageListProst::decode(deep_list_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "deep list | proto_rs decode", deep_list_bytes.len(), || {
        let _ = BenchDeepMessageList::decode(deep_list_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    run_component_bench(GROUP, &mut group, "leaf lookup | prost decode", leaf_lookup_bytes.len(), || {
        let _ = BenchLeafLookupProst::decode(leaf_lookup_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "leaf lookup | proto_rs decode", leaf_lookup_bytes.len(), || {
        let _ = BenchLeafLookup::decode(leaf_lookup_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    run_component_bench(GROUP, &mut group, "deep lookup | prost decode", deep_lookup_bytes.len(), || {
        let _ = BenchDeepLookupProst::decode(deep_lookup_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "deep lookup | proto_rs decode", deep_lookup_bytes.len(), || {
        let _ = BenchDeepLookup::decode(deep_lookup_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    run_component_bench(
        GROUP,
        &mut group,
        "status history | prost decode",
        status_history_bytes.len(),
        || {
            let _ = BenchStatusHistoryProst::decode(status_history_bytes.as_slice()).unwrap();
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "status history | proto_rs decode",
        status_history_bytes.len(),
        || {
            let _ = BenchStatusHistory::decode(status_history_bytes.as_slice(), DecodeContext::default()).unwrap();
        },
    );

    run_component_bench(GROUP, &mut group, "status lookup | prost decode", status_lookup_bytes.len(), || {
        let _ = BenchStatusLookupProst::decode(status_lookup_bytes.as_slice()).unwrap();
    });
    run_component_bench(
        GROUP,
        &mut group,
        "status lookup | proto_rs decode",
        status_lookup_bytes.len(),
        || {
            let _ = BenchStatusLookup::decode(status_lookup_bytes.as_slice(), DecodeContext::default()).unwrap();
        },
    );

    run_component_bench(GROUP, &mut group, "attachments | prost decode", attachments_bytes.len(), || {
        let _ = BenchAttachmentsProst::decode(attachments_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "attachments | proto_rs decode", attachments_bytes.len(), || {
        let _ = BenchAttachments::decode(attachments_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    run_component_bench(GROUP, &mut group, "audit log | prost decode", audit_log_bytes.len(), || {
        let _ = BenchAuditLogProst::decode(audit_log_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "audit log | proto_rs decode", audit_log_bytes.len(), || {
        let _ = BenchAuditLog::decode(audit_log_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    run_component_bench(GROUP, &mut group, "codes | prost decode", codes_bytes.len(), || {
        let _ = BenchCodesProst::decode(codes_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "codes | proto_rs decode", codes_bytes.len(), || {
        let _ = BenchCodes::decode(codes_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    run_component_bench(GROUP, &mut group, "tags | prost decode", tags_bytes.len(), || {
        let _ = BenchTagsProst::decode(tags_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "tags | proto_rs decode", tags_bytes.len(), || {
        let _ = BenchTags::decode(tags_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    group.finish();
}

fn bench_micro_fields_decode(c: &mut Criterion) {
    const GROUP: &str = "micro_fields_decode";

    let root = sample_complex_root();

    let one_string_bytes = OneString::encode_to_vec(&OneString { v: root.id.clone() });
    let one_bytes_bytes = OneBytes::encode_to_vec(&OneBytes { v: root.payload.clone() });
    let one_enum_bytes = OneEnum::encode_to_vec(&OneEnum { v: root.codes[0] });
    let one_leaf_bytes = OneNestedLeaf::encode_to_vec(&OneNestedLeaf { v: root.leaves[0].clone() });
    let one_deep_bytes = OneDeepMessage::encode_to_vec(&OneDeepMessage {
        v: root.deep_list[0].clone(),
    });
    let one_ce_bytes = OneComplexEnum::encode_to_vec(&OneComplexEnum { v: root.status.clone() });

    let mut group = c.benchmark_group(GROUP);

    run_component_bench(GROUP, &mut group, "one_string | prost decode", one_string_bytes.len(), || {
        let _ = OneStringProst::decode(one_string_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "one_string | proto_rs decode", one_string_bytes.len(), || {
        let _ = OneString::decode(one_string_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    run_component_bench(GROUP, &mut group, "one_bytes | prost decode", one_bytes_bytes.len(), || {
        let _ = OneBytesProst::decode(one_bytes_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "one_bytes | proto_rs decode", one_bytes_bytes.len(), || {
        let _ = OneBytes::decode(one_bytes_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    run_component_bench(GROUP, &mut group, "one_enum | prost decode", one_enum_bytes.len(), || {
        let _ = OneEnumProst::decode(one_enum_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "one_enum | proto_rs decode", one_enum_bytes.len(), || {
        let _ = OneEnum::decode(one_enum_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    run_component_bench(GROUP, &mut group, "one_nested_leaf | prost decode", one_leaf_bytes.len(), || {
        let _ = OneNestedLeafProst::decode(one_leaf_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "one_nested_leaf | proto_rs decode", one_leaf_bytes.len(), || {
        let _ = OneNestedLeaf::decode(one_leaf_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    run_component_bench(GROUP, &mut group, "one_deep_message | prost decode", one_deep_bytes.len(), || {
        let _ = OneDeepMessageProst::decode(one_deep_bytes.as_slice()).unwrap();
    });
    run_component_bench(
        GROUP,
        &mut group,
        "one_deep_message | proto_rs decode",
        one_deep_bytes.len(),
        || {
            let _ = OneDeepMessage::decode(one_deep_bytes.as_slice(), DecodeContext::default()).unwrap();
        },
    );

    run_component_bench(GROUP, &mut group, "one_complex_enum | prost decode", one_ce_bytes.len(), || {
        let _ = OneComplexEnumProst::decode(one_ce_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "one_complex_enum | proto_rs decode", one_ce_bytes.len(), || {
        let _ = OneComplexEnum::decode(one_ce_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    group.finish();
}

fn bench_collection_overhead_decode(c: &mut Criterion) {
    const GROUP: &str = "collection_overhead_decode";

    let root = sample_complex_root();

    // Vec<String> (tags) len=1 vs single-string
    let one_tag_bytes = BenchTags::encode_to_vec(&BenchTags {
        items: vec![root.tags[0].clone()],
    });
    let one_str_bytes = OneString::encode_to_vec(&OneString { v: root.tags[0].clone() });

    let one_bytes_vec_bytes = BenchAttachments::encode_to_vec(&BenchAttachments {
        items: vec![root.attachments[0].clone()],
    });
    let single_bytes_bytes = OneBytes::encode_to_vec(&OneBytes {
        v: root.attachments[0].clone(),
    });

    let one_enum_vec_bytes = BenchCodes::encode_to_vec(&BenchCodes {
        items: vec![root.codes[0]],
    });
    let single_enum_bytes = OneEnum::encode_to_vec(&OneEnum { v: root.codes[0] });

    let one_leaf_vec_bytes = BenchNestedLeafList::encode_to_vec(&BenchNestedLeafList {
        items: vec![root.leaves[0].clone()],
    });
    let single_leaf_bytes = OneNestedLeaf::encode_to_vec(&OneNestedLeaf { v: root.leaves[0].clone() });

    let one_deep_vec_bytes = BenchDeepMessageList::encode_to_vec(&BenchDeepMessageList {
        items: vec![root.deep_list[0].clone()],
    });
    let single_deep_bytes = OneDeepMessage::encode_to_vec(&OneDeepMessage {
        v: root.deep_list[0].clone(),
    });

    let one_ce_vec_bytes = BenchStatusHistory::encode_to_vec(&BenchStatusHistory {
        items: vec![root.status.clone()],
    });
    let single_ce_bytes = OneComplexEnum::encode_to_vec(&OneComplexEnum { v: root.status.clone() });

    let one_leaf_map_bytes = BenchLeafLookup::encode_to_vec(&BenchLeafLookup {
        entries: HashMap::from([("k".to_string(), root.leaves[0].clone())]),
    });

    let mut group = c.benchmark_group(GROUP);

    // ---- Vec<String> ----
    run_component_bench(GROUP, &mut group, "tags_len1 | prost decode", one_tag_bytes.len(), || {
        let _ = BenchTagsProst::decode(one_tag_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "tags_len1 | proto_rs decode", one_tag_bytes.len(), || {
        let _ = BenchTags::decode(one_tag_bytes.as_slice(), DecodeContext::default()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "one_string | prost decode", one_str_bytes.len(), || {
        let _ = OneStringProst::decode(one_str_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "one_string | proto_rs decode", one_str_bytes.len(), || {
        let _ = OneString::decode(one_str_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    // ---- Vec<Bytes> ----
    run_component_bench(
        GROUP,
        &mut group,
        "attachments_len1 | prost decode",
        one_bytes_vec_bytes.len(),
        || {
            let _ = BenchAttachmentsProst::decode(one_bytes_vec_bytes.as_slice()).unwrap();
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "attachments_len1 | proto_rs decode",
        one_bytes_vec_bytes.len(),
        || {
            let _ = BenchAttachments::decode(one_bytes_vec_bytes.as_slice(), DecodeContext::default()).unwrap();
        },
    );
    run_component_bench(GROUP, &mut group, "one_bytes | prost decode", single_bytes_bytes.len(), || {
        let _ = OneBytesProst::decode(single_bytes_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "one_bytes | proto_rs decode", single_bytes_bytes.len(), || {
        let _ = OneBytes::decode(single_bytes_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    // ---- Vec<Enum> ----
    run_component_bench(GROUP, &mut group, "codes_len1 | prost decode", one_enum_vec_bytes.len(), || {
        let _ = BenchCodesProst::decode(one_enum_vec_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "codes_len1 | proto_rs decode", one_enum_vec_bytes.len(), || {
        let _ = BenchCodes::decode(one_enum_vec_bytes.as_slice(), DecodeContext::default()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "one_enum | prost decode", single_enum_bytes.len(), || {
        let _ = OneEnumProst::decode(single_enum_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "one_enum | proto_rs decode", single_enum_bytes.len(), || {
        let _ = OneEnum::decode(single_enum_bytes.as_slice(), DecodeContext::default()).unwrap();
    });

    // ---- Vec<NestedLeaf> ----
    run_component_bench(GROUP, &mut group, "leaves_len1 | prost decode", one_leaf_vec_bytes.len(), || {
        let _ = BenchNestedLeafListProst::decode(one_leaf_vec_bytes.as_slice()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "leaves_len1 | proto_rs decode", one_leaf_vec_bytes.len(), || {
        let _ = BenchNestedLeafList::decode(one_leaf_vec_bytes.as_slice(), DecodeContext::default()).unwrap();
    });
    run_component_bench(GROUP, &mut group, "one_nested_leaf | prost decode", single_leaf_bytes.len(), || {
        let _ = OneNestedLeafProst::decode(single_leaf_bytes.as_slice()).unwrap();
    });
    run_component_bench(
        GROUP,
        &mut group,
        "one_nested_leaf | proto_rs decode",
        single_leaf_bytes.len(),
        || {
            let _ = OneNestedLeaf::decode(single_leaf_bytes.as_slice(), DecodeContext::default()).unwrap();
        },
    );

    // ---- Vec<DeepMessage> ----
    run_component_bench(GROUP, &mut group, "deep_list_len1 | prost decode", one_deep_vec_bytes.len(), || {
        let _ = BenchDeepMessageListProst::decode(one_deep_vec_bytes.as_slice()).unwrap();
    });
    run_component_bench(
        GROUP,
        &mut group,
        "deep_list_len1 | proto_rs decode",
        one_deep_vec_bytes.len(),
        || {
            let _ = BenchDeepMessageList::decode(one_deep_vec_bytes.as_slice(), DecodeContext::default()).unwrap();
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "one_deep_message | prost decode",
        single_deep_bytes.len(),
        || {
            let _ = OneDeepMessageProst::decode(single_deep_bytes.as_slice()).unwrap();
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "one_deep_message | proto_rs decode",
        single_deep_bytes.len(),
        || {
            let _ = OneDeepMessage::decode(single_deep_bytes.as_slice(), DecodeContext::default()).unwrap();
        },
    );
    // ---- Vec<ComplexEnum> ----
    run_component_bench(
        GROUP,
        &mut group,
        "status_history_len1 | prost decode",
        one_ce_vec_bytes.len(),
        || {
            let _ = BenchStatusHistoryProst::decode(one_ce_vec_bytes.as_slice()).unwrap();
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "status_history_len1 | proto_rs decode",
        one_ce_vec_bytes.len(),
        || {
            let _ = BenchStatusHistory::decode(one_ce_vec_bytes.as_slice(), DecodeContext::default()).unwrap();
        },
    );
    run_component_bench(GROUP, &mut group, "one_complex_enum | prost decode", single_ce_bytes.len(), || {
        let _ = OneComplexEnumProst::decode(single_ce_bytes.as_slice()).unwrap();
    });
    run_component_bench(
        GROUP,
        &mut group,
        "one_complex_enum | proto_rs decode",
        single_ce_bytes.len(),
        || {
            let _ = OneComplexEnum::decode(single_ce_bytes.as_slice(), DecodeContext::default()).unwrap();
        },
    );

    // ---- Maps (1 entry) ----
    run_component_bench(
        GROUP,
        &mut group,
        "leaf_lookup_len1 | prost decode",
        one_leaf_map_bytes.len(),
        || {
            let _ = BenchLeafLookupProst::decode(one_leaf_map_bytes.as_slice()).unwrap();
        },
    );
    run_component_bench(
        GROUP,
        &mut group,
        "leaf_lookup_len1 | proto_rs decode",
        one_leaf_map_bytes.len(),
        || {
            let _ = BenchLeafLookup::decode(one_leaf_map_bytes.as_slice(), DecodeContext::default()).unwrap();
        },
    );

    group.finish();
}

fn main() {
    use criterion::Criterion;

    let mut c = Criterion::default().configure_from_args();

    bench_encode_decode(&mut c);
    bench_zero_copy_vs_prost(&mut c);
    bench_complex_components(&mut c);

    bench_micro_fields_encode(&mut c);
    bench_complex_components_decode(&mut c);
    bench_micro_fields_decode(&mut c);
    bench_collection_overhead_decode(&mut c);
    bench_collection_overhead_encode(&mut c);

    c.final_summary();
    bench_recorder().write_markdown().unwrap();
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
    pub payload: Vec<u8>,
    pub attachments: Vec<Vec<u8>>,
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
    pub blob: Vec<u8>,
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
    pub payload: Vec<u8>,
    pub leaves: Vec<NestedLeaf>,
    pub deep_list: Vec<DeepMessage>,
    pub leaf_lookup: HashMap<String, NestedLeaf>,
    pub deep_lookup: HashMap<String, DeepMessage>,
    pub status: ComplexEnum,
    pub status_history: Vec<ComplexEnum>,
    pub status_lookup: HashMap<String, ComplexEnum>,
    pub codes: Vec<SimpleEnum>,
    pub code_lookup: HashMap<String, SimpleEnum>,
    pub attachments: Vec<Vec<u8>>,
    pub tags: Vec<String>,
    pub count: i64,
    pub ratio: f64,
    pub active: bool,
    pub big_numbers: Vec<u64>,
    pub audit_log: HashMap<String, DeepMessage>,
    pub primary_focus: Option<Box<NestedLeaf>>,
    pub secondary_focus: Option<Box<DeepMessage>>,
}

// Single-field micro messages
#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct OneString {
    pub v: String,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct OneBytes {
    pub v: Vec<u8>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct OneEnum {
    pub v: SimpleEnum,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct OneNestedLeaf {
    pub v: NestedLeaf,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct OneDeepMessage {
    pub v: DeepMessage,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct OneComplexEnum {
    pub v: ComplexEnum,
}

// Prost equivalents
#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct OneStringProst {
    #[prost(string, tag = "1")]
    pub v: String,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct OneBytesProst {
    #[prost(bytes, tag = "1")]
    pub v: Vec<u8>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct OneEnumProst {
    #[prost(enumeration = "SimpleEnumProst", tag = "1")]
    pub v: i32,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct OneNestedLeafProst {
    #[prost(message, tag = "1")]
    pub v: Option<NestedLeafProst>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct OneDeepMessageProst {
    #[prost(message, tag = "1")]
    pub v: Option<DeepMessageProst>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct OneComplexEnumProst {
    #[prost(message, tag = "1")]
    pub v: Option<ComplexEnumProst>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BenchNestedLeafList {
    pub items: Vec<NestedLeaf>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BenchDeepMessageList {
    pub items: Vec<DeepMessage>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BenchLeafLookup {
    pub entries: HashMap<String, NestedLeaf>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BenchDeepLookup {
    pub entries: HashMap<String, DeepMessage>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BenchStatusHistory {
    pub items: Vec<ComplexEnum>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BenchStatusLookup {
    pub entries: HashMap<String, ComplexEnum>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BenchAttachments {
    pub items: Vec<Vec<u8>>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BenchAuditLog {
    pub entries: HashMap<String, DeepMessage>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BenchCodes {
    pub items: Vec<SimpleEnum>,
}

#[proto_message(proto_path = "protos/bench/complex.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BenchTags {
    pub items: Vec<String>,
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
pub struct BenchNestedLeafListProst {
    #[prost(message, repeated, tag = "1")]
    pub items: Vec<NestedLeafProst>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct BenchDeepMessageListProst {
    #[prost(message, repeated, tag = "1")]
    pub items: Vec<DeepMessageProst>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct BenchLeafLookupProst {
    #[prost(map = "string, message", tag = "1")]
    pub entries: HashMap<String, NestedLeafProst>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct BenchDeepLookupProst {
    #[prost(map = "string, message", tag = "1")]
    pub entries: HashMap<String, DeepMessageProst>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct BenchStatusHistoryProst {
    #[prost(message, repeated, tag = "1")]
    pub items: Vec<ComplexEnumProst>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct BenchStatusLookupProst {
    #[prost(map = "string, message", tag = "1")]
    pub entries: HashMap<String, ComplexEnumProst>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct BenchAttachmentsProst {
    #[prost(bytes, repeated, tag = "1")]
    pub items: Vec<Vec<u8>>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct BenchAuditLogProst {
    #[prost(map = "string, message", tag = "1")]
    pub entries: HashMap<String, DeepMessageProst>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct BenchCodesProst {
    #[prost(enumeration = "SimpleEnumProst", repeated, tag = "1")]
    pub items: Vec<i32>,
}

#[derive(Clone, PartialEq, prost::Message)]
#[prost(message, package = "bench_types")]
pub struct BenchTagsProst {
    #[prost(string, repeated, tag = "1")]
    pub items: Vec<String>,
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
        payload: vec![id as u8, (id + 1) as u8, (id + 2) as u8],
        attachments: vec![vec![1, 2, 3, id as u8], vec![4, 5, 6, (id + 1) as u8]],
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
        blob: vec![7, 8, 9, base as u8],
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
        payload: b"complex-payload".to_vec(),
        leaves: vec![main_leaf.clone(), aux_leaf.clone()],
        deep_list: vec![deep_primary.clone(), deep_secondary.clone()],
        leaf_lookup: HashMap::from([("main".into(), main_leaf.clone()), ("aux".into(), aux_leaf.clone())]),
        deep_lookup: HashMap::from([
            ("primary".into(), deep_primary.clone()),
            ("secondary".into(), deep_secondary.clone()),
        ]),
        status: ComplexEnum::Details(ExtraDetails {
            description: "aggregated".into(),
            counters: HashMap::from([("total".into(), 5u32), ("errors".into(), 1u32)]),
        }),
        status_history: vec![
            ComplexEnum::Leaf(main_leaf.clone()),
            ComplexEnum::Deep(deep_secondary.clone()),
            ComplexEnum::Empty(ComplexEnumEmpty {}),
        ],
        status_lookup: HashMap::from([
            ("ready".into(), ComplexEnum::Leaf(main_leaf.clone())),
            ("processing".into(), ComplexEnum::Deep(deep_primary.clone())),
        ]),
        codes: vec![SimpleEnum::Alpha, SimpleEnum::Beta, SimpleEnum::Delta],
        code_lookup: HashMap::from([("alpha".into(), SimpleEnum::Alpha), ("gamma".into(), SimpleEnum::Gamma)]),
        attachments: vec![b"attachment-a".to_vec(), b"attachment-b".to_vec(), vec![0, 1, 2, 3]],
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
