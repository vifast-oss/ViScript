<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — prototype `TaskId` trong `Value` + proxy completion + hook evaluator

**Trạng thái**: **prototype + design**, chưa merge production, chưa nối evaluator.
**Baseline**: `c85b523` (v0.3.1 QA); đã `git merge main` trên `vs3/w5-spike`; **không** push/merge.
**Phạm vi sửa**: `crates/vis-interp/src/rt/worker.rs`,
`crates/vis-interp/src/value.rs` (1 dòng re-export), `crates/vis-interp/Cargo.toml` (feature),
`crates/vis-cli/Cargo.toml` (bật feature cho test), `crates/vis-cli/tests/vs3_worker.rs`, journal.
**Không đụng**: `interp.rs`, `vm.rs`, `frame/**`, `async_net.rs`, `net/*`, `pkg.rs`.

---

## 1. Hình dạng prototype

### 1.1 `TaskId` và `Value::Task(TaskId)`

- `TaskId(u64)` opaque, `Display` = `#<n>`, `Hash/Ord/Copy` — đã có từ B5.
- Hình dạng đích: `Value::Task(TaskId)` thay `Value::Task(Rc<crate::rt::Task>)`.
- `Show` giữ **`<tác vụ>`**, `eq` luôn **false**, `to_send` vẫn **`Err`** (`VS0508`) — không đổi
  ngữ nghĩa quan sát được; chỉ đổi **handle** (u64 thay con trỏ heap).

### 1.2 `ProtoValue` (feature `taskid-proto`, mặc định tắt)

Vì `Value` là enum công khai và `interp.rs`/`vm.rs` đang match `Value::Task(Rc<Task>)`, đổi
trực tiếp sẽ phá build của tệp W1 sở hữu. Prototype dùng **enum mirror** cạnh `Value`:

```rust
// crates/vis-interp/src/rt/worker.rs, trong `pub mod worker`
#[cfg(feature = "taskid-proto")]
pub enum ProtoValue { /* mọi biến thể của Value; Task(TaskId) */ }
```

- `ProtoValue::from_value(&Value) -> Result<Self, String>` / `to_value()`;
  `Rc<Task>` ↔ `TaskId` **cố ý lỗi**: phải qua registry/proxy của W1, không bịa id.
- `show()`: cây không có `Task` ⇒ gọi đúng `Value::show` (một nguồn sự thật);
  cây có `Task` ⇒ render tại chỗ với `<tác vụ>`.
- `eq_value()`: có `Task` ⇒ false; còn lại chuyển `Value` và dùng `Value::eq_value` (giữ
  `Percent/Float`, map last-wins, record đa tập…).
- `task_id()` cho evaluator lấy handle.
- **Không đổi enum `Value`/ABI mặc định**. Feature bật ở `vis-cli` để test:
  `vis-interp = { path = "../vis-interp", features = ["taskid-proto"] }`; thư viện dùng
  mặc định vẫn không thấy `ProtoValue`. Khi W1 đổi `Value` thật thì **xoá** mirror + feature.

### 1.3 Proxy completion `TaskWaitBoard`

```rust
pub struct TaskWaitBoard { /* Mutex<HashMap<TaskId, Result<SendValue,String>>> + Condvar */ }
pub fn publish(id, result);          // first-wins
pub fn try_take(id) -> Option<...>;  // không chờ
pub fn wait_timeout(id, dur) -> Result<Result<SendValue,String>, WaitError>;
pub fn close();
```

- `WorkerPool::attach_wait_board(Arc<TaskWaitBoard>)` (gọi trước `spawn`).
- W>1: wrapper task **publish ngay khi task xong**, nên main `await` được khi pool còn chạy.
- Task bị huỷ trước khi chạy: `worker_loop` publish `Err("tác vụ bị huỷ")`.
- W=1 và task huỷ: `join` publish nốt theo `outcomes`; `publish` first-wins.
- `Drop` của pool `close()` để waiter không treo; kết quả đã publish vẫn `try_take` được.
- **Chỉ main thread** được `wait_timeout`; worker không block chờ worker khác (evaluator phải
  `yield`, xem §2.3). `wait_timeout` không có chế độ chờ vô hạn nên không thể treo cứng.
