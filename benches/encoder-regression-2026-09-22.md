# Encoder regression subset — 2026-09-22

This run checks encoder performance after the optional Linux owned-buffer
transport was added. No encoder code was changed during this check.

**Verdict using the requested ±4% practical noise band:** no repeatable regression
in the selected current direct-encoder cases (16 B through 64 MiB byte payloads)
or raw protobuf cases. Some legacy scratch controls exceed that band; those
unconfirmed historical slowdowns are recorded separately below.

## Method

- Compared against the existing September 22 post-optimization Criterion `base`
  measurements, not the much older August 13 pre-optimization results.
- Used `--baseline base` throughout, preserving those saved baselines.
- Treat relative mean changes within ±4% as noise in either direction, as
  requested. The original console logs use Criterion's default reporting
  threshold; their smaller "improved/regressed" labels are not the practical
  verdict here. Future runs can set `--noise-threshold 0.04` explicitly.
- Initial runs enabled `linux-zerocopy,tonic-gzip`, with default nightly features,
  rustc 1.100.0-nightly (bba531001), fat LTO, and one codegen unit.
- Ran timed targets sequentially. No network, TLS, gzip compression, decode-only
  or varint benchmarks were measured. Tonic cases include output allocation,
  framing and body polling; raw cases measure `encode_to_vec`.
- Selected 21 unique cases: six Tonic direct cases and six scratch controls,
  three 64 MiB body variants, and three raw encoders with matched Prost controls.
  Repeated the two initially slower direct/raw cases rather than expanding to the
  full suite.

Tonic: 50 samples, 1 s warmup, 3 s measurement. Small-stream repeats: 100 samples,
2 s warmup, 5 s measurement. Raw encoders: 100 samples, 3 s warmup, 5 s measurement.
64 MiB body: 10 samples, 100 ms warmup, 1 s requested measurement.

## Direct Tonic encoder

Sizes describe the byte field; messages also contain scalar/string/list fields.
Streaming timings cover the whole batch, not one message.

| Bytes field / batch | Saved baseline | Current point estimate | Verdict with ±4% noise band |
| --- | ---: | ---: | --- |
| 16 B × 1 | 68.904 ns | 67.257 ns | Noise |
| 16 B × 128 | 2.433 µs | 2.520 µs; repeat 2.492 µs | Noise (+2.39% mean on repeat) |
| 16 KiB × 1 | 177.69 ns | 180.51 ns | Noise |
| 16 KiB × 128 | 11.578 µs | 11.592 µs | Noise |
| 256 KiB × 1 | 1.950 µs | 1.897 µs | Noise |
| 256 KiB × 16 | 42.392 µs | 28.903 µs | Faster; historically variable case |
| 64 MiB, 20-record vector | 3.772 ms | 3.729 ms | Noise |

Table values are Criterion point estimates (slope when available). Criterion's
relative-change test compares resampled means, so its percentages need not equal
the ratio of the displayed slope estimates. Small-stream repeat mean change:
**+2.08% to +2.69%** at 95% confidence, point estimate +2.39%.

A further small-stream run with both opt-in transport/gzip features disabled
measured **2.501 µs**, mean change +2.67% (95% interval +2.43% to +2.90%). That is
also noise under the requested band and does not isolate a transport-feature
regression. The unchanged scratch control in that run measured +0.74% mean change.

The 256 KiB streaming benchmark already varied roughly 28–42 µs across earlier
builds/runs. Its current improvement is not evidence that the socket transport
accelerated encoding; no socket participates in these measurements.

The scratch-path controls were slower than the direct encoder in all six matched
cases. However, versus their own historical baselines, scratch 16 KiB unary,
256 KiB unary and 256 KiB streaming measured approximately +7.1%, +12.4% and
+5.4% by Criterion's mean comparison. Those control regressions were not repeated
or attributed to a source change; they must not be described as a clean
whole-encoder-suite no-regression result.

For the 64 MiB fixture, scratch measured 14.747 ms and the default 1 MiB-cap
direct/spill path measured 13.527 ms; neither was classified as a regression.
The 3.729 ms direct case uses the existing 128 MiB reservation cap.

## Raw protobuf encoders

Sizes below are encoded wire sizes. Each case retained its existing fixture and
performed the benchmark's size-compatibility checks against Prost.

| Case | Wire size | proto_rs baseline | proto_rs current | Matched Prost current |
| --- | ---: | ---: | ---: | ---: |
| Nested leaf | 34 B | 27.664 ns | 24.044 ns | 36.545 ns |
| Deep message | 345 B | 195.72 ns | 186.80 ns | 270.36 ns |
| Complex root, first run | 3,778 B | 2.041 µs | 2.141 µs | 3.840 µs |
| Complex root, repeat | 3,778 B | 2.041 µs | 1.995 µs | 3.909 µs |

The leaf encoder improved (−6.68% mean); the deep-message change (−2.34% mean)
is noise under the requested band.
Complex-root initially showed +4.48% in the relative-mean test, but the identical
binary, narrowed-filter repeat showed −3.17% (noise); that slowdown did **not** reproduce. Both
runs are retained here rather than selecting only the favorable measurement.
proto_rs remained faster than Prost in every selected matched raw case.

## Reproduction

```sh
cargo bench -p proto_rs --bench tonic_encode \
  --features linux-zerocopy,tonic-gzip --locked --offline -- --baseline base

PROTO_RS_BENCH_NO_NETWORK=1 cargo bench -p proto_rs --bench bytes_pipeline \
  --features linux-zerocopy,tonic-gzip --locked --offline -- \
  'body/plain' --baseline base

cargo bench -p bench_runner --bench main_bench \
  --features proto_rs/linux-zerocopy,proto_rs/tonic-gzip --locked --offline -- \
  '^(micro_fields_encode/one_nested_leaf \| (proto_rs|prost) encode_to_vec|complex_root_components_encode/deep_message \| (proto_rs|prost) encode_to_vec|complex_root_encode_decode/(proto_rs|prost) encode_to_vec)$' \
  --baseline base

# Longer small-stream repeat, with features enabled:
cargo bench -p proto_rs --bench tonic_encode \
  --features linux-zerocopy,tonic-gzip --locked --offline -- \
  small_stream --baseline base --sample-size 100 --warm-up-time 2 --measurement-time 5

# Repeat raw complex root with the same features and saved baseline:
cargo bench -p bench_runner --bench main_bench \
  --features proto_rs/linux-zerocopy,proto_rs/tonic-gzip --locked --offline -- \
  '^complex_root_encode_decode/(proto_rs|prost) encode_to_vec$' --baseline base

# Small-stream check with default features only:
cargo bench -p proto_rs --bench tonic_encode --locked --offline -- \
  small_stream --baseline base --sample-size 100 --warm-up-time 2 --measurement-time 5
```

These commands require the historical Criterion baselines in `target/criterion`.
Without them, omit `--baseline base` to establish measurements, not to reproduce
the historical regression comparison. Raw-run aggregate output was also recorded
by the existing runner to [bench.md](bench.md).
