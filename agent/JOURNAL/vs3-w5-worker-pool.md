<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b (W5) — worker pool opt-in: TaskId/proxy, snapshot, print merge, budget atomic

**Trạng thái**: mã + test **opt-in/test-only**, **chưa** nối evaluator ViScript, **chưa** bật default.
**Nhánh**: `vs3/w5-spike` (merge main `f710e5b`; user nêu `77b71fb`, main thực tế đã tiến thêm docs).
**Commit**: xem `git log -1` (file này nằm trong cùng commit).
**Căn cứ**: `agent/DECISIONS.md` VD-271 (bổ sung B1–B7), `agent/VS3-TIEN-DO.md` §3b.

## 1. Phạm vi đã làm

| Tệp | Nội dung |
|---|---|
| `crates/vis-interp/src/rt/worker.rs` (mới) | `TaskId`, `TaskHandle`, `SendValue`, `WorkerPool` N luồng, `WorkerCtx` (Env snapshot/locals/output/budget), `CompletionBoard`, `WorkerError::CrossWrite`, cancel hợp tác. |
| `crates/vis-interp/src/value.rs` | **Chỉ 2 dòng hook**: `include!("rt/worker.rs"); pub use worker::TaskId;`. **Không đổi enum `Value`/ABI** (B5). |
| `crates/vis-interp/src/host.rs` | `TaskOutput` — buffer `print` theo task (worker dùng; W1 có thể dùng cho Host). |
| `crates/vis-interp/src/frame/fee.rs` | `SharedBudget` atomic, `BudgetLease` chunk reservation, `BudgetSnapshot`. Không đụng `BUDGET` thread-local/test-hook cũ. |
| `crates/vis-cli/tests/vs3_worker.rs` | 8 ca: W=1/2/4 byte-identical, shared env vs snapshot, cross-write, budget chunk, cạn budget, cancel/join, merge khi completion lệch, `Value::Task` Show/SendValue. |

## 2. Hành vi

- **W=1**: task chạy tuần tự trên caller, **shared env**; task sau thấy ghi của task trước (B2).
- **W>1**: mỗi task nhận snapshot top-level `var` tại spawn; worker riêng; output buffer riêng; merge theo **spawn order** (B3).
- **Cross-write**: W>1 task ghi top-level ⇒ `WorkerError::CrossWrite { vars, tasks }`; main env không đổi (B1/B2). Chưa có mã chẩn đoán sản phẩm.
- **Budget**: `SharedBudget::new(limit, chunk)` + `BudgetLease` reserve chunk; `used + remaining == limit`; cạn ⇒ task lỗi, không deadlock; default off (B4).
- **Cancel**: `TaskHandle::cancel()` + `WorkerCtx::check_cancel()`; hợp tác, task phải kiểm; join không treo.
- **TaskId/proxy**: `TaskId(u64)`; completion qua channel; `CompletionBoard` buffer; output merge spawn order; không `Rc<Task>`/`Arc<Mutex<Task>>` trong worker.
- **Value/API**: `Value::Task(Rc<Task>)` **giữ nguyên** (B5), `Show` `<tác vụ>`; `TaskId` là handle mới cho worker; `Value::to_send` chuyển tập Sendable, non-sendable ⇒ lỗi.

## 3. Hook cần W1 nối (không sửa `rt.rs`)

1. `value.rs` đang `include!("rt/worker.rs")` để giữ R1 (value.rs đúng **1500** dòng). W1 chuyển sang `rt.rs`:
   - thêm `mod worker;` (hoặc `pub mod worker;`) trong `rt.rs`;
   - xoá `include!` + `pub use worker::TaskId` ở `value.rs`;
   - nếu muốn `value::TaskId`, thêm `pub use crate::rt::worker::TaskId;`.
2. Thay `Value::Task(Rc<Task>)` bằng handle `TaskId`/`TaskHandle` **khi W1 tích hợp**; cần sửa `interp.rs`, `vm.rs`, `frame/**`, `rt.rs` — ngoài ownership W5.
3. Nối evaluator: mỗi worker cần `Interp`/`Vm` + `Host` riêng; `spawn` chuyển args/capture qua `Value::to_send`; `await` đọc completion theo `TaskId`; output task đưa vào `TaskOutput` và merge spawn order ở final join.
4. `host.rs`: thay `Interp.out`/`Host.out` ghi trực tiếp bằng buffer task; cần W1 sửa `host.rs`/`interp`/`frame` driver.
5. Budget: thay `frame/fee.rs` thread-local `BUDGET` bằng `SharedBudget`/lease khi pool opt-in; giữ default off; nối `VS3-1a` (BUDGET/Q/default) và chunk size.
6. `Chan`: `Arc<Mutex<ChanBuf<Wire>>>` + `Condvar` vẫn thuộc VS3-2a/3b-3; worker pool hiện chỉ có completion channel, **chưa** data channel.
7. `VS0508`: `Value::to_send` đã trả lỗi; checker/spawn cần dùng để từ chối sớm (W1/checker).

## 4. Bằng chứng

- `cargo test -p vis-cli --test vs3_worker`: **8 passed**.
- `cargo test -p vis-cli --test vs3_budget --test vs3_cancel --test vs3_chan0`: **8 + 14 + 13 passed**.
- `cargo test -p vis-interp --lib`: **303 passed**.
- `cargo fmt --all -- --check`: PASS; `cargo clippy -p vis-cli --all-targets -- -D warnings`: PASS.
- `CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5`; df /home còn >5G.

## 5. Chưa nối / blocker

- **B5**: `Value::Task` vẫn `Rc<Task>`; worker handle `TaskId` chưa thay enum — W1 sở hữu tích hợp, đổi ABI cần W1 sửa các tệp bị cấm.
- **Evaluator**: `WorkerPool` nhận closure Rust + `SendValue`, **không** chạy `async fn` thật; W1 phải map `spawn`/`await`/frame.
- **W=1**: worker pool tự mô phỏng shared env; runtime mặc định vẫn single-thread `Scheduler`, chưa dùng pool.
- **Host.out**: `TaskOutput` có sẵn nhưng `Interp`/`Vm` chưa ghi vào; W1 nối.
- **Budget**: `SharedBudget` chưa nối `BUDGET` thread-local/VS3-1a; chunk mặc định do caller; chưa có CLI/budget default.
- **Cross-write**: `WorkerError::CrossWrite` là lỗi nội bộ; A0 chưa cấp mã/thông điệp sản phẩm (B1).
- **Cancel**: hợp tác, task phải tự gọi `check_cancel`; chưa nối safe point/frame.
- **Chan data/select/timeout**: chưa làm (VS3-2a/3b-3).
- **Gate**: chỉ targeted test, không full `gate --fast`/nightly; W1/A0 chạy trên merge.

## 6. Không tuyên bố

Không “VS3-3b xong”, không “worker pool default”, không đổi `Value` ABI/API, không đụng `net.rs`/`pkg.rs`/`interp.rs`/`vm.rs`/`frame/**`/`rt.rs`.
