<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — 100 vòng stress `WorkerPool` (10k/vòng)

**Trạng thái**: hardening run dài, **không sửa production/test code**. **Không** nối evaluator,
không bật default, **không** GitHub Actions.
**Baseline**: `d1f8f70` (v0.3.2 ổn định); `git merge main` fast-forward trên `vs3/w5-spike`;
**không** push/merge.
**Commit**: chỉ journal này + `vs3-w5-100vong.log`; không đổi `crates/**`.

## 1. Cách chạy

Chạy test `stress_dai_5x10k_cancel_budget_print_cross` thành **20 batch**, mỗi batch 5 vòng × 10k
→ **100 vòng × 10k** (1.000.000 task mỗi pha; 4 pha = 4M lượt task). Vòng lặp **dừng ngay**
nếu một batch trả mã lỗi; thực tế **20/20 batch pass**.

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5 \
  cargo test -q -p vis-cli --test vs3_worker stress_dai \
    -- --nocapture --test-threads=1
```

Mỗi vòng kiểm: print W=1 == W=4 **byte-identical** 10k dòng; cancel nửa 10k W=1/W=4;
budget 10k task `limit=5.000` chunk 256; cross-write 10k W=4; thread/RSS sau vòng.
Log đầy đủ: [`vs3-w5-100vong.log`](vs3-w5-100vong.log) (20 batch + 100 round + summary + load).

## 2. Kết quả 20 batch

Máy: i5-1235U (2 P-core + 8 E-core), `nproc=12`; build **debug**; load 1m tăng dần
0,47 → 16,18 trong lúc chạy (hệ thống có tải ngoài), nên thời gian batch sau cao hơn.

| Batch | Load 1m | Total (s) | Cancel | Print | Budget | Cross | RSS0 → RSS1 (KB) | HWM (KB) | Threads |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 0,47 | 10,25 | 9,47 | 0,35 | 0,20 | 0,22 | 3.296 → 7.556 | 9.928 | 2 → 2 |
| 2 | 1,65 | 10,25 | 9,45 | 0,36 | 0,20 | 0,23 | 3.296 → 10.340 | 10.340 | 2 → 2 |
| 3 | 2,17 | 10,20 | 9,45 | 0,34 | 0,19 | 0,22 | 3.312 → 10.420 | 10.420 | 2 → 2 |
| 4 | 2,68 | 10,26 | 9,45 | 0,37 | 0,19 | 0,23 | 3.280 → 10.216 | 10.216 | 2 → 2 |
| 5 | 3,04 | 10,56 | 9,54 | 0,42 | 0,28 | 0,32 | 3.296 → 10.432 | 10.432 | 2 → 2 |
| 6 | 3,56 | 10,94 | 9,62 | 0,58 | 0,35 | 0,39 | 3.296 → 10.300 | 10.300 | 2 → 2 |
| 7 | 3,86 | 10,91 | 9,62 | 0,60 | 0,32 | 0,36 | 3.292 → 7.480 | 10.080 | 2 → 2 |
| 8 | 4,04 | 14,15 | 11,74 | 1,08 | 0,69 | 0,63 | 3.264 → 10.220 | 10.220 | 2 → 2 |
| 9 | 6,79 | 12,68 | 10,62 | 0,83 | 0,54 | 0,69 | 3.268 → 7.436 | 9.684 | 2 → 2 |
| 10 | 7,78 | 12,79 | 10,69 | 1,10 | 0,47 | 0,52 | 3.324 → 11.128 | 11.128 | 2 → 2 |
| 11 | 7,66 | 11,33 | 9,80 | 0,59 | 0,47 | 0,47 | 3.328 → 7.596 | 9.944 | 2 → 2 |
| 12 | 7,70 | 17,09 | 13,90 | 1,36 | 0,79 | 1,03 | 3.328 → 11.280 | 11.280 | 2 → 2 |
| 13 | 10,85 | 14,43 | 12,08 | 1,13 | 0,61 | 0,60 | 3.280 → 10.388 | 10.388 | 2 → 2 |
| 14 | 12,70 | 14,38 | 12,05 | 0,81 | 0,74 | 0,79 | 3.312 → 11.340 | 11.340 | 2 → 2 |
| 15 | 14,61 | 14,63 | 12,02 | 1,06 | 0,84 | 0,71 | 3.296 → 10.264 | 10.264 | 2 → 2 |
| 16 | 16,18 | 12,07 | 10,03 | 0,92 | 0,54 | 0,57 | 3.296 → 7.336 | 9.816 | 2 → 2 |
| 17 | 14,92 | 11,14 | 9,66 | 0,74 | 0,34 | 0,40 | 3.324 → 10.368 | 10.808 | 2 → 2 |
| 18 | 13,70 | 11,04 | 9,64 | 0,62 | 0,36 | 0,41 | 3.324 → 7.340 | 9.468 | 2 → 2 |
| 19 | 12,30 | 10,76 | 9,68 | 0,45 | 0,30 | 0,33 | 3.280 → 10.268 | 10.268 | 2 → 2 |
| 20 | 11,49 | 12,79 | 10,93 | 0,88 | 0,42 | 0,55 | 3.268 → 7.536 | 10.132 | 2 → 2 |
| **Median** | — | **11,23** | **9,74** | **0,68** | **0,39** | **0,44** | **3.296 → 10.266** | **10.266** | **2 → 2** |

- 20/20 batch `test result: ok. 1 passed; 0 failed; 0 ignored`.
- Tổng thời gian test (cộng 20 batch): **242,65 s** (~4 phút).
- Total batch **10,20–17,09 s**; batch 12 cao nhất khi load 7,7 (13,90 s cancel).
- **Cancel vẫn chiếm phần lớn** (~9,5–13,9 s) vì W=1 chạy ~5k task × sleep 200 µs mỗi vòng.
- **RSS vòng** 9,8–11,4 MB; HWM 9,2–11,4 MB; **không tăng theo thời gian**:
  first10 round RSS median **10.228 KB**, last10 **10.246 KB**; HWM median **10.260 → 10.266 KB**.
- **Threads = 2** ở **100/100 vòng** và cuối mỗi batch **2 → 2** — không rò worker thread.

### Thời gian 100 vòng

- Median **2,22 s/vòng**; range **2,03–3,74 s**.
- 5 vòng/batch đều ~2,0–2,2 s khi load thấp; tăng ~2,5–3,7 s khi load 8–16.
- Không có vòng nào timeout hay lỗi; 100/100 join trả về.

## 3. Kiểm chứng

- **Budget 100/100 vòng**: `used=5.000`, `remaining=0`, `used+remaining==limit=5.000`,
  `exhausted=true`, đủ 10k outcome. Không reset, không lệch sau join.
- **Print 100/100 vòng**: 10k dòng, W=1 == W=4 **byte-identical**.
- **Cancel 100/100 vòng**: join đủ 10k, `cancelled <= 5k`, W=1/W=4 không deadlock.
- **Cross-write 100/100 vòng**: `vars=["counter"]`, đủ 10k task id, trả `Err` thay vì merge dở.
- **Không rò thread**: 2 trước/sau cả 20 batch; 100/100 round sample = 2.
- **RSS không rò**: first10 vs last10 gần như nhau; HWM max 11.340 KB.
- Full suite `cargo test -p vis-cli --test vs3_worker`: **12 passed / 0 failed / 0 ignored**,
  **14,91 s**.
- `cargo fmt --all -- --check`: PASS; `cargo clippy -p vis-cli --all-targets -- -D warnings`: PASS.
- df `/home`: **18 GB free** (>5G).

## 4. Blocker / giới hạn

- **Evaluator chưa nối**: stress vẫn là closure + `SendValue`, chưa phải `async fn`/frame/VM thật.
- **Cancel W=1** bị chi phối bởi sleep 200 µs nhân tạo; chưa đo throughput cancel thuần.
- **Chưa soak 24h**: 100 vòng (~4 phút, 1M task/pha) đủ bắt rò thread/RSS cấp vòng, nhưng chưa
  phải soak hàng giờ; RSS ~10 MB là của test, không phải cap production.
- **Load biến động mạnh** 0,47–16,18 trong lúc chạy; máy hybrid 2P+8E, số tuyệt đối phụ thuộc tải.
- **Test stress +~11 s** vào suite; không dùng `#[ignore]`, không đổi production.
- **Map/payload lớn**: không lặp lại ma trận B7; worker-copy list10k/map1k vẫn là tham chiếu riêng.
- Không tuyên bố worker pool production/default; không đổi `Value`/bytecode/cache.
