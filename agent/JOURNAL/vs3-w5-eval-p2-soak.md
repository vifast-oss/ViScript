<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — soak P2 W=4 (test-only, ~30 phút)

**Trạng thái**: **SOAK TEST-ONLY** — chỉ thêm example + journal; **không** sửa
production/default/API/ABI.
**Nhánh**: `vs3/w5-spike`, đã `git merge main` → **`f5a243f`** (P2 `0b4ad7e` đã merge).
**Mã soak**: `crates/vis-cli/examples/vs3_worker_eval_p2_soak.rs` (**396 dòng**, example chạy tay,
không nằm trong gate/test suite).
**Log**: `agent/JOURNAL/vs3-w5-eval-p2-soak.log`.
**Thời lượng**: `VS3_SOAK_SECS=1800` (30 phút). Mỗi batch log 5 giây/lần; dừng ngay nếu assert.
**Đĩa**: 23G đầu phiên, 12G sau build/test, **22G** sau khi xoá `target` riêng (xem §6).

---

## 1. Mỗi batch kiểm gì

- **8 task CPU × 20.000 vòng** (`nang`): chạy W=1 (local), W=2, W=4; so `stdout`, `exit`,
  `codes` byte-identical.
- **10.000 task** `spawn`/`await`: W=1/2/4; so `stdout`, `exit`, `codes`.
- **Budget**: `SharedBudget(10.000, chunk=256)` + task 1M vòng; phải cạn, diagnostic chứa
  `VS0501-BUDGET`, `used+remaining==limit`, `exhausted`.
- **Cancel-after 5 ms**: task 2M vòng; phải dừng với message `huỷ`, không treo.
- **Cross-write**: W=1 shared env in `1`; W=4 phải `VS0501` + message `ghi chéo`.
- **Tài nguyên**: RSS (`VmRSS`/`VmHWM`), threads `/proc/self/task`, fd `/proc/self/fd` sau
  mỗi batch; threads ≤ baseline+1, fd ≤ baseline+4; RSS sau warm-up 60 s không lệch quá 64 MB.
- Mỗi batch ghi log `SOAK_BATCH`; cuối log `SOAK_TONG`/`SOAK_WALL`/`SOAK_TAI_NGUYEN`.

---

## 2. Kết quả 30 phút

**PASS — exit 0, 0 lỗi.**

| Mục | Số đo |
|---|---:|
| Thời gian | **1.800,1 s** (30 phút) |
| Batch | **2.310** (~1,28 batch/s); log mẫu mỗi 5 s (327 dòng `SOAK_BATCH`) |
| Task | **69.362.370** (~38.532 task/s; 8×20k + 10k + budget/cancel/cross mỗi batch) |
| Output W=1/2/4 | **byte-identical mọi batch** (assert mỗi batch) |
| CPU W=1 / W=2 / W=4 p50 | **143 / 71 / 36 ms** |
| CPU W=1 / W=2 / W=4 p95 | **316 / 205 / 144 ms** |
| 10k W=1 / W=4 p50 | **20 / 177 ms** |
| 10k W=1 / W=4 p95 | **54 / 588 ms** |
| RSS đầu / sau warm / cuối | **4.768 / 10.940–13.088 / 11.276 kB** |
| Biên độ RSS sau warm | **2.148 kB** (plateau; ngưỡng 64 MB) |
| VmHWM | **19.288 kB** |
| Threads / fd | **1 / 4** ở **mọi** batch |
| Budget | `used+remaining==limit=10.000`, `exhausted=true` mỗi batch |
| Cancel-after 5 ms | task 2M vòng dừng, message `huỷ`, mỗi batch |
| Cross-write | W=1 in `1`; W=4 `VS0501` + `ghi chéo`, mỗi batch |
| Lỗi | **0**; dừng ngay nếu assert (không có assert nào fail) |

- Log đầy đủ: `agent/JOURNAL/vs3-w5-eval-p2-soak.log` (327 mẫu batch + summary).
- Không có dấu hiệu rò: threads/fd phẳng; RSS giảm nhẹ sau warm-up rồi phẳng, HWM không tăng
  theo thời gian.

---

## 3. Gate / test (trước soak, production không đổi)

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-cli --test vs3_worker_eval_parallel -- --test-threads=1` | **14/14 PASS** (~5,9 s) |
| `cargo test -p vis-cli --test vs3_worker_eval_local -- --test-threads=1` | **8/8 PASS** (~0,5 s) |
| `./vis-dev gate --fast` | **PASS**; nextest **3561 passed / 3 skipped** (73,742 s) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy -p vis-cli --all-targets -- -D warnings` | PASS |

- Gate không tăng test count (example không phải test): vẫn **3561/3**.
- R1: example **396 dòng**; không tệp production nào bị sửa.

---

## 4. Giới hạn của soak

- Soak đo **CPU-only non-frame** đúng phạm vi P2: không host/`chan(0)`/frame/VM.
- `10k` dùng `spawn`/`await` tuần tự (không phải 10k task song song) để giữ output nhỏ; CPU
  tasks 8 × 20k mới là tải song song chính.
- `RunOutput.steps` W>1 chỉ main, không cộng steps worker — soak không dùng steps để so.
- Cancel dùng hook test-only `run_workers_parallel_cancel_after`; chưa có API sản phẩm.
- Không chạy GitHub Actions; không push/merge/tag.

---

## 5. Blocker

- Không có blocker code trong soak; mọi batch PASS (nếu log ghi `KET_LUAN PASS`).
- Blocker quyết định vẫn từ plan: mã cross-write W>1 (`VS0509`?) và chính sách W>1 bỏ
  frame/`chan(0)`/host; P2 dùng `VS0501` + message.
- Hiệu năng 10k task cực nhỏ vẫn là nợ P2b (xem journal P2); soak không tuyên bố speedup
  cho 10k.

---

## 6. Lệnh

```bash
VS3_SOAK_SECS=20  cargo run --release -p vis-cli --example vs3_worker_eval_p2_soak  # smoke
VS3_SOAK_SECS=1800 cargo run --release -p vis-cli --example vs3_worker_eval_p2_soak # soak 30'
cargo test -p vis-cli --test vs3_worker_eval_parallel -- --test-threads=1
cargo test -p vis-cli --test vs3_worker_eval_local -- --test-threads=1
./vis-dev gate --fast
cargo fmt --all -- --check
cargo clippy -p vis-cli --all-targets -- -D warnings
```

Đã xoá `target` riêng của worktree sau khi chốt; `df /home` còn **22G**; không đụng target
repo chính.
