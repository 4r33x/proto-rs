
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

