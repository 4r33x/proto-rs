
# Benchmark Run — 2025-11-11 20:19:57

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_decode | attachments_len1 | prost decode | 18709830.01 | 249.80 | 1.00× |
| collection_overhead_decode | attachments_len1 | proto_rs decode | 22191313.67 | 296.29 | 1.19× faster |
| collection_overhead_decode | codes_len1 | prost decode | 31654010.67 | 90.56 | 1.00× |
| collection_overhead_decode | codes_len1 | proto_rs decode | 40193514.24 | 114.99 | 1.27× faster |
| collection_overhead_decode | deep_list_len1 | prost decode | 781307.63 | 259.30 | 1.00× |
| collection_overhead_decode | deep_list_len1 | proto_rs decode | 809771.59 | 268.75 | 1.04× faster |
| collection_overhead_decode | leaf_lookup_len1 | prost decode | 5679441.30 | 211.24 | 1.00× |
| collection_overhead_decode | leaf_lookup_len1 | proto_rs decode | 5075510.47 | 188.77 | 0.89× slower |
| collection_overhead_decode | leaves_len1 | prost decode | 6981348.93 | 226.37 | 1.00× |
| collection_overhead_decode | leaves_len1 | proto_rs decode | 7612526.45 | 246.84 | 1.09× faster |
| collection_overhead_decode | one_bytes | prost decode | 25138540.25 | 335.64 | 1.00× |
| collection_overhead_decode | one_bytes | proto_rs decode | 26001486.19 | 347.16 | 1.03× faster |
| collection_overhead_decode | one_complex_enum | prost decode | 8235288.29 | 306.30 | 1.00× |
| collection_overhead_decode | one_complex_enum | proto_rs decode | 7065590.19 | 262.79 | 0.86× slower |
| collection_overhead_decode | one_deep_message | prost decode | 773876.55 | 256.83 | 1.00× |
| collection_overhead_decode | one_deep_message | proto_rs decode | 813229.21 | 269.89 | 1.05× faster |
| collection_overhead_decode | one_enum | prost decode | 54304550.21 | 0.00 | 1.00× |
| collection_overhead_decode | one_enum | proto_rs decode | 56562409.07 | 0.00 | 1.04× faster |
| collection_overhead_decode | one_nested_leaf | prost decode | 6551429.80 | 212.43 | 1.00× |
| collection_overhead_decode | one_nested_leaf | proto_rs decode | 8648697.85 | 280.43 | 1.32× faster |
| collection_overhead_decode | one_string | prost decode | 31739391.68 | 272.42 | 1.00× |
| collection_overhead_decode | one_string | proto_rs decode | 35575909.57 | 305.35 | 1.12× faster |
| collection_overhead_decode | status_history_len1 | prost decode | 7931952.41 | 295.02 | 1.00× |
| collection_overhead_decode | status_history_len1 | proto_rs decode | 6807238.96 | 253.18 | 0.86× slower |
| collection_overhead_decode | tags_len1 | prost decode | 24106270.60 | 206.91 | 1.00× |
| collection_overhead_decode | tags_len1 | proto_rs decode | 27401684.52 | 235.19 | 1.14× faster |
| collection_overhead_encode | attachments_len1 | prost encode_to_vec | 36706208.86 | 490.08 | 1.00× |
| collection_overhead_encode | attachments_len1 | proto_rs encode_to_vec | 36717737.40 | 490.23 | 1.00× |
| collection_overhead_encode | codes_len1 | prost encode_to_vec | 43830062.26 | 125.40 | 1.00× |
| collection_overhead_encode | codes_len1 | proto_rs encode_to_vec | 45954172.77 | 131.48 | 1.05× faster |
| collection_overhead_encode | deep_list_len1 | prost encode_to_vec | 2918072.58 | 968.45 | 1.00× |
| collection_overhead_encode | deep_list_len1 | proto_rs encode_to_vec | 2860698.69 | 949.40 | 0.98× slower |
| collection_overhead_encode | leaf_lookup_len1 | prost encode_to_vec | 15172646.06 | 564.32 | 1.00× |
| collection_overhead_encode | leaf_lookup_len1 | proto_rs encode_to_vec | 15595580.71 | 580.05 | 1.03× faster |
| collection_overhead_encode | leaves_len1 | prost encode_to_vec | 20961469.22 | 679.67 | 1.00× |
| collection_overhead_encode | leaves_len1 | proto_rs encode_to_vec | 21912636.87 | 710.52 | 1.05× faster |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 41262222.42 | 550.91 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 37161172.35 | 496.16 | 0.90× slower |
| collection_overhead_encode | one_complex_enum | prost encode_to_vec | 18046451.71 | 671.21 | 1.00× |
| collection_overhead_encode | one_complex_enum | proto_rs encode_to_vec | 19258482.85 | 716.29 | 1.07× faster |
| collection_overhead_encode | one_deep_message | prost encode_to_vec | 2814431.43 | 934.05 | 1.00× |
| collection_overhead_encode | one_deep_message | proto_rs encode_to_vec | 2839551.19 | 942.39 | 1.00× |
| collection_overhead_encode | one_enum | prost encode_to_vec | 54237788.77 | 0.00 | 1.00× |
| collection_overhead_encode | one_enum | proto_rs encode_to_vec | 57076273.34 | 0.00 | 1.05× faster |
| collection_overhead_encode | one_nested_leaf | prost encode_to_vec | 19500834.50 | 632.31 | 1.00× |
| collection_overhead_encode | one_nested_leaf | proto_rs encode_to_vec | 21536693.74 | 698.33 | 1.10× faster |
| collection_overhead_encode | one_string | prost encode_to_vec | 41473475.69 | 355.97 | 1.00× |
| collection_overhead_encode | one_string | proto_rs encode_to_vec | 37348119.02 | 320.56 | 0.90× slower |
| collection_overhead_encode | status_history_len1 | prost encode_to_vec | 19001774.54 | 706.74 | 1.00× |
| collection_overhead_encode | status_history_len1 | proto_rs encode_to_vec | 18419746.63 | 685.09 | 0.97× slower |
| collection_overhead_encode | tags_len1 | prost encode_to_vec | 41142548.54 | 353.13 | 1.00× |
| collection_overhead_encode | tags_len1 | proto_rs encode_to_vec | 36979890.76 | 317.40 | 0.90× slower |
| complex_root_components_decode | attachments | prost decode | 10098904.62 | 327.46 | 1.00× |
| complex_root_components_decode | attachments | proto_rs decode | 10846351.78 | 351.69 | 1.07× faster |
| complex_root_components_decode | audit log | prost decode | 375988.91 | 266.78 | 1.00× |
| complex_root_components_decode | audit log | proto_rs decode | 341053.54 | 241.99 | 0.91× slower |
| complex_root_components_decode | codes | prost decode | 31818202.50 | 151.72 | 1.00× |
| complex_root_components_decode | codes | proto_rs decode | 33763882.49 | 161.00 | 1.06× faster |
| complex_root_components_decode | complex_enum | prost decode | 8466639.38 | 298.75 | 1.00× |
| complex_root_components_decode | complex_enum | proto_rs decode | 8463098.42 | 298.63 | 1.00× |
| complex_root_components_decode | deep list | prost decode | 384794.93 | 264.58 | 1.00× |
| complex_root_components_decode | deep list | proto_rs decode | 398201.68 | 273.80 | 1.03× faster |
| complex_root_components_decode | deep lookup | prost decode | 371276.68 | 264.50 | 1.00× |
| complex_root_components_decode | deep lookup | proto_rs decode | 332506.27 | 236.88 | 0.90× slower |
| complex_root_components_decode | deep_message | prost decode | 812963.27 | 267.48 | 1.00× |
| complex_root_components_decode | deep_message | proto_rs decode | 819025.31 | 269.47 | 1.00× |
| complex_root_components_decode | leaf lookup | prost decode | 2660921.52 | 203.01 | 1.00× |
| complex_root_components_decode | leaf lookup | proto_rs decode | 2377893.39 | 181.42 | 0.89× slower |
| complex_root_components_decode | leaves list | prost decode | 3037062.81 | 188.26 | 1.00× |
| complex_root_components_decode | leaves list | proto_rs decode | 3293777.77 | 204.18 | 1.08× faster |
| complex_root_components_decode | nested_leaf | prost decode | 8201224.71 | 250.28 | 1.00× |
| complex_root_components_decode | nested_leaf | proto_rs decode | 8940266.38 | 272.84 | 1.09× faster |
| complex_root_components_decode | status history | prost decode | 665897.88 | 264.18 | 1.00× |
| complex_root_components_decode | status history | proto_rs decode | 607019.60 | 240.82 | 0.91× slower |
| complex_root_components_decode | status lookup | prost decode | 665094.45 | 260.69 | 1.00× |
| complex_root_components_decode | status lookup | proto_rs decode | 567891.16 | 222.59 | 0.85× slower |
| complex_root_components_decode | tags | prost decode | 14587573.98 | 375.62 | 1.00× |
| complex_root_components_decode | tags | proto_rs decode | 15991736.48 | 411.77 | 1.10× faster |
| complex_root_components_encode | attachments | prost encode_to_vec | 31725352.87 | 1028.69 | 1.00× |
| complex_root_components_encode | attachments | proto_rs encode_to_vec | 34714670.03 | 1125.62 | 1.09× faster |
| complex_root_components_encode | audit log | prost encode_to_vec | 1237883.05 | 878.32 | 1.00× |
| complex_root_components_encode | audit log | proto_rs encode_to_vec | 1431467.22 | 1015.67 | 1.16× faster |
| complex_root_components_encode | codes | prost encode_to_vec | 39400842.69 | 187.88 | 1.00× |
| complex_root_components_encode | codes | proto_rs encode_to_vec | 37172154.95 | 177.25 | 0.94× slower |
| complex_root_components_encode | complex_enum | prost encode_to_vec | 21964046.87 | 775.02 | 1.00× |
| complex_root_components_encode | complex_enum | proto_rs encode_to_vec | 22234809.09 | 784.58 | 1.01× faster |
| complex_root_components_encode | deep list | prost encode_to_vec | 1463113.13 | 1006.04 | 1.00× |
| complex_root_components_encode | deep list | proto_rs encode_to_vec | 1470830.39 | 1011.34 | 1.00× |
| complex_root_components_encode | deep lookup | prost encode_to_vec | 1250143.79 | 890.60 | 1.00× |
| complex_root_components_encode | deep lookup | proto_rs encode_to_vec | 1428936.54 | 1017.97 | 1.14× faster |
| complex_root_components_encode | deep_message | prost encode_to_vec | 3125572.34 | 1028.37 | 1.00× |
| complex_root_components_encode | deep_message | proto_rs encode_to_vec | 3279452.69 | 1079.00 | 1.05× faster |
| complex_root_components_encode | leaf lookup | prost encode_to_vec | 9109593.40 | 695.01 | 1.00× |
| complex_root_components_encode | leaf lookup | proto_rs encode_to_vec | 8854903.09 | 675.58 | 0.97× slower |
| complex_root_components_encode | leaves list | prost encode_to_vec | 11342059.42 | 703.08 | 1.00× |
| complex_root_components_encode | leaves list | proto_rs encode_to_vec | 13219321.22 | 819.45 | 1.17× faster |
| complex_root_components_encode | nested_leaf | prost encode_to_vec | 19443029.78 | 593.35 | 1.00× |
| complex_root_components_encode | nested_leaf | proto_rs encode_to_vec | 25282007.07 | 771.55 | 1.30× faster |
| complex_root_components_encode | status history | prost encode_to_vec | 2173989.17 | 862.48 | 1.00× |
| complex_root_components_encode | status history | proto_rs encode_to_vec | 2232541.90 | 885.71 | 1.03× faster |
| complex_root_components_encode | status lookup | prost encode_to_vec | 1881743.79 | 737.57 | 1.00× |
| complex_root_components_encode | status lookup | proto_rs encode_to_vec | 2185358.62 | 856.57 | 1.16× faster |
| complex_root_components_encode | tags | prost encode_to_vec | 36439874.57 | 938.30 | 1.00× |
| complex_root_components_encode | tags | proto_rs encode_to_vec | 34600190.67 | 890.93 | 0.95× slower |
| complex_root_decode | prost decode canonical input | 74179.34 | 267.27 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 74611.50 | 268.82 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 63120.22 | 227.42 | 0.85× slower |
| complex_root_decode | proto_rs decode proto_rs input | 62990.02 | 226.95 | 0.84× slower |
| complex_root_encode | prost encode_to_vec | 261623.31 | 942.62 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 267078.68 | 962.28 | 1.02× faster |
| micro_fields_decode | one_bytes | prost decode | 24809941.59 | 402.23 | 1.00× |
| micro_fields_decode | one_bytes | proto_rs decode | 25915089.65 | 420.15 | 1.04× faster |
| micro_fields_decode | one_complex_enum | prost decode | 8049030.36 | 299.37 | 1.00× |
| micro_fields_decode | one_complex_enum | proto_rs decode | 7058241.71 | 262.52 | 0.88× slower |
| micro_fields_decode | one_deep_message | prost decode | 774194.54 | 256.94 | 1.00× |
| micro_fields_decode | one_deep_message | proto_rs decode | 812765.32 | 269.74 | 1.05× faster |
| micro_fields_decode | one_enum | prost decode | 53577748.34 | 0.00 | 1.00× |
| micro_fields_decode | one_enum | proto_rs decode | 56690800.64 | 0.00 | 1.06× faster |
| micro_fields_decode | one_nested_leaf | prost decode | 6503549.57 | 210.88 | 1.00× |
| micro_fields_decode | one_nested_leaf | proto_rs decode | 8619715.75 | 279.49 | 1.33× faster |
| micro_fields_decode | one_string | prost decode | 29813126.81 | 398.05 | 1.00× |
| micro_fields_decode | one_string | proto_rs decode | 34021369.84 | 454.23 | 1.14× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 41839553.46 | 678.32 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 37217590.26 | 603.39 | 0.89× slower |
| micro_fields_encode | one_complex_enum | prost encode_to_vec | 18952324.60 | 704.90 | 1.00× |
| micro_fields_encode | one_complex_enum | proto_rs encode_to_vec | 19226305.04 | 715.09 | 1.01× faster |
| micro_fields_encode | one_deep_message | prost encode_to_vec | 2805472.57 | 931.08 | 1.00× |
| micro_fields_encode | one_deep_message | proto_rs encode_to_vec | 2870077.26 | 952.52 | 1.02× faster |
| micro_fields_encode | one_enum | prost encode_to_vec | 54937183.71 | 0.00 | 1.00× |
| micro_fields_encode | one_enum | proto_rs encode_to_vec | 57423479.53 | 0.00 | 1.05× faster |
| micro_fields_encode | one_nested_leaf | prost encode_to_vec | 19560006.00 | 634.23 | 1.00× |
| micro_fields_encode | one_nested_leaf | proto_rs encode_to_vec | 21903865.61 | 710.23 | 1.12× faster |
| micro_fields_encode | one_string | prost encode_to_vec | 41627801.37 | 555.79 | 1.00× |
| micro_fields_encode | one_string | proto_rs encode_to_vec | 37530405.62 | 501.08 | 0.90× slower |
| zero_copy_vs_clone | prost clone + encode | 126513.75 | 455.83 | 1.00× |
| zero_copy_vs_clone | proto_rs zero_copy | 249535.07 | 899.07 | 1.97× faster |


