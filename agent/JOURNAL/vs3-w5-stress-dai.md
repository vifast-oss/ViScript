<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — stress dài `WorkerPool` 5 × 10k task

**Trạng thái**: stress + đo, không đổi runtime/API. **Không** nối evaluator, không bật default.
**Baseline**: `77e2899` (v0.3.2 tagged); đã `git merge main` trên `vs3/w5-spike`; **không** push/merge.
**Phạm vi sửa**: `crates/vis-cli/tests/vs3_worker.rs`, journal này, 1 dòng trỏ tiếp ở
`vs3-w5-stress-50k.md`. Không đụng `interp.rs`/`vm.rs`/`frame/**`/`async_net.rs`/`net/*`/`pkg.rs`.

## 1. Test

`stress_dai_5x10k_cancel_budget_print_cross` — **5 vòng × 10k task**, **không** `#[ignore]`,
chạy trong suite thường. Thay test 50k một lượt (đã pass ở commit `2b02176`): tổng task mỗi pha
vẫn **50k**, nhưng tạo/drop pool 5 lần nên thấy RSS/thread theo vòng và bắt rò tốt hơn.

Mỗi vòng (`N = 10.000`) đủ 4 pha:

| Pha | Nội dung | Kiểm mỗi vòng |
|---|---|---|
| Print | W=1 và W=4, mỗi task 1 dòng | 10k outcome; output W=1 == W=4 **byte-identical**; đúng **10.000 dòng** |
| Cancel | 10k task, huỷ **một nửa** handle trước join; check cancel trước/sau sleep 200 µs | join đủ 10k; `cancelled <= 5k`; W=1/W=4 không deadlock |
| Budget | 10k task, `limit=5.000`, chunk 256, mỗi task `charge(1)` | `used=5.000`; `remaining=0`; `used+remaining==limit`; `exhausted=true`; 10k outcome |
| Cross-write | 10k task W=4 cùng ghi `counter` | `CrossWrite { vars:["counter"], tasks:10k }`; không treo |
| Thread | `Threads:` sau mỗi vòng | **2** ở cả 5 vòng; cuối **2 → 2** |
| RSS | `/proc/self/status` sau mỗi vòng | xem §2; cuối kỳ vọng không tăng >256 MB (assert) |

Máy: i5-1235U (2 P-core + 8 E-core), `nproc=12`; build **debug** (`cargo test`).
Ba lần đo riêng `--test-threads=1`; 1-min load lúc chạy: **12,81 / 11,99 / 11,07**.

## 2. Số đo (3 lần)

| Lần | Total | Print | Cancel | Budget | Cross | RSS0 (KB) | RSS1 (KB) | HWM (KB) | Threads |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 11,23 | 0,63 | 9,80 | 0,39 | 0,41 | 3.408 | 10.376 | 10.376 | 2 → 2 |
| 2 | 10,87 | 0,58 | 9,61 | 0,32 | 0,35 | 3.412 | 10.320 | 10.320 | 2 → 2 |
| 3 | 10,67 | 0,43 | 9,65 | 0,28 | 0,31 | 3.296 | 7.304 | 9.928 | 2 → 2 |
| **Median** | **10,87** | **0,58** | **9,65** | **0,32** | **0,35** | **3.408** | **10.320** | **10.320** | **2 → 2** |

Thời gian từng vòng (3 lần):

| Vòng | Lần 1 | Lần 2 | Lần 3 | Median |
|---:|---:|---:|---:|---:|
| 0 | 2,37 | 2,19 | 2,08 | **2,19** |
| 1 | 2,20 | 2,19 | 2,09 | **2,19** |
| 2 | 2,21 | 2,17 | 2,10 | **2,17** |
| 3 | 2,25 | 2,15 | 2,19 | **2,19** |
| 4 | 2,19 | 2,16 | 2,22 | **2,19** |

RSS sau từng vòng (KB):

| Vòng | Lần 1 | Lần 2 | Lần 3 | Median |
|---:|---:|---:|---:|---:|
| 0 | 9.904 | 9.912 | 9.872 | **9.904** |
| 1 | 10.300 | 9.948 | 9.868 | **9.948** |
| 2 | 10.300 | 9.984 | 10.036 | **10.036** |
| 3 | 10.376 | 9.988 | 10.172 | **10.172** |
| 4 | 10.376 | 10.320 | 10.632 | **10.376** |

- **Cancel chiếm ~89%** thời gian (9,65/10,87 s) vì mỗi vòng W=1 chạy tuần tự ~5k task ×
  sleep 200 µs (giới hạn nhân tạo để task kịp thấy cancel), không phải chi phí pool.
  Print/budget/cross mỗi vòng rất nhẹ (0,28–0,63 s tổng 5 vòng mỗi pha).
- **RSS ổn định**: ~9,9–10,6 MB sau mỗi vòng, không tăng đơn điệu; chênh vòng 0 → 4 ~0,5 MB
  (median), cuối kỳ có lúc allocator trả về 7,3 MB. HWM ~9,9–10,4 MB. Đây là RSS của **test**,
  không phải cap production.
- **Thread 2 → 2** sau cả 5 vòng: không rò worker thread.
- Không có deadlock: 5 vòng × 4 pha đều join trả về; budget cạn vẫn trả đủ 10k outcome;
  cross-write trả lỗi thay vì treo.

## 3. Kiểm chứng

- `cargo test -p vis-cli --test vs3_worker`: **12 passed / 0 failed / 0 ignored**, **13,90 s**
  (mặc định song song). Không thêm `#[ignore]`; skipped suite giữ **3** (các suite khác).
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy -p vis-cli --all-targets -- -D warnings`: PASS.
- df `/home`: **8,7 GB free** (>5G).
- Budget từng vòng: `used=5.000`, `remaining=0`, `used+remaining==5.000`, `exhausted=true`.
- Print từng vòng: 10k dòng, W=1 == W=4 byte-identical.
- Cross-write từng vòng: đủ 10k task id, `vars=["counter"]`.

## 4. Blocker / giới hạn

- **Evaluator chưa nối**: stress vẫn là closure + `SendValue`, chưa phải `async fn`/frame/VM
  thật; W1 1d/3b sở hữu hook.
- **Cancel W=1** bị chi phối bởi sleep nhân tạo 200 µs; muốn đo throughput cancel thuần cần
  test riêng không sleep.
- **Chưa soak hàng giờ**: 5 vòng (50k task/pha) đủ bắt rò thread/RSS cấp vòng, chưa phải soak
  dài 24h; RSS chỉ ~10 MB vì mỗi vòng drop pool, chưa đo cao điểm 50k cùng lúc.
- **RSS ~10 MB là của test**, gồm output/outcome/allocator; không phải cap production.
- **Suite +~13 s** so với trước: phù hợp gate/nightly; nếu CI yếu có thể giảm số vòng có kiểm
  soát, nhưng **không** dùng `#[ignore]`.
- **Load/hybrid**: 1-min load 11–13 khi đo; 2P+8E nên số tuyệt đối phụ thuộc tải.
- **Map/payload lớn**: stress này không lặp lại ma trận B7; worker-copy list10k/map1k vẫn là
  tham chiếu riêng (`vs3-w5-worker-bench.md`).
- Không tuyên bố worker pool production/default; không đổi `Value`/bytecode/cache.

## 5. Tái lập

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5 \
  cargo test -p vis-cli --test vs3_worker stress_dai -- --nocapture --test-threads=1
```

Toàn suite:

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5 \
  cargo test -p vis-cli --test vs3_worker
```
