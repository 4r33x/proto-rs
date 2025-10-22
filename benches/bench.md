
# Benchmark Run — 2025-10-21 15:36:11

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 89524.66 | 322.56 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 95119.11 | 343.62 | 1.06× faster |
| complex_root_decode | prost decode canonical input | 53689.55 | 193.44 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 55255.80 | 199.61 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 53540.67 | 192.91 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 53192.17 | 192.16 | 0.96× slower |
| complex_root_encode | prost encode_to_vec | 168142.38 | 605.81 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 94341.34 | 340.81 | 0.56× slower |


# Benchmark Run — 2025-10-21 01:12:12

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 109647.84 | 395.06 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 110285.17 | 398.41 | 1.00× |
| complex_root_decode | prost decode canonical input | 69234.10 | 249.45 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 70406.07 | 254.34 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 70865.64 | 255.33 | 1.02× faster |
| complex_root_decode | proto_rs decode proto_rs input | 68902.35 | 248.91 | 0.98× slower |
| complex_root_encode | prost encode_to_vec | 214757.11 | 773.77 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 115589.22 | 417.57 | 0.54× slower |


# Benchmark Run — 2025-10-21 01:10:01

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 97002.81 | 349.50 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 101057.34 | 365.07 | 1.04× faster |
| complex_root_decode | prost decode canonical input | 64591.35 | 232.72 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 62932.93 | 227.35 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 64613.97 | 232.80 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 63955.13 | 231.04 | 1.02× faster |
| complex_root_encode | prost encode_to_vec | 179887.48 | 648.13 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 103852.52 | 375.17 | 0.58× slower |


# Benchmark Run — 2025-10-21 01:08:13

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 91809.69 | 330.79 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 94787.02 | 342.42 | 1.03× faster |
| complex_root_decode | prost decode canonical input | 62656.21 | 225.75 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 60727.19 | 219.38 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 60597.47 | 218.33 | 0.97× slower |
| complex_root_decode | proto_rs decode proto_rs input | 60609.33 | 218.95 | 1.00× |
| complex_root_encode | prost encode_to_vec | 166972.76 | 601.60 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 101570.52 | 366.93 | 0.61× slower |


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