# Benchmark Run — 2025-10-30 19:29:18

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_decode | attachments_len1 | prost decode | 17799940.06 | 237.65 | 1.00× |
| collection_overhead_decode | attachments_len1 | proto_rs decode | 21469212.97 | 286.64 | 1.21× faster |
| collection_overhead_decode | codes_len1 | prost decode | 30731259.44 | 87.92 | 1.00× |
| collection_overhead_decode | codes_len1 | proto_rs decode | 36090430.49 | 103.26 | 1.17× faster |
| collection_overhead_decode | deep_list_len1 | prost decode | 760008.81 | 252.23 | 1.00× |
| collection_overhead_decode | deep_list_len1 | proto_rs decode | 716696.63 | 237.86 | 0.94× slower |
| collection_overhead_decode | leaf_lookup_len1 | prost decode | 5015449.47 | 186.54 | 1.00× |
| collection_overhead_decode | leaf_lookup_len1 | proto_rs decode | 4373770.79 | 162.67 | 0.87× slower |
| collection_overhead_decode | leaves_len1 | prost decode | 6573466.26 | 213.14 | 1.00× |
| collection_overhead_decode | leaves_len1 | proto_rs decode | 6313125.06 | 204.70 | 0.96× slower |
| collection_overhead_decode | one_bytes | prost decode | 25354608.22 | 338.52 | 1.00× |
| collection_overhead_decode | one_bytes | proto_rs decode | 25868597.48 | 345.38 | 1.02× faster |
| collection_overhead_decode | one_complex_enum | prost decode | 6597922.01 | 245.40 | 1.00× |
| collection_overhead_decode | one_complex_enum | proto_rs decode | 4981834.97 | 185.29 | 0.76× slower |
| collection_overhead_decode | one_deep_message | prost decode | 787062.33 | 261.21 | 1.00× |
| collection_overhead_decode | one_deep_message | proto_rs decode | 708100.93 | 235.00 | 0.90× slower |
| collection_overhead_decode | one_enum | prost decode | 56074392.29 | 0.00 | 1.00× |
| collection_overhead_decode | one_enum | proto_rs decode | 58032163.71 | 0.00 | 1.03× faster |
| collection_overhead_decode | one_nested_leaf | prost decode | 6710587.58 | 217.59 | 1.00× |
| collection_overhead_decode | one_nested_leaf | proto_rs decode | 7489958.21 | 242.86 | 1.12× faster |
| collection_overhead_decode | one_string | prost decode | 29844533.07 | 256.16 | 1.00× |
| collection_overhead_decode | one_string | proto_rs decode | 34018489.41 | 291.98 | 1.14× faster |
| collection_overhead_decode | status_history_len1 | prost decode | 6858955.96 | 255.11 | 1.00× |
| collection_overhead_decode | status_history_len1 | proto_rs decode | 4842353.64 | 180.10 | 0.71× slower |
| collection_overhead_decode | tags_len1 | prost decode | 21790723.81 | 187.03 | 1.00× |
| collection_overhead_decode | tags_len1 | proto_rs decode | 25435474.54 | 218.31 | 1.17× faster |
| collection_overhead_encode | attachments_len1 | prost encode_to_vec | 35483328.20 | 473.75 | 1.00× |
| collection_overhead_encode | attachments_len1 | proto_rs encode_to_vec | 35626282.73 | 475.66 | 1.00× |
| collection_overhead_encode | codes_len1 | prost encode_to_vec | 41268939.34 | 118.07 | 1.00× |
| collection_overhead_encode | codes_len1 | proto_rs encode_to_vec | 36067047.83 | 103.19 | 0.87× slower |
| collection_overhead_encode | deep_list_len1 | prost encode_to_vec | 2560525.96 | 849.78 | 1.00× |
| collection_overhead_encode | deep_list_len1 | proto_rs encode_to_vec | 2697721.31 | 895.32 | 1.05× faster |
| collection_overhead_encode | leaf_lookup_len1 | prost encode_to_vec | 11741194.89 | 436.69 | 1.00× |
| collection_overhead_encode | leaf_lookup_len1 | proto_rs encode_to_vec | 13014012.91 | 484.03 | 1.11× faster |
| collection_overhead_encode | leaves_len1 | prost encode_to_vec | 16316007.90 | 529.05 | 1.00× |
| collection_overhead_encode | leaves_len1 | proto_rs encode_to_vec | 17242601.84 | 559.09 | 1.06× faster |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 39830833.56 | 531.80 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 35934068.65 | 479.77 | 0.90× slower |
| collection_overhead_encode | one_complex_enum | prost encode_to_vec | 15108104.19 | 561.92 | 1.00× |
| collection_overhead_encode | one_complex_enum | proto_rs encode_to_vec | 15285379.97 | 568.51 | 1.01× faster |
| collection_overhead_encode | one_deep_message | prost encode_to_vec | 2548563.04 | 845.81 | 1.00× |
| collection_overhead_encode | one_deep_message | proto_rs encode_to_vec | 2637467.81 | 875.32 | 1.03× faster |
| collection_overhead_encode | one_enum | prost encode_to_vec | 56303732.84 | 0.00 | 1.00× |
| collection_overhead_encode | one_enum | proto_rs encode_to_vec | 58248435.97 | 0.00 | 1.03× faster |
| collection_overhead_encode | one_nested_leaf | prost encode_to_vec | 16316906.18 | 529.07 | 1.00× |
| collection_overhead_encode | one_nested_leaf | proto_rs encode_to_vec | 17185344.36 | 557.23 | 1.05× faster |
| collection_overhead_encode | one_string | prost encode_to_vec | 40400351.64 | 346.76 | 1.00× |
| collection_overhead_encode | one_string | proto_rs encode_to_vec | 36294909.42 | 311.52 | 0.90× slower |
| collection_overhead_encode | status_history_len1 | prost encode_to_vec | 14729703.03 | 547.85 | 1.00× |
| collection_overhead_encode | status_history_len1 | proto_rs encode_to_vec | 15225357.11 | 566.28 | 1.03× faster |
| collection_overhead_encode | tags_len1 | prost encode_to_vec | 39514636.44 | 339.16 | 1.00× |
| collection_overhead_encode | tags_len1 | proto_rs encode_to_vec | 35699430.62 | 306.41 | 0.90× slower |
| complex_root_components_decode | attachments | prost decode | 10280493.17 | 333.34 | 1.00× |
| complex_root_components_decode | attachments | proto_rs decode | 10733335.58 | 348.03 | 1.04× faster |
| complex_root_components_decode | audit log | prost decode | 375247.66 | 266.25 | 1.00× |
| complex_root_components_decode | audit log | proto_rs decode | 289814.99 | 205.63 | 0.77× slower |
| complex_root_components_decode | codes | prost decode | 30161602.93 | 143.82 | 1.00× |
| complex_root_components_decode | codes | proto_rs decode | 30369716.64 | 144.81 | 1.00× |
| complex_root_components_decode | complex_enum | prost decode | 7361149.46 | 259.75 | 1.00× |
| complex_root_components_decode | complex_enum | proto_rs decode | 6355788.35 | 224.27 | 0.86× slower |
| complex_root_components_decode | deep list | prost decode | 382832.55 | 263.24 | 1.00× |
| complex_root_components_decode | deep list | proto_rs decode | 354086.38 | 243.47 | 0.92× slower |
| complex_root_components_decode | deep lookup | prost decode | 370151.56 | 263.69 | 1.00× |
| complex_root_components_decode | deep lookup | proto_rs decode | 290851.07 | 207.20 | 0.79× slower |
| complex_root_components_decode | deep_message | prost decode | 768779.11 | 252.94 | 1.00× |
| complex_root_components_decode | deep_message | proto_rs decode | 738062.79 | 242.84 | 0.96× slower |
| complex_root_components_decode | leaf lookup | prost decode | 2549455.93 | 194.51 | 1.00× |
| complex_root_components_decode | leaf lookup | proto_rs decode | 2076858.78 | 158.45 | 0.81× slower |
| complex_root_components_decode | leaves list | prost decode | 3197215.67 | 198.19 | 1.00× |
| complex_root_components_decode | leaves list | proto_rs decode | 3138953.79 | 194.58 | 0.98× slower |
| complex_root_components_decode | nested_leaf | prost decode | 5811708.25 | 177.36 | 1.00× |
| complex_root_components_decode | nested_leaf | proto_rs decode | 7477260.00 | 228.19 | 1.29× faster |
| complex_root_components_decode | status history | prost decode | 667393.80 | 264.77 | 1.00× |
| complex_root_components_decode | status history | proto_rs decode | 572251.94 | 227.03 | 0.86× slower |
| complex_root_components_decode | status lookup | prost decode | 632959.22 | 248.09 | 1.00× |
| complex_root_components_decode | status lookup | proto_rs decode | 486124.14 | 190.54 | 0.77× slower |
| complex_root_components_decode | tags | prost decode | 13656157.97 | 351.64 | 1.00× |
| complex_root_components_decode | tags | proto_rs decode | 14412842.72 | 371.12 | 1.06× faster |
| complex_root_components_encode | attachments | prost encode_to_vec | 26902624.58 | 872.32 | 1.00× |
| complex_root_components_encode | attachments | proto_rs encode_to_vec | 31628243.32 | 1025.54 | 1.18× faster |
| complex_root_components_encode | audit log | prost encode_to_vec | 1116195.26 | 791.98 | 1.00× |
| complex_root_components_encode | audit log | proto_rs encode_to_vec | 1359463.26 | 964.58 | 1.22× faster |
| complex_root_components_encode | codes | prost encode_to_vec | 34592777.90 | 164.95 | 1.00× |
| complex_root_components_encode | codes | proto_rs encode_to_vec | 35628401.66 | 169.89 | 1.03× faster |
| complex_root_components_encode | complex_enum | prost encode_to_vec | 16846109.29 | 594.43 | 1.00× |
| complex_root_components_encode | complex_enum | proto_rs encode_to_vec | 17016177.66 | 600.43 | 1.01× faster |
| complex_root_components_encode | deep list | prost encode_to_vec | 1325316.97 | 911.29 | 1.00× |
| complex_root_components_encode | deep list | proto_rs encode_to_vec | 1389347.19 | 955.31 | 1.05× faster |
| complex_root_components_encode | deep lookup | prost encode_to_vec | 1107719.65 | 789.13 | 1.00× |
| complex_root_components_encode | deep lookup | proto_rs encode_to_vec | 1350134.69 | 961.83 | 1.22× faster |
| complex_root_components_encode | deep_message | prost encode_to_vec | 2948179.95 | 970.00 | 1.00× |
| complex_root_components_encode | deep_message | proto_rs encode_to_vec | 2886118.62 | 949.58 | 0.98× slower |
| complex_root_components_encode | leaf lookup | prost encode_to_vec | 7223889.81 | 551.14 | 1.00× |
| complex_root_components_encode | leaf lookup | proto_rs encode_to_vec | 8128928.80 | 620.19 | 1.13× faster |
| complex_root_components_encode | leaves list | prost encode_to_vec | 10109571.40 | 626.68 | 1.00× |
| complex_root_components_encode | leaves list | proto_rs encode_to_vec | 10307703.55 | 638.96 | 1.02× faster |
| complex_root_components_encode | nested_leaf | prost encode_to_vec | 18668663.41 | 569.72 | 1.00× |
| complex_root_components_encode | nested_leaf | proto_rs encode_to_vec | 20012245.20 | 610.73 | 1.07× faster |
| complex_root_components_encode | status history | prost encode_to_vec | 1974507.11 | 783.34 | 1.00× |
| complex_root_components_encode | status history | proto_rs encode_to_vec | 2067361.30 | 820.18 | 1.05× faster |
| complex_root_components_encode | status lookup | prost encode_to_vec | 1736615.43 | 680.68 | 1.00× |
| complex_root_components_encode | status lookup | proto_rs encode_to_vec | 2015555.02 | 790.02 | 1.16× faster |
| complex_root_components_encode | tags | prost encode_to_vec | 33921214.29 | 873.44 | 1.00× |
| complex_root_components_encode | tags | proto_rs encode_to_vec | 31275032.70 | 805.31 | 0.92× slower |
| complex_root_decode | prost decode canonical input | 67970.97 | 244.90 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 67938.35 | 244.78 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 57353.66 | 206.64 | 0.84× slower |
| complex_root_decode | proto_rs decode proto_rs input | 57211.96 | 206.13 | 0.84× slower |
| complex_root_encode | prost encode_to_vec | 224079.68 | 807.35 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 243115.56 | 875.94 | 1.08× faster |
| micro_fields_decode | one_bytes | prost decode | 24842550.74 | 402.76 | 1.00× |
| micro_fields_decode | one_bytes | proto_rs decode | 26458110.13 | 428.95 | 1.07× faster |
| micro_fields_decode | one_complex_enum | prost decode | 7101831.20 | 264.14 | 1.00× |
| micro_fields_decode | one_complex_enum | proto_rs decode | 5295122.66 | 196.94 | 0.75× slower |
| micro_fields_decode | one_deep_message | prost decode | 779036.55 | 258.55 | 1.00× |
| micro_fields_decode | one_deep_message | proto_rs decode | 732330.31 | 243.04 | 0.94× slower |
| micro_fields_decode | one_enum | prost decode | 56098886.78 | 0.00 | 1.00× |
| micro_fields_decode | one_enum | proto_rs decode | 57786599.67 | 0.00 | 1.03× faster |
| micro_fields_decode | one_nested_leaf | prost decode | 6700245.81 | 217.25 | 1.00× |
| micro_fields_decode | one_nested_leaf | proto_rs decode | 7298217.19 | 236.64 | 1.09× faster |
| micro_fields_decode | one_string | prost decode | 29951884.06 | 399.90 | 1.00× |
| micro_fields_decode | one_string | proto_rs decode | 31582982.55 | 421.68 | 1.05× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 31797553.27 | 515.52 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 36412742.05 | 590.34 | 1.15× faster |
| micro_fields_encode | one_complex_enum | prost encode_to_vec | 15393694.88 | 572.54 | 1.00× |
| micro_fields_encode | one_complex_enum | proto_rs encode_to_vec | 15333678.40 | 570.31 | 1.00× |
| micro_fields_encode | one_deep_message | prost encode_to_vec | 2560663.34 | 849.83 | 1.00× |
| micro_fields_encode | one_deep_message | proto_rs encode_to_vec | 2637636.25 | 875.38 | 1.03× faster |
| micro_fields_encode | one_enum | prost encode_to_vec | 56791411.27 | 0.00 | 1.00× |
| micro_fields_encode | one_enum | proto_rs encode_to_vec | 58542790.93 | 0.00 | 1.03× faster |
| micro_fields_encode | one_nested_leaf | prost encode_to_vec | 16662596.29 | 540.28 | 1.00× |
| micro_fields_encode | one_nested_leaf | proto_rs encode_to_vec | 17260001.94 | 559.65 | 1.04× faster |
| micro_fields_encode | one_string | prost encode_to_vec | 40255601.15 | 537.47 | 1.00× |
| micro_fields_encode | one_string | proto_rs encode_to_vec | 36273279.14 | 484.30 | 0.90× slower |
| zero_copy_vs_clone | prost clone + encode | 115225.35 | 415.15 | 1.00× |
| zero_copy_vs_clone | proto_rs zero_copy | 213296.77 | 768.50 | 1.85× faster |


