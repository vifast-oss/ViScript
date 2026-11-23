<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — soak P3 VM/interp (test-only, ~30 phút)

**Trạng thái**: **SOAK TEST-ONLY** — chỉ thêm example + journal; **không** sửa
production/default/API/ABI.
**Nhánh**: `vs3/w5-spike`, đã `git merge main` → **`647cc29`** (P3 `736dc5c` đã merge).
**Mã soak**: `crates/vis-cli/examples/vs3_worker_eval_p3_soak.rs` (**405 dòng**, example chạy tay,
không nằm trong gate/test suite).
**Log**: `agent/JOURNAL/vs3-w5-eval-p3-soak.log`.
**Thời lượng**: `VS3_SOAK_SECS=1800` (30 phút); log mẫu 5 giây/lần; assert dừng ngay nếu fail.
**Đĩa**: 21G đầu phiên, 13G sau build/test, **21G** sau khi xoá target riêng của worktree.

---

## 1. Mỗi batch kiểm gì

- **8 task CPU × 20.000 vòng**: interp P2 W=4 vs VM P3 W=4; so `stdout`, `exit`, `codes`,
  `value.show()` byte-identical.
- **10.000 task** `spawn`/`await`: interp W=4 vs VM W=4; cùng output.
- **Budget** `SharedBudget(10.000, chunk 256)` trên **cả** interp/VM; task 1M vòng; mỗi bên phải
  cạn, message `VS0501-BUDGET`, `used+remaining==limit`, `exhausted`.
- **Cancel-after 5 ms** trên cả interp/VM; task 2M vòng dừng, message `huỷ`, không treo.
- **Cross-write W=4** trên cả interp/VM; `VS0501` + `ghi chéo`.
- **Tài nguyên**: RSS (`VmRSS`/`VmHWM`), threads `/proc/self/task`, fd `/proc/self/fd` sau
  mỗi batch; threads ≤ baseline+1, fd ≤ baseline+4; RSS sau warm-up 60 s lệch ≤64 MB.
- Log `SOAK_BATCH` mỗi 5 s; cuối `SOAK_TONG`/`SOAK_WALL`/`SOAK_TAI_NGUYEN`.

---

## 2. Kết quả 30 phút

**PASS — exit 0, 0 lỗi.**

| Mục | Số đo |
|---|---:|
| Thời gian | **1.801,7 s** (30 phút) |
| Batch | **1.915** (~1,06 batch/s); log mẫu 5 s (324 dòng `SOAK_BATCH`) |
| Task | **38.342.130** (~21.281 task/s; 8×20k interp+VM + 10k interp+VM + budget/cancel/cross) |
| Output interp/VM | **byte-identical mọi batch** (stdout/exit/codes/value) |
| CPU interp p50/p95 | **36 / 150 ms** |
| CPU VM p50/p95 | **18 / 77 ms** |
| 10k interp p50/p95 | **177 / 605 ms** |
| 10k VM p50/p95 | **158 / 546 ms** |
| RSS đầu / sau warm / cuối | **4.912 / 10.292–13.708 / 10.448 kB** |
| Biên độ RSS sau warm | **3.416 kB** (plateau; ngưỡng 64 MB) |
| VmHWM | **18.540 kB** |
| Threads / fd | **1 / 4** ở **mọi** batch |
| Budget | interp & VM đều cạn `10.000`, `used+remaining==limit`, `exhausted` |
| Cancel-after 5 ms | interp & VM dừng task 2M vòng, message `huỷ` |
| Cross-write | interp & VM đều `VS0501` + `ghi chéo` |
| Lỗi | **0**; assert dừng ngay nếu fail (không có assert fail) |

- Log đầy đủ: `agent/JOURNAL/vs3-w5-eval-p3-soak.log` (324 mẫu batch + summary).
- VM p50 nhanh hơn interp ở cả CPU (~2×) và 10k (~1,1×); p95 chịu nhiễu load 9–19, nhưng
  output/parity không lệch. 10k task cực nhỏ vẫn chịu overhead evaluator/pool (~0,16–0,6 s/lần).
- RSS giảm nhẹ sau warm-up rồi phẳng; threads/fd không tăng qua 1.915 batch.

---

## 3. Gate / test trước soak

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-cli --test vs3_worker_eval_vm -- --test-threads=1` | **10/10 PASS** (~5,9 s) |
| `cargo test -p vis-cli --test vs3_worker_eval_local -- --test-threads=1` | **8/8 PASS** |
| `cargo test -p vis-cli --test vs3_worker_eval_parallel -- --test-threads=1` | **14/14 PASS** |
| `./vis-dev gate --fast` | **PASS**; nextest **3571 passed / 3 skipped** (91,202 s) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy -p vis-cli --all-targets -- -D warnings` | PASS |

- Gate không tăng test count (example không phải test): **3571/3**.
- R1: example **405 dòng**; không sửa production.

---

## 4. Giới hạn

- Soak đo **CPU-only non-frame** đúng P3; không host/`chan(0)`/frame.
- VM budget charge **sau thân** theo chunk 256 (không cắt giữa loop như interp P2); tổng
  `used+remaining` vẫn đúng, nhưng thời gian budget batch của VM có thể lâu hơn.
- Cancel và budget là hook test-only; chưa có API sản phẩm.
- `10k` spawn/await tuần tự để output nhỏ; overhead evaluator/pool được ghi riêng.
- Không chạy GitHub Actions; không push/merge/tag.

---

## 5. Blocker

- Không có blocker code nếu soak PASS.
- Blocker quyết định vẫn: mã cross-write W>1 (`VS0509`?) và chính sách W>1 bỏ frame/`chan(0)`/
  host; P3 dùng `VS0501` + message như P2.
- P4 frame suspend/resume chưa làm; `vm.rs` đã đúng 1.500 dòng nên P4 phải chuyển bớt code.

---

## 6. Lệnh

```bash
VS3_SOAK_SECS=20  cargo run --release -p vis-cli --example vs3_worker_eval_p3_soak   # smoke
VS3_SOAK_SECS=1800 cargo run --release -p vis-cli --example vs3_worker_eval_p3_soak  # soak 30'
cargo test -p vis-cli --test vs3_worker_eval_vm -- --test-threads=1
cargo test -p vis-cli --test vs3_worker_eval_local --test vs3_worker_eval_parallel -- --test-threads=1
./vis-dev gate --fast
cargo fmt --all -- --check
cargo clippy -p vis-cli --all-targets -- -D warnings
```

Đã xoá `target` riêng của worktree sau chốt; `df /home` còn **21G**; không đụng target
repo chính.