- Board **một pool một board**; chỉ chứa `SendValue`/`String` nên `Send + Sync`; không shared
  mutable heap của ngôn ngữ.

## 2. Hook evaluator cho W1 (design-only — không sửa tệp W1)

### 2.1 `value.rs`

```diff
-    Task(Rc<crate::rt::Task>),
+    Task(TaskId),
```

- `Show` giữ `"<tác vụ>"`; `eq_value` giữ false; `to_send` giữ `Err(VS0508)`.
- `TaskId` lấy từ `crate::rt::worker::TaskId` (sau khi chuyển `include!` → `mod worker;` trong
  `rt.rs`). Trong giai đoạn chuyển, registry `Rc<Task> → TaskId` chỉ để đọc task cũ.
- Không đổi bytecode/cache; `Value` là public API nên cần **A0/W1 duyệt** trước khi land.

### 2.2 `rt.rs` — bảng task + scheduler

```rust
pub struct Scheduler {
    next: u64,                       // global TaskId (1,2,...)
    local: HashMap<TaskId, Rc<Task>>,// task affine theo worker/main
    board: Arc<TaskWaitBoard>,       // completion proxy
}
impl Scheduler {
    pub fn alloc_id(&mut self) -> TaskId;
    pub fn spawn_local(&mut self, id, task: Task) -> TaskId;
    pub fn poll(&mut self, id: TaskId) -> Option<Result<Value, Trap>>; // cache
    pub fn wait_local(&mut self, id) -> Wait; // yield nếu chưa xong
}
```

- `Task` vẫn `!Send`; chỉ `TaskId`/`SendValue`/completion qua biên worker.
- Không `Arc<Mutex<Task>>` toàn cục; bảng task nằm trong worker sở hữu task.
- Final join theo **global id tăng dần** (spawn order), kể cả task không `await`.

### 2.3 `interp.rs` — spawn/await

```rust
// HExpr::Spawn: args -> SendValue, cấp id, chọn mode
let sends: Vec<SendValue> = args.iter().map(Value::to_send).collect::<Result<_,_>>()?;
let id = sched.alloc_id();
if sched.workers() == 1 {
    sched.spawn_local(id, Task { .. });
} else {
    let payload = Arc::new(SendValue::List(sends));
    pool.spawn_worker_copy(payload, move |ctx, p| run_body_in_worker(ctx, p))?;
}
Value::Task(id)

// HExpr::Await: local cache trước, remote thì yield chứ không block
match sched.poll(id) {
    Some(kq) => kq?,                    // await lần hai trả cache
    None => return Yield::Await(id),    // scheduler resume khi completion tới
}
```

- `spawn` vẫn chỉ nhận **tên `async fn` cấp tệp** (không closure).
- `Value::Task(TaskId)` không Sendable: truyền task handle qua worker khác phải là
  **yêu cầu await/proxy**, không nhét handle vào payload.
- Lỗi thân task giữ `VS0506` + span lúc tạo; await task huỷ trả lỗi huỷ đã surface.

### 2.4 `vm.rs`

- `goi_than(than, args)` map sang `TaskId`; mỗi worker có `Vm`/`Interp` + `Host` riêng.
- Kết quả task `to_send()` trước khi qua completion; `Value::Task` chỉ là handle.
- Không đổi bytecode; chỉ đổi nơi giữ `Rc<Task>` → bảng `TaskId → Rc<Task>`.

### 2.5 Env snapshot

- **W=1**: giữ shared env hiện tại (B2), không snapshot.
- **W>1**: snapshot top-level `var` tại spawn; task ghi local + `dirty`; main từ chối
  `WorkerError::CrossWrite` trước merge (B1). W1 cần xác định ranh giới `var` top-level
  (frame/interp) — W5 không tự quyết.
- `TaskWaitBoard` không chứa env; env chỉ đi qua `EnvMap`/`SendValue`.

### 2.6 Output

- Mỗi task `TaskOutput`; pool merge theo **spawn order**; W=1 == W=4 byte-identical.
- Output task lỗi/cancel giữ tới trap theo B3; `join` trả `output` đã merge.

### 2.7 Budget

- Một `SharedBudget` mỗi pool; mỗi task `BudgetLease` chunk **256** (đo B7: ~48 ns/charge;
  chunk 1 ~193 ns). `used + remaining == limit` luôn đúng; cạn ⇒ task lỗi
  `VS0501-BUDGET`, không reset.