# Benchmark Run — 2025-10-30 18:49:43

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_decode | one_bytes | prost decode | 23046078.35 | 307.70 | 1.00× |
| collection_overhead_decode | one_bytes | proto_rs decode | 24175633.93 | 322.78 | 1.05× faster |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 39617726.61 | 528.95 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 35127491.01 | 469.00 | 0.89× slower |
| micro_fields_decode | one_bytes | prost decode | 22722900.02 | 368.39 | 1.00× |
| micro_fields_decode | one_bytes | proto_rs decode | 23792491.86 | 385.73 | 1.05× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 39362199.06 | 638.16 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 35291358.40 | 572.16 | 0.90× slower |


# Benchmark Run — 2025-10-30 18:45:35

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_decode | one_bytes | prost decode | 25589918.05 | 341.66 | 1.00× |
| collection_overhead_decode | one_bytes | proto_rs decode | 26073443.50 | 348.12 | 1.02× faster |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 40773865.85 | 544.39 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 36691080.95 | 489.88 | 0.90× slower |
| micro_fields_decode | one_bytes | prost decode | 25290470.51 | 410.02 | 1.00× |
| micro_fields_decode | one_bytes | proto_rs decode | 25886179.75 | 419.68 | 1.02× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 40823546.66 | 661.85 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 36643210.26 | 594.08 | 0.90× slower |


