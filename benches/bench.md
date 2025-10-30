
# Benchmark Run — 2025-10-29 21:53:31

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 36821.49 | 132.67 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 49915.98 | 179.85 | 1.36× faster |
| complex_root_components_encode | attachments | prost encode_to_vec | 11639008.33 | 377.39 | 1.00× |
| complex_root_components_encode | attachments | proto_rs encode_to_vec | 3751153.61 | 121.63 | 1.00× |
| complex_root_components_encode | audit log | prost encode_to_vec | 396680.22 | 281.46 | 1.00× |
| complex_root_components_encode | audit log | proto_rs encode_to_vec | 342704.17 | 243.16 | 1.00× |
| complex_root_components_encode | codes | prost encode_to_vec | 14822183.66 | 70.68 | 1.00× |
| complex_root_components_encode | codes | proto_rs encode_to_vec | 15547760.05 | 74.14 | 1.00× |
| complex_root_components_encode | complex_enum | prost encode_to_vec | 7419758.05 | 261.81 | 1.00× |
| complex_root_components_encode | complex_enum | proto_rs encode_to_vec | 3584592.01 | 126.49 | 1.00× |
| complex_root_components_encode | deep list | prost encode_to_vec | 454599.24 | 312.58 | 1.00× |
| complex_root_components_encode | deep list | proto_rs encode_to_vec | 368290.52 | 253.24 | 1.00× |
| complex_root_components_encode | deep lookup | prost encode_to_vec | 404884.32 | 288.44 | 1.00× |
| complex_root_components_encode | deep lookup | proto_rs encode_to_vec | 310312.82 | 221.07 | 1.00× |
| complex_root_components_encode | deep_message | prost encode_to_vec | 1083865.66 | 356.61 | 1.00× |
| complex_root_components_encode | deep_message | proto_rs encode_to_vec | 723421.21 | 238.02 | 1.00× |
| complex_root_components_encode | leaf lookup | prost encode_to_vec | 2659417.28 | 202.90 | 1.00× |
| complex_root_components_encode | leaf lookup | proto_rs encode_to_vec | 1648330.71 | 125.76 | 1.00× |
| complex_root_components_encode | leaves list | prost encode_to_vec | 3598880.42 | 223.09 | 1.00× |
| complex_root_components_encode | leaves list | proto_rs encode_to_vec | 1905891.15 | 118.14 | 1.00× |
| complex_root_components_encode | nested_leaf | prost encode_to_vec | 6645305.24 | 202.80 | 1.00× |
| complex_root_components_encode | nested_leaf | proto_rs encode_to_vec | 3916852.40 | 119.53 | 1.00× |
| complex_root_components_encode | status history | prost encode_to_vec | 735009.17 | 291.60 | 1.00× |
| complex_root_components_encode | status history | proto_rs encode_to_vec | 526159.55 | 208.74 | 1.00× |
| complex_root_components_encode | status lookup | prost encode_to_vec | 651064.20 | 255.19 | 1.00× |
| complex_root_components_encode | status lookup | proto_rs encode_to_vec | 545038.79 | 213.63 | 1.00× |
| complex_root_components_encode | tags | prost encode_to_vec | 13102689.50 | 337.38 | 1.00× |
| complex_root_components_encode | tags | proto_rs encode_to_vec | 6317832.55 | 162.68 | 1.00× |
| complex_root_decode | prost decode canonical input | 22701.68 | 81.79 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 23399.88 | 84.31 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 24592.00 | 88.60 | 1.08× faster |
| complex_root_decode | proto_rs decode proto_rs input | 24667.63 | 88.88 | 1.05× faster |
| complex_root_encode | prost encode_to_vec | 77549.75 | 279.41 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 83404.39 | 300.50 | 1.08× faster |


# Benchmark Run — 2025-10-29 18:46:30

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 115005.41 | 414.36 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 161873.47 | 583.23 | 1.41× faster |
| complex_root_decode | prost decode canonical input | 62198.37 | 224.10 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 61850.12 | 222.84 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 62481.49 | 225.12 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 62599.61 | 225.55 | 1.01× faster |
| complex_root_encode | prost encode_to_vec | 226085.64 | 814.58 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 185720.51 | 669.15 | 0.82× slower |


# Benchmark Run — 2025-10-29 18:42:14

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 114968.71 | 414.23 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 149685.65 | 539.31 | 1.30× faster |
| complex_root_decode | prost decode canonical input | 60617.36 | 218.40 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 60624.31 | 218.43 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 60961.13 | 219.64 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 59230.88 | 213.41 | 0.98× slower |
| complex_root_encode | prost encode_to_vec | 221953.67 | 799.69 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 183480.84 | 661.08 | 0.83× slower |


