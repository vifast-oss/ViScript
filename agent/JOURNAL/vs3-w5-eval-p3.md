<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — P3 VM parity (evaluator W>1, CPU-only non-frame)

**Trạng thái**: **P3 IMPLEMENTED — gate xanh**, CPU-only non-frame; P1/P2 giữ nguyên.
**Nhánh**: `vs3/w5-spike`, đã `git merge main` → **`ffe202b`** (P2/soak đã merge).
**Không** push/merge/tag; **không** default/CLI; không đổi ABI/`VERSION`/cache key.
**Timebox**: 4 h. **Đĩa**: 23G đầu phiên, 12G sau build/test, **22G** sau khi xoá
target riêng của worktree.
**Log**: `agent/JOURNAL/vs3-w5-eval-p3.log`.

---

## 1. Diff / file

| Tệp | Thay đổi | Dòng |
|---|---|---:|
| `crates/vis-vm/src/vm/worker.rs` | **mới**: `ModuleImage`, `Mode`, `WorkerState`, spawn/await/join VM, snapshot globals, budget/cancel/dirty | **454** |
| `crates/vis-vm/src/vm.rs` | field `worker`, init, `mod worker`, `join_tasks`, 3 override `Host`, gọi `worker_tick` | **1.500** (+18/−1) |
| `crates/vis-vm/src/lib.rs` | `run_workers_local_vm`, `run_workers_parallel_vm`, `_with_budget`, `_cancel_after` (hidden) | **106** (+61) |
| `crates/vis-interp/src/builtin.rs` | `$spawn`/`$await` gọi `Host::spawn_task`/`await_task`; thêm 3 method trait | **1.323** (+28/−7) |
| `crates/vis-interp/src/host.rs` | `Interp` impl 3 method mới (default cũ) | **117** (+13) |
| `crates/vis-vm/src/vm_frame.rs` | `budget_charge` gọi `worker_tick` mỗi 1.024 lệnh | **988** (+3) |
| `crates/vis-cli/tests/vs3_worker_eval_vm.rs` | **mới**: 10 test parity P2 interp vs P3 VM | **363** |

Tổng production: **+116/−7** tracked + **454** dòng `vm/worker.rs` mới; test **363**.
R1: `vm.rs` **đúng 1.500** (hết headroom), các tệp còn lại ≤1.500.
**Không đụng** `value.rs`, `frame.rs`, `net.rs`; `vm.rs` chỉ hook tối thiểu, logic ở `vm/worker.rs`.

---

## 2. Thiết kế P3

- `Vm` có `worker: RefCell<WorkerState>` mặc định `Mode::Off`; `Off` gọi `spawn_task`/`await_task`/
  `join_tasks` qua scheduler cũ, **không đổi** default.
- `builtin $spawn`/`$await` gọi `Host::spawn_task`/`await_task`; `Vm::run` gọi `self.join_tasks()`.
  Interp impl 3 method theo đường cũ; VM override rẽ nhánh `Parallel`.
- `run_workers_parallel_vm*` (hidden): dựng `Arc<ModuleImage{Module,text}>` một lần; `Vm` chạy
  non-frame, mọi `$spawn` đẩy job lên `WorkerPool`.
- Worker: `Vm::new` từ ảnh; `init_globals` rồi nạp snapshot `SendValue::Map(ô global → value)`;
  `Vm` worker state `Off` để nested spawn/await chạy cục bộ trong worker; join nested ở cuối task.
- Output buffer `WorkerCtx.out`; board outcome đầy đủ; `await` flush tại tiêu thụ, final join flush
  task chưa await theo **spawn order**.
- Cross-write: so `globals` trước/sau; khác ⇒ `mark_dirty("g<i>")` ⇒ `VS0501` + `ghi chéo`.
- Budget: `WorkerPool::with_budget` cấp lease/task; runner charge **chunk 256** sau thân
  (`ctx.charge`), cạn ⇒ `VS0501-BUDGET`; `used+remaining==limit` sau join.
- Cancel: `worker_tick` mỗi 1.024 lệnh (gọi từ `budget_charge`) kiểm cờ/hạn; test cancel-after.
- Host/`chan(0)`/async I/O: worker VM không host; lỗi `VS0604`/`VS0505` bọc `VS0506`, không treo.

---

## 3. Test mới (`vs3_worker_eval_vm.rs`, 10 ca)

Cùng nguồn: lower HIR cho interp P2, compile VBC1 cho VM P3; so `stdout`, `exit`, `codes`,
`value.show()` (không so `steps` — đơn vị khác nhau theo hợp đồng VS3-1a).