# Benchmark Run — 2025-10-30 18:43:56

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_decode | one_bytes | prost decode | 22435752.32 | 299.55 | 1.00× |
| collection_overhead_decode | one_bytes | proto_rs decode | 23672767.41 | 316.07 | 1.06× faster |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 31688711.37 | 423.09 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 31304985.33 | 417.97 | 0.99× slower |
| micro_fields_decode | one_bytes | prost decode | 22670615.96 | 367.55 | 1.00× |
| micro_fields_decode | one_bytes | proto_rs decode | 23998686.12 | 389.08 | 1.06× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 31376177.73 | 508.69 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 30572622.35 | 495.66 | 0.97× slower |


# Benchmark Run — 2025-10-30 18:38:49

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_decode | one_bytes | prost decode | 20302134.76 | 271.06 | 1.00× |
| collection_overhead_decode | one_bytes | proto_rs decode | 25501617.83 | 340.48 | 1.26× faster |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 40776935.15 | 544.43 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 36557517.68 | 488.10 | 0.90× slower |
| micro_fields_decode | one_bytes | prost decode | 20364882.21 | 330.16 | 1.00× |
| micro_fields_decode | one_bytes | proto_rs decode | 25117755.63 | 407.22 | 1.23× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 40026971.77 | 648.94 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 36201244.88 | 586.91 | 0.90× slower |

# Benchmark Run — 2025-10-30 17:30:44

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_decode | one_bytes | prost decode | 24953045.13 | 333.16 | 1.00× |
| collection_overhead_decode | one_bytes | proto_rs decode | 26467310.41 | 353.38 | 1.06× faster |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 40688756.13 | 543.25 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 36627995.16 | 489.04 | 0.90× slower |
| micro_fields_decode | one_bytes | prost decode | 25029110.28 | 405.78 | 1.00× |
| micro_fields_decode | one_bytes | proto_rs decode | 26047196.02 | 422.29 | 1.04× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 40768685.09 | 660.96 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 36548501.54 | 592.54 | 0.90× slower |

# Benchmark Run — 2025-10-30 17:23:52

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_decode | one_bytes | prost decode | 25095500.19 | 335.06 | 1.00× |
| collection_overhead_decode | one_bytes | proto_rs decode | 26643392.60 | 355.73 | 1.06× faster |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 40701544.29 | 543.42 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 36371767.11 | 485.62 | 0.89× slower |
| micro_fields_decode | one_bytes | prost decode | 25045844.83 | 406.05 | 1.00× |
| micro_fields_decode | one_bytes | proto_rs decode | 26509658.55 | 429.79 | 1.06× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 40824842.50 | 661.87 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 36353784.52 | 589.38 | 0.89× slower |


# Benchmark Run — 2025-10-30 17:09:39

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| complex_root_components_decode | audit log | prost decode | 370802.95 | 263.10 | 1.00× |
| complex_root_components_decode | audit log | proto_rs decode | 334209.58 | 237.13 | 0.90× slower |
| complex_root_components_encode | audit log | prost encode_to_vec | 1132989.71 | 803.89 | 1.00× |
| complex_root_components_encode | audit log | proto_rs encode_to_vec | 1351045.61 | 958.61 | 1.19× faster |


# Benchmark Run — 2025-10-30 17:07:49

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| complex_root_components_decode | audit log | prost decode | 378036.08 | 268.23 | 1.00× |
| complex_root_components_decode | audit log | proto_rs decode | 343549.65 | 243.76 | 0.91× slower |
| complex_root_components_encode | audit log | prost encode_to_vec | 1123465.96 | 797.14 | 1.00× |
| complex_root_components_encode | audit log | proto_rs encode_to_vec | 1359301.27 | 964.47 | 1.21× faster |


# Benchmark Run — 2025-10-30 17:06:26

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| complex_root_components_decode | status lookup | prost decode | 633931.10 | 248.48 | 1.00× |
| complex_root_components_decode | status lookup | proto_rs decode | 552561.64 | 216.58 | 0.87× slower |
| complex_root_components_encode | status lookup | prost encode_to_vec | 1764055.59 | 691.44 | 1.00× |
| complex_root_components_encode | status lookup | proto_rs encode_to_vec | 2039301.34 | 799.32 | 1.16× faster |


# Benchmark Run — 2025-10-30 17:03:17

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| complex_root_components_decode | status lookup | prost decode | 620164.04 | 243.08 | 1.00× |
| complex_root_components_decode | status lookup | proto_rs decode | 489777.45 | 191.97 | 0.79× slower |
| complex_root_components_encode | status lookup | prost encode_to_vec | 1781144.49 | 698.14 | 1.00× |
| complex_root_components_encode | status lookup | proto_rs encode_to_vec | 2030765.70 | 795.98 | 1.14× faster |


# Benchmark Run — 2025-10-30 16:55:30

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_decode | one_bytes | prost decode | 24655108.17 | 329.18 | 1.00× |
| collection_overhead_decode | one_bytes | proto_rs decode | 26549716.96 | 354.48 | 1.08× faster |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 40279400.43 | 537.79 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 36425859.66 | 486.34 | 0.90× slower |
| micro_fields_decode | one_bytes | prost decode | 24838822.92 | 402.70 | 1.00× |
| micro_fields_decode | one_bytes | proto_rs decode | 25035427.85 | 405.89 | 1.00× |
| micro_fields_encode | one_bytes | prost encode_to_vec | 40571827.67 | 657.77 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 36502041.70 | 591.79 | 0.90× slower |


# Benchmark Run — 2025-10-30 16:47:20

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_decode | one_bytes | prost decode | 25356084.44 | 338.54 | 1.00× |
| collection_overhead_decode | one_bytes | proto_rs decode | 24630299.81 | 328.85 | 0.97× slower |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 40705837.67 | 543.48 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 36418693.54 | 486.24 | 0.89× slower |
| micro_fields_decode | one_bytes | prost decode | 24988701.54 | 405.13 | 1.00× |
| micro_fields_decode | one_bytes | proto_rs decode | 26191726.33 | 424.63 | 1.05× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 40252442.82 | 652.59 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 36314193.45 | 588.74 | 0.90× slower |


# Benchmark Run — 2025-10-30 16:20:23

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_decode | one_bytes | prost decode | 24914590.57 | 332.65 | 1.00× |
| collection_overhead_decode | one_bytes | proto_rs decode | 26319443.95 | 351.40 | 1.06× faster |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 40654588.42 | 542.80 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 36432209.52 | 486.42 | 0.90× slower |
| micro_fields_decode | one_bytes | prost decode | 24984699.21 | 405.06 | 1.00× |
| micro_fields_decode | one_bytes | proto_rs decode | 26135011.22 | 423.71 | 1.05× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 40325303.09 | 653.77 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 36252203.65 | 587.74 | 0.90× slower |