# Benchmark Run — 2025-10-29 18:00:36

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 116512.77 | 419.79 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 167042.55 | 601.85 | 1.43× faster |
| complex_root_decode | prost decode canonical input | 62918.44 | 226.69 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 63463.84 | 228.66 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 63517.00 | 228.85 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 63313.38 | 228.12 | 1.00× |
| complex_root_encode | prost encode_to_vec | 226333.65 | 815.48 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 188943.18 | 680.76 | 0.83× slower |


# Benchmark Run — 2025-10-29 17:58:56

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 112246.17 | 404.42 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 166995.50 | 601.68 | 1.49× faster |
| complex_root_decode | prost decode canonical input | 61833.35 | 222.78 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 61430.31 | 221.33 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 61198.79 | 220.50 | 0.99× slower |
| complex_root_decode | proto_rs decode proto_rs input | 62104.23 | 223.76 | 1.01× faster |
| complex_root_encode | prost encode_to_vec | 223664.01 | 805.86 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 189488.12 | 682.72 | 0.85× slower |


# Benchmark Run — 2025-10-29 15:28:09

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 117091.70 | 421.88 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 174520.98 | 628.80 | 1.49× faster |
| complex_root_decode | prost decode canonical input | 63702.67 | 229.52 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 63321.53 | 228.15 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 63645.90 | 229.31 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 63642.92 | 229.30 | 1.00× |
| complex_root_encode | prost encode_to_vec | 229814.81 | 828.02 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 194285.54 | 700.01 | 0.85× slower |


# Benchmark Run — 2025-10-29 13:58:41

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 118773.34 | 427.94 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 179548.82 | 646.91 | 1.51× faster |
| complex_root_decode | prost decode canonical input | 64596.24 | 232.74 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 64334.70 | 231.80 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 64584.21 | 232.70 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 64188.04 | 231.27 | 1.00× |
| complex_root_encode | prost encode_to_vec | 227596.83 | 820.03 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 194354.67 | 700.26 | 0.85× slower |


# Benchmark Run — 2025-10-29 13:43:53

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 101713.71 | 366.47 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 171022.56 | 616.19 | 1.68× faster |
| complex_root_decode | prost decode canonical input | 62262.92 | 224.33 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 62054.42 | 223.58 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 62385.26 | 224.77 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 62088.36 | 223.70 | 1.00× |
| complex_root_encode | prost encode_to_vec | 196426.09 | 707.72 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 187855.37 | 676.84 | 0.96× slower |


# Benchmark Run — 2025-10-29 13:41:38

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 100820.89 | 363.26 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 171942.12 | 619.50 | 1.71× faster |
| complex_root_decode | prost decode canonical input | 62425.81 | 224.92 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 62533.04 | 225.31 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 62539.32 | 225.33 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 62485.55 | 225.13 | 1.00× |
| complex_root_encode | prost encode_to_vec | 196208.79 | 706.94 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 187405.41 | 675.22 | 0.96× slower |


# Benchmark Run — 2025-10-29 13:38:38

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 100917.54 | 363.60 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 170634.25 | 614.79 | 1.69× faster |
| complex_root_decode | prost decode canonical input | 62302.27 | 224.47 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 62284.99 | 224.41 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 62680.01 | 225.83 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 62081.57 | 223.68 | 1.00× |
| complex_root_encode | prost encode_to_vec | 193909.76 | 698.65 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 180996.24 | 652.13 | 0.93× slower |


# Benchmark Run — 2025-10-29 12:19:31

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 98009.63 | 353.13 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 168972.99 | 612.03 | 1.72× faster |
| complex_root_decode | prost decode canonical input | 60176.59 | 216.82 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 59970.46 | 217.22 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 60542.30 | 218.13 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 59673.26 | 216.14 | 1.00× |
| complex_root_encode | prost encode_to_vec | 192822.75 | 694.74 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 183597.98 | 665.00 | 0.95× slower |


# Benchmark Run — 2025-10-29 11:17:07

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| bench_zero_copy_vs_clone | prost clone + encode | 99940.72 | 360.08 | 1.00× |
| bench_zero_copy_vs_clone | proto_rs zero_copy | 163402.17 | 591.85 | 1.63× faster |
| complex_root_decode | prost decode canonical input | 60751.41 | 218.89 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 59833.92 | 216.72 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 61028.27 | 219.88 | 1.00× |
| complex_root_decode | proto_rs decode proto_rs input | 60296.93 | 218.40 | 1.00× |
| complex_root_encode | prost encode_to_vec | 195433.90 | 704.14 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 175552.61 | 635.86 | 0.90× slower |


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



