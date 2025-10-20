
# Benchmark Run — 2025-10-20 21:42:20

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 95013.76 | 342.33 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 101135.23 | 365.35 | 1.06× faster |
| complex_root_decode | prost decode canonical input | 66307.31 | 238.90 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 64214.27 | 231.98 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 63546.99 | 228.96 | 0.96× slower |
| complex_root_decode | proto_rs decode proto_rs input | 64759.86 | 233.95 | 1.00× |
| complex_root_encode | prost encode_to_vec | 193926.95 | 698.72 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 107341.78 | 387.77 | 0.55× slower |


# Benchmark Run — 2025-10-20 20:11:32

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 93642.77 | 337.39 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 104567.29 | 377.75 | 1.12× faster |
| complex_root_decode | prost decode canonical input | 67435.56 | 242.97 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 66580.17 | 240.52 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 66198.03 | 238.51 | 0.98× slower |
| complex_root_decode | proto_rs decode proto_rs input | 65692.69 | 237.32 | 0.99× slower |
| complex_root_encode | prost encode_to_vec | 199851.16 | 720.06 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 110199.33 | 398.10 | 0.55× slower |


# Benchmark Run — 2025-10-20 17:07:04

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 94175.62 | 339.31 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 75079.58 | 287.12 | 0.80× slower |
| complex_root_decode | prost decode canonical input | 59733.27 | 215.22 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 58615.63 | 224.16 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 60461.76 | 217.84 | 1.01× faster |
| complex_root_decode | proto_rs decode proto_rs input | 59219.25 | 226.47 | 1.01× faster |
| complex_root_encode | prost encode_to_vec | 191876.75 | 691.33 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 76167.30 | 291.28 | 0.40× slower |



