<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — tổng hợp phát hành evaluator W>1 (P1/P2/P3)

**Trạng thái**: **OPT-IN / TEST-ONLY** — evaluator W>1 đã có cho interp (P1/P2) và VM (P3),
**chưa bật default/CLI**, chưa đổi ABI/`VERSION`/cache key.
**Nhánh**: `vs3/w5-spike`, đã `git merge main` → **`0298785`**.
**Gate Linux hiện hành**: **3571 passed / 3 skipped**; P1 8/8, P2 14/14, P3 10/10.
**Windows**: P1/P2/P3 debug + release xanh, workspace **3528/0/3**.
**Không** push/merge/tag; **không** GitHub Actions.

---

## 1. API opt-in (đều `#[doc(hidden)]`)

### interp (`vis-interp`)

| Hàm | Dùng |
|---|---|
| `run_workers_local(text, hir, syms, types, args)` | **W=1 Local**: chạy trên caller, shared env, đúng đường cũ |
| `run_workers_local_with_hosts_caps(..., ngoai, fs, net, db, caps)` | W=1 có host/caps |
| `run_workers_parallel(text, hir, syms, types, args, workers)` | **W=2/4** CPU-only non-frame |
| `run_workers_parallel_with_budget(..., workers, Arc<SharedBudget>)` | W>1 + budget atomic |
| `run_workers_parallel_cancel_before(..., workers)` | test hook huỷ trước job |
| `run_workers_parallel_cancel_after(..., workers, ms)` | test hook huỷ giữa chừng |

### VM (`vis-vm`)

| Hàm | Dùng |
|---|---|
| `run_workers_local_vm(module, text, args)` | **W=1**: `Vm::run` bình thường (shared globals) |
| `run_workers_parallel_vm(module, text, args, workers)` | **W=2/4** CPU-only non-frame |
| `run_workers_parallel_vm_with_budget(..., Arc<SharedBudget>)` | W>1 + budget |
| `run_workers_parallel_vm_cancel_after(..., ms)` | test hook cancel-after |

`WorkerMode` nội bộ: `Off` (mặc định) / `Local` (W=1) / `Parallel(n)` (W>1). VM dùng
`Mode::{Off,Parallel(n)}` tương đương; W=1 VM đi `Vm::run` thường. Không có env/CLI nào bật
mode này; chỉ test/example gọi API trên.

---

## 2. Ngữ nghĩa

| Khía cạnh | W=1 (`Local`) | W=2/4 (`Parallel`) |
|---|---|---|
| Nơi chạy | caller | `WorkerPool` N thread |
| Evaluator | `Interp` mới/caller; VM `Vm::run` | `Interp`/`Vm` **riêng mỗi task** |
| Env | shared `Interp.env` /VM globals | snapshot top-level tại spawn |
| Snapshot | không | interp: `SendValue::Map(Int(sym)→value)`; VM: `Map(Int(global_slot)→value)` |
| Args | by value như cũ | `Value::to_send()` **trước** khi tạo job; lỗi `VS0508` |
| Print | in trực tiếp | buffer/task; flush tại `await`, final join theo **spawn order** |
| Budget | thread-local như cũ | lease `BudgetLease`/task; cạn `VS0501-BUDGET` |
| Cancel | đường cũ | cờ/hạn kiểm mỗi **1.024** bước (interp) / lệnh (VM) |
| Cross-write | ghi shared env (thấy ngay) | **từ chối** `VS0501` + `ghi chéo worker (W>1): biến …` |
| Host/`chan(0)` | đầy đủ như default | **từ chối rõ** (`VS0604`/`VS0505`, bọc `VS0506`) |
| Nested spawn/await | scheduler cũ | chạy **cục bộ trong worker**, output gộp task cha |
| `steps` | đủ | chỉ main (không cộng steps worker) |

