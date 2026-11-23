<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — 10 lần chạy stress dài `WorkerPool` (5 × 10k mỗi lần)

**Trạng thái**: hardening run, **không sửa production/test code**. **Không** nối evaluator,
không bật default. **Không** GitHub Actions.
**Baseline**: `70aeddc` (v0.3.2 tagged); `git merge main` fast-forward trên `vs3/w5-spike`;
**không** push/merge.
**Commit**: chỉ journal này + `vs3-w5-10vong.log`; không đổi `crates/**`.

## 1. Cách chạy

Chạy **10 lần** test `stress_dai_5x10k_cancel_budget_print_cross` (mỗi lần **5 vòng × 10k**),
trên main `70aeddc`, build debug:

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5 \
  cargo test -q -p vis-cli --test vs3_worker stress_dai \
    -- --nocapture --test-threads=1
```

- Tổng: **10 lần × 5 vòng = 50 vòng 10k**; **500k task/pha** (print/cancel/budget/cross).
- Mỗi vòng kiểm: print W=1 == W=4 **byte-identical** 10k dòng; cancel nửa 10k; budget
  `limit=5.000`/chunk 256; cross-write 10k W=4; thread/RSS sau vòng.
- Log đầy đủ: [`vs3-w5-10vong.log`](vs3-w5-10vong.log) (10 run, từng round + summary).
- 10/10 run exit 0; tất cả `test result: ok. 1 passed; 0 failed; 0 ignored`.

## 2. Kết quả 10 lần

| Run | Load 1m | Total (s) | Print | Cancel | Budget | Cross | RSS0 (KB) | RSS1 (KB) | HWM (KB) | Threads |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 1,08 | 11,09 | 0,28 | 10,42 | 0,19 | 0,19 | 3.244 | 10.668 | 10.668 | 2 → 2 |
| 2 | 0,84 | 11,03 | 0,25 | 10,44 | 0,16 | 0,17 | 3.312 | 7.580 | 9.324 | 2 → 2 |
| 3 | 0,87 | 10,74 | 0,32 | 9,86 | 0,27 | 0,29 | 3.280 | 7.324 | 9.212 | 2 → 2 |
| 4 | 1,35 | 11,11 | 0,48 | 10,09 | 0,24 | 0,30 | 3.308 | 7.588 | 9.764 | 2 → 2 |
| 5 | 2,92 | 10,27 | 0,32 | 9,53 | 0,20 | 0,22 | 3.292 | 10.356 | 10.356 | 2 → 2 |
| 6 | 3,38 | 10,20 | 0,34 | 9,46 | 0,19 | 0,21 | 3.328 | 10.368 | 10.368 | 2 → 2 |
| 7 | 3,47 | 10,23 | 0,36 | 9,45 | 0,20 | 0,22 | 3.264 | 10.428 | 10.428 | 2 → 2 |
| 8 | 3,79 | 10,39 | 0,37 | 9,49 | 0,24 | 0,29 | 3.296 | 7.556 | 9.680 | 2 → 2 |
| 9 | 4,06 | 10,86 | 0,60 | 9,58 | 0,32 | 0,36 | 3.296 | 7.476 | 10.340 | 2 → 2 |
| 10 | 4,20 | 10,69 | 0,51 | 9,55 | 0,30 | 0,32 | 3.280 | 11.440 | 11.440 | 2 → 2 |
| **Median** | — | **10,71** | **0,35** | **9,57** | **0,22** | **0,26** | **3.294** | **8.972** | **10.348** | **2 → 2** |

- Total range **10,20–11,11 s**; 10 lần cộng lại **106,61 s**.
- **Cancel ~89%** thời gian vì W=1 chạy tuần tự ~5k task/vòng × sleep 200 µs (nhân tạo);
  print/budget/cross mỗi lần chỉ 0,16–0,60 s.
- **RSS vòng** ~9,8–11,4 MB; cuối kỳ 7,3–11,4 MB (allocator trả bớt ở một số lần);
  HWM **9,2–11,4 MB**, không tăng theo số lần chạy.
- **Threads = 2** ở mọi vòng của cả 10 lần (50/50 round), cuối mỗi lần **2 → 2**.
- Không deadlock: 10/10 lần hoàn tất trong ~11 s, không timeout.

### Thời gian từng vòng (median 10 lần)

| Vòng | Median (s) | Range (s) |
|---:|---:|---:|
| 0 | 2,12 | 2,05–2,46 |
| 1 | 2,09 | 2,03–2,30 |
| 2 | 2,11 | 2,03–2,23 |
| 3 | 2,08 | 2,03–2,23 |
| 4 | 2,14 | 2,02–2,57 |

### RSS từng vòng (median 10 lần, KB)

| Vòng | Median | Range |
|---:|---:|---:|
| 0 | 10.134 | 9.832–10.360 |
| 1 | 10.280 | 9.936–10.692 |
| 2 | 10.332 | 9.936–11.440 |
| 3 | 10.326 | 9.940–11.440 |
| 4 | 10.362 | 10.084–11.440 |

## 3. Kiểm chứng

- **Budget 50/50 vòng**: `used=5.000`, `remaining=0`, `used+remaining==limit=5.000`,
  `exhausted=true`, đủ 10k outcome. Không reset, không lệch sau join.
- **Print 50/50 vòng**: 10k dòng, W=1 == W=4 byte-identical.
- **Cancel 50/50 vòng**: join đủ 10k, `cancelled <= 5k`, W=1/W=4 không treo.
- **Cross-write 50/50 vòng**: `vars=["counter"]`, đủ 10k task id, trả `Err` thay vì merge dở.
- **Thread**: 2 trước/sau cả 10 lần; không rò worker thread.
- **RSS**: không tăng đơn điệu qua 10 lần; HWM ~10,3 MB median.
- Full suite `cargo test -p vis-cli --test vs3_worker`: **12 passed / 0 failed / 0 ignored**,
  **10,74 s**.
- `cargo fmt --all -- --check`: PASS; `cargo clippy -p vis-cli --all-targets -- -D warnings`: PASS.
- df `/home`: **19 GB free** (>5G).

## 4. Blocker / giới hạn

- **Evaluator chưa nối**: stress vẫn là closure + `SendValue`, chưa phải `async fn`/frame/VM thật.
- **Cancel W=1** bị chi phối bởi sleep 200 µs nhân tạo; chưa đo throughput cancel thuần.
- **Chưa soak hàng giờ**: 10 lần (~107 s, 50 vòng) đủ bắt rò thread/RSS cấp vòng, nhưng chưa
  phải soak 24h; RSS ~10 MB là của test, không phải cap production.
- **Load biến động** 0,84–4,20 (1m) trong lúc chạy; máy hybrid 2P+8E nên số tuyệt đối phụ thuộc tải.
- **Suite +~11 s** cho test stress; nếu CI yếu có thể giảm số vòng có kiểm soát, nhưng
  **không** dùng `#[ignore]`.
- **Map/payload lớn**: không lặp lại ma trận B7; worker-copy list10k/map1k vẫn là tham chiếu riêng.
- Không tuyên bố worker pool production/default; không đổi `Value`/bytecode/cache.
