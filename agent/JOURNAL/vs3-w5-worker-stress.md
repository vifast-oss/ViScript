<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b hardening — stress `WorkerPool` + kế hoạch nối evaluator (W1)

**Trạng thái**: stress + design only. **Không** nối evaluator, **không** bật default.
**Baseline**: `4d016c8` (v0.3.0 tagged; đã `git merge main`).
**Commit**: xem `git log -1` (báo cáo + stress test + example trong cùng commit).
**Phạm vi**: `rt/worker.rs`, `crates/vis-cli/tests/vs3_worker.rs` (stress non-ignored),
`crates/vis-cli/examples/vs3_worker_bench.rs`, journal. Không đụng
`interp.rs`/`vm.rs`/`frame/**`/`net.rs`/`pkg.rs`.

## 1. Stress 10k task

Test **không** `#[ignore]`: `stress_10k_cancel_budget_print_cross` trong
`crates/vis-cli/tests/vs3_worker.rs`; chạy trong suite thường.

| Pha | Nội dung | Kết quả |
|---|---|---|
| Print merge | 10k task, W=1 và W=4, mỗi task 1 dòng | output **10.000 dòng**, W=1 == W=4 byte-identical; 10k outcomes |
| Cancel | 10k task, huỷ một nửa; task check cancel sau sleep 200 µs | W=1/W=4 join trả 10k outcome, có task `cancelled`, không deadlock |
| Budget cạn | 10k task, pool `limit=5.000`, chunk 256 | `used=5.000`, `remaining=0`, `used+remaining==limit`, `exhausted=true`, 10k outcome, task lỗi surfaced |
| Cross-write | 100 task cùng ghi `counter`, W=4 | `WorkerError::CrossWrite { vars:["counter"], tasks:100 }`, không merge output, không deadlock |
| Thread | đếm `Threads:` trước/sau | **2 → 2** (không rò worker thread) |
| RSS | `/proc/self/status` trước/sau | **~3,34–3,37 MB → ~8,76–9,32 MB**, HWM ~8,8–9,3 MB |
| Thời gian | cả 4 pha, 3 lần | **2,03 / 2,08 / 2,10 s** |

- `cargo test -p vis-cli --test vs3_worker`: **10 passed / 0 ignored** (9 ca cũ + stress).
- Suite skipped giữ **3** (không thêm `#[ignore]`).
- RSS tăng chủ yếu do 10k outcome/output (W1+W4) và allocator giữ; thread về baseline.
  Không có API đếm task sống trong `WorkerPool`; test dùng join/threads làm bằng chứng.
- Test dùng closure + `SendValue` (mô hình WorkerPool), chưa phải `async fn` thật.

## 2. Kế hoạch nối evaluator cho W1 (design-only)

> W5 **không** sửa các tệp dưới. Đây là hợp đồng đề xuất để W1 làm sau VS3-1d/2a.

### 2.1 Hiện trạng cần chạm

| Vùng | Hiện trạng | Việc W1 |
|---|---|---|
| `value.rs:364` | `Value::Task(Rc<Task>)`; `Show` `<tác vụ>` | Đổi handle sang `TaskId`/`TaskHandle`; giữ `Show` `<tác vụ>`, `eq` false; TaskId hiện re-export từ `value.rs:1500`. |
| `rt.rs:56` | `Task { id, than, args, ten, span, trang_thai, kq }`; `Scheduler` một luồng | Tách **task nội bộ** (giữ `Rc<Task>` trong worker) khỏi **handle** `TaskId`; bảng task theo worker + bảng chuyển tiếp completion. |
| `interp.rs:1058/1073` | `HExpr::Await` gọi `rt::await_task`; `HExpr::Spawn` tạo `Scheduler::spawn` | Map args → `SendValue`, gọi `WorkerPool::spawn_worker_copy`/`spawn_with_payload`; await theo `TaskId`; W1 local-first. |
| `crates/vis-vm/src/vm.rs:568` | `goi_than` chạy task bằng `Interp`/VM hiện tại | Mỗi worker `Vm`/`Interp` + `Host` riêng; task body chạy trong worker, kết quả `to_send`. |
| `frame/**` + `lib.rs:244` | `run_async_with_hosts_caps_backend` dùng `run_frame_driver` | Không đổi driver; chỉ chọn worker pool opt-in/test; mỗi task dùng frame heap riêng. |
| `host.rs` | `Host.out() -> &mut String`; mới có `TaskOutput` | Task print vào `TaskOutput`; completion mang buffer; merge **spawn order** ở final join. |
| `frame/fee.rs:47` | `BUDGET` thread-local | Worker path dùng `Arc<SharedBudget>`/`BudgetLease` (chunk 256); default off; `VS0501-BUDGET`. |

### 2.2 Hợp đồng handle/proxy