# Benchmark Run — 2025-10-30 15:23:45

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_decode | attachments_len1 | prost decode | 18959212.72 | 253.13 | 1.00× |
| collection_overhead_decode | attachments_len1 | proto_rs decode | 21331419.22 | 284.81 | 1.13× faster |
| collection_overhead_decode | codes_len1 | prost decode | 31472390.72 | 90.04 | 1.00× |
| collection_overhead_decode | codes_len1 | proto_rs decode | 36848149.27 | 105.42 | 1.17× faster |
| collection_overhead_decode | deep_list_len1 | prost decode | 779743.34 | 258.78 | 1.00× |
| collection_overhead_decode | deep_list_len1 | proto_rs decode | 745440.43 | 247.40 | 0.96× slower |
| collection_overhead_decode | leaf_lookup_len1 | prost decode | 5067394.40 | 188.47 | 1.00× |
| collection_overhead_decode | leaf_lookup_len1 | proto_rs decode | 4388269.99 | 163.21 | 0.87× slower |
| collection_overhead_decode | leaves_len1 | prost decode | 6536170.99 | 211.93 | 1.00× |
| collection_overhead_decode | leaves_len1 | proto_rs decode | 6496230.30 | 210.64 | 1.00× |
| collection_overhead_decode | one_bytes | prost decode | 25308472.51 | 337.90 | 1.00× |
| collection_overhead_decode | one_bytes | proto_rs decode | 26931194.56 | 359.57 | 1.06× faster |
| collection_overhead_decode | one_complex_enum | prost decode | 7368212.30 | 274.05 | 1.00× |
| collection_overhead_decode | one_complex_enum | proto_rs decode | 5669449.56 | 210.87 | 0.77× slower |
| collection_overhead_decode | one_deep_message | prost decode | 795586.41 | 264.04 | 1.00× |
| collection_overhead_decode | one_deep_message | proto_rs decode | 755624.17 | 250.78 | 0.95× slower |
| collection_overhead_decode | one_enum | prost decode | 56827433.60 | 0.00 | 1.00× |
| collection_overhead_decode | one_enum | proto_rs decode | 58955260.34 | 0.00 | 1.04× faster |
| collection_overhead_decode | one_nested_leaf | prost decode | 6920490.82 | 224.40 | 1.00× |
| collection_overhead_decode | one_nested_leaf | proto_rs decode | 7717024.73 | 250.22 | 1.12× faster |
| collection_overhead_decode | one_string | prost decode | 32406821.57 | 278.15 | 1.00× |
| collection_overhead_decode | one_string | proto_rs decode | 34745457.65 | 298.22 | 1.07× faster |
| collection_overhead_decode | status_history_len1 | prost decode | 6908185.93 | 256.94 | 1.00× |
| collection_overhead_decode | status_history_len1 | proto_rs decode | 5203329.32 | 193.53 | 0.75× slower |
| collection_overhead_decode | tags_len1 | prost decode | 22811194.97 | 195.79 | 1.00× |
| collection_overhead_decode | tags_len1 | proto_rs decode | 25847949.74 | 221.85 | 1.13× faster |
| collection_overhead_encode | attachments_len1 | prost encode_to_vec | 35330683.15 | 471.72 | 1.00× |
| collection_overhead_encode | attachments_len1 | proto_rs encode_to_vec | 36472293.68 | 486.96 | 1.03× faster |
| collection_overhead_encode | codes_len1 | prost encode_to_vec | 42751549.93 | 122.31 | 1.00× |
| collection_overhead_encode | codes_len1 | proto_rs encode_to_vec | 36661189.02 | 104.89 | 0.86× slower |
| collection_overhead_encode | deep_list_len1 | prost encode_to_vec | 2658836.13 | 882.41 | 1.00× |
| collection_overhead_encode | deep_list_len1 | proto_rs encode_to_vec | 2789351.73 | 925.73 | 1.05× faster |
| collection_overhead_encode | leaf_lookup_len1 | prost encode_to_vec | 12607222.03 | 468.90 | 1.00× |
| collection_overhead_encode | leaf_lookup_len1 | proto_rs encode_to_vec | 13305267.31 | 494.87 | 1.06× faster |
| collection_overhead_encode | leaves_len1 | prost encode_to_vec | 15698318.51 | 509.02 | 1.00× |
| collection_overhead_encode | leaves_len1 | proto_rs encode_to_vec | 17753789.01 | 575.67 | 1.13× faster |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 41162765.01 | 549.58 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 36721964.57 | 490.29 | 0.89× slower |
| collection_overhead_encode | one_complex_enum | prost encode_to_vec | 15653551.78 | 582.21 | 1.00× |
| collection_overhead_encode | one_complex_enum | proto_rs encode_to_vec | 14667729.26 | 545.54 | 0.94× slower |
| collection_overhead_encode | one_deep_message | prost encode_to_vec | 2657891.89 | 882.10 | 1.00× |
| collection_overhead_encode | one_deep_message | proto_rs encode_to_vec | 2713208.15 | 900.46 | 1.02× faster |
| collection_overhead_encode | one_enum | prost encode_to_vec | 57636750.52 | 0.00 | 1.00× |
| collection_overhead_encode | one_enum | proto_rs encode_to_vec | 59655764.52 | 0.00 | 1.04× faster |
| collection_overhead_encode | one_nested_leaf | prost encode_to_vec | 16853232.77 | 546.46 | 1.00× |
| collection_overhead_encode | one_nested_leaf | proto_rs encode_to_vec | 16976417.27 | 550.46 | 1.00× |
| collection_overhead_encode | one_string | prost encode_to_vec | 41151445.99 | 353.21 | 1.00× |
| collection_overhead_encode | one_string | proto_rs encode_to_vec | 36930114.70 | 316.97 | 0.90× slower |
| collection_overhead_encode | status_history_len1 | prost encode_to_vec | 15138745.73 | 563.06 | 1.00× |
| collection_overhead_encode | status_history_len1 | proto_rs encode_to_vec | 15723215.17 | 584.80 | 1.04× faster |
| collection_overhead_encode | tags_len1 | prost encode_to_vec | 40507466.81 | 347.68 | 1.00× |
| collection_overhead_encode | tags_len1 | proto_rs encode_to_vec | 36139087.29 | 310.18 | 0.89× slower |
| complex_root_components_decode | attachments | prost decode | 10509167.75 | 340.76 | 1.00× |
| complex_root_components_decode | attachments | proto_rs decode | 11053492.87 | 358.41 | 1.05× faster |
| complex_root_components_decode | audit log | prost decode | 381638.13 | 270.79 | 1.00× |
| complex_root_components_decode | audit log | proto_rs decode | 308981.54 | 219.23 | 0.81× slower |
| complex_root_components_decode | codes | prost decode | 29995140.11 | 143.03 | 1.00× |
| complex_root_components_decode | codes | proto_rs decode | 28622371.80 | 136.48 | 0.95× slower |
| complex_root_components_decode | complex_enum | prost decode | 7428950.89 | 262.14 | 1.00× |
| complex_root_components_decode | complex_enum | proto_rs decode | 6860906.28 | 242.09 | 0.92× slower |
| complex_root_components_decode | deep list | prost decode | 381116.95 | 262.06 | 1.00× |
| complex_root_components_decode | deep list | proto_rs decode | 366576.88 | 252.06 | 0.96× slower |
| complex_root_components_decode | deep lookup | prost decode | 378846.01 | 269.89 | 1.00× |
| complex_root_components_decode | deep lookup | proto_rs decode | 308454.36 | 219.74 | 0.81× slower |
| complex_root_components_decode | deep_message | prost decode | 814112.26 | 267.86 | 1.00× |
| complex_root_components_decode | deep_message | proto_rs decode | 732896.56 | 241.14 | 0.90× slower |
| complex_root_components_decode | leaf lookup | prost decode | 2610169.95 | 199.14 | 1.00× |
| complex_root_components_decode | leaf lookup | proto_rs decode | 2187548.67 | 166.90 | 0.84× slower |
| complex_root_components_decode | leaves list | prost decode | 3203626.81 | 198.59 | 1.00× |
| complex_root_components_decode | leaves list | proto_rs decode | 3322591.25 | 205.96 | 1.04× faster |
| complex_root_components_decode | nested_leaf | prost decode | 7518124.55 | 229.43 | 1.00× |
| complex_root_components_decode | nested_leaf | proto_rs decode | 7941590.80 | 242.36 | 1.06× faster |
| complex_root_components_decode | status history | prost decode | 681883.20 | 270.52 | 1.00× |
| complex_root_components_decode | status history | proto_rs decode | 578235.97 | 229.40 | 0.85× slower |
| complex_root_components_decode | status lookup | prost decode | 659308.74 | 258.42 | 1.00× |
| complex_root_components_decode | status lookup | proto_rs decode | 519446.41 | 203.60 | 0.79× slower |
| complex_root_components_decode | tags | prost decode | 13989527.63 | 360.22 | 1.00× |
| complex_root_components_decode | tags | proto_rs decode | 14720436.70 | 379.04 | 1.05× faster |
| complex_root_components_encode | attachments | prost encode_to_vec | 28644265.31 | 928.79 | 1.00× |
| complex_root_components_encode | attachments | proto_rs encode_to_vec | 31596033.45 | 1024.50 | 1.10× faster |
| complex_root_components_encode | audit log | prost encode_to_vec | 1161913.76 | 824.42 | 1.00× |
| complex_root_components_encode | audit log | proto_rs encode_to_vec | 1376847.72 | 976.92 | 1.18× faster |
| complex_root_components_encode | codes | prost encode_to_vec | 35237296.38 | 168.02 | 1.00× |
| complex_root_components_encode | codes | proto_rs encode_to_vec | 36775921.34 | 175.36 | 1.04× faster |
| complex_root_components_encode | complex_enum | prost encode_to_vec | 17488197.69 | 617.09 | 1.00× |
| complex_root_components_encode | complex_enum | proto_rs encode_to_vec | 17448328.54 | 615.68 | 1.00× |
| complex_root_components_encode | deep list | prost encode_to_vec | 1338614.41 | 920.43 | 1.00× |
| complex_root_components_encode | deep list | proto_rs encode_to_vec | 1423232.47 | 978.61 | 1.06× faster |
| complex_root_components_encode | deep lookup | prost encode_to_vec | 1162646.44 | 828.26 | 1.00× |
| complex_root_components_encode | deep lookup | proto_rs encode_to_vec | 1380311.95 | 983.33 | 1.19× faster |
| complex_root_components_encode | deep_message | prost encode_to_vec | 3059389.73 | 1006.59 | 1.00× |
| complex_root_components_encode | deep_message | proto_rs encode_to_vec | 2968938.68 | 976.83 | 0.97× slower |
| complex_root_components_encode | leaf lookup | prost encode_to_vec | 7534598.28 | 574.84 | 1.00× |
| complex_root_components_encode | leaf lookup | proto_rs encode_to_vec | 8279645.97 | 631.69 | 1.10× faster |
| complex_root_components_encode | leaves list | prost encode_to_vec | 10360208.21 | 642.22 | 1.00× |
| complex_root_components_encode | leaves list | proto_rs encode_to_vec | 10574880.22 | 655.52 | 1.02× faster |
| complex_root_components_encode | nested_leaf | prost encode_to_vec | 19229055.31 | 586.82 | 1.00× |
| complex_root_components_encode | nested_leaf | proto_rs encode_to_vec | 21057294.07 | 642.62 | 1.10× faster |
| complex_root_components_encode | status history | prost encode_to_vec | 2080784.25 | 825.51 | 1.00× |
| complex_root_components_encode | status history | proto_rs encode_to_vec | 2136027.82 | 847.42 | 1.03× faster |
| complex_root_components_encode | status lookup | prost encode_to_vec | 1817654.87 | 712.45 | 1.00× |
| complex_root_components_encode | status lookup | proto_rs encode_to_vec | 2079074.06 | 814.91 | 1.14× faster |
| complex_root_components_encode | tags | prost encode_to_vec | 34731091.91 | 894.30 | 1.00× |
| complex_root_components_encode | tags | proto_rs encode_to_vec | 31600145.24 | 813.68 | 0.91× slower |
| complex_root_decode | prost decode canonical input | 69613.12 | 250.81 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 69542.14 | 250.56 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 59577.10 | 214.66 | 0.86× slower |
| complex_root_decode | proto_rs decode proto_rs input | 59497.93 | 214.37 | 0.86× slower |
| complex_root_encode | prost encode_to_vec | 231082.67 | 832.59 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 242511.26 | 873.76 | 1.05× faster |
| micro_fields_decode | one_bytes | prost decode | 25485629.18 | 413.18 | 1.00× |
| micro_fields_decode | one_bytes | proto_rs decode | 26957880.49 | 437.05 | 1.06× faster |
| micro_fields_decode | one_complex_enum | prost decode | 7352204.75 | 273.45 | 1.00× |
| micro_fields_decode | one_complex_enum | proto_rs decode | 5624061.71 | 209.18 | 0.76× slower |
| micro_fields_decode | one_deep_message | prost decode | 768907.05 | 255.18 | 1.00× |
| micro_fields_decode | one_deep_message | proto_rs decode | 726575.33 | 241.13 | 0.94× slower |
| micro_fields_decode | one_enum | prost decode | 56956842.22 | 0.00 | 1.00× |
| micro_fields_decode | one_enum | proto_rs decode | 58953253.65 | 0.00 | 1.04× faster |
| micro_fields_decode | one_nested_leaf | prost decode | 6859914.47 | 222.43 | 1.00× |
| micro_fields_decode | one_nested_leaf | proto_rs decode | 7787574.26 | 252.51 | 1.14× faster |
| micro_fields_decode | one_string | prost decode | 31454775.95 | 419.97 | 1.00× |
| micro_fields_decode | one_string | proto_rs decode | 32801476.32 | 437.95 | 1.04× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 40837660.59 | 662.08 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 36699278.45 | 594.99 | 0.90× slower |
| micro_fields_encode | one_complex_enum | prost encode_to_vec | 15699323.31 | 583.91 | 1.00× |
| micro_fields_encode | one_complex_enum | proto_rs encode_to_vec | 15089701.46 | 561.24 | 0.96× slower |
| micro_fields_encode | one_deep_message | prost encode_to_vec | 2640414.43 | 876.30 | 1.00× |
| micro_fields_encode | one_deep_message | proto_rs encode_to_vec | 2705824.35 | 898.01 | 1.02× faster |
| micro_fields_encode | one_enum | prost encode_to_vec | 57633063.22 | 0.00 | 1.00× |
| micro_fields_encode | one_enum | proto_rs encode_to_vec | 59595171.23 | 0.00 | 1.03× faster |
| micro_fields_encode | one_nested_leaf | prost encode_to_vec | 16882429.65 | 547.41 | 1.00× |
| micro_fields_encode | one_nested_leaf | proto_rs encode_to_vec | 17622537.38 | 571.41 | 1.04× faster |
| micro_fields_encode | one_string | prost encode_to_vec | 41207777.13 | 550.18 | 1.00× |
| micro_fields_encode | one_string | proto_rs encode_to_vec | 36986794.03 | 493.83 | 0.90× slower |
| zero_copy_vs_clone | prost clone + encode | 118569.93 | 427.21 | 1.00× |
| zero_copy_vs_clone | proto_rs zero_copy | 221340.24 | 797.48 | 1.87× faster |


