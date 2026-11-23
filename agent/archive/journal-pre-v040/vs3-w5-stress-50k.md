<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — stress `WorkerPool` **50k task**

**Kế tiếp**: test 50k một lượt được thay bằng **5×10k vòng** ở
[`vs3-w5-stress-dai.md`](vs3-w5-stress-dai.md) (tổng 50k task/pha, đo RSS theo vòng).

**Trạng thái**: stress + đo, không đổi runtime/API. **Không** nối evaluator, không bật default.
**Baseline**: `a33f46c`; đã `git merge main` trên `vs3/w5-spike`; **không** push/merge.
**Phạm vi sửa**: `crates/vis-cli/tests/vs3_worker.rs` + journal này. Không đụng
`interp.rs`/`vm.rs`/`frame/**`/`async_net.rs`/`net/*`/`pkg.rs`.

## 1. Test

Nâng stress cũ 10k → **50k** trong `crates/vis-cli/tests/vs3_worker.rs`:
`stress_50k_cancel_budget_print_cross`, **không** `#[ignore]`, chạy trong suite thường.
10k cũ được thay (50k phủ rộng hơn, cùng 4 pha); không thêm test trùng.

| Pha | Nội dung | Kiểm |
|---|---|---|
| Print | 50k task, W=1 và W=4, mỗi task 1 dòng | 50k outcome; output W=1 == W=4 **byte-identical**; đúng **50.000 dòng** |
| Cancel | 50k task, huỷ **một nửa** handle trước join; task check cancel trước/sau sleep 200 µs | join đủ 50k outcome; `cancelled <= 25k`; W=1/W=4 không deadlock |
| Budget | 50k task, pool `limit=25.000`, chunk 256, mỗi task `charge(1)` | `used=25.000`; `remaining=0`; `used+remaining==limit`; `exhausted=true`; 50k outcome, task lỗi surfaced |
| Cross-write | 50k task W=4 cùng ghi `counter` | `WorkerError::CrossWrite { vars:["counter"], tasks:50k }`; không merge output, không treo |
| Thread | `Threads:` trước/sau | **2 → 2**, không rò worker thread |
| RSS | `/proc/self/status` trước/sau | đo dưới đây; HWM = RSS cuối |

Máy: i5-1235U (2 P-core + 8 E-core), `nproc=12`; build **debug** (`cargo test`), không phải release.
Mỗi lần chạy riêng `--test-threads=1`; 1-min load lúc chạy: **12,97 / 11,58 / 10,11**.

## 2. Số đo (3 lần)

| Lần | total (s) | print | cancel | budget | cross | RSS0 (KB) | RSS1 (KB) | HWM (KB) | Threads |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 10,62 | 0,60 | 9,59 | 0,20 | 0,24 | 3.392 | 34.820 | 34.820 | 2 → 2 |
| 2 | 10,31 | 0,37 | 9,52 | 0,16 | 0,26 | 3.360 | 37.708 | 37.708 | 2 → 2 |
| 3 | 10,13 | 0,27 | 9,51 | 0,17 | 0,18 | 3.388 | 37.508 | 37.508 | 2 → 2 |
| **Median** | **10,31** | **0,37** | **9,52** | **0,17** | **0,24** | **3.388** | **37.508** | **37.508** | **2 → 2** |

- **50k khả thi**, không cần fallback 20k: 10,1–10,6 s, dưới ngưỡng assert 120 s.
- **Cancel chiếm ~92% thời gian** vì pha W=1 chạy tuần tự ~25k task × sleep 200 µs (giới hạn
  nhân tạo để task kịp thấy cancel), không phải chi phí pool. Print/budget/cross rất nhẹ
  (0,17–0,37 s mỗi pha).
- **RSS** ~3,3 MB → ~36,6 MiB (tăng ~33 MiB): 50k `TaskOutcome` + output 50k dòng của cả
  W=1/W=4 + allocator giữ; **thread về baseline**. Đây là RSS của test, **không phải cap
  production**.
- Full suite `vs3_worker` chạy mặc định (song song): **12 passed / 0 failed / 0 ignored**,
  **10,81 s**. Không thêm `#[ignore]`; skipped suite giữ **3**.
- `cargo fmt --all -- --check`: PASS. `cargo clippy -p vis-cli --all-targets -- -D warnings`: PASS.
- df `/home`: **16 GB free** (>5G).

## 3. Đọc kết quả

- **Không deadlock**: cả 4 pha join trả về; budget cạn vẫn trả đủ 50k outcome; cross-write
  trả lỗi thay vì treo.
- **Không rò thread**: `Threads` 2 → 2 sau khi pool drop; không có worker sống sót.
- **Print merge ổn định**: 50k dòng, W=1 == W=4 byte-identical → merge theo spawn order giữ
  nguyên ở quy mô 50k.
- **Budget atomic**: `used+remaining==limit` đúng ở 50k task/25k limit; chunk 256 không làm
  lệch accounting sau join.
- **Cross-write**: phát hiện đủ 50k task ghi chéo, không merge một phần.

## 4. Blocker / giới hạn

- **Evaluator chưa nối**: stress vẫn là closure + `SendValue`, chưa phải `async fn`/frame/VM
  thật; W1 1d/3b sở hữu hook.
- **RSS ~37 MB là của test**, gồm output/outcome/allocator; chưa phải cap production và chưa
  đo RSS theo từng pha.
- **Thời gian suite +~10 s**: 50k phù hợp gate/nightly; nếu CI yếu có thể hạ N có kiểm soát,
  nhưng **không** dùng `#[ignore]`.
- **Cancel W=1** bị chi phối bởi sleep nhân tạo; muốn đo throughput cancel thuần cần test riêng
  không sleep.
- **Load/hybrid**: 1-min load 10–13 khi đo; 2P+8E nên số tuyệt đối phụ thuộc tải.
- **Map/payload lớn**: stress này không lặp lại ma trận B7; worker-copy list10k/map1k vẫn là
  tham chiếu riêng (`vs3-w5-worker-bench.md`).
- **Chưa soak dài**: 50k một lượt, không phải chạy lặp hàng giờ; chưa đo RSS ổn định theo thời
  gian.
- Không tuyên bố worker pool production/default, không đổi `Value`/bytecode/cache.

## 5. Tái lập

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5 \
  cargo test -p vis-cli --test vs3_worker stress_50k -- --nocapture --test-threads=1
```

Toàn suite:

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5 \
  cargo test -p vis-cli --test vs3_worker
```
