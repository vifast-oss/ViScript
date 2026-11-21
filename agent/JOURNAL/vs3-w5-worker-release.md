<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — release-mode worker tests + benchmark (debug vs release)

**Trạng thái**: hardening release-mode, **không sửa production/test code**. Không nối evaluator,
không bật default. **Không** GitHub Actions.
**Baseline**: `14310f4` (v0.3.2); `git merge main` fast-forward trên `vs3/w5-spike`;
**không** push/merge.
**Commit**: chỉ journal này + `vs3-w5-worker-release.log`; không đổi `crates/**`.

## 1. Cách đo

- Release target riêng: `/home/ps/ViScript/target/vs3-w5-release`; debug target cũ:
  `/home/ps/ViScript/target/vs3-w5`.
- `cargo test --release -p vis-cli --test vs3_worker` — **12 passed / 0 failed / 0 ignored**.
- `cargo run --release -p vis-cli --example vs3_worker_bench` — matrix/payload, chunk, probe,
  check (`print W=1==W=4 124 byte`, cross-write, budget).
- Benchmark chạy **5 cặp release/debug xen kẽ**; test stress chạy **5 cặp xen kẽ**; đo thêm
  full suite một cặp ở load thấp.
- RSS/threads: chạy binary trực tiếp dưới monitor `/proc/<pid>/status` (poll 2 ms), lấy
  `VmRSS`/`Threads` cao nhất; `ru_maxrss` tham chiếu. Log đầy đủ:
  [`vs3-w5-worker-release.log`](vs3-w5-worker-release.log) (1011 dòng).

Máy: i5-1235U (2 P-core + 8 E-core), `nproc=12`, Linux; load 1m lúc đo **3,1–13,2**
(benchmark 8,5–11,8; stress 3,1–13,2). Build release mất 2m48s.

## 2. Test suite / stress

| Phép đo | Debug | Release | Nhận xét |
|---|---:|---:|---|
| Full suite (cùng load 4,2–4,8) | **10,16 s** | **9,81 s** | release nhanh ~3,4% (test bị sleep chi phối) |
| Full suite lần release đầu (load 10,45) | — | 14,13 s | 12 passed; load cao |
| Stress 5 vòng × 10k, median 5 lần | **11,10 s** | **10,05 s** | release nhanh ~9,5% |
| — print | 0,56 s | **0,26 s** | ~2,2× |
| — cancel | 9,78 s | 9,44 s | sleep 200 µs chi phối |
| — budget | 0,32 s | **0,19 s** | ~1,7× |
| — cross-write | 0,39 s | **0,18 s** | ~2,2× |

- Cả hai build: **100%** vòng (5 lần × 5 vòng = 25 vòng/build) giữ
  `used+remaining==limit`, print W=1 == W=4 10k dòng, cancel/cross đúng.
- Release full suite: **12 passed / 0 failed / 0 ignored**; stress vẫn là test 5×10k,
  không `#[ignore]`.

## 3. Payload matrix (W=4, median 5 lần chạy)

`ms` = thời gian W=4; `×` = speedup so với W=1 cùng lần. Mọi dòng có
`used=6.400 / remaining=10.000 / limit=16.400`.

### 3.1 Producer-copy (`MATRIX`)

| Payload | Debug ms | Debug × | Release ms | Release × |
|---|---:|---:|---:|---:|
| **unit (CPU-only)** | 4,26 | 2,87 | 5,09 | **2,68** |
| list1k | 5,23 | 3,57 | 5,32 | 2,56 |
| list10k | 33,52 | 2,13 | **8,41** | **4,00** |
| map1k | 33,55 | **0,77** | 29,37 | **0,82** |
| record | 5,31 | 3,83 | 5,82 | 3,62 |
| enum | 5,11 | 3,99 | 5,86 | 4,04 |
| str1MiB | 9,05 | 4,93 | 11,66 | 4,46 |

### 3.2 Worker-copy (`MATRIX_WC`)

| Payload | Debug ms | Debug × | Release ms | Release × |
|---|---:|---:|---:|---:|
| **unit (CPU-only)** | 4,25 | 3,57 | 5,55 | **3,67** |
| list1k | 5,16 | 3,38 | 5,62 | 3,68 |
| **list10k** | 12,70 | 3,41 | **7,60** | **3,38** |
| **map1k** | 6,70 | 3,17 | **5,52** | **3,64** |
| record | 5,29 | 3,35 | 4,28 | 2,88 |
| enum | 6,19 | 3,16 | 4,12 | 2,51 |
| str1MiB | 8,01 | 2,78 | 6,81 | **2,26** |

- **Release cải thiện mạnh producer-copy list10k** (33,52 → 8,41 ms; ×2,13 → 4,00) và
  worker-copy list10k (12,70 → 7,60 ms), map worker-copy (6,70 → 5,52 ms).
- **Map producer-copy vẫn dưới 1×** (0,82 release / 0,77 debug) — blocker allocator/drop như B7;
  worker-copy map đạt 3,64×.
