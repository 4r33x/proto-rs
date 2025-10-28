use std::hint::black_box;
use std::mem;

use bytes::Buf;
use criterion::Criterion;
use criterion::Throughput;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

fn benchmark_varint_prost(criterion: &mut Criterion, name: &str, mut values: Vec<u64>) {
    use prost::encoding::varint::decode_varint;
    use prost::encoding::varint::encode_varint;
    use prost::encoding::varint::encoded_len_varint;
    // Shuffle the values in a stable order.
    values.shuffle(&mut StdRng::seed_from_u64(0));
    let name = format!("prost_varint/{name}");

    let encoded_len = values.iter().copied().map(encoded_len_varint).sum::<usize>() as u64;
    let decoded_len = (values.len() * mem::size_of::<u64>()) as u64;

    criterion
        .benchmark_group(&name)
        .bench_function("encode", {
            let encode_values = values.clone();
            move |b| {
                let mut buf = Vec::<u8>::with_capacity(encode_values.len() * 10);
                b.iter(|| {
                    buf.clear();
                    for &value in &encode_values {
                        encode_varint(value, &mut buf);
                    }
                    black_box(&buf);
                });
            }
        })
        .throughput(Throughput::Bytes(encoded_len));

    criterion
        .benchmark_group(&name)
        .bench_function("decode", {
            let decode_values = values.clone();

            move |b| {
                let mut buf = Vec::with_capacity(decode_values.len() * 10);
                for &value in &decode_values {
                    encode_varint(value, &mut buf);
                }

                b.iter(|| {
                    let mut buf = &mut buf.as_slice();
                    while buf.has_remaining() {
                        let result = decode_varint(&mut buf);
                        debug_assert!(result.is_ok());
                        black_box(&result);
                    }
                });
            }
        })
        .throughput(Throughput::Bytes(decoded_len));

    criterion
        .benchmark_group(&name)
        .bench_function("encoded_len", move |b| {
            b.iter(|| {
                let mut sum = 0;
                for &value in &values {
                    sum += encoded_len_varint(value);
                }
                black_box(sum);
            });
        })
        .throughput(Throughput::Bytes(decoded_len));
}

fn benchmark_varint_proto(criterion: &mut Criterion, name: &str, mut values: Vec<u64>) {
    // use proto_rs::encoding::encode_padded_varint;
    use proto_rs::encoding::varint::decode_varint;
    use proto_rs::encoding::varint::encode_varint;
    use proto_rs::encoding::varint::encoded_len_varint;
    // Shuffle the values in a stable order.
    values.shuffle(&mut StdRng::seed_from_u64(0));
    let name = format!("proto_varint/{name}");

    let encoded_len = values.iter().copied().map(encoded_len_varint).sum::<usize>() as u64;
    let decoded_len = (values.len() * mem::size_of::<u64>()) as u64;

    criterion
        .benchmark_group(&name)
        .bench_function("encode", {
            let encode_values = values.clone();
            move |b| {
                let mut buf = Vec::<u8>::with_capacity(encode_values.len() * 10);
                b.iter(|| {
                    buf.clear();
                    for &value in &encode_values {
                        encode_varint(value, &mut buf);
                    }
                    black_box(&buf);
                });
            }
        })
        .throughput(Throughput::Bytes(encoded_len));
    // criterion
    //     .benchmark_group(&name)
    //     .bench_function("encode_padded", {
    //         let encode_values = values.clone();
    //         move |b| {
    //             let mut buf = Vec::<u8>::with_capacity(encode_values.len() * 10);
    //             b.iter(|| {
    //                 buf.clear();
    //                 for &value in &encode_values {
    //                     unsafe { encode_padded_varint(value, &mut buf) };
    //                 }
    //                 black_box(&buf);
    //             });
    //         }
    //     })
    //     .throughput(Throughput::Bytes(encoded_len));

    criterion
        .benchmark_group(&name)
        .bench_function("decode", {
            let decode_values = values.clone();

            move |b| {
                let mut buf = Vec::with_capacity(decode_values.len() * 10);
                for &value in &decode_values {
                    encode_varint(value, &mut buf);
                }

                b.iter(|| {
                    let mut buf = &mut buf.as_slice();
                    while buf.has_remaining() {
                        let result = decode_varint(&mut buf);
                        debug_assert!(result.is_ok());
                        black_box(&result);
                    }
                });
            }
        })
        .throughput(Throughput::Bytes(decoded_len));

    criterion
        .benchmark_group(&name)
        .bench_function("encoded_len", move |b| {
            b.iter(|| {
                let mut sum = 0;
                for &value in &values {
                    sum += encoded_len_varint(value);
                }
                black_box(sum);
            });
        })
        .throughput(Throughput::Bytes(decoded_len));
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();

    // Benchmark encoding and decoding 100 small (1 byte) varints.
    benchmark_varint_proto(&mut criterion, "small", (0..100).collect());
    benchmark_varint_prost(&mut criterion, "small", (0..100).collect());

    // Benchmark encoding and decoding 100 medium (5 byte) varints.
    benchmark_varint_prost(&mut criterion, "medium", (1 << 28..).take(100).collect());
    benchmark_varint_proto(&mut criterion, "medium", (1 << 28..).take(100).collect());

    // Benchmark encoding and decoding 100 large (10 byte) varints.
    benchmark_varint_prost(&mut criterion, "large", (1 << 63..).take(100).collect());
    benchmark_varint_proto(&mut criterion, "large", (1 << 63..).take(100).collect());

    // Benchmark encoding and decoding 100 varints of mixed width (average 5.5 bytes).
    benchmark_varint_prost(
        &mut criterion,
        "mixed",
        (0..10)
            .flat_map(move |width| {
                let exponent = width * 7;
                (0..10).map(move |offset| offset + (1 << exponent))
            })
            .collect(),
    );
    benchmark_varint_proto(
        &mut criterion,
        "mixed",
        (0..10)
            .flat_map(move |width| {
                let exponent = width * 7;
                (0..10).map(move |offset| offset + (1 << exponent))
            })
            .collect(),
    );

    criterion.final_summary();
}
