
# Benchmark Run — 2025-10-20 13:54:54

| Group | Benchmark | Avg ns/op | Avg µs/op | MiB/s |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_prost | prost clone + encode | 10151.39 | 10.15 | 354.92 |
| bench_zero_copy_vs_prost | proto_rs zero_copy response | 12889.74 | 12.89 | 296.69 |
| complex_root_decode | prost decode prost input | 16959.48 | 16.96 | 212.45 |
| complex_root_decode | prost decode proto_rs input | 17234.97 | 17.23 | 221.89 |
| complex_root_decode | proto_rs decode prost input | 16970.75 | 16.97 | 212.31 |
| complex_root_decode | proto_rs decode proto_rs input | 16528.24 | 16.53 | 231.38 |
| complex_root_encode | prost encode_to_vec | 5533.61 | 5.53 | 651.11 |
| complex_root_encode | proto_rs encode_to_vec | 12455.81 | 12.46 | 307.02 |


# Benchmark Run — 2025-10-20 12:05:28

| Group | Benchmark | Avg ns/op | Avg µs/op | MiB/s |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_prost | prost clone + encode | 15796.39 | 15.80 | 228.09 |
| bench_zero_copy_vs_prost | proto_rs zero_copy response | 22843.97 | 22.84 | 158.14 |
| complex_root_decode | prost decode prost input | 27133.59 | 27.13 | 132.79 |
| complex_root_decode | prost decode proto_rs input | 24599.52 | 24.60 | 146.85 |
| complex_root_decode | proto_rs decode prost input | 25130.37 | 25.13 | 143.37 |
| complex_root_decode | proto_rs decode proto_rs input | 24455.96 | 24.46 | 147.72 |
| complex_root_encode | prost encode_to_vec | 8668.30 | 8.67 | 415.65 |
| complex_root_encode | proto_rs encode_to_vec | 16792.53 | 16.79 | 215.13 |

