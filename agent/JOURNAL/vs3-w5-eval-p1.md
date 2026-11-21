<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — P1 evaluator W=1 non-frame (trong crate)

**Trạng thái**: **P1 IMPLEMENTED** — `run_workers_local*` ẩn, mặc định `Off` không đổi.
**Nhánh**: `vs3/w5-spike`, đã `git merge main` → **`a8100ab`** (Windows async net + evaluator
13M job đã merge; gate baseline 3539/3 theo mốc user).
**Không** push/merge/tag; không bật default; không đổi ABI/`VERSION`/cache key.
**Log**: `agent/JOURNAL/vs3-w5-eval-p1.log`.
**Timebox**: 3 h. `df /home`: bắt đầu **11G**, kết thúc **2,3G** — release test làm tăng
target; **dừng mọi build/test bổ sung** theo ngưỡng <5G (xem §7).

---

## 1. Diff và file đụng tới

| Tệp | Thay đổi | Dòng |
|---|---|---:|
| `crates/vis-interp/src/rt/exec.rs` | **mới**: `WorkerMode::{Off,Local}`, `ExecState { mode, order }`, `spawn_local`, `await_local`, `join_local` | **62** |
| `crates/vis-interp/src/interp/worker.rs` | **mới**: `chay_than_local` — chạy thân trên caller, bọc `VS0506` đúng một lần | **34** |
| `crates/vis-interp/src/rt.rs` | `pub(crate) mod exec`; `Scheduler.exec`; tách `await_task_off`/`join_all_off` + dispatch `Local`; `wrap_task_error` → `pub(crate)` | **+28/−5** (1.269 dòng) |
| `crates/vis-interp/src/interp.rs` | `pub(crate) mod worker`; `HExpr::Spawn` rẽ nhánh `Local` (Off giữ `get_mut` cũ) | **+13/−5** (1.431 dòng) |
| `crates/vis-interp/src/lib.rs` | `run_with_hosts_caps_mode`; `#[doc(hidden)] run_workers_local`, `run_workers_local_with_hosts_caps` | **+79** (392 dòng) |
| `crates/vis-cli/tests/vs3_worker_eval_local.rs` | **mới**: 8 test W=1 vs default | **256** |

Tổng production: **+120/−10** (tracked) + **96** dòng hai module mới ⇒ **~206 dòng net**;
test **256**. Toàn bộ ≤1.500 dòng/tệp. **Không đụng** `value.rs`, `frame.rs`, `vm.rs`, `net.rs`,
`Cargo.toml`, `VERSION`.

---

## 2. Thiết kế P1

- `WorkerMode::Off` là `Default`; `ExecState` nằm trong `Scheduler` và chỉ mang `mode` + `order`.
- `HExpr::Spawn`:
  - `Off`: **nguyên văn** `self.sched.get_mut().spawn(...)` (giữ đúng default cũ);
  - `Local`: `exec::spawn_local` gọi cùng `Scheduler::spawn`, thêm `Task.id` vào `order`.
- `await_task` / `join_all`:
  - `Off`: `await_task_off` / `join_all_off` — thân cũ, không đổi ngữ nghĩa;
  - `Local`: `exec::await_local` / `exec::join_local` — cùng thân `*_off`, chạy caller.
- `chay_than_local` thay cho `host.goi_than` trực tiếp trong `await_task_off`:
  - `Ok`/`Abort::Ret` ⇒ kết quả; `Trap` ⇒ bọc `VS0506` đúng **một lần**;
  - `Pending` (không xảy ra non-frame) ⇒ cùng thông điệp phòng thủ.
- `run_with_hosts_caps_mode(..., WorkerMode::Off|Local)` là thân chung; wrapper công khai cũ
  truyền `Off`, API ẩn W=1 truyền `Local` **sau** `Interp::new`, trước `run_program`.
- W=1 dùng **shared `Interp.env`/`Interp.out`**, không snapshot, không buffer, không thread.
  `chan(0)` vẫn `VS0505` như default non-frame; frame async parity là P4.
- Không thêm dependency, không `unsafe`, không đổi `RunOutput`/`Value`/`Task`.

---

## 3. Test mới (`vs3_worker_eval_local.rs`)

Mọi ca so `vis_interp::run` (Off) với `run_workers_local` (Local) trên: `stdout`, `exit`,
`steps`, `codes`, `(code, message, line, col)` của diagnostics, `value.show()`.

