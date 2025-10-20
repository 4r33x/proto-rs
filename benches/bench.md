
# Benchmark Run — 2025-10-20 14:41:01

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 94633.26 | 340.96 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 74510.01 | 284.94 | 0.79× slower |
| complex_root_decode | prost decode canonical input | 61067.79 | 220.03 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 59053.67 | 225.84 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 61743.66 | 222.46 | 1.01× faster |
| complex_root_decode | proto_rs decode proto_rs input | 61204.18 | 234.06 | 1.04× faster |
| complex_root_encode | prost encode_to_vec | 201130.08 | 724.67 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 79384.11 | 303.58 | 0.39× slower |


# Benchmark Run — 2025-10-20 14:38:35

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 97350.76 | 350.75 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 76292.24 | 291.76 | 0.78× slower |
| complex_root_decode | prost decode canonical input | 61954.23 | 223.22 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 60489.62 | 231.33 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 61480.86 | 221.51 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 62408.33 | 238.66 | 1.03× faster |
| complex_root_encode | prost encode_to_vec | 198134.82 | 713.88 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 78287.68 | 299.39 | 0.40× slower |


# Benchmark Run — 2025-10-20 14:34:54

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 95052.29 | 342.47 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 75130.64 | 287.32 | 0.79× slower |
| complex_root_decode | prost decode canonical input | 61134.44 | 220.27 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 58869.93 | 225.13 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 60505.68 | 218.00 | 0.99× slower |
| complex_root_decode | proto_rs decode proto_rs input | 61618.45 | 235.64 | 1.05× faster |
| complex_root_encode | prost encode_to_vec | 198232.07 | 714.23 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 77427.56 | 296.10 | 0.39× slower |


# Benchmark Run — 2025-10-20 14:27:40

| Group | Benchmark | Ops / µs | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 0.094365 | 340.00 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 0.074328 | 284.25 | 0.79× slower |
| complex_root_decode | prost decode canonical input | 0.062023 | 223.47 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 0.059920 | 229.15 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 0.061822 | 222.74 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 0.059547 | 227.72 | 1.00× |
| complex_root_encode | prost encode_to_vec | 0.196015 | 706.24 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 0.074815 | 286.11 | 0.38× slower |


# Benchmark Run — 2025-10-20 14:25:35

| Group | Benchmark | Ops / µs | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 0.093056 | 335.28 | 1000.00× faster |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 0.076541 | 292.71 | 822.53× faster |
| complex_root_decode | prost decode canonical input | 0.060019 | 216.25 | 1000.00× faster |
| complex_root_decode | prost decode proto_rs input | 0.060330 | 230.72 | 1000.00× faster |
| complex_root_decode | proto_rs decode canonical input | 0.060323 | 217.34 | 1005.07× faster |
| complex_root_decode | proto_rs decode proto_rs input | 0.060362 | 230.84 | 1000.52× faster |
| complex_root_encode | prost encode_to_vec | 0.191954 | 691.61 | 1000.00× faster |
| complex_root_encode | proto_rs encode_to_vec | 0.076065 | 290.89 | 396.27× faster |


# Benchmark Run — 2025-10-20 14:22:53

| Group | Benchmark | Ops / us | Speedup vs Prost |
| --- | --- | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 0.094761 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 0.072897 | 0.77× slower |
| complex_root_decode | prost decode canonical input | 0.063276 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 0.061988 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 0.063063 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 0.062366 | 1.00× |
| complex_root_encode | prost encode_to_vec | 0.200013 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 0.078947 | 0.39× slower |