- Default **off** tới khi VS3-1a chốt; `BUDGET` thread-local hiện tại do W1 nối.

### 2.8 Cancel

- `TaskHandle::cancel` đặt token; task kiểm `ctx.check_cancel()` tại safe point.
- Completion vẫn publish (Err) để await không treo; final join surface `cancelled`.
- Không đóng handle ngôn ngữ (VS3-1b); W1 quyết `Task` cleanup.

### 2.9 Lát W1 đề xuất

| Lát | Việc | Cổng |
|---|---|---|
| W1-a | `TaskId` + bảng task; `Value::Task(TaskId)`; W=1 shared env | toàn bộ async hiện có + W=1 byte-identical |
| W1-b | spawn local-first + `spawn_with_payload`; await local cache | `vs3_worker`, budget |
| W1-c | spawn remote + `TaskWaitBoard` + snapshot Env | W=1/W=4 output, `VS0508`, CrossWrite |
| W1-d | `TaskOutput` merge + cancel/join + final join order | stress 10k, cancel, `VS0506` |
| W1-e | B7 CPU-only + payload matrix; RSS; default off | benchmark/gate/nightly |

## 3. Test prototype (đã chạy)

- `taskid_proto_value_show_eq_roundtrip`: `Value ↔ ProtoValue` cho int/str/list/map/record/enum/
  range; `Show` khớp `Value::show`; eq khớp (trừ `Range` nội bộ); `TaskId` show `<tác vụ>`,
  eq false, `from_value(Rc<Task>)`/`to_value(TaskId)` đều **lỗi có chủ đích**.
- `taskid_proxy_await_w1_w4`: 64 task W=1/W=4; W=4 await `id 0` **trong lúc pool chạy**;
  join 64 outcome; board đủ kết quả; take xong bỏ; cancel publish Err; timeout 10 ms trên id
  không tồn tại trả `WaitError::Timeout` (không treo).
- `cargo test -p vis-cli --test vs3_worker`: **12 passed / 0 failed / 0 ignored**
  (10 ca cũ + 2 ca mới). Không thêm `#[ignore]`; skipped suite giữ **3**.
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy -p vis-cli --all-targets -- -D warnings`: PASS.
- `cargo clippy -p vis-interp --all-targets [--features taskid-proto] -- -D warnings`: PASS cả hai.
- Không chạy lại benchmark B7/RSS trong vòng này; số B7 vẫn là tham chiếu.

## 4. Blocker

- **ABI `Value::Task`**: đổi `Rc<Task>` → `TaskId` là thay đổi public; cần A0/W1 duyệt và sửa
  mọi match/`Show`/`eq`/test. Prototype mirror chỉ là hợp đồng, **không phải** thay đổi thật.
- **Registry chuyển tiếp**: `Rc<Task>` ↔ `TaskId` chỉ W1 dựng được (bảng task); W5 cố ý lỗi
  thay vì bịa id.
- **Await phải yield**: `TaskWaitBoard::wait_timeout` chỉ cho main thread; worker block chờ
  worker khác sẽ chiếm worker và có thể deadlock khi pool bão hoà. Scheduler W1 phải resume
  bằng completion, không block.
- **W=1 chạy lazy**: queue chỉ chạy lúc `join`, nên await trước join ở W=1 phải **drive queue**
  (chạy task) chứ không chờ board. W1 chịu trách nhiệm ở scheduler.
- **Env snapshot/output/host**: ranh giới `var` top-level, `TaskOutput` merge, `Host.out` chưa nối;
  W5 không sửa.
- **Budget thread-local**: `frame/fee.rs BUDGET` vẫn là đường mặc định; nối `SharedBudget`
  cần VS3-1a + W1.
- **Board một pool**: `Drop` đóng board; không tái dùng cho pool khác. W1 có thể thay bằng
  completion channel của scheduler nếu muốn.
- **Chưa production**: không tuyên bố worker pool/async default; không đổi bytecode/cache.

## 5. Tái lập

```bash
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5 \
  cargo test -p vis-cli --test vs3_worker
```

Feature prototype (thư viện):

```bash
cargo test -p vis-interp --features taskid-proto
```
