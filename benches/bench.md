
# Benchmark Run — 2025-10-20 12:02:13

| Group | Benchmark | Avg ns/op | Avg µs/op | MiB/s | Rel to Prost - lower is better |
| --- | --- | ---: | ---: | ---: | ---: |
| bench_zero_copy_vs_prost | prost clone + encode | 10699.86 | 10.70 | 336.73 | 1.00× |
| bench_zero_copy_vs_prost | proto_rs zero_copy response | 14241.55 | 14.24 | 268.53 | 1.33× |
| complex_root_decode | prost decode prost input | 18216.91 | 18.22 | 197.78 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 18163.44 | 18.16 | 210.55 | 1.00× |
| complex_root_decode | proto_rs decode prost input | 18217.42 | 18.22 | 197.78 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 18267.79 | 18.27 | 209.34 | 1.00× |
| complex_root_encode | prost encode_to_vec | 5931.30 | 5.93 | 607.45 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 13728.38 | 13.73 | 278.56 | 2.31× |

# Benchmark Run — 2025-10-20 14:54:24

| Group | Benchmark | Avg ns/op | Avg µs/op | MiB/s | Rel to Prost |
| --- | --- | ---: | ---: | ---: | ---: |
| bench_zero_copy_vs_prost | prost clone + encode | 10134.34 | 10.13 | 355.52 | 1.00× |
| bench_zero_copy_vs_prost | proto_rs zero_copy response | 13755.34 | 13.76 | 278.02 | 1.36× |
| complex_root_decode | prost decode prost input | 17322.33 | 17.32 | 208.00 | 1.01× |
| complex_root_decode | prost decode proto_rs input | 17424.57 | 17.42 | 219.47 | 1.02× |
| complex_root_decode | proto_rs decode prost input | 17157.89 | 17.16 | 209.99 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 17530.67 | 17.53 | 218.15 | 1.02× |
| complex_root_encode | prost encode_to_vec | 5891.15 | 5.89 | 611.59 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 13929.60 | 13.93 | 274.54 | 2.36× |

