<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — soak evaluator 2 giờ (test-only, P2 interp vs P3 VM)

**Trạng thái**: **SOAK TEST-ONLY** — dùng example có sẵn, **không sửa production/default/API**.
**Nhánh**: `vs3/w5-spike`, đã `git merge main` → **`c80f553`** (evaluator release doc).
**Mã soak**: `crates/vis-cli/examples/vs3_worker_eval_p3_soak.rs` (release, 405 dòng).
**Log**: `agent/JOURNAL/vs3-w5-eval-2h-soak.log`.
**Thời lượng**: `VS3_SOAK_SECS=7200` (2 giờ); dừng ngay nếu assert fail.
**Đĩa**: 18G đầu phiên; sau soak/test còn 21G; đã xoá target riêng của worktree sau chốt.

---

## 1. Cấu hình mỗi batch

- 8 task CPU × 20.000 vòng: **interp P2 W=4 vs VM P3 W=4**, so stdout/exit/codes/value.
- 10.000 `spawn`/`await`: interp W=4 vs VM W=4.
- Budget `SharedBudget(10.000, chunk 256)`: cả interp/VM cạn, `used+remaining==limit`.
- Cancel-after 5 ms: task 2M vòng dừng, message `huỷ`.
- Cross-write W=4: cả hai `VS0501` + `ghi chéo`.
- RSS (`VmRSS`/`VmHWM`), threads `/proc/self/task`, fd `/proc/self/fd`; log mẫu 5 s/lần.
- RSS sau warm 60 s phải plateau (≤64 MB); threads/fd không rò; sai ⇒ panic/dừng.

---

## 2. Kết quả 2 giờ

**PASS — exit 0, 0 lỗi.**

| Mục | Số đo |
|---|---:|
| Thời gian | **7.200,9 s** (2 giờ) |
| Batch | **8.417** (~1,17 batch/s); log mẫu 5 s (1.307 dòng `SOAK_BATCH`) |
| Task | **168.525.174** (~23.403 task/s; 8×20k interp+VM + 10k interp+VM + budget/cancel/cross) |
| Output interp/VM | **byte-identical mọi batch** (stdout/exit/codes/value) |
| CPU interp p50/p95 | **36 / 133 ms** |
| CPU VM p50/p95 | **18 / 67 ms** |
| 10k interp p50/p95 | **176 / 502 ms** |
| 10k VM p50/p95 | **155 / 451 ms** |
| RSS đầu / sau warm / cuối | **5.108 / 10.828–13.880 / 10.828 kB** |
| Biên độ RSS sau warm | **3.052 kB** (plateau; ngưỡng 64 MB) |
| VmHWM | **18.644 kB** |
| Threads / fd | **1 / 4** ở **mọi** batch |
| Budget | interp & VM đều cạn `10.000`, `used+remaining==limit`, `exhausted` |
| Cancel-after 5 ms | interp & VM dừng task 2M vòng, message `huỷ` |
| Cross-write | interp & VM đều `VS0501` + `ghi chéo` |
| Lỗi | **0**; assert dừng ngay nếu fail (không có assert fail) |

- Log đầy đủ: `agent/JOURNAL/vs3-w5-eval-2h-soak.log` (1.307 mẫu + summary).
- VM p50 nhanh hơn interp: CPU 36→18 ms (~2×), 10k 176→155 ms (~1,14×). p95 chịu nhiễu
  load 6–14 nhưng output/parity không lệch.
- RSS giảm nhẹ sau warm-up rồi phẳng; threads/fd không tăng qua 8.417 batch.

---

## 3. Test / gate cuối soak

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-cli --test vs3_worker_eval_vm -- --test-threads=1` | **10/10 PASS** (16,12 s) |
| `cargo test -p vis-cli --test vs3_worker_eval_parallel -- --test-threads=1` | **14/14 PASS** (8,51 s) |
| `cargo test -p vis-cli --test vs3_worker_eval_local -- --test-threads=1` | **8/8 PASS** (0,58 s) |
| `./vis-dev gate --fast` | **PASS**; nextest **3571 passed / 3 skipped** (125,985 s) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy -p vis-cli --all-targets -- -D warnings` | PASS |

- R1: example soak **405 dòng**; không sửa production/default/API/ABI.
- Gate vẫn **3571/3**; không thêm test, không xoá/nới test.
- `df /home`: **đã xoá target riêng** của worktree sau chốt; không đụng target repo chính.

---

## 4. Giới hạn / blocker

- Soak CPU-only non-frame; không host/`chan(0)`/frame.
- VM budget charge sau thân theo chunk 256; cancel là hook test-only.
- 10k task cực nhỏ vẫn chịu overhead evaluator/pool (không tuyên bố speedup).
- Không GitHub Actions; không push/merge/tag.
- `vm.rs` đúng 1.500 dòng — P4 phải refactor trước khi thêm hook.