- `Off` mặc định: `Scheduler`/`Vm::run` cũ nguyên văn; gate chứng minh byte-identical.
- Không đổi `Value`/`Task`/`VERSION`/`VBC1`/cache key/`Show`; API mới chỉ thêm hàm ẩn.
- `run_workers_local`/VM W=1 là **oracle** cho W>1: 10k task, print, nested, lỗi, budget,
  cross-write đều so với W=1.

---

## 3. Bằng chứng

### Linux

| Hạng mục | Kết quả |
|---|---|
| P1 `vs3_worker_eval_local` | **8/8** debug + release |
| P2 `vs3_worker_eval_parallel` | **14/14** debug + release |
| P3 `vs3_worker_eval_vm` | **10/10** debug + release |
| 10k task | P1/P2 interp & P3 VM **byte-identical** W=1/2/4 (stdout/exit/codes/value) |
| P2 soak 30 phút | **2.310 batch / 69.362.370 task**; 1.800,1 s; 0 lỗi; RSS 10.940–13.088 kB |
| P3 soak 30 phút | **1.915 batch / 38.342.130 task**; 1.801,7 s; 0 lỗi; VM/interp parity; RSS 10.292–13.708 kB |
| Interp vs VM (CPU 4×5k, median) | W=1 **142,4 → 60,2 ms**, W=2 **71,6 → 30,5 ms**, W=4 **36,1 → 16,0 ms** — VM nhanh **~2,3×** |
| Gate | **3571 passed / 3 skipped** (P1 3547 → P2 3561 → P3 3571) |
| `vs3_worker` | **14/14**; `vs3_default` 22/22; `vs3_budget` 8/8; `vis-interp --lib` 303/303 |

### Windows (psvn, MSVC, cargo/rustc 1.100.0)

| Mốc | P1 | P2 | P3 | Workspace |
|---|---:|---:|---:|---:|
| `c943793` (P2 verify) | 8/8 | 14/14 debug+release | — | 3518/0/3 |
| `d9e33f3` (P3 verify) | 8/8 | 14/14 | 10/10 debug+release | 3528/0/3 |
| `3aec977` (release) | 8/8 release | 14/14 release | 10/10 release | **3528/0/3** release |

- Không có ca debug-only bị bỏ ở release (cùng 3528 pass/3 ignored).
- `async-h2` vẫn Unix-only (expected red trên Windows); không liên quan evaluator.

---

## 4. Giới hạn đã biết

1. **CPU-only, non-frame**: worker không host/`fs`/`net`/`db`/`ngoai`, không reactor, không frame
   suspend/resume. `chan(0)` worker giữ `VS0505`; host `VS0604`; lỗi bọc `VS0506`.
2. **P4 chưa làm**: main chưa await worker trong lúc treo frame; `chan(0)` async/frame chưa parity.
3. **VM budget** charge **sau thân** theo chunk 256 (không cắt giữa loop như interp P2); tổng
   `used+remaining==limit` đúng nhưng wall có thể dài hơn.
4. **Cancel** chỉ có hook test (`cancel_before`/`cancel_after`); chưa có API sản phẩm để user
   huỷ handle; safe point 1.024 bước/lệnh, không preempt task bị chặn host/chan.
5. **Direct `await f()`** (không qua `spawn`) không có proxy ⇒ chạy local/không song song.
6. **10k task nhỏ không nhanh hơn**: P2 W=4 **0,14–0,69×** W=1; P3 VM/interp **0,74–0,98×**.
   Overhead evaluator/pool/spawn là nợ P2b.
7. `vm.rs` đã đúng **1.500 dòng** (R1) — P4 phải chuyển bớt code trước khi thêm hook.
8. `Host` trait thêm 3 method required (`spawn_task`/`await_task`/`join_tasks`); chỉ Interp/Vm
   trong cây impl; không đổi ABI bytecode/`Value`.
9. Cross-write dùng `VS0501` + message (chưa có mã riêng); A0 cần chốt `VS0509` nếu muốn.

---

## 5. Roadmap