# Benchmark Run — 2025-10-30 14:56:04

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_encode | attachments_len1 | prost encode_to_vec | 35951256.10 | 480.00 | 1.00× |
| collection_overhead_encode | attachments_len1 | proto_rs encode_to_vec | 35959136.18 | 480.11 | 1.00× |
| collection_overhead_encode | codes_len1 | prost encode_to_vec | 42443533.38 | 121.43 | 1.00× |
| collection_overhead_encode | codes_len1 | proto_rs encode_to_vec | 36601775.66 | 104.72 | 0.86× slower |
| collection_overhead_encode | deep_list_len1 | prost encode_to_vec | 2507062.25 | 832.04 | 1.00× |
| collection_overhead_encode | deep_list_len1 | proto_rs encode_to_vec | 2712999.03 | 900.39 | 1.08× faster |
| collection_overhead_encode | leaf_lookup_len1 | prost encode_to_vec | 12300331.67 | 457.49 | 1.00× |
| collection_overhead_encode | leaf_lookup_len1 | proto_rs encode_to_vec | 12867030.10 | 478.57 | 1.05× faster |
| collection_overhead_encode | leaves_len1 | prost encode_to_vec | 16801376.73 | 544.78 | 1.00× |
| collection_overhead_encode | leaves_len1 | proto_rs encode_to_vec | 17699321.70 | 573.90 | 1.05× faster |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 40840558.60 | 545.28 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 36408340.04 | 486.10 | 0.89× slower |
| collection_overhead_encode | one_complex_enum | prost encode_to_vec | 15598266.71 | 580.15 | 1.00× |
| collection_overhead_encode | one_complex_enum | proto_rs encode_to_vec | 15086036.64 | 561.10 | 0.97× slower |
| collection_overhead_encode | one_deep_message | prost encode_to_vec | 2529593.73 | 839.52 | 1.00× |
| collection_overhead_encode | one_deep_message | proto_rs encode_to_vec | 2560623.14 | 849.82 | 1.01× faster |
| collection_overhead_encode | one_enum | prost encode_to_vec | 57212697.55 | 0.00 | 1.00× |
| collection_overhead_encode | one_enum | proto_rs encode_to_vec | 59399059.92 | 0.00 | 1.04× faster |
| collection_overhead_encode | one_nested_leaf | prost encode_to_vec | 16672595.58 | 540.61 | 1.00× |
| collection_overhead_encode | one_nested_leaf | proto_rs encode_to_vec | 17382640.23 | 563.63 | 1.04× faster |
| collection_overhead_encode | one_string | prost encode_to_vec | 40719948.75 | 349.50 | 1.00× |
| collection_overhead_encode | one_string | proto_rs encode_to_vec | 36474194.36 | 313.06 | 0.90× slower |
| collection_overhead_encode | status_history_len1 | prost encode_to_vec | 15070173.73 | 560.51 | 1.00× |
| collection_overhead_encode | status_history_len1 | proto_rs encode_to_vec | 15680053.36 | 583.19 | 1.04× faster |
| collection_overhead_encode | tags_len1 | prost encode_to_vec | 40347479.05 | 346.31 | 1.00× |
| collection_overhead_encode | tags_len1 | proto_rs encode_to_vec | 36147325.26 | 310.25 | 0.90× slower |
| complex_root_components_encode | attachments | prost encode_to_vec | 27297789.67 | 885.13 | 1.00× |
| complex_root_components_encode | attachments | proto_rs encode_to_vec | 31470226.13 | 1020.42 | 1.15× faster |
| complex_root_components_encode | audit log | prost encode_to_vec | 1157762.41 | 821.47 | 1.00× |
| complex_root_components_encode | audit log | proto_rs encode_to_vec | 1362076.04 | 966.44 | 1.18× faster |
| complex_root_components_encode | codes | prost encode_to_vec | 34652440.22 | 165.24 | 1.00× |
| complex_root_components_encode | codes | proto_rs encode_to_vec | 36444427.24 | 173.78 | 1.05× faster |
| complex_root_components_encode | complex_enum | prost encode_to_vec | 17184665.25 | 606.38 | 1.00× |
| complex_root_components_encode | complex_enum | proto_rs encode_to_vec | 15409911.64 | 543.75 | 0.90× slower |
| complex_root_components_encode | deep list | prost encode_to_vec | 1278833.48 | 879.32 | 1.00× |
| complex_root_components_encode | deep list | proto_rs encode_to_vec | 1403367.50 | 964.95 | 1.10× faster |
| complex_root_components_encode | deep lookup | prost encode_to_vec | 1152755.34 | 821.22 | 1.00× |
| complex_root_components_encode | deep lookup | proto_rs encode_to_vec | 1258854.43 | 896.80 | 1.09× faster |
| complex_root_components_encode | deep_message | prost encode_to_vec | 2973978.82 | 978.49 | 1.00× |
| complex_root_components_encode | deep_message | proto_rs encode_to_vec | 3158293.09 | 1039.13 | 1.06× faster |
| complex_root_components_encode | leaf lookup | prost encode_to_vec | 7526397.61 | 574.22 | 1.00× |
| complex_root_components_encode | leaf lookup | proto_rs encode_to_vec | 8217742.82 | 626.96 | 1.09× faster |
| complex_root_components_encode | leaves list | prost encode_to_vec | 10255948.49 | 635.75 | 1.00× |
| complex_root_components_encode | leaves list | proto_rs encode_to_vec | 10482794.46 | 649.82 | 1.02× faster |
| complex_root_components_encode | nested_leaf | prost encode_to_vec | 18874880.60 | 576.02 | 1.00× |
| complex_root_components_encode | nested_leaf | proto_rs encode_to_vec | 20981486.24 | 640.30 | 1.11× faster |
| complex_root_components_encode | status history | prost encode_to_vec | 2061244.32 | 817.75 | 1.00× |
| complex_root_components_encode | status history | proto_rs encode_to_vec | 2128627.60 | 844.49 | 1.03× faster |
| complex_root_components_encode | status lookup | prost encode_to_vec | 1788880.92 | 701.17 | 1.00× |
| complex_root_components_encode | status lookup | proto_rs encode_to_vec | 2060889.43 | 807.79 | 1.15× faster |
| complex_root_components_encode | tags | prost encode_to_vec | 34060021.63 | 877.02 | 1.00× |
| complex_root_components_encode | tags | proto_rs encode_to_vec | 31414720.04 | 808.90 | 0.92× slower |
| complex_root_decode | prost decode canonical input | 71363.61 | 257.12 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 71376.70 | 257.17 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 58079.29 | 209.26 | 0.81× slower |
| complex_root_decode | proto_rs decode proto_rs input | 57091.70 | 205.70 | 0.80× slower |
| complex_root_encode | prost encode_to_vec | 230376.58 | 830.04 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 244659.31 | 881.50 | 1.06× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 41178507.12 | 667.61 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 36719529.19 | 595.31 | 0.89× slower |
| micro_fields_encode | one_complex_enum | prost encode_to_vec | 15543762.62 | 578.12 | 1.00× |
| micro_fields_encode | one_complex_enum | proto_rs encode_to_vec | 15094418.22 | 561.41 | 0.97× slower |
| micro_fields_encode | one_deep_message | prost encode_to_vec | 2631013.17 | 873.18 | 1.00× |
| micro_fields_encode | one_deep_message | proto_rs encode_to_vec | 2501412.96 | 830.17 | 0.95× slower |
| micro_fields_encode | one_enum | prost encode_to_vec | 57657021.99 | 0.00 | 1.00× |
| micro_fields_encode | one_enum | proto_rs encode_to_vec | 59677900.67 | 0.00 | 1.04× faster |
| micro_fields_encode | one_nested_leaf | prost encode_to_vec | 16764358.58 | 543.58 | 1.00× |
| micro_fields_encode | one_nested_leaf | proto_rs encode_to_vec | 17548469.66 | 569.01 | 1.05× faster |
| micro_fields_encode | one_string | prost encode_to_vec | 41074037.72 | 548.40 | 1.00× |
| micro_fields_encode | one_string | proto_rs encode_to_vec | 36981093.09 | 493.75 | 0.90× slower |
| zero_copy_vs_clone | prost clone + encode | 117238.59 | 422.41 | 1.00× |
| zero_copy_vs_clone | proto_rs zero_copy | 215232.88 | 775.48 | 1.84× faster |


