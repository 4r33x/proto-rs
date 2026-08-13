use std::hint::black_box;
use std::mem;

use bytes::Buf;
use criterion::Criterion;
use criterion::Throughput;
use proto_rs::DecodeContext;
use proto_rs::ProtoDecode;
use proto_rs::ProtoEncode;
use proto_rs::proto_message;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

#[proto_message]
#[derive(Debug, Default, PartialEq)]
struct PackedVarints {
    values: Vec<u64>,
}

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

fn benchmark_packed_varint_decode(criterion: &mut Criterion, name: &str, values: Vec<u64>) {
    let encoded = PackedVarints::encode_to_vec(&PackedVarints { values });
    let mut group = criterion.benchmark_group(format!("packed_varint_decode/{name}"));
    group.throughput(Throughput::Bytes(encoded.len() as u64));
    group.bench_function("full_message", |b| {
        b.iter(|| {
            let decoded = PackedVarints::decode(black_box(encoded.as_slice()), DecodeContext::default());
            debug_assert!(decoded.is_ok());
            black_box(decoded)
        });
    });
    group.finish();
}

fn benchmark_tag_decode(criterion: &mut Criterion, name: &str, tags: &[u32]) {
    use proto_rs::encoding::WireType;
    use proto_rs::encoding::decode_key;
    use proto_rs::encoding::encode_key;

    let mut encoded = Vec::new();
    for &tag in tags {
        encode_key(tag, WireType::Varint, &mut encoded);
    }

    let mut group = criterion.benchmark_group(format!("tag_decode/{name}"));
    group.throughput(Throughput::Elements(tags.len() as u64));
    group.bench_function("decode_key", |b| {
        b.iter(|| {
            let mut remaining = encoded.as_slice();
            while remaining.has_remaining() {
                black_box(decode_key(&mut remaining).unwrap());
            }
        });
    });
    group.finish();
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

    benchmark_packed_varint_decode(&mut criterion, "one_byte", (0..128).cycle().take(4096).collect());
    benchmark_packed_varint_decode(&mut criterion, "two_byte", (128..16_384).cycle().take(4096).collect());
    benchmark_packed_varint_decode(&mut criterion, "five_byte", (1 << 28..).take(4096).collect());
    benchmark_packed_varint_decode(&mut criterion, "ten_byte", (1 << 63..).take(4096).collect());

    benchmark_tag_decode(&mut criterion, "one_byte", &(1..=15).cycle().take(4095).collect::<Vec<_>>());
    benchmark_tag_decode(&mut criterion, "two_byte", &(16..=31).cycle().take(4096).collect::<Vec<_>>());

    criterion.final_summary();
}