### P2b — giảm overhead evaluator/pool

- Tái dùng `Interp`/`Vm` theo worker + `reset_for_task` (env/out/sched/steps/closure_cells),
  test cách ly 2 task; chỉ làm nếu đo >10–15%.
- Snapshot COW/generation: một `Arc<SendValue>` cho nhiều spawn, invalidate khi main ghi top-level.
- Một đường completion (OutcomeBoard **hoặc** CompletionBoard), tránh clone outcome gấp đôi.
- Giảm overhead `spawn` 10k (payload/args, channel 64, board, lazy pool); mục tiêu 10k task
  không chậm quá 1,2× W=1.
- Có thể lọc snapshot theo free-var thay vì toàn bộ top-level (bảo thủ hiện tại).

### P4 — frame suspend/resume

- `exec_pump`, `frame_await`, `frame_final_join`, bỏ qua task worker khi driver tự đẩy `Moi`.
- Main await worker mà không block reactor; `chan(0)`/async net parity; test 21–26 trong plan.
- Phải refactor `vm.rs` (R1) và `frame.rs` (1.494 dòng) trước khi thêm hook.

### A0 cần quyết

- Mã lỗi cross-write W>1: giữ `VS0501` hay cấp `VS0509` + snapshot 3 ngôn ngữ/explain.
- Chính sách W>1: có bỏ frame/`chan(0)`/host không; có bật pool cho chương trình “thuần” không.
- Có mở `VIS_WORKERS`/CLI opt-in (P5) hay chỉ giữ test-only; ngưỡng soak Windows/Linux.
- Có chấp nhận VM budget post-hoc hay bắt buộc early-cut trước khi merge production.

---

## 6. Lệnh kiểm tra

```bash
# Linux — test theo tầng
cargo test -p vis-cli --test vs3_worker_eval_local -- --test-threads=1
cargo test -p vis-cli --test vs3_worker_eval_parallel -- --test-threads=1
cargo test -p vis-cli --test vs3_worker_eval_vm -- --test-threads=1
cargo test -p vis-cli --test vs3_worker --test vs3_default --test vs3_budget -- --test-threads=1
cargo test -p vis-interp --lib

# Gate / luật
./vis-dev gate --fast
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy -p vis-cli --all-targets -- -D warnings

# Soak (release, test-only; mặc định 1800 s)
cargo run --release -p vis-cli --example vs3_worker_eval_p2_soak
cargo run --release -p vis-cli --example vs3_worker_eval_p3_soak
VS3_SOAK_SECS=120 cargo run --release -p vis-cli --example vs3_worker_eval_p3_soak

# Windows (psvn)
cargo test --workspace --no-fail-fast
cargo test --release --workspace --no-fail-fast
cargo test --release -p vis-cli --test vs3_worker_eval_vm
```

Không chạy GitHub Actions; soak/Windows chạy tay trên worktree sạch.

---

## 7. Mốc commit

| Commit | Nội dung | Gate |
|---|---|---|
| `bc7a543` | merge plan evaluator W5 | design-only |
| `54021de` | **P1** `run_workers_local`, 8 test | 3547/3 |
| `0b4ad7e` | **P2** `WorkerMode::Parallel`, 14 test | 3561/3 |
| `736dc5c` | **P3** VM runner, 10 test | 3571/3 |
| `9e113ba` | P2 soak 69M task | 3561/3 |
| `3586e66` | P3 soak 38M task | 3571/3 |
| `0298785` | main hiện hành (merge P3 soak) | 3571/3 |
| *commit docs này* | tổng hợp release evaluator | docs-only |

**Kết luận**: Evaluator W>1 đã đủ chín để **opt-in/test-only** trên interp + VM (CPU non-frame),
với parity/soak/Windows/gate đầy đủ. Chưa đủ điều kiện bật default hoặc nhận task frame/host/
`chan(0)`; P2b/P4 và hai quyết định A0 là điều kiện tiếp theo.
