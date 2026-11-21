<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — 100 vòng stress **release-mode** trên `bbf3484`

**Trạng thái**: hardening release-mode, **không sửa production/test code** — chỉ journal + log.
Không nối evaluator, không bật default, **không** GitHub Actions.
**Baseline**: `bbf3484` (v0.3.3 tagged); `git merge main` (`fefe9bd`) trên `vs3/w5-spike`;
**không** push/merge. `git diff bbf3484..HEAD` cho **worker/test/example/Cargo là rỗng** — code
chạy stress giống tag; khác biệt chỉ ở docs/net ngoài phạm vi W5.
**Commit**: chỉ journal này + `vs3-w5-100vong-release.log`; không đổi `crates/**`.

## 1. Cách chạy

- Chạy `stress_dai_5x10k_cancel_budget_print_cross` thành **20 batch**, mỗi batch 5 vòng × 10k
  → **100 vòng × 10k** (1M task/pha), **release**.
- Vòng lặp **dừng ngay** nếu batch lỗi; thực tế **20/20 batch pass**, **100/100 vòng**.
- Release target: `/home/ps/ViScript/target/vs3-w5-release`; build release sau merge.
- Log đầy đủ: [`vs3-w5-100vong-release.log`](vs3-w5-100vong-release.log) (330 dòng, 20 batch +
  monitor + full suite).

Máy: i5-1235U (2 P-core + 8 E-core), `nproc=12`, Linux; build **release**.
Load 1m lúc bắt đầu batch: **9,20–18,48** (hệ thống có tải ngoài).

## 2. Kết quả 20 batch (mỗi batch 5 vòng × 10k)

| Batch | Load | Total (s) | Print | Cancel | Budget | Cross | RSS1 (KB) | HWM (KB) | Threads |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 9,20 | 11,77 | 0,34 | 10,72 | 0,35 | 0,35 | 7.360 | 9.624 | 2 → 2 |
| 2 | 10,61 | 13,89 | 0,54 | 12,11 | 0,59 | 0,64 | 10.396 | 10.412 | 2 → 2 |
| 3 | 13,02 | 12,29 | 0,51 | 11,01 | 0,41 | 0,35 | 10.112 | 10.112 | 2 → 2 |
| 4 | 13,54 | 12,46 | 0,52 | 11,15 | 0,47 | 0,31 | 6.948 | 9.560 | 2 → 2 |
| 5 | 14,56 | 11,70 | 0,46 | 10,50 | 0,42 | 0,31 | 6.996 | 9.304 | 2 → 2 |
| 6 | 16,07 | 12,12 | 0,56 | 10,70 | 0,37 | 0,49 | 6.960 | 10.216 | 2 → 2 |
| 7 | 17,22 | 11,41 | 0,71 | 9,77 | 0,51 | 0,42 | 7.160 | 9.744 | 2 → 2 |
| 8 | 16,74 | 10,49 | 0,34 | 9,48 | 0,38 | 0,28 | 7.208 | 9.492 | 2 → 2 |
| 9 | 15,54 | 10,46 | 0,35 | 9,52 | 0,29 | 0,29 | 7.316 | 9.692 | 2 → 2 |
| 10 | 14,70 | 10,36 | 0,33 | 9,55 | 0,25 | 0,23 | 7.140 | 9.512 | 2 → 2 |
| 11 | 13,73 | 10,30 | 0,28 | 9,58 | 0,24 | 0,20 | 10.288 | 10.288 | 2 → 2 |
| 12 | 13,16 | 11,17 | 0,43 | 10,17 | 0,29 | 0,28 | 7.240 | 9.552 | 2 → 2 |
| 13 | 13,02 | 11,32 | 0,37 | 10,15 | 0,42 | 0,38 | 7.220 | 9.768 | 2 → 2 |
| 14 | 12,39 | 14,45 | 0,60 | 12,83 | 0,70 | 0,33 | 9.924 | 10.152 | 2 → 2 |
| 15 | 14,98 | 12,48 | 0,56 | 11,12 | 0,42 | 0,38 | 7.232 | 10.744 | 2 → 2 |
| 16 | 15,28 | 13,40 | 0,58 | 11,91 | 0,49 | 0,42 | 11.116 | 11.116 | 2 → 2 |
| 17 | 16,86 | 12,13 | 0,63 | 10,56 | 0,49 | 0,45 | 7.000 | 9.336 | 2 → 2 |
| 18 | 18,24 | 12,02 | 0,69 | 10,10 | 0,68 | 0,55 | 6.932 | 10.020 | 2 → 2 |
| 19 | 18,48 | 10,52 | 0,32 | 9,69 | 0,25 | 0,26 | 10.036 | 10.612 | 2 → 2 |
| 20 | 16,77 | 10,61 | 0,43 | 9,52 | 0,31 | 0,35 | 7.300 | 9.868 | 2 → 2 |
| **Median** | — | **11,73** | **0,48** | **10,34** | **0,41** | **0,35** | **7.236** | **9.818** | **2 → 2** |

