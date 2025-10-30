
# Benchmark Run — 2025-10-30 12:37:42

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_encode | attachments_len1 | prost encode_to_vec | 36305699.96 | 484.73 | 1.00× |
| collection_overhead_encode | attachments_len1 | proto_rs encode_to_vec | 31006421.52 | 413.98 | 0.85× slower |
| collection_overhead_encode | codes_len1 | prost encode_to_vec | 42738147.27 | 122.27 | 1.00× |
| collection_overhead_encode | codes_len1 | proto_rs encode_to_vec | 43666119.85 | 124.93 | 1.02× faster |
| collection_overhead_encode | deep_list_len1 | prost encode_to_vec | 2642081.62 | 876.85 | 1.00× |
| collection_overhead_encode | deep_list_len1 | proto_rs encode_to_vec | 2067305.01 | 686.09 | 0.78× slower |
| collection_overhead_encode | leaf_lookup_len1 | prost encode_to_vec | 12419400.62 | 461.92 | 1.00× |
| collection_overhead_encode | leaf_lookup_len1 | proto_rs encode_to_vec | 8784866.43 | 326.74 | 0.71× slower |
| collection_overhead_encode | leaves_len1 | prost encode_to_vec | 16960795.05 | 549.95 | 1.00× |
| collection_overhead_encode | leaves_len1 | proto_rs encode_to_vec | 9280852.23 | 300.93 | 0.55× slower |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 41192010.02 | 549.97 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 30144415.18 | 402.47 | 0.73× slower |
| collection_overhead_encode | one_complex_enum | prost encode_to_vec | 15444923.11 | 574.45 | 1.00× |
| collection_overhead_encode | one_complex_enum | proto_rs encode_to_vec | 8879304.35 | 330.25 | 0.57× slower |
| collection_overhead_encode | one_deep_message | prost encode_to_vec | 2638617.05 | 875.70 | 1.00× |
| collection_overhead_encode | one_deep_message | proto_rs encode_to_vec | 1964720.70 | 652.05 | 0.74× slower |
| collection_overhead_encode | one_enum | prost encode_to_vec | 57608178.03 | 0.00 | 1.00× |
| collection_overhead_encode | one_enum | proto_rs encode_to_vec | 59451024.30 | 0.00 | 1.03× faster |
| collection_overhead_encode | one_nested_leaf | prost encode_to_vec | 16708011.49 | 541.76 | 1.00× |
| collection_overhead_encode | one_nested_leaf | proto_rs encode_to_vec | 8041477.81 | 260.74 | 0.48× slower |
| collection_overhead_encode | one_string | prost encode_to_vec | 41097743.48 | 352.74 | 1.00× |
| collection_overhead_encode | one_string | proto_rs encode_to_vec | 31506018.39 | 270.42 | 0.77× slower |
| collection_overhead_encode | status_history_len1 | prost encode_to_vec | 15130163.47 | 562.74 | 1.00× |
| collection_overhead_encode | status_history_len1 | proto_rs encode_to_vec | 9391383.71 | 349.30 | 0.62× slower |
| collection_overhead_encode | tags_len1 | prost encode_to_vec | 40496702.56 | 347.59 | 1.00× |
| collection_overhead_encode | tags_len1 | proto_rs encode_to_vec | 30150722.47 | 258.79 | 0.74× slower |
| complex_root_components_encode | attachments | prost encode_to_vec | 28146235.08 | 912.64 | 1.00× |
| complex_root_components_encode | attachments | proto_rs encode_to_vec | 12221349.71 | 396.28 | 0.43× slower |
| complex_root_components_encode | audit log | prost encode_to_vec | 1157810.11 | 821.51 | 1.00× |
| complex_root_components_encode | audit log | proto_rs encode_to_vec | 1072758.24 | 761.16 | 0.93× slower |
| complex_root_components_encode | codes | prost encode_to_vec | 35053470.77 | 167.15 | 1.00× |
| complex_root_components_encode | codes | proto_rs encode_to_vec | 38913523.09 | 185.55 | 1.11× faster |
| complex_root_components_encode | complex_enum | prost encode_to_vec | 17023196.11 | 600.68 | 1.00× |
| complex_root_components_encode | complex_enum | proto_rs encode_to_vec | 9852482.47 | 347.65 | 0.58× slower |
| complex_root_components_encode | deep list | prost encode_to_vec | 1359163.90 | 934.56 | 1.00× |
| complex_root_components_encode | deep list | proto_rs encode_to_vec | 1020124.32 | 701.44 | 0.75× slower |
| complex_root_components_encode | deep lookup | prost encode_to_vec | 1145899.30 | 816.33 | 1.00× |
| complex_root_components_encode | deep lookup | proto_rs encode_to_vec | 1086159.48 | 773.77 | 0.95× slower |
| complex_root_components_encode | deep_message | prost encode_to_vec | 2997140.36 | 986.11 | 1.00× |
| complex_root_components_encode | deep_message | proto_rs encode_to_vec | 2014495.74 | 662.80 | 0.67× slower |
| complex_root_components_encode | leaf lookup | prost encode_to_vec | 7473649.87 | 570.19 | 1.00× |
| complex_root_components_encode | leaf lookup | proto_rs encode_to_vec | 5097052.85 | 388.87 | 0.68× slower |
| complex_root_components_encode | leaves list | prost encode_to_vec | 10195592.40 | 632.01 | 1.00× |
| complex_root_components_encode | leaves list | proto_rs encode_to_vec | 5656028.31 | 350.61 | 0.55× slower |
| complex_root_components_encode | nested_leaf | prost encode_to_vec | 19163518.62 | 584.82 | 1.00× |
| complex_root_components_encode | nested_leaf | proto_rs encode_to_vec | 11686870.60 | 356.65 | 0.61× slower |
| complex_root_components_encode | status history | prost encode_to_vec | 2029182.28 | 805.03 | 1.00× |
| complex_root_components_encode | status history | proto_rs encode_to_vec | 1595534.61 | 632.99 | 0.79× slower |
| complex_root_components_encode | status lookup | prost encode_to_vec | 1806914.19 | 708.24 | 1.00× |
| complex_root_components_encode | status lookup | proto_rs encode_to_vec | 1570824.88 | 615.70 | 0.87× slower |
| complex_root_components_encode | tags | prost encode_to_vec | 34368667.14 | 884.97 | 1.00× |
| complex_root_components_encode | tags | proto_rs encode_to_vec | 16589926.58 | 427.18 | 0.48× slower |
| complex_root_decode | prost decode canonical input | 71743.85 | 258.49 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 70795.87 | 255.08 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 62112.61 | 223.79 | 0.87× slower |
| complex_root_decode | proto_rs decode proto_rs input | 61551.38 | 221.77 | 0.87× slower |
| complex_root_encode | prost encode_to_vec | 230248.53 | 829.58 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 221039.80 | 796.40 | 0.96× slower |
| micro_fields_encode | one_bytes | prost encode_to_vec | 40371362.50 | 654.52 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 29658852.14 | 480.84 | 0.73× slower |
| micro_fields_encode | one_complex_enum | prost encode_to_vec | 13127840.99 | 488.27 | 1.00× |
| micro_fields_encode | one_complex_enum | proto_rs encode_to_vec | 8696965.72 | 323.47 | 0.66× slower |
| micro_fields_encode | one_deep_message | prost encode_to_vec | 2641924.92 | 876.80 | 1.00× |
| micro_fields_encode | one_deep_message | proto_rs encode_to_vec | 1957602.74 | 649.69 | 0.74× slower |
| micro_fields_encode | one_enum | prost encode_to_vec | 57165442.89 | 0.00 | 1.00× |
| micro_fields_encode | one_enum | proto_rs encode_to_vec | 59476086.93 | 0.00 | 1.04× faster |
| micro_fields_encode | one_nested_leaf | prost encode_to_vec | 16816631.46 | 545.28 | 1.00× |
| micro_fields_encode | one_nested_leaf | proto_rs encode_to_vec | 9011450.29 | 292.20 | 0.54× slower |
| micro_fields_encode | one_string | prost encode_to_vec | 40943816.72 | 546.66 | 1.00× |
| micro_fields_encode | one_string | proto_rs encode_to_vec | 30824485.91 | 411.55 | 0.75× slower |
| zero_copy_vs_clone | prost clone + encode | 118616.53 | 427.37 | 1.00× |
| zero_copy_vs_clone | proto_rs zero_copy | 205131.91 | 739.09 | 1.73× faster |