- **str1MiB worker-copy release 2,26×** (dưới ngưỡng 2,4); producer-copy 4,46× vẫn tốt hơn.
- **unit worker-copy release 3,67×** — PASS ngưỡng 2,4; absolute ms cao hơn debug một phần do
  calibration release median 235,3 µs/task vs debug 222,8 µs/task (xem caveat).

### 3.3 Chunk reservation (`CHUNK`, W=4, 64 task × 2.000 charge)

| chunk | Debug ms | Debug ns/charge | Release ms | Release ns/charge |
|---:|---:|---:|---:|---:|
| 1 | 21,66 | 169,2 | 16,80 | **131,3** |
| 16 | 6,69 | 52,2 | 4,58 | 35,8 |
| **256** | 6,04 | 47,2 | **3,54** | **27,6** |
| 4096 | 5,92 | 46,2 | 3,45 | 27,0 |

Release giảm ~1,7× chi phí charge; **256** vẫn là đề xuất (4096 tương đương, chunk lớn giữ
reservation lâu hơn).

### 3.4 PROBE list10k/map1k (ms, W=4)

| Payload | Mode | Debug | Release |
|---|---|---:|---:|
| list10k | producer | 34,76 | **9,21** |
| list10k | worker-copy | 12,93 | **6,62** |
| list10k | move | 10,09 | 10,19 |
| map1k | producer | 27,91 | 24,69 |
| map1k | worker-copy | 6,24 | 7,74 |
| map1k | move | 9,97 | 12,76 |

Map move/producer vẫn kém; list10k producer release nhanh hơn rõ rệt.

## 4. RSS / threads

| Đối tượng | Debug | Release |
|---|---:|---:|
| Benchmark example — max RSS (poll) | 71,0 MB | **70,7 MB** |
| Benchmark — max threads | 5 (peak 7) | 5 (peak 6) |
| Test stress — max RSS (poll) | 10,2–11,3 MB | **10,0–10,3 MB** |
| Test stress — max threads | 6 | 6 (peak 7) |
| Sau pool / hết vòng | 2 | 2 |
| Full suite — cargo RSS (`time -v`) | 31,9 MB | 31,9 MB |

- Benchmark ~70 MB chủ yếu do payload list10k/map1k/str1MiB + matrix 64 task; **không phải**
  cap production.
- Stress ~10 MB: output/outcome/allocator; thread về **2** sau mỗi vòng, không rò.
- Peak thread khi W=4 là main + 4 worker (+ harness); monitor bắt 5–7 tuỳ thời điểm.

## 5. Debug vs release — kết luận

- **Test suite**: release 9,81 s vs debug 10,16 s (cùng load) — chỉ ~3,4% vì stress bị sleep
  200 µs chi phối; phase CPU (print/budget/cross) nhanh hơn ~1,7–2,2×.
- **Benchmark CPU/payload**: release nhanh hơn rõ ở producer list10k (4,0× so với 2,1×),
  worker list10k/map1k (7,6/5,5 ms so với 12,7/6,7 ms), chunk 256 (27,6 vs 47,2 ns/charge).
  Một số payload nhỏ (unit/record/enum/str) dao động do calibration/load; **speedup** ổn định hơn
  absolute ms.
- **RSS**: hai build tương đương (benchmark ~70,7–71,0 MB; stress ~10–11 MB); release không
  tăng RSS.
- **Threads**: cùng pattern 5–7 đỉnh khi W=4, về 2 sau pool; không rò.
- **Check**: 5/5 lần benchmark release + debug đều `print_merge 124 byte`, cross-write,
  `used+remaining==limit` OK.

## 6. Blocker / giới hạn

- **Map producer-copy** vẫn <1× (0,82 release / 0,77 debug); worker-copy map đạt nhưng move/
  producer cần điều tra allocator/drop nếu W1 chọn mode đó.
- **str1MiB worker-copy release 2,26×** dưới 2,4; producer-copy 4,46× nên chọn theo payload.
- **Calibration/load**: release calib median 235,3 µs vs debug 222,8 µs (target 200 µs); absolute
  ms cross-build có sai số, dùng speedup/relative. Load 3,1–13,2, máy hybrid 2P+8E.
- **Evaluator chưa nối**: benchmark/stress vẫn closure + `SendValue`, chưa phải `async fn`/
  frame/VM thật.
- **Chưa soak release dài**: stress 5 cặp ×5 vòng; chưa chạy 100 vòng release như debug.
- **RSS benchmark ~70 MB** là của example, không phải cap production.
- Test suite release không nhanh hơn nhiều vì sleep; không tuyên bố release tăng tốc mọi thứ.
- Không tuyên bố worker pool production/default; không đổi `Value`/bytecode/cache.

## 7. Tái lập

```bash
# Test release
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5-release \
  cargo test --release -p vis-cli --test vs3_worker

# Benchmark release
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5-release \
  cargo run --release -p vis-cli --example vs3_worker_bench
```

Debug đối chiếu: bỏ `--release` và dùng `CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5`.