- 20/20 batch `test result: ok. 1 passed; 0 failed; 0 ignored`.
- **100/100 vòng**: budget `used=5.000`, `remaining=0`, `used+remaining==limit=5.000`,
  `exhausted=true`, đủ 10k outcome.
- **100/100 vòng**: print 10k dòng, W=1 == W=4 **byte-identical**.
- **100/100 vòng**: cancel join đủ 10k, `cancelled <= 5k`, W=1/W=4 không deadlock.
- **100/100 vòng**: cross-write `vars=["counter"]`, đủ 10k task id, trả `Err`.
- Per-round time: median **2,26 s** (2,00–2,99); batch total **10,30–14,45 s**.
- Tổng thời gian test 20 batch: **235,35 s** (~3,9 phút).

## 3. RSS / threads

| Đối tượng | Giá trị |
|---|---:|
| RSS sau vòng (median / range) | **9.968 KB** (9.660–11.260) |
| RSS1 sau batch (median / range) | **7.236 KB** (6.932–11.116) |
| HWM batch (median / max) | **9.818 / 11.116 KB** |
| Monitor max RSS (batch) | 9.876–11.260 KB |
| `ru_maxrss` (batch) | ~12,3–12,7 MB |
| Peak threads (W=4) | **6** (5 batch bắt 7) |
| Threads sau mỗi vòng | **2** (100/100) |

- First10 batch RSS1 median **7.184 KB**, HWM **9.658 KB**; last10 **7.270 / 10.086 KB** —
  chênh nhỏ, không tăng đơn điệu; load cuối cao hơn (18,5) nên HWM nhích.
- Không rò thread: `threads0→threads1 = 2→2` ở 20/20 batch; 100/100 round sample = 2.
- Peak 6 thread khi W=4 (main + 4 worker + harness), về 2 sau pool.

## 4. Full suite / kiểm chứng

- `cargo test --release -p vis-cli --test vs3_worker`: **12 passed / 0 failed / 0 ignored**,
  **10,35 s** (load ~14,75); `time -v` RSS ~32 MB (cargo).
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy -p vis-cli --all-targets -- -D warnings`: PASS.
- df `/home`: **17 GB free** (>5G).
- Không sửa production/test; không `#[ignore]`; không push/merge.

## 5. So với các vòng trước

- 100 vòng release hiện tại: median batch **11,73 s**, round **2,26 s**, trong điều kiện load
  **9,2–18,5**.
- 50 vòng release trên `71d52b4` (load 9,9–16,2): median batch **10,88 s**, round **2,12 s**.
  Chênh chủ yếu do load cao hơn; cùng pattern: cancel chiếm ~88% (sleep 200 µs nhân tạo).
- RSS/thread pattern không đổi: ~10 MB, 2 thread sau pool, peak 6–7 khi W=4.

## 6. Blocker / giới hạn

- **Evaluator chưa nối**: stress vẫn closure + `SendValue`, chưa phải `async fn`/frame/VM thật.
- **Cancel W=1** bị chi phối bởi sleep 200 µs nhân tạo; chưa đo throughput cancel thuần.
- **Chưa soak 24h**: 100 vòng (~4 phút) đủ bắt rò cấp vòng, chưa phải soak hàng giờ.
- **Load biến động mạnh** 9,2–18,5; máy hybrid 2P+8E, số tuyệt đối phụ thuộc tải.
- **RSS ~10 MB là của test**, không phải cap production.
- **Map/payload lớn**: không lặp lại ma trận B7/map alloc; worker-copy list10k/map1k vẫn là
  tham chiếu riêng (`vs3-w5-map-alloc.md`).
- Không tuyên bố worker pool production/default; không đổi `Value`/bytecode/cache.

## 7. Tái lập

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5-release \
  cargo test --release -p vis-cli --test vs3_worker stress_dai \
    -- --nocapture --test-threads=1
```