# Benchmark Run — 2025-10-30 14:43:07

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 40984753.37 | 547.21 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 36563284.70 | 488.17 | 0.89× slower |
| micro_fields_encode | one_bytes | prost encode_to_vec | 40784476.76 | 661.22 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 36452944.01 | 590.99 | 0.89× slower |


# Benchmark Run — 2025-10-30 13:54:11

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_encode | attachments_len1 | prost encode_to_vec | 36337277.20 | 485.15 | 1.00× |
| collection_overhead_encode | attachments_len1 | proto_rs encode_to_vec | 36198996.09 | 483.31 | 1.00× |
| collection_overhead_encode | codes_len1 | prost encode_to_vec | 42490788.95 | 121.57 | 1.00× |
| collection_overhead_encode | codes_len1 | proto_rs encode_to_vec | 34902669.25 | 99.86 | 0.82× slower |
| collection_overhead_encode | deep_list_len1 | prost encode_to_vec | 2658867.95 | 882.42 | 1.00× |
| collection_overhead_encode | deep_list_len1 | proto_rs encode_to_vec | 2678835.22 | 889.05 | 1.00× |
| collection_overhead_encode | leaf_lookup_len1 | prost encode_to_vec | 12323759.53 | 458.36 | 1.00× |
| collection_overhead_encode | leaf_lookup_len1 | proto_rs encode_to_vec | 13541459.79 | 503.65 | 1.10× faster |
| collection_overhead_encode | leaves_len1 | prost encode_to_vec | 17178352.04 | 557.01 | 1.00× |
| collection_overhead_encode | leaves_len1 | proto_rs encode_to_vec | 16498745.14 | 534.97 | 0.96× slower |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 41199973.27 | 550.08 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 36796445.24 | 491.29 | 0.89× slower |
| collection_overhead_encode | one_complex_enum | prost encode_to_vec | 15336536.17 | 570.42 | 1.00× |
| collection_overhead_encode | one_complex_enum | proto_rs encode_to_vec | 14629352.22 | 544.11 | 0.95× slower |
| collection_overhead_encode | one_deep_message | prost encode_to_vec | 2666488.76 | 884.95 | 1.00× |
| collection_overhead_encode | one_deep_message | proto_rs encode_to_vec | 2651821.27 | 880.08 | 1.00× |
| collection_overhead_encode | one_enum | prost encode_to_vec | 57616505.36 | 0.00 | 1.00× |
| collection_overhead_encode | one_enum | proto_rs encode_to_vec | 59520410.13 | 0.00 | 1.03× faster |
| collection_overhead_encode | one_nested_leaf | prost encode_to_vec | 16792017.49 | 544.48 | 1.00× |
| collection_overhead_encode | one_nested_leaf | proto_rs encode_to_vec | 17578641.11 | 569.99 | 1.05× faster |
| collection_overhead_encode | one_string | prost encode_to_vec | 41168651.65 | 353.35 | 1.00× |
| collection_overhead_encode | one_string | proto_rs encode_to_vec | 37045785.52 | 317.97 | 0.90× slower |
| collection_overhead_encode | status_history_len1 | prost encode_to_vec | 15037051.97 | 559.28 | 1.00× |
| collection_overhead_encode | status_history_len1 | proto_rs encode_to_vec | 14048601.46 | 522.51 | 0.93× slower |
| collection_overhead_encode | tags_len1 | prost encode_to_vec | 40210536.20 | 345.13 | 1.00× |
| collection_overhead_encode | tags_len1 | proto_rs encode_to_vec | 36405124.41 | 312.47 | 0.91× slower |
| complex_root_components_encode | attachments | prost encode_to_vec | 27656305.36 | 896.75 | 1.00× |
| complex_root_components_encode | attachments | proto_rs encode_to_vec | 31591402.86 | 1024.35 | 1.14× faster |
| complex_root_components_encode | audit log | prost encode_to_vec | 1155473.84 | 819.85 | 1.00× |
| complex_root_components_encode | audit log | proto_rs encode_to_vec | 1334398.70 | 946.80 | 1.15× faster |
| complex_root_components_encode | codes | prost encode_to_vec | 35140473.08 | 167.56 | 1.00× |
| complex_root_components_encode | codes | proto_rs encode_to_vec | 36682345.82 | 174.92 | 1.04× faster |
| complex_root_components_encode | complex_enum | prost encode_to_vec | 17090631.72 | 603.06 | 1.00× |
| complex_root_components_encode | complex_enum | proto_rs encode_to_vec | 17465034.50 | 616.27 | 1.02× faster |
| complex_root_components_encode | deep list | prost encode_to_vec | 1365564.18 | 938.96 | 1.00× |
| complex_root_components_encode | deep list | proto_rs encode_to_vec | 1366392.48 | 939.53 | 1.00× |
| complex_root_components_encode | deep lookup | prost encode_to_vec | 1154755.96 | 822.64 | 1.00× |
| complex_root_components_encode | deep lookup | proto_rs encode_to_vec | 1346622.03 | 959.33 | 1.17× faster |
| complex_root_components_encode | deep_message | prost encode_to_vec | 3043214.45 | 1001.27 | 1.00× |
| complex_root_components_encode | deep_message | proto_rs encode_to_vec | 3042927.00 | 1001.18 | 1.00× |
| complex_root_components_encode | leaf lookup | prost encode_to_vec | 7470519.97 | 569.96 | 1.00× |
| complex_root_components_encode | leaf lookup | proto_rs encode_to_vec | 8083628.19 | 616.73 | 1.08× faster |
| complex_root_components_encode | leaves list | prost encode_to_vec | 10328580.25 | 640.26 | 1.00× |
| complex_root_components_encode | leaves list | proto_rs encode_to_vec | 10345921.07 | 641.33 | 1.00× |
| complex_root_components_encode | nested_leaf | prost encode_to_vec | 18994082.90 | 579.65 | 1.00× |
| complex_root_components_encode | nested_leaf | proto_rs encode_to_vec | 19642523.74 | 599.44 | 1.03× faster |
| complex_root_components_encode | status history | prost encode_to_vec | 2077441.28 | 824.18 | 1.00× |
| complex_root_components_encode | status history | proto_rs encode_to_vec | 2101914.34 | 833.89 | 1.01× faster |
| complex_root_components_encode | status lookup | prost encode_to_vec | 1791720.61 | 702.28 | 1.00× |
| complex_root_components_encode | status lookup | proto_rs encode_to_vec | 1997987.64 | 783.13 | 1.12× faster |
| complex_root_components_encode | tags | prost encode_to_vec | 34555195.40 | 889.77 | 1.00× |
| complex_root_components_encode | tags | proto_rs encode_to_vec | 31730535.68 | 817.04 | 0.92× slower |
| complex_root_decode | prost decode canonical input | 73751.58 | 265.73 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 73595.36 | 265.16 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 59051.58 | 212.76 | 0.80× slower |
| complex_root_decode | proto_rs decode proto_rs input | 59037.16 | 212.71 | 0.80× slower |
| complex_root_encode | prost encode_to_vec | 231049.75 | 832.47 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 232501.48 | 837.70 | 1.00× |
| micro_fields_encode | one_bytes | prost encode_to_vec | 41091089.45 | 666.19 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 36643740.83 | 594.09 | 0.89× slower |
| micro_fields_encode | one_complex_enum | prost encode_to_vec | 15785716.27 | 587.12 | 1.00× |
| micro_fields_encode | one_complex_enum | proto_rs encode_to_vec | 14951232.21 | 556.09 | 0.95× slower |
| micro_fields_encode | one_deep_message | prost encode_to_vec | 2657382.67 | 881.93 | 1.00× |
| micro_fields_encode | one_deep_message | proto_rs encode_to_vec | 2644356.06 | 877.61 | 1.00× |
| micro_fields_encode | one_enum | prost encode_to_vec | 57517631.67 | 0.00 | 1.00× |
| micro_fields_encode | one_enum | proto_rs encode_to_vec | 59573471.33 | 0.00 | 1.04× faster |
| micro_fields_encode | one_nested_leaf | prost encode_to_vec | 16876683.78 | 547.23 | 1.00× |
| micro_fields_encode | one_nested_leaf | proto_rs encode_to_vec | 17579966.55 | 570.03 | 1.04× faster |
| micro_fields_encode | one_string | prost encode_to_vec | 41073651.89 | 548.39 | 1.00× |
| micro_fields_encode | one_string | proto_rs encode_to_vec | 36902272.53 | 492.70 | 0.90× slower |
| zero_copy_vs_clone | prost clone + encode | 118928.45 | 428.50 | 1.00× |
| zero_copy_vs_clone | proto_rs zero_copy | 209306.83 | 754.13 | 1.76× faster |


