<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — 50 vòng stress **release-mode** (10 batch × 5 × 10k)

**Trạng thái**: hardening release-mode, **không sửa production/test code** — chỉ journal + log.
Không nối evaluator, không bật default, **không** GitHub Actions.
**Baseline**: `71d52b4` (CLI merged, gate 3530); `git merge main` fast-forward trên
`vs3/w5-spike`; **không** push/merge.
**Commit**: chỉ journal này + `vs3-w5-50vong-release.log`; không đổi `crates/**`.

## 1. Cách chạy

- Chạy test `stress_dai_5x10k_cancel_budget_print_cross` thành **10 batch**, mỗi batch 5 vòng
  × 10k → **50 vòng × 10k** (500k task/pha) cho **release**, và **50 vòng** tương ứng cho
  **debug** (10 cặp xen kẽ release/debug).
- Dừng ngay nếu một batch lỗi; thực tế **10/10 cặp pass**, **50/50 vòng release**,
  **50/50 vòng debug**.
- Release target: `/home/ps/ViScript/target/vs3-w5-release`; debug target:
  `/home/ps/ViScript/target/vs3-w5`.
- Log đầy đủ: [`vs3-w5-50vong-release.log`](vs3-w5-50vong-release.log) (323 dòng, 20 batch +
  summary + monitor).

Máy: i5-1235U (2 P-core + 8 E-core), `nproc=12`, Linux; build **release**.
Load 1m lúc bắt đầu batch: **9,94–16,21** (hệ thống có tải ngoài).

## 2. Kết quả 50 vòng release (so debug)

| Batch | Load 1m | Release total (s) | Debug total (s) | Release RSS1/HWM (KB) | Debug RSS1/HWM (KB) |
|---:|---:|---:|---:|---:|---:|
| 1 | 9,94 | 14,44 | 13,74 | 6.960 / 10.328 | 10.420 / 10.944 |
| 2 | 13,11 | 12,86 | 14,25 | 8.176 / 10.168 | 10.740 / 10.740 |
| 3 | 16,04 | 11,34 | 11,21 | 10.144 / 10.144 | 10.188 / 10.368 |
| 4 | 14,71 | 10,20 | 10,91 | 10.124 / 10.124 | 7.392 / 9.820 |
| 5 | 12,44 | 10,14 | 12,73 | 7.212 / 9.616 | 10.408 / 10.408 |
| 6 | 12,07 | 10,20 | 14,68 | 7.240 / 9.100 | 11.456 / 11.456 |
| 7 | 14,01 | 13,47 | 14,30 | 6.896 / 10.108 | 8.424 / 10.772 |
| 8 | 15,13 | 11,98 | 12,90 | 10.332 / 10.332 | 10.700 / 10.700 |
| 9 | 16,21 | 10,43 | 10,98 | 10.576 / 10.576 | 7.484 / 10.040 |
| 10 | 13,90 | 10,11 | 11,19 | 10.132 / 10.132 | 10.332 / 10.332 |
| **Median** | — | **10,88** | **12,82** | **9.150 / 10.138** | **10.370 / 10.554** |

| Pha (median 10 batch) | Release | Debug | Release nhanh |
|---|---:|---:|---:|
| Print | **0,40 s** | 0,79 s | ~2,0× |
| Cancel | **9,86 s** | 10,71 s | ~8% |
| Budget | **0,32 s** | 0,51 s | ~1,6× |
| Cross-write | **0,29 s** | 0,53 s | ~1,8× |
| **Tổng batch** | **10,88 s** | **12,82 s** | **~15%** |

- **50/50 vòng release** và **50/50 vòng debug**: `used=5.000`, `remaining=0`,
  `used+remaining==limit=5.000`, `exhausted=true`, đủ 10k outcome.
- **Print 50/50**: 10k dòng, W=1 == W=4 **byte-identical**.
- **Cancel 50/50**: join đủ 10k, `cancelled <= 5k`, W=1/W=4 không deadlock.
- **Cross-write 50/50**: `vars=["counter"]`, đủ 10k task id, trả `Err`.
- Per-round (50 vòng): release median **2,12 s** (1,98–3,05), debug **2,35 s** (2,12–3,34).
- Tổng thời gian test: release **115,17 s**, debug **126,89 s** cho 50 vòng mỗi build.

## 3. RSS / threads

| Đối tượng | Release | Debug |
|---|---:|---:|
| RSS sau vòng (median / range) | **10.046 KB** (9.680–10.864) | 10.330 KB (9.848–11.456) |
| RSS1 sau batch (median) | **9.150 KB** | 10.370 KB |
| HWM batch (median / max) | **10.138 / 10.576 KB** | 10.554 / 11.456 KB |
| Monitor max RSS (batch) | 9.816–10.864 KB | 10.220–11.456 KB |
| Peak threads (W=4) | **6** (2 batch bắt 7) | **6** |
| Threads sau mỗi vòng | **2** | **2** |
| `ru_maxrss` (batch) | ~12,5 MB | ~12,5–12,6 MB |

- Không rò thread: `threads0→threads1 = 2→2` ở **10/10 batch**, cả 50 vòng release.
- RSS vòng ổn định ~9,7–10,9 MB (release), không tăng đơn điệu; HWM max ~10,6 MB.
- Peak 6 thread khi W=4 (main + 4 worker + harness), về 2 sau pool.

## 4. Full suite / kiểm chứng

- `cargo test --release -p vis-cli --test vs3_worker`: **12 passed / 0 failed / 0 ignored**,
  **10,08 s** (load ~9,2); debug cùng lúc **10,86 s** (load ~8,7).
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy -p vis-cli --all-targets -- -D warnings`: PASS.
- df `/home`: **17 GB free** (>5G).
- Không sửa production/test; không thêm `#[ignore]`; không push/merge.

## 5. Release vs debug — kết luận

- **Stress 50 vòng**: release **10,88 s** vs debug **12,82 s** median (~15% nhanh hơn); các pha
  CPU (print/budget/cross) nhanh ~1,6–2,0×; cancel chỉ ~8% vì sleep 200 µs chi phối.
- **Full suite**: release **10,08 s** vs debug **10,86 s** (~7% nhanh hơn) — cùng lý do stress
  chiếm phần lớn thời gian suite.
- **RSS/threads**: tương đương; release không tăng RSS, không rò thread.
- **Budget/print/cancel/cross**: giống hệt về ngữ nghĩa; chỉ khác thời gian.

## 6. Blocker / giới hạn

- **Evaluator chưa nối**: stress vẫn closure + `SendValue`, chưa phải `async fn`/frame/VM thật.
- **Cancel W=1** bị chi phối bởi sleep 200 µs nhân tạo; muốn đo throughput cancel thuần cần test
  riêng không sleep.
- **Chưa soak 24h**: 50 vòng (~2 phút release) đủ bắt rò cấp vòng, chưa phải soak dài.
- **Load biến động mạnh** 9,9–16,2 trong lúc chạy; máy hybrid 2P+8E, số tuyệt đối phụ thuộc tải.
- **RSS ~10 MB là của test**, không phải cap production.
- **Map/payload lớn**: không lặp lại ma trận B7; worker-copy list10k/map1k vẫn là tham chiếu riêng.
- Không tuyên bố worker pool production/default; không đổi `Value`/bytecode/cache.

## 7. Tái lập

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5-release \
  cargo test --release -p vis-cli --test vs3_worker stress_dai \
    -- --nocapture --test-threads=1
```

Debug đối chiếu: bỏ `--release` và dùng `CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5`.
