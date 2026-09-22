//! TLS slot costs, overlapping ownership, per-thread pools and remote returns.
use std::hint::black_box;
use std::sync::Barrier;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use proto_rs::EncodePoolConfig;
use proto_rs::EncodedSnapshot;
use proto_rs::configure_encode_pool;

fn hold_slots(count: usize) -> Vec<EncodedSnapshot<u64>> {
    (0..count).map(|_| EncodedSnapshot::new(&42u64)).collect()
}

fn bench(c: &mut Criterion) {
    const WORKERS: usize = 4;
    const BATCH: usize = 64;
    let config = EncodePoolConfig {
        max_buffers: std::env::var("PROTO_RS_BENCH_POOL_SLOTS").map_or(8, |v| v.parse().unwrap()),
        max_buffer_capacity: std::env::var("PROTO_RS_BENCH_POOL_CAP").map_or(32 * 1024 * 1024, |v| v.parse().unwrap()),
    };
    configure_encode_pool(config).unwrap();
    let mut group = c.benchmark_group("encode_pool");
    for size in [16, 16 * 1024] {
        let value = vec![42u8; size];
        group.throughput(Throughput::Elements(1));
        group.bench_function(BenchmarkId::new("tls_warm", size), |b| {
            b.iter(|| black_box(EncodedSnapshot::new(black_box(&value))));
        });
        let held = EncodedSnapshot::new(&value);
        group.bench_function(BenchmarkId::new("overlapping_owner", size), |b| {
            b.iter(|| black_box(EncodedSnapshot::new(black_box(&value))));
        });
        drop(held);
        group.bench_function(BenchmarkId::new("exhausted", size), |b| {
            let _held = hold_slots(config.max_buffers);
            b.iter(|| black_box(EncodedSnapshot::new(black_box(&value))));
        });

        // Four workers on independent TLS pools, not one artificially shared
        // pool. Setup and first-touch warming finish before timing starts.
        group.throughput(Throughput::Elements((WORKERS * BATCH) as u64));
        for exhausted in [false, true] {
            let name = if exhausted { "parallel_exhausted" } else { "parallel_tls" };
            group.bench_function(BenchmarkId::new(name, size), |b| {
                b.iter_custom(|iterations| {
                    let ready = &Barrier::new(WORKERS + 1);
                    let go = &Barrier::new(WORKERS + 1);
                    std::thread::scope(|scope| {
                        let mut workers = Vec::new();
                        for _ in 0..WORKERS {
                            let value = &value;
                            workers.push(scope.spawn(move || {
                                let _held = hold_slots(if exhausted { config.max_buffers } else { 0 });
                                for _ in 0..config.max_buffers {
                                    drop(EncodedSnapshot::new(value));
                                }
                                ready.wait();
                                go.wait();
                                for _ in 0..iterations {
                                    for _ in 0..BATCH {
                                        black_box(EncodedSnapshot::new(value));
                                    }
                                }
                            }));
                        }
                        ready.wait();
                        let start = Instant::now();
                        go.wait();
                        for worker in workers {
                            worker.join().unwrap();
                        }
                        start.elapsed()
                    })
                });
            });
        }

        // Producer leases from its TLS pool. Consumer releases the originating
        // slot on another thread; channel costs are included.
        group.throughput(Throughput::Elements(1));
        for exhausted in [false, true] {
            let name = if exhausted { "remote_exhausted" } else { "remote_return" };
            group.bench_function(BenchmarkId::new(name, size), |b| {
                let _held = hold_slots(if exhausted { config.max_buffers } else { 0 });
                b.iter_custom(|iterations| {
                    let ready = &Barrier::new(2);
                    std::thread::scope(|scope| {
                        let (sender, receiver) = mpsc::sync_channel::<EncodedSnapshot<Vec<u8>>>(32);
                        let worker = scope.spawn(move || {
                            ready.wait();
                            for snapshot in receiver {
                                drop(black_box(snapshot));
                            }
                        });
                        ready.wait();
                        let start = Instant::now();
                        for _ in 0..iterations {
                            sender.send(EncodedSnapshot::new(&value)).unwrap();
                        }
                        drop(sender);
                        worker.join().unwrap();
                        start.elapsed()
                    })
                });
            });
        }
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(30).warm_up_time(Duration::from_secs(1)).measurement_time(Duration::from_secs(3));
    targets = bench
}
criterion_main!(benches);