# Benchmark Run — 2025-10-30 13:40:06

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 40819810.19 | 545.00 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 40862461.74 | 545.57 | 1.00× |
| micro_fields_encode | one_bytes | prost encode_to_vec | 40573865.02 | 657.80 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 40881654.36 | 662.79 | 1.00× |


# Benchmark Run — 2025-10-30 13:23:35

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 41213676.04 | 550.26 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 31844119.36 | 425.16 | 0.77× slower |
| micro_fields_encode | one_bytes | prost encode_to_vec | 41143430.63 | 667.04 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 31694892.30 | 513.85 | 0.77× slower |


# Benchmark Run — 2025-10-30 13:00:13

| Group | Benchmark | Ops / s | MiB/s | Speedup vs Prost |
| --- | --- | ---: | ---: | ---: |
| collection_overhead_encode | attachments_len1 | prost encode_to_vec | 36312888.92 | 484.83 | 1.00× |
| collection_overhead_encode | attachments_len1 | proto_rs encode_to_vec | 29693175.11 | 396.45 | 0.82× slower |
| collection_overhead_encode | codes_len1 | prost encode_to_vec | 42525350.82 | 121.67 | 1.00× |
| collection_overhead_encode | codes_len1 | proto_rs encode_to_vec | 44433777.69 | 127.13 | 1.04× faster |
| collection_overhead_encode | deep_list_len1 | prost encode_to_vec | 2637954.04 | 875.48 | 1.00× |
| collection_overhead_encode | deep_list_len1 | proto_rs encode_to_vec | 1944165.35 | 645.23 | 0.74× slower |
| collection_overhead_encode | leaf_lookup_len1 | prost encode_to_vec | 12423248.41 | 462.06 | 1.00× |
| collection_overhead_encode | leaf_lookup_len1 | proto_rs encode_to_vec | 8813476.17 | 327.80 | 0.71× slower |
| collection_overhead_encode | leaves_len1 | prost encode_to_vec | 17068935.29 | 553.46 | 1.00× |
| collection_overhead_encode | leaves_len1 | proto_rs encode_to_vec | 9447998.45 | 306.35 | 0.55× slower |
| collection_overhead_encode | one_bytes | prost encode_to_vec | 41064583.85 | 548.27 | 1.00× |
| collection_overhead_encode | one_bytes | proto_rs encode_to_vec | 30169548.70 | 402.81 | 0.73× slower |
| collection_overhead_encode | one_complex_enum | prost encode_to_vec | 15704350.26 | 584.10 | 1.00× |
| collection_overhead_encode | one_complex_enum | proto_rs encode_to_vec | 9177823.72 | 341.35 | 0.58× slower |
| collection_overhead_encode | one_deep_message | prost encode_to_vec | 2650862.48 | 879.76 | 1.00× |
| collection_overhead_encode | one_deep_message | proto_rs encode_to_vec | 2089901.02 | 693.59 | 0.79× slower |
| collection_overhead_encode | one_enum | prost encode_to_vec | 57544013.37 | 0.00 | 1.00× |
| collection_overhead_encode | one_enum | proto_rs encode_to_vec | 59431081.26 | 0.00 | 1.03× faster |
| collection_overhead_encode | one_nested_leaf | prost encode_to_vec | 16846528.83 | 546.25 | 1.00× |
| collection_overhead_encode | one_nested_leaf | proto_rs encode_to_vec | 8847953.92 | 286.89 | 0.53× slower |
| collection_overhead_encode | one_string | prost encode_to_vec | 41227924.56 | 353.86 | 1.00× |
| collection_overhead_encode | one_string | proto_rs encode_to_vec | 32551446.34 | 279.39 | 0.79× slower |
| collection_overhead_encode | status_history_len1 | prost encode_to_vec | 15018841.37 | 558.60 | 1.00× |
| collection_overhead_encode | status_history_len1 | proto_rs encode_to_vec | 9033552.65 | 335.99 | 0.60× slower |
| collection_overhead_encode | tags_len1 | prost encode_to_vec | 40371843.50 | 346.51 | 1.00× |
| collection_overhead_encode | tags_len1 | proto_rs encode_to_vec | 31801682.29 | 272.96 | 0.79× slower |
| complex_root_components_encode | attachments | prost encode_to_vec | 27600114.01 | 894.93 | 1.00× |
| complex_root_components_encode | attachments | proto_rs encode_to_vec | 11002567.24 | 356.76 | 0.40× slower |
| complex_root_components_encode | audit log | prost encode_to_vec | 1156071.73 | 820.27 | 1.00× |
| complex_root_components_encode | audit log | proto_rs encode_to_vec | 1202360.99 | 853.12 | 1.04× faster |
| complex_root_components_encode | codes | prost encode_to_vec | 35175853.78 | 167.73 | 1.00× |
| complex_root_components_encode | codes | proto_rs encode_to_vec | 39648028.66 | 189.06 | 1.13× faster |
| complex_root_components_encode | complex_enum | prost encode_to_vec | 17223409.69 | 607.74 | 1.00× |
| complex_root_components_encode | complex_enum | proto_rs encode_to_vec | 8426199.94 | 297.33 | 0.49× slower |
| complex_root_components_encode | deep list | prost encode_to_vec | 1362630.09 | 936.94 | 1.00× |
| complex_root_components_encode | deep list | proto_rs encode_to_vec | 1191256.82 | 819.11 | 0.87× slower |
| complex_root_components_encode | deep lookup | prost encode_to_vec | 1166558.71 | 831.05 | 1.00× |
| complex_root_components_encode | deep lookup | proto_rs encode_to_vec | 1188918.12 | 846.98 | 1.02× faster |
| complex_root_components_encode | deep_message | prost encode_to_vec | 3040740.50 | 1000.46 | 1.00× |
| complex_root_components_encode | deep_message | proto_rs encode_to_vec | 2445429.26 | 804.59 | 0.80× slower |
| complex_root_components_encode | leaf lookup | prost encode_to_vec | 7515367.30 | 573.38 | 1.00× |
| complex_root_components_encode | leaf lookup | proto_rs encode_to_vec | 5532280.99 | 422.08 | 0.74× slower |
| complex_root_components_encode | leaves list | prost encode_to_vec | 10341452.12 | 641.05 | 1.00× |
| complex_root_components_encode | leaves list | proto_rs encode_to_vec | 6426081.54 | 398.35 | 0.62× slower |
| complex_root_components_encode | nested_leaf | prost encode_to_vec | 18793431.05 | 573.53 | 1.00× |
| complex_root_components_encode | nested_leaf | proto_rs encode_to_vec | 12717912.27 | 388.12 | 0.68× slower |
| complex_root_components_encode | status history | prost encode_to_vec | 2070628.85 | 821.48 | 1.00× |
| complex_root_components_encode | status history | proto_rs encode_to_vec | 1739088.27 | 689.95 | 0.84× slower |
| complex_root_components_encode | status lookup | prost encode_to_vec | 1796997.64 | 704.35 | 1.00× |
| complex_root_components_encode | status lookup | proto_rs encode_to_vec | 1722998.10 | 675.35 | 0.96× slower |
| complex_root_components_encode | tags | prost encode_to_vec | 34610394.61 | 891.19 | 1.00× |
| complex_root_components_encode | tags | proto_rs encode_to_vec | 16741611.55 | 431.08 | 0.48× slower |
| complex_root_decode | prost decode canonical input | 74896.45 | 269.85 | 1.00× |
| complex_root_decode | prost decode proto_rs input | 74502.77 | 268.43 | 1.00× |
| complex_root_decode | proto_rs decode canonical input | 59179.45 | 213.22 | 0.79× slower |
| complex_root_decode | proto_rs decode proto_rs input | 59185.18 | 213.24 | 0.79× slower |
| complex_root_encode | prost encode_to_vec | 231494.63 | 834.07 | 1.00× |
| complex_root_encode | proto_rs encode_to_vec | 243368.45 | 876.85 | 1.05× faster |
| micro_fields_encode | one_bytes | prost encode_to_vec | 41085304.96 | 666.09 | 1.00× |
| micro_fields_encode | one_bytes | proto_rs encode_to_vec | 29935111.56 | 485.32 | 0.73× slower |
| micro_fields_encode | one_complex_enum | prost encode_to_vec | 15726597.62 | 584.92 | 1.00× |
| micro_fields_encode | one_complex_enum | proto_rs encode_to_vec | 9206389.26 | 342.42 | 0.59× slower |
| micro_fields_encode | one_deep_message | prost encode_to_vec | 2640536.41 | 876.34 | 1.00× |
| micro_fields_encode | one_deep_message | proto_rs encode_to_vec | 2099841.48 | 696.89 | 0.80× slower |
| micro_fields_encode | one_enum | prost encode_to_vec | 57580351.91 | 0.00 | 1.00× |
| micro_fields_encode | one_enum | proto_rs encode_to_vec | 59547306.74 | 0.00 | 1.03× faster |
| micro_fields_encode | one_nested_leaf | prost encode_to_vec | 16880096.39 | 547.34 | 1.00× |
| micro_fields_encode | one_nested_leaf | proto_rs encode_to_vec | 9071377.67 | 294.14 | 0.54× slower |
| micro_fields_encode | one_string | prost encode_to_vec | 41052589.30 | 548.11 | 1.00× |
| micro_fields_encode | one_string | proto_rs encode_to_vec | 31950326.80 | 426.58 | 0.78× slower |
| zero_copy_vs_clone | prost clone + encode | 119110.80 | 429.15 | 1.00× |
| zero_copy_vs_clone | proto_rs zero_copy | 220700.06 | 795.18 | 1.85× faster |





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