| Test | Kỳ vọng |
|---|---|
| `cpu_w1_w2_w4_parity` | 4 task × 5k, W=1/2/4 interp==VM, `49990000` |
| `muoi_nghin_task_parity` | 10k spawn/await W=1/2/4, `50005000` |
| `nested_spawn_await_parity` | task spawn task, `41` |
| `print_merge_parity` | 4 task in xen kẽ, `1 2 3 4 20`, byte-identical |
| `chan0_bi_tu_choi_parity` | `VS0505` bọc `VS0506`, exit 1 |
| `host_fs_bi_tu_choi_parity` | `VS0604` bọc `VS0506`, exit 1 |
| `budget_vm_used_cong_remaining_bang_limit` | `VS0501-BUDGET`, `9984+16==10000`, exhausted |
| `cancel_vm_giua_chung` | task 2M vòng dừng sau 5 ms, message `huỷ` |
| `cross_write_vm_voi_w1_w4` | W=1 in `1`; W=2/4 `VS0501` `ghi chéo` |
| `vm_khong_tao_them_thread_sau_khi_join` | thread không rò sau W=4 |

---

## 4. Số đo interp P2 vs VM P3 (test, CPU-only, load 8–15)

- **CPU 4 task × 5k**, median 3 lần (interp → VM):
  W=1 **142,4 → 60,2 ms** (VM/interp **0,42×**); W=2 **71,6 → 30,5 ms** (**0,43×**);
  W=4 **36,1 → 16,0 ms** (**0,44×**) — VM nhanh hơn ~2,3× ở workload này.
- **10k spawn/await**, median 3 lần: W=1 **160,5 → 250,1 ms** (0,98×); W=2 **792,5 → 599,3 ms**
  (0,74×); W=4 **789,8 → 625,9 ms** (0,79×). VM không chậm hơn đáng kể; task cực nhỏ vẫn bị
  overhead evaluator/pool như P2.
- **Budget VM**: used **9984**, remaining **16**, limit **10000**, exhausted; interp P2 used
  10000/0 (khác đơn vị chunk, cùng tổng).
- **Cancel-after 5 ms** dừng task 2M vòng; không treo.
- Cross/host/chan0/print/nested parity: PASS.

---

## 5. Gate / test

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-cli --test vs3_worker_eval_vm -- --test-threads=1` | **10/10 PASS** (~4,9 s) |
| `cargo test --release ... vs3_worker_eval_vm` | **10/10 PASS** (2,71 s) |
| P1 `vs3_worker_eval_local` | **8/8 PASS** |
| P2 `vs3_worker_eval_parallel` | **14/14 PASS** |
| `vs3_worker` / `vs3_default` / `vs3_budget` | **14/14 / 22/22 / 8/8 PASS** |
| `cargo test -p vis-interp --lib` | **303/303 PASS** |
| `./vis-dev gate --fast` | **PASS**; nextest **3571 passed / 3 skipped** (108,984 s) |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --all-targets` | PASS |
| `cargo clippy -p vis-vm -p vis-cli --all-targets -- -D warnings` | PASS |

Gate tăng **+10 test** so P2 3561/3 ⇒ **3571/3**; 0 test bị xoá/nới.

---

## 6. Giới hạn / P4

- **CPU-only non-frame**: worker VM không host/`chan(0)`/async I/O; lỗi rõ (`VS0604`/`VS0505`).
  `chan(0)` vẫn `VS0505` như VM non-frame; frame async parity là **P4**.
- **Budget VM** charge **sau thân** theo chunk 256, không cắt giữa loop như interp P2; tổng
  `used+remaining` vẫn đúng, nhưng thời gian chạy có thể dài hơn. Early cut là P3b/P2b.
- **Cancel VM** dùng hook test `run_workers_parallel_vm_cancel_after`; chưa có API sản phẩm để
  huỷ handle; safe point 1.024 lệnh, task bị chặn host/chan không tick.
- **Direct `await f()`** không qua `$spawn` không có proxy; P3 chỉ song song task `$spawn`.
- `vm.rs` đúng **1.500 dòng**: P4 phải chuyển bớt code (hoặc hook vào module mới) trước khi thêm.
- `Host` thêm 3 method required; chỉ `Interp`/`Vm` trong cây impl; không đổi ABI bytecode/
  `VERSION`/cache key/`Value`/`Task`.

---

## 7. Blocker

- Không có blocker code cho P3; gate xanh, parity 10/10, P1/P2 xanh.
- Blocker quyết định vẫn: mã cross-write W>1 (`VS0509`?) và chính sách W>1 bỏ frame/`chan(0)`/
  host. P3 dùng `VS0501` + message như P2.
- Đĩa: đã xoá `target` riêng của worktree; `df /home` còn **22G**; không đụng target repo chính.

---

## 8. Lệnh

```bash
cargo test -p vis-cli --test vs3_worker_eval_vm -- --test-threads=1
cargo test --release -p vis-cli --test vs3_worker_eval_vm -- --test-threads=1
cargo test -p vis-cli --test vs3_worker_eval_local --test vs3_worker_eval_parallel \
  --test vs3_worker --test vs3_default --test vs3_budget -- --test-threads=1
cargo test -p vis-interp --lib
./vis-dev gate --fast
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy -p vis-vm -p vis-cli --all-targets -- -D warnings
```