| Test | Nội dung | Kết quả |
|---|---|---|
| `sync_byte_identical` | `var`/`while`/`if`/print; output `ok\n10101\n` | PASS |
| `print_va_thu_tu_byte_identical` | 2 task + main xen kẽ; `M1 M2 A1 A2 M3 B1 B2 M4` | PASS |
| `nested_spawn_await_byte_identical` | task spawn task rồi await; output `41` | PASS |
| `muoi_nghin_task_byte_identical` | **10.000** `spawn`+`await`, output `50005000`, `steps>0` | PASS |
| `loi_than_boc_vs0506_giong_default` | `panic` trong task ⇒ `VS0506` giữ `VS0501`, exit 1 | PASS |
| `chan0_giu_vs0505_nhu_non_frame` | `chan[int](0)` ⇒ `VS0505` cả Off/Local | PASS |
| `budget_can_giong_default` | 2M loop, limit 10.000 ⇒ `VS0501-BUDGET`, cùng steps | PASS |
| `await_hai_lan_tra_cache_byte_identical` | await lần hai cache; output `41 42 42` | PASS |

- Debug `--test-threads=1`: **8/8 PASS** (~0,47–0,51 s).
- Release `--release --test-threads=1`: **8/8 PASS** (0,03 s; build 23,9 s).
- Không test thread-count trong integration (tránh nhiễu harness); P1 không tạo thread theo
  thiết kế, và gate/vs3_default/vs3_worker cùng chạy.

---

## 4. Hồi quy / gate

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-cli --test vs3_worker_eval_local -- --test-threads=1` | **8/8 PASS** |
| `cargo test --release ... vs3_worker_eval_local` | **8/8 PASS** |
| `cargo test -p vis-cli --test vs3_default --test vs3_worker --test vs3_budget -- --test-threads=1` | **22/22, 14/14, 8/8 PASS** |
| `cargo test -p vis-interp --lib` | **303/303 PASS** |
| `./vis-dev gate --fast` | **PASS**; nextest **3547 passed, 3 skipped** (76,856 s) |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --all-targets` | PASS |
| `cargo clippy -p vis-cli --all-targets -- -D warnings` | PASS |

Gate tăng **+8 test** so baseline 3539/3 ⇒ **3547/3**, đúng bằng 8 test mới; không xoá/nới test.

---

## 5. Phần còn thiếu (P2/P3/P4)

- **P2 (W>1)**: `WorkerMode::Parallel(usize)`, `ExecState.proxy`, `OutcomeBoard`, snapshot
  top-level `var` (`Arc<SendValue>::Map`), runner HIR trong worker (`Interp::new_for_task`),
  dirty/cross-write hook mọi đường ghi, `SharedBudget` lease theo task, cancel, merge output
  theo spawn/await order. P1 **chưa** có pool/thread/snapshot.
- **P3 (VM parity)**: `run_workers_local` hiện chỉ nhận HIR/interp. `builtin.rs` `$spawn/$await`
  chưa đổi; VM cần runner `ModuleImage` + `Vm::new_for_task`/budget/cancel ở pha P3.
- **P4 (frame)**: W=1 Local là **non-frame**; không suspend/resume, không `chan(0)`/reactor.
  `chan(0)` giữ `VS0505` như hợp đồng; parity frame là P4.

---

## 6. Blocker

- **Không có blocker code** cho P1: `Interp::new` private đã được xử lý bằng API cùng crate
  `run_workers_local*`; không hack `unsafe`; không đổi default/ABI.
- Blocker **quyết định** vẫn còn từ plan: mã cross-write W>1 (`VS0509`?) và chính sách W>1 bỏ
  frame/`chan(0)`/host — không cần cho P1.
- **Đĩa**: `df /home` còn **2,3G** sau release test (<5G). Theo điều kiện dừng, phiên này
  **không chạy thêm build/test**; đề xuất dọn `target/debug/incremental`/`target/debug/build`
  của worktree (9,5G) trước P2. Không xoá source/cache chung của repo chính.

---

## 7. Lệnh đã chạy

```bash
~/.cargo/bin/cargo check -p vis-interp
~/.cargo/bin/cargo test -p vis-cli --test vs3_worker_eval_local -- --test-threads=1
~/.cargo/bin/cargo test --release -p vis-cli --test vs3_worker_eval_local -- --test-threads=1
~/.cargo/bin/cargo test -p vis-cli --test vs3_default --test vs3_worker --test vs3_budget -- --test-threads=1
~/.cargo/bin/cargo test -p vis-interp --lib
./vis-dev gate --fast
~/.cargo/bin/cargo fmt --all -- --check
~/.cargo/bin/cargo check --all-targets
~/.cargo/bin/cargo clippy -p vis-cli --all-targets -- -D warnings
```