1. `TaskId` opaque (u64), duy nhất toàn chương trình; `Value::Task(TaskId)`.
2. Mỗi worker giữ `HashMap<TaskId, Rc<Task>>` nội bộ; task **affine**, frame không di trú.
3. Handle ghi danh `(owner_worker, TaskId)` ở scheduler chính; `await` local trả cache,
   remote gửi yêu cầu completion qua channel/proxy rồi `Wait::Task(TaskId)`.
4. Final join theo **thứ tự tạo** (global id), kể cả task không await; cache kết quả;
   lỗi `VS0506` giữ mã gốc + span lúc tạo.
5. Không `Arc<Mutex<Task>>` toàn cục; `Task` không Send; chỉ `SendValue`/completion Send.

### 2.3 Spawn/await

- Giữ `spawn` chỉ nhận tên `async fn` cấp tệp (hiện tại); không closure.
- Eval args → `Value`; nếu W>1 dùng `Value::to_send`/`spawn_worker_copy`; non-sendable
  ⇒ `VS0508`; W=1 shared env không snapshot.
- **Local-first**: task phát sinh trên worker hiện tại chạy local queue nếu không có
  yêu cầu song song; chỉ `spawn_worker_copy`/`spawn_with_payload` khi biết đích khác.
- `await`: local cache; remote proxy; result `SendValue` chuyển lại `Value` trong heap
  nhận; lỗi task vẫn `VS0506`.

### 2.4 Env snapshot, output, budget, cancel

- **Env**: W=1 giữ shared env hiện tại (B2); W>1 snapshot top-level `var` tại spawn,
  task ghi local + `dirty`; main từ chối `WorkerError::CrossWrite` trước merge (B1).
- **Output**: mỗi task `TaskOutput`; merge theo spawn order; output task lỗi/cancel giữ
  tới trap theo B3; W=1 và W=4 byte-identical.
- **Budget**: `SharedBudget` một pool; mỗi task lease chunk 256; `used+remaining==limit`;
  cạn ⇒ task lỗi/`VS0501-BUDGET`; không reset; default off tới khi VS3-1a chốt.
- **Cancel**: `TaskHandle::cancel` → token; worker kiểm tại safe point; gỡ waiter/timer;
  final join surfaced; không đóng handle ngôn ngữ (VS3-1b).

### 2.5 Thứ tự lát W1

| Lát | Việc | Test |
|---|---|---|
| W1-3b-a | `TaskId`/handle + bảng task; `Value::Task(TaskId)`; W=1 giữ shared env | `vs3_worker` + toàn bộ async hiện có; W=1 byte-identical |
| W1-3b-b | Spawn local-first + `spawn_with_payload`; wait local; budget lease | W=1 parity, budget used+remaining |
| W1-3b-c | Spawn remote + proxy completion + `TaskId` await; snapshot Env | W=1/W=4 output, `VS0508`, cross-write refusal |
| W1-3b-d | Output `TaskOutput` + merge; cancel/join; final join order | 10k stress, cancel, error `VS0506` |
| W1-3b-e | Gate B7 CPU-only + payload matrix; RSS; no default | benchmark, gate/nightly |

### 2.6 Hook hiện tại (W5 không sửa)

`value.rs:1499-1500` đang:
```rust
include!("rt/worker.rs");
pub use worker::TaskId;
```
W1 chuyển sang `rt.rs`:
```rust
mod worker; // hoặc pub mod
```
rồi bỏ `include!`/`pub use` (hoặc re-export `pub use crate::rt::worker::TaskId`).
`worker.rs` đã dùng `crate::value::Value` nên move được; API `spawn`,
`spawn_with_payload`, `spawn_worker_copy`, `join`, `SharedBudget` giữ nguyên.

## 3. Blocker

- **ABI/API `Value::Task`**: đổi từ `Rc<Task>` sang `TaskId` là thay đổi public `Value`;
  cần W1 sửa mọi match/`Show`/`eq`/test; không đổi bytecode nhưng cần quyết định A0/W1.
- **Evaluator chưa nối**: stress/benchmark chỉ chứng minh cơ chế closure + `SendValue`;
  chưa có `async fn`/frame/VM thật trong worker.
- **Env snapshot**: chưa có cách xác định top-level `var`/closure capture trong runtime;
  W>1 snapshot semantics và cross-write policy cần W1 kiểm.
- **Budget**: `BUDGET` vẫn thread-local; nối `SharedBudget` cần W1 + VS3-1a (default/Q).
- **Output**: `Host.out` chưa dùng `TaskOutput`; cần W1 sửa host/driver/final join.
- **Map allocator**: stress không đo map; B7 worker-copy map đạt nhưng move/producer kém
  (xem `vs3-w5-worker-bench.md`); W1 chọn mode theo payload.
- **Channel**: `Chan` liên worker/rendezvous timeout chưa nối; VS3-2a/3b-3.
- **Stress**: 10k closure tasks, không phải 10k `async fn`; RSS ~9 MB chỉ tham khảo,
  không phải cap production.

## 4. Không tuyên bố

Không “worker pool production”, không “async default dùng worker”, không đổi
`Value`/bytecode/cache, không full gate/nightly.