# Benchmark Run — 2025-10-30 10:59:04

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| complex_root_components_encode | attachments | prost encode_to_vec | 27996632.60 | 907.79 | 1.00× |
| complex_root_components_encode | attachments | proto_rs encode_to_vec | 12610949.25 | 408.91 | 0.45× slower |
| complex_root_components_encode | audit log | prost encode_to_vec | 1159763.84 | 822.89 | 1.00× |
| complex_root_components_encode | audit log | proto_rs encode_to_vec | 1097552.56 | 778.75 | 0.95× slower |
| complex_root_components_encode | codes | prost encode_to_vec | 35285248.17 | 168.25 | 1.00× |
| complex_root_components_encode | codes | proto_rs encode_to_vec | 40142502.97 | 191.41 | 1.14× faster |
| complex_root_components_encode | complex_enum | prost encode_to_vec | 17750113.52 | 626.33 | 1.00× |
| complex_root_components_encode | complex_enum | proto_rs encode_to_vec | 9894549.40 | 349.14 | 0.56× slower |
| complex_root_components_encode | deep list | prost encode_to_vec | 1358940.63 | 934.41 | 1.00× |
| complex_root_components_encode | deep list | proto_rs encode_to_vec | 1083841.90 | 745.25 | 0.80× slower |
| complex_root_components_encode | deep lookup | prost encode_to_vec | 1145615.13 | 816.13 | 1.00× |
| complex_root_components_encode | deep lookup | proto_rs encode_to_vec | 1104658.62 | 786.95 | 0.96× slower |
| complex_root_components_encode | deep_message | prost encode_to_vec | 3034643.91 | 998.45 | 1.00× |
| complex_root_components_encode | deep_message | proto_rs encode_to_vec | 2138593.84 | 703.64 | 0.70× slower |
| complex_root_components_encode | leaf lookup | prost encode_to_vec | 7444515.19 | 567.97 | 1.00× |
| complex_root_components_encode | leaf lookup | proto_rs encode_to_vec | 5199798.27 | 396.71 | 0.70× slower |
| complex_root_components_encode | leaves list | prost encode_to_vec | 10413961.04 | 645.55 | 1.00× |
| complex_root_components_encode | leaves list | proto_rs encode_to_vec | 5628835.60 | 348.92 | 0.54× slower |
| complex_root_components_encode | nested_leaf | prost encode_to_vec | 19351597.16 | 590.56 | 1.00× |
| complex_root_components_encode | nested_leaf | proto_rs encode_to_vec | 11896498.03 | 363.05 | 0.61× slower |
| complex_root_components_encode | status history | prost encode_to_vec | 2065012.30 | 819.25 | 1.00× |
| complex_root_components_encode | status history | proto_rs encode_to_vec | 1658128.27 | 657.83 | 0.80× slower |
| complex_root_components_encode | status lookup | prost encode_to_vec | 1810026.22 | 709.46 | 1.00× |
| complex_root_components_encode | status lookup | proto_rs encode_to_vec | 1625954.69 | 637.31 | 0.90× slower |
| complex_root_components_encode | tags | prost encode_to_vec | 34676449.11 | 892.89 | 1.00× |
| complex_root_components_encode | tags | proto_rs encode_to_vec | 17009842.52 | 437.99 | 0.49× slower |
| complex_root_decode | prost decode canonical input | 73131.19 | 263.49 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 74455.70 | 268.26 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 64515.36 | 232.45 | 0.88× slower |
| complex_root_decode | proto_rs decode proto_rs input | 64524.16 | 232.48 | 0.87× slower |
| complex_root_encode | prost encode_to_vec | 229865.66 | 828.20 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 221047.26 | 796.43 | 0.96× slower |
| zero_copy_vs_clone | prost clone + encode | 118668.71 | 427.56 | 1.00× |
| zero_copy_vs_clone | proto_rs zero_copy | 194330.78 | 700.17 | 1.64× faster |

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



