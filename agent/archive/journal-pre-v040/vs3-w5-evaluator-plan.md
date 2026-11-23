<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — kế hoạch nối evaluator với worker pool

**Trạng thái**: **DESIGN-ONLY — KHÔNG sửa production, KHÔNG build/gate trong phiên này.**
**Nhánh**: `vs3/w5-spike` (worktree `/home/ps/.local/share/opencode/worktree/cd45e5/vs3-w5-spike`).
**Baseline**: đã `git merge --ff-only main` → **`f75b246`** (2026-11-22). Mốc user nêu
`974accc` (gate 3538/3) là **mốc cũ hơn**; main đã tiến thêm các commit W3/tag v0.3.3 trước khi
merge. Số gate/test phải đo lại ở đầu P1, không dùng số cũ làm bằng chứng.
**Phạm vi**: chỉ ghi tệp này; không đụng `crates/`, `Cargo.toml`, `VERSION`, cache, SPEC.
**Timebox**: 90 phút, ưu tiên ít CPU/đĩa. `df /home` lúc viết: **12G còn trống (>5G)**.

---

## 0. Kết luận ngắn

1. **Khả thi có điều kiện.** Nối evaluator với worker pool làm được **ngay ở W=1** (chạy trên
   caller, shared env, không đổi ngữ nghĩa) và **W>1 cho tác vụ CPU thuần** (snapshot env,
   kết quả/đối số `SendValue`, in theo thứ tự tiêu thụ, budget atomic, cancel hợp tác).
2. **Không khả thi trong một lát** cho tác vụ async có I/O/host/`Chan`/`cap` đi qua biên worker:
   `Value` là `!Send` theo VD-160, `Interp`/`Vm` mượn HIR/Module + host `&mut dyn` không `Send`,
   `Chan` chưa có wire dùng chung. W>1 vì vậy **chỉ nhận tác vụ thuần**; vi phạm ⇒ lỗi rõ.
3. **Đề xuất thứ tự**: P1 (khung `ExecState` + W=1 = oracle) → P2 (W>1 interp, non-frame) →
   P3 (VM parity) → P4 (frame suspend/resume) → P5 (opt-in CLI + soak). P1/P2 mới là phần
   mở khoá giá trị; P4 là phần khó nhất và **không** cần cho gate CPU 2,4×.
4. **Không đổi ABI**: giữ `Value::Task(Rc<Task>)`, `VERSION`/`VBC1`, cache key, `Show`.
   `TaskId` là handle **song song** trong registry `Task.id ↔ pool TaskId`; không thay enum.
5. **Ước lượng**: ~1.000–1.700 dòng production + ~1.000–1.400 dòng test cho cả 5 pha;
   MVP P1+P2 ~680–940 dòng production. Rủi ro cao nhất: chi phí dựng evaluator mỗi task
   (10k task), `Send/Sync` của ảnh chương trình, và ngữ nghĩa thứ tự in khi await lệch thứ tự.

---

## 1. Phạm vi, ràng buộc, không tuyên bố

### 1.1 Ràng buộc cứng

| Nguồn | Ràng buộc áp vào kế hoạch |
|---|---|
| **VD-160** | Mỗi worker heap riêng; `Rc`/`RefCell` không atomic; `Value`/`Task` `!Send`; `Chan`/`Host` `Send+Sync`; qua biên worker phải **sao chép sâu**; closure/handle/Task không gửi được. |
| **VD-178 QĐ-1** | `spawn` chỉ gửi được `int/float/bool/str/bytes/()/list/map/set/tuple/record/enum`; closure/handle/Task ⇒ `VS0508`; đo song song bằng task CPU, ngưỡng `0,6 × min(nproc,N,4)`. |
| **VD-271 B1–B7** | Opt-in/test-only; W=1 shared env, W>1 snapshot + từ chối ghi chéo; print merge tất định; budget atomic + chunk; `TaskId`/proxy, **không đổi ABI `Value`**; chưa cấp mã mới cho ghi chéo. |
| **AGENTS.md / R1–R4** | ≤1.500 dòng/tệp nguồn; không `unsafe`; không dependency mới; SPDX; code+test là nguồn sự thật. |
| **User (phiên này)** | Design-only; không push/merge/tag; không GitHub Actions; kiểm `df` (<5G là điều kiện dừng, hiện 12G). |

### 1.2 Không tuyên bố

- Không "VS3-3b xong", không "worker pool default", không đổi ngữ nghĩa runtime mặc định.
- Không hứa tăng tốc cho `std.net`/HTTP async, kênh `cap(0)`, `cap`, `exec`, `std.fs`, `std.sqlite`
  đi qua worker. Các đường đó vẫn ở scheduler một luồng/frame như hiện tại.
- Không đổi `VBC1`/`VERSION`, không đổi cache key, không đổi `Value`, không đổi `Task` struct
  (test hiện tại dựng `Task { … }` trực tiếp — xem `vs3_worker.rs:236,526`).

---

## 2. Hiện trạng (đọc mã, không suy đoán)

### 2.1 Evaluator một luồng

| Thành phần | Vị trí | Hành vi hiện tại |
|---|---|---|
| `spawn` interp | `interp.rs:1073-1110` (`HExpr::Spawn`) | Bắt buộc callee là **tên hàm async cấp tệp**; eval đối số; `Scheduler::spawn(than,args,ten,span)`; trả `Value::Task(Rc<Task>)`. |
| `spawn` VM | `builtin.rs:664-671` (`$spawn`) | Nhận handle `Int` (chỉ số hàm bytecode) + đối số; gọi `Scheduler::spawn`; trả `Value::Task`. |
| `await` | `interp.rs:1058-1071`, `builtin.rs:639-645` | Gọi `rt::await_task`; `await` chéo chính nó ⇒ `VS0501`; cache kết quả. |
| `Task`/`TrangThai` | `rt.rs:37-73` | `Task.id: u32` (1-based), `than: u32`, `args`, `ten`, `span`, `trang_thai`, `kq` cache. |
| `Scheduler` | `rt.rs:465-485` | `tasks: Vec<Rc<Task>>`, `ready: VecDeque<usize>`, `tiep: u32`, `sau: usize`, `kenh`, `frame`. |
| `tao`/`spawn` | `rt.rs:523-600` | `tao` chỉ tạo; `spawn` tạo + đẩy `ready`/`frame.ready`; frame nhận nguồn giả theo tên. |
| `await_task` | `rt.rs:1143-1227` | `Xong` ⇒ cache; `DangChay` ⇒ `VS0501`; `Huy` ⇒ cache/`VS0506`; frame bật ⇒ `Abort::Pending`; ngược lại chạy `goi_than` **tới hoàn tất** trên caller, tăng `sau` (trần `MAX_TASK_NEST=256`). |
| `join_all` | `rt.rs:1239-1245` | Await **mọi** task theo thứ tự tạo, chỉ khi main thành công. |
| `chay_mot` | `rt.rs:1122-1140` | Chạy task đầu `ready`; chỉ dùng ở test/đường cũ, không phải `run_program`. |

### 2.2 Frame driver (async default)

- `frame.rs:767+` (`run_frame_driver`): vòng turn; `frames: HashMap<u32, IFrame>`; `Pending`
  ⇒ đăng ký `Waiter` qua `Scheduler::frame_await` (`rt/sched.rs:54`), lưu frame, chọn frame kế
  bằng `frame_next` (`rt/sched.rs:93`).
- `frame.rs:1022-1046`: khi không còn frame sẵn sàng, driver tự đẩy mọi task `Moi` vào
  `frame.ready` (final-join kiểu “không ai await”).
- `frame_final_join` (`rt/sched.rs:485`) chỉ quét `kq` đã có, **không chờ** task chưa chạy;
  `frame_stuck_trap` (`rt/sched.rs:380`) biến “hết frame + còn task/wait” thành `VS0505`.
- VM frame: `vm_frame.rs:701-764` cùng cấu trúc; `call_builtin_frame` (`vm_frame.rs:388`)
  retry `$await` bằng cách đẩy lại `Value::Task` và `Pending`.
- **Điểm mấu chốt**: cơ chế treo/đánh thức `await` **đã có**; chỗ thiếu chỉ là (a) task chạy
  ngoài driver, (b) nguồn completion ngoài event giả/reactor, (c) bỏ qua task worker khi driver
  tự đẩy `Moi`, (d) final join phải chờ worker.

### 2.3 Worker pool prototype

- `rt/worker.rs` (1.015 dòng) — **đang được `include!` từ `value.rs:1499`**, nên `value.rs`
  đúng **1.500 dòng**; thêm bất kỳ dòng nào vào worker.rs là vượt R1. Comment đầu tệp đã ghi
  việc chuyển thành `rt/worker.rs` + `mod worker;` là bước W1 phải làm.
- Đã có: `TaskId(u64)`, `SendValue`, `WorkerCtx` (env snapshot/locals/out/budget/cancel),
  `WorkerPool` N luồng/W=1 tuần tự, `CompletionBoard`, `TaskWaitBoard` (publish/try_take/
  wait_timeout/close), `WorkerError::{InvalidWorkers,ChannelClosed,CrossWrite}`, `TaskOutcome`
  (result/output/cancelled/dirty/budget_used), budget `SharedBudget`/`BudgetLease`,
  `spawn_with_payload`, `spawn_worker_copy`, `spawn_shared(Arc<SendValue>)`.
- **Chưa có**: chạy `async fn` thật; `await` theo `TaskId`; output/error/dirty đi qua board;
  cancel từ evaluator; panic safety trong `worker_loop`; registry id evaluator ↔ pool.
- `worker_loop` hiện không `catch_unwind`; một panic làm chết worker, `join` nhận
  `ChannelClosed`. `spawn` sao `self.root.clone()` **mỗi task** (O(env)); W=1 cũng sao.

### 2.4 Ảnh chương trình — có `Send + Sync` không?

Đã rà mã nguồn (không chỉ grep tên):

- `Hir`, `HFn`, `HExpr`, `Symbols`, `TypeTable`, `Type` đều chỉ chứa `String/Vec/Box/enum`,
  **không** `Rc/RefCell/Cell/Arc/Mutex` (`crates/vis-types/src/**`). Vì vậy **có thể** đóng gói
  `Arc<HirImage>` và gửi sang worker, **với điều kiện** thêm test khẳng định `Send + Sync`
  bằng `fn assert_send_sync<T: Send + Sync>()` trong P1 (không dùng `unsafe`).
- `vis_bytecode::Module` + `Func/Const/Instr` cũng chỉ dữ liệu thuần (`format.rs:114-552`);
  `Arc<Module>` gửi được.
- `Trap` (`frame/core.rs:46-65`) là `&'static Code + String + Span` ⇒ `Send + Sync`; có thể
  chuyển mã gốc/span qua worker thay vì bịa `String`.
- `Interp`/`Vm` **không** `Send` (mượn `&Hir`/`&Module`, `Rc<Env>`, `&mut dyn Host`,
  `Rc<RefCell<Scheduler>>`) ⇒ **không gửi instance**; mỗi worker phải **dựng instance mới**
  từ ảnh chương trình bất biến.

---

## 3. Quyết định thiết kế

### D1 — Chế độ opt-in

| Mode | Ai chạy | Env | Host/Chan/Cap | Dùng cho |
|---|---|---|---|---|
| `Off` (default) | scheduler 1 luồng như hiện tại | shared | đầy đủ | mọi chương trình |
| `Local` (W=1) | **caller thread**, đúng đường cũ | **shared env** | đầy đủ | oracle ngữ nghĩa, test in/budget/cancel, chương trình có host |
| `Parallel` (W=N>1) | N OS thread, mỗi task một evaluator mới | **snapshot tại spawn** | **thuần** (không host/cap/chan qua biên) | task CPU, payload Sendable |

- Opt-in **đầu tiên bằng API ẩn** (`#[doc(hidden)] run_workers_*`), không env/CLI, không đổi
  `vis run`. Lý do: giữ default tuyệt đối, không đụng parse CLI/cache; đúng tiền lệ
  `run_async_with_*`/`frame_*_for_test`.
- P5 mới bàn `VIS_WORKERS=N` (unset = off; `0`/`>64`/không phải số ⇒ exit **2**). Không đặt
  tên `VIS_THREADS` để tránh lẫn với thread OS.

### D2 — Handle, không đổi `Value`

- `Value::Task(Rc<Task>)` **giữ nguyên** (B5). Không thêm field vào `Task` (test dựng literal).
- `ExecState` (mới, nằm trong `Scheduler`) giữ:
  - `proxy: HashMap<u32 /*Task.id*/, TaskProxy>` — `TaskProxy { pool_id: TaskId, ten, span, out_flushed: bool, dirty: Vec<String>, cancelled, state }`;
  - `pool: Option<WorkerPool>`, `runner: Option<Arc<dyn TaskRunner>>`, `order: Vec<u32>`;
  - `mode`, `budget`, `joined`…
- `TaskId` của pool (0-based) **không** cần trùng `Task.id` (1-based); registry ánh xạ hai chiều.
  `Show`/`eq`/`to_send` của `Value::Task` không đổi.

### D3 — Ảnh chương trình và “capture”

- `HirImage { hir: Hir, symbols: Symbols, types: TypeTable, text: Arc<str>, top: Arc<TopIndex> }`
  dựng **một lần** cho mỗi lần chạy khi bật worker; `Arc` chia sẻ cho mọi job (đây là
  “`spawn_shared`” ở mức chương trình: producer clone `Arc` O(1), worker chỉ đọc).
- `Vm` dùng `Arc<Module>` + `Arc<str>` tương tự.
- **Capture của ngôn ngữ**: `spawn` chỉ nhận **tên hàm async cấp tệp**; không có closure capture
  ẩn. Biến cấp tệp (const/let/var) là capture duy nhất ⇒ snapshot ở D4. Đối số do người dùng
  viết là **by value** (đã clone khi eval) ⇒ không có “shared arg” ở mức ngôn ngữ; `spawn_shared`
  chỉ dùng cho **env snapshot** và **ảnh chương trình**, không dùng để tránh clone đối số.
  Ở W>1, vector đối số **đã eval** được **move** sang `SendValue` (`into_send`, không clone
  thêm); `Task.args` để rỗng — đây là khoản tiết kiệm thật cho list10k/map1k.
- Nếu đối số/const/var cấp tệp không `Sendable` ⇒ `VS0508` **tại spawn** (main-side, không phải
  lỗi thân task), exit 1.

### D4 — Env: W=1 shared, W>1 snapshot

- `Interp` thêm `top_syms: Vec<u32>` (từ `Hir.stmts`: mọi `HStmt::Var` và **mọi `sym` trong
  pattern** của `HStmt::Let` cấp tệp) và `top_snapshot: Option<Arc<SendValue>>` (payload
  `SendValue::Map`); `Vm` dùng `globals: Vec<Option<Value>>` + cache tương tự.
- `Host::worker_snapshot(&mut self) -> Result<Arc<SendValue>, String>` (method mặc định trả lỗi
  “host không hỗ trợ worker”) do `Interp`/`Vm` cài (`&mut` để COW snapshot):
  - Interp: chỉ lấy **đúng** `top_syms` từ `env` (không lấy param/local của `main` — `env` phẳng);
    `Value::to_send` từng giá trị; lỗi ⇒ `VS0508`; đóng gói `SendValue::Map` một lần.
  - VM: lấy các ô `globals[k] = Some(v)`; khoá `g<k>` trong Map; ô `None` bỏ qua (worker để `None`).  - **P4 (frame)**: `frame/expr.rs:816` gọi `it.worker_snapshot_from_env(&frame.cur().env)`
    (inherent, `&mut Interp`) rồi `exec::spawn_with_snapshot(it, snap, …)`. Không có hook này,
    spawn từ `main`/hàm lồng sẽ snapshot **thiếu** biến vừa ghi (env frame là bản clone, không
    phải `it.env`). `Host::worker_snapshot` (non-frame) chỉ là wrapper gọi cùng helper với `it.env`.
    P4 có thể snapshot **mỗi spawn** (chưa COW); nếu 10k spawn + global lớn bị chậm, thêm
    generation/dirty hook ở `frame/proc.rs` (P4b) — đo trước, chưa hứa.
- **COW**: nếu main ghi một `top_sym` giữa hai lần spawn, `top_snapshot` bị dựng lại **một lần**
  (hoặc `Arc::make_mut`); 10k spawn không ghi xen kẽ ⇒ **một** payload `Arc` dùng chung, spawn
  chỉ `Arc::clone` O(1). Spawn sau ghi ⇒ task trước vẫn thấy snapshot cũ (đúng B2).
- **Mọi đường ghi** vào biến top-level phải làm mất hiệu lực snapshot: `HStmt::Var`, `HStmt::Assign`
  đích `Var`, và đích `Field`/`Index` có gốc là top-level var (ghi qua `gan_truong`/`gan_chi_so`
  rồi ghi lại ô). Bỏ sót đường Field/Index ⇒ snapshot cũ, test 10/14 sẽ bắt.
- **Giới hạn P2**: snapshot **toàn bộ** binding top-level; chỉ cần **một** binding không Sendable
  (closure/chan/cap/task) là **mọi** spawn W>1 báo `VS0508`, kể cả task không dùng biến đó.
  Đây là lựa chọn bảo thủ cho P2; lọc theo **free-var** của hàm + callee (hoặc bỏ binding không
  Sendable và trap khi task *đọc* nó) là tối ưu P2b — cần phân tích tĩnh, không trộn P2.
- Worker nhận env snapshot qua `spawn_shared(Arc<SendValue>)` với payload **`SendValue::Map`**
  `(khoá "s<sym>" → giá trị)`; **không** thêm variant `SendValue` mới (tránh sửa mọi match
  `to_value`/`show`/test). Khoá theo **sym**, không theo tên, để hai module có biến cùng tên
  không đụng nhau. Worker chuyển `SendValue → Value` vào `env`/`globals` **của riêng nó**;
  ghi vào đó không đụng ai. `new_for_task` phải nạp `mutable` bằng **top-level `var`** (task
  không chạy khởi tạo cấp tệp, nếu không gán `var` sẽ bị coi là gán `let` và trap sai).
- W=1: **không** snapshot; chạy trên caller với `Interp.env`/`Vm.globals` hiện tại ⇒ ghi thấy ngay
  (đúng “shared env”), không cần dirty. **Phạm vi**: P1/P2 dùng entry non-frame, nên shared env
  là thật; nếu P4 bật frame, task frame vẫn clone `globals` như ngữ nghĩa frame hiện tại —
  không hứa merge ghi của task frame vào main (giữ nguyên hành vi cũ, không phải tính năng mới).

### D5 — `spawn_shared(Arc<SendValue>)`

- Dùng cho **env snapshot** (Arc chia sẻ, worker chỉ đọc rồi tự dựng env) và cho **ảnh chương
  trình** (closure capture `Arc<HirImage>`, không phải `SendValue`).
- Không dùng cho đối số người dùng: đối số đã by-value; muốn zero-copy đối số lớn cần
  `spawn f(&x)`/“shared param” — **ngoài phạm vi**, ghi nợ.
- Read-only ở mức kiểu: worker chỉ nhận `&SendValue` cho env; mọi ghi vào env là bản sao cục bộ
  và bị `CrossWrite` từ chối ở W>1.
- **Giới hạn quan trọng**: evaluator vẫn phải dựng `Value` từ `SendValue` trong worker, nên
  `spawn_shared` chỉ bỏ copy **phía main** (snapshot/ảnh), **không** bỏ copy phía worker. Với
  top-level list10k đọc bởi mọi task, worker vẫn copy list mỗi task; bỏ được cần **read-only
  view** hoặc tái dùng evaluator + cache env theo worker (P2b, chưa hứa). Đừng dùng số đo
  `spawn_shared` của prototype closure để tuyên bố tốc độ cho evaluator.

### D6 — In và merge

- **Parallel**: mỗi task có buffer riêng (`WorkerCtx.out`). Buffer được **flush khi kết quả được
  tiêu thụ**:
  - `await t` ⇒ flush output của `t` (kể cả khi `t` lỗi/cancel) **trước** khi trả/trap;
  - task không ai await ⇒ flush ở `join_outcomes` theo **thứ tự tạo** (spawn order);
  - await lệch thứ tự ⇒ flush theo **thứ tự await** — đúng thứ tự run-to-completion hiện tại
    của runtime một luồng, nên **byte-identical với non-frame default** cho cùng chương trình.
- Đây là cách đọc B3 (“merge theo spawn order”) **không** phá parity: “spawn order” áp cho final
  join; ở await, thứ tự tiêu thụ mới là thứ tự quan sát được của runtime cũ. Cần A0 xác nhận
  (Q2); nếu A0 muốn **tuyệt đối** spawn order kể cả await lệch, phải thêm prefix-barrier
  (chờ các task trước) — đắt và đổi ngữ nghĩa await, **không khuyến nghị**.
- **OutcomeBoard là một đường completion** khi evaluator bật: `worker_loop` gửi outcome vào
  board này, **không** gửi thêm `CompletionBoard` cũ (tránh đôi bộ nhớ output 10k). Prototype
  không gắn board giữ nguyên `CompletionBoard`/`join`. `join_outcomes` chờ đủ `order`, join
  thread, trả các outcome **chưa** bị await lấy, flush theo spawn order.
- **Local (W=1)**: không buffer; in trực tiếp như default ⇒ W=1 = oracle. Vì vậy test “W=1 vs
  W=4 byte-identical” dùng chương trình await theo thứ tự hoặc so với non-frame default.

### D7 — Budget

- `SharedBudget`/`BudgetLease` (đã có, `frame/fee.rs:243-365`) là **nguồn duy nhất** ở W>1;
  không dùng `BUDGET` thread-local trong worker.
- Worker evaluator giữ `Option<BudgetLease>`; mỗi điểm `charge` (interp/VM) gọi lease trước/
  thay thread-local. Cạn ⇒ `TaskError` với mã `VS0501-BUDGET`, message có `VS0501-BUDGET` để
  test `vs3_budget` vẫn nhận ra; main bọc `VS0506` như mọi lỗi thân task.
- Default **off**; chỉ bật khi API test truyền `SharedBudget`. `used + remaining == limit` sau
  khi mọi lease drop; trong lúc chạy có thể lệch do chunk reservation (đã ghi ở test hiện tại).
  **Lease phải lazy**: `WorkerCtx`/runner chỉ reserve chunk ở lần `charge` đầu; nếu mỗi task
  reserve 256 đơn vị lúc tạo, 10k task rỗng sẽ rút cạn pool oan (chunk 256 × 10k > limit).
- Chunk mặc định 256 (theo B7); không reset giữa các task (một pool cho cả chương trình).

### D8 — Cancel

- `TaskHandle::cancel()` đã có; `ExecState` giữ handle của từng job; API test `cancel_for_test(id)`
  (ẩn) và đường frame cancel (P4) gọi nó.
- Worker evaluator kiểm `AtomicBool` tại safe point: mỗi **1.024** node AST/instruction (dùng
  `steps` sẵn có) + mỗi `charge`; không preempt tức thời. Task bị huỷ trả
  `TaskError { msg: "tác vụ bị huỷ" }`; `out` đã in được giữ và flush theo D6.
- Hợp đồng cache giữ nguyên: huỷ **trước khi có kết quả** ⇒ `VS0506`; đã có cache ⇒ trả cache
  (`await_task` nhánh `Huy` hiện tại).
- Cancel không đổi mã mới; không thêm `VS05xx` cho cancel.
- **Thoát khi main trap**: `run_program` hiện **không** join khi main lỗi; worker mode phải
  **shutdown** pool thay vì chạy nốt hàng đợi. `WorkerPool` thêm cờ `stopping: Arc<AtomicBool>`;
  `Drop` set cờ + cancel mọi handle + đóng board rồi join; `worker_loop` bỏ qua job khi cờ bật.
  Output của task chưa flush **không** được in sau trap (khớp default: join chỉ chạy khi main
  thành công). Test 26 bắt buộc (10k job, main trap, thoát <1 s).

### D9 — Nested spawn

- Trong W>1, thân task chạy trên **Scheduler riêng của worker** (không frame). `spawn` lồng
  tạo task trong scheduler đó; `await` lồng chạy **tuần tự trên cùng worker** (run-to-completion),
  không quay lại pool ⇒ không deadlock chéo worker, không nhân job.
- Nested task **không** có proxy ở main; output của nó nằm trong buffer của task cha (cùng
  `Interp.out`) ⇒ flush cùng cha.
- Ghi top-level var từ nested task vẫn ghi vào env của worker; dirty của **task cha** gồm mọi ghi
  của cây con ⇒ `CrossWrite` báo theo task cha (đúng “một task logic”).
- Trần lồng `MAX_TASK_NEST=256` áp **trong** worker; vượt ⇒ `VS0501` bọc `VS0506` của task cha.
- W=1 nested dùng scheduler cũ nguyên văn.
- **P4 (frame)**: chỉ frame `main` (`frame.current == 0`) được đẩy job vào pool. Mọi `spawn`
  phát sinh khi `frame.current != 0` (thân task, kể cả async fn gọi trực tiếp) chạy **local**
  trên chính frame đó, không đẩy pool — nếu không sẽ đệ quy pool và sai thứ tự in. Hook này nằm
  ở đầu `exec::spawn` (1 điều kiện), dùng chung interp/VM.

### D10 — Host/thuần

- W>1: evaluator của worker có `fs/net/db/ngoai = None`, `caps = []`. Gọi host ⇒ `VS0604`
  (rõ “worker không có host”) bọc `VS0506`. `Chan` tạo **bên trong** worker được (kênh cục bộ,
  cùng luồng), nhưng `Chan` **không** đi qua biên (đối số/capture) ⇒ `VS0508`.
- Không cố “giả lập host” ở worker: net async cần reactor của main, fs/db cần quyền/caps không
  gửi được. Đây là giới hạn sản phẩm, phải ghi trong tài liệu opt-in.
- W=1 đầy đủ host như default (vì chạy trên caller) ⇒ dùng để test các chương trình có host mà
  không cần song song.

---

## 4. Kiến trúc

```
Interp / Vm (main, caller)
  └─ Scheduler (rt.rs)
       └─ ExecState (mới, rt/exec.rs)
            ├─ mode: Off | Local | Parallel(n)
            ├─ proxy: Task.id → TaskProxy { pool_id, dirty, out_flushed, cancelled }
            ├─ pool: WorkerPool (chỉ Parallel)
            ├─ runner: Arc<dyn TaskRunner>   // ảnh chương trình, Send+Sync
            └─ outcome board: TaskId → TaskOutcome { value, out, dirty, error, budget_used }

WorkerPool (rt/worker.rs, đã có)
  N thread ── Job { TaskId, TaskFn, snapshot(empty), board }
                └─ TaskFn = closure Send + 'static:
                     dựng evaluator từ Arc<HirImage>/Arc<Module>
                     env ← SendValue::Map(khoá "s<sym>" → giá trị)
                     chạy thân async fn (không init top-level)
                     ghi out/dirty qua WorkerCtx; trả Result<SendValue, TaskError>
```

### 4.1 Kiểu mới (đều `#[doc(hidden)]`, nội bộ)

```rust
pub enum WorkerMode { Off, Local, Parallel(usize) }

pub struct TaskError {           // Send + Sync + Clone
    pub code: &'static vis_diag::Code,
    pub msg: String,
    pub span: vis_syntax::Span,
}

pub struct TaskJob {
    pub than: u32,
    pub args: Vec<SendValue>,
    pub span: vis_syntax::Span,
}

/// Runner chạy **trong worker**, một lần cho mỗi job; ghi output/dirty qua `WorkerCtx`
/// (pool đã có buffer/dirty set), lấy cancel/budget qua `ctx.cancel_flag()/budget_handle()`,
/// và trả `Result<SendValue, TaskError>`.
pub trait TaskRunner: Send + Sync {
    fn run(
        &self,
        job: TaskJob,
        env: &SendValue,          // snapshot read-only từ `spawn_shared`
        ctx: &mut WorkerCtx,      // out/dirty/cancel/budget của task
    ) -> Result<SendValue, TaskError>;
}
```

- `TaskRunner` do `vis-interp` định nghĩa; `Interp` cài bản HIR, `vis-vm` cài bản Module.
  `vis-vm` đã phụ thuộc `vis-interp` nên không vòng.
- Mọi trường `Send + Sync`; `&'static Code` là `Sync`; `Span` là `Copy`.
- Không cần kiểu `TaskRun` riêng: `WorkerCtx` đã có `print`/`mark_dirty`, `TaskOutcome` đã có
  `output`/`dirty`; `worker_loop` chỉ cần thêm `error: Option<TaskError>` vào outcome.

### 4.2 `ExecState` — API nội bộ

| Hàm | Việc |
|---|---|
| `spawn(host, than, args, ten, span) -> R<Value>` | Off: `sched.spawn`; Local: `sched.spawn` + proxy; Parallel: `sched.tao` + `to_send` args + `worker_snapshot` + `pool.spawn_shared` + proxy/handle. |
| `await_task(host, t, span) -> R<Value>` | Off/Local: đường cũ; Parallel: `outcome_board.wait(id)` (non-frame) hoặc `Pending` (frame); flush output; cache `kq`; map `TaskError`/`CrossWrite`. |
| `join(host) -> R<()>` | await mọi task theo `order`; flush phần chưa flush; check CrossWrite lần cuối. |
| `pump(sched, block) -> usize` | (P4) rút outcome đã xong, ghi `kq`, `frame_task_done`, trả số frame được đánh thức. |
| `cancel_for_test(id)` | set `AtomicBool` qua handle. |
| `final_join(sched) -> Option<Trap>` | (P4) chờ mọi job, flush, trả trap đầu tiên theo order. |

---

## 5. Luồng

### 5.1 `spawn` (interp)

```
HExpr::Spawn:
  eval args (như cũ)
  rt::exec::spawn(self, f.sym, args, f.name, span)
```

`exec::spawn`:
1. `Off` → `self.sched.borrow_mut().spawn(...)` (nguyên văn).
2. `Local` → `Scheduler::spawn` (nguyên văn) + `proxy.insert(t.id, TaskProxy{state:Local})`.
   Không snapshot, không buffer, không thread.
3. `Parallel`:
   - `let args_send = args.iter().map(Value::to_send)` — lỗi ⇒ `Trap(VS0508, span)` **trước khi**
     tạo task/job;
   - `let env = host.worker_snapshot()?` (payload `Arc<SendValue>` đã đóng gói, COW);
   - `let t = sched.tao(than, Vec::new(), ten, span)` (không đẩy ready; **args rỗng** vì thân
     không chạy trên main — tiết kiệm bộ nhớ; giữ `than/ten/span` cho `Show`/thông điệp);
   - `let h = pool.spawn_shared(env, move |ctx, env| runner.run(job, env, ctx))`;
   - `proxy.insert(t.id, {pool_id:h.id(), state:Running})`; lưu `h` để cancel;
   - trả `Value::Task(t)`.
   - Nếu vì lỗi nội bộ mà task Parallel không có proxy, `await_task` phải trap rõ
     (`VS0501 "task worker thiếu proxy"`), **không** chạy thân với args rỗng.
4. Nếu pool chưa tạo: tạo **lười** ở spawn đầu (N thread, stack 16 MiB, channel 64/job) — không
   tạo thread cho chương trình không spawn.

VM: `$spawn` trong `builtin.rs` gọi cùng `rt::exec::spawn(host, fi, con, ten, span)`; phần còn
lại giống hệt vì `Host` che engine.

### 5.2 `await` non-frame (P1/P2)

```
await_task(host, t, span):
  if proxy(t.id).mode == Parallel:
      match t.trang_thai:
        Xong => trả cache (kể cả Err)            // await lần hai
        DangChay/Huy => như cũ
        _ =>
          if host.sched().frame.enabled:
              return Err(Abort::Pending{task: Some(t), span, wait: None})  // P4
          let outcome = board.wait_timeout(pool_id, 50ms) loop:
              Timeout => tiếp tục (không có deadline sản phẩm)
              Closed  => Trap(VS0501 "worker pool đóng bất ngờ")
              Ok(o)   => o
          host.out().push_str(&o.output)        // flush tại điểm tiêu thụ
          check dirty: nếu !empty => Trap(CrossWrite)   // W>1
          set t.kq = convert(o.value); t.trang_thai = Xong
          trả value / wrap VS0506 nếu TaskError
  else: đường cũ (Moi ⇒ goi_than, ...)
```

- Không giữ `Mutex` khi chạy thân; board chỉ chứa dữ liệu `Send`.
- Lấy `Arc<OutcomeBoard>` (và `pool_id`) ra khỏi `Scheduler` **trước** khi `wait_timeout`;
  không giữ `RefCell` borrow của `Scheduler` khi block — nếu không, mọi truy cập scheduler
  khác (kể cả `frame_task_done`/pump) sẽ panic `already borrowed`.
- `wait_timeout` lặp để không treo cứng nếu board bị đóng; **không** có timeout sản phẩm (task
  chạy lâu vẫn chờ), chỉ dùng timeout nội bộ 50 ms để kiểm tra trạng thái/close.
- Task CPU không tự dừng: budget/cancel hợp tác; ghi rõ “không preempt” (D8).

### 5.3 `await` frame (P4)

1. `ApplyAwait` gọi `await_task`; task Parallel chưa xong ⇒ `Err(Abort::Pending{task: Some(t)})`
   (đúng đường hiện có, `frame/proc.rs:28-33`).
2. Driver `Pending` ⇒ `frame_await(current, t)` trả `Wait::Task(t.id)`; đăng ký; main treo.
3. Mỗi turn driver gọi `sched.exec_pump(block=false)`:
   - rút outcome không chờ; với mỗi id: ghi `t.kq`, `trang_thai=Xong`,
     `frame_task_done(id, kq.clone())` (wake waiter), flush output? **Không** — chỉ flush khi
     frame tiêu thụ (ApplyAwait retry) để giữ thứ tự.
4. Nếu `frame_next()==None` và `exec.in_flight()>0`:
   - `exec_pump(block=true)` (chờ 1 outcome, timeout ngắn để còn xử lý reactor);
   - lặp; **không** break, **không** `frame_stuck_trap` khi còn in-flight.
5. Resume: `frame_next` trả id main; `ApplyAwait` chạy lại; `await_task` thấy `Xong` ⇒ cache;
   flush output tại đây; push value; main chạy tiếp.
6. `frame_final_join`: trước khi quét `kq`, gọi `exec.final_join` (chờ mọi job, flush output
   chưa flush theo spawn order, trả trap CrossWrite/lỗi đầu tiên). Sau đó logic cũ.
7. **Bắt buộc** sửa fallback `frame.rs:1022-1046` và `vm_frame.rs:762`: chỉ đẩy task `Moi`
   **không có proxy** vào `frame.ready`; task Parallel đang chạy **không** được dựng frame trên
   main (nếu không sẽ chạy thân lần hai).
8. `frame_stuck_trap` thêm điều kiện “không còn in-flight” trước khi trap.

### 5.4 `join` cuối chương trình

- Non-frame: `join_all` giữ nguyên vòng lặp; `await_task` lo phần Parallel; task không ai await
  vẫn được chờ và flush theo order.
- Frame: `frame_final_join` gọi `exec.final_join` trước (P4).
- Lỗi task không ai await: `join_outcomes` trả lỗi **đầu tiên theo thứ tự tạo**, bọc `VS0506`
  như `frame_final_join` hiện tại (`sched.rs:485-503`), exit 1.

---

## 6. Hook tối thiểu và ước lượng dòng

> R1: `interp.rs` 1.423, `interp/goi.rs` **1.479**, `vm.rs` 1.484, `frame.rs` **1.494**,
> `value.rs` **1.500**, `rt.rs` 1.246, `rt/sched.rs` 1.004. Mọi hook phải đặt trong module mới;
> không thêm quá ~10 dòng vào `goi.rs`/`vm.rs`/`frame.rs`/`value.rs`; `frame.rs` phải **giảm**
> dòng bằng cách chuyển khối fallback sang `frame/worker.rs`.

### 6.1 Bước 0 — chuyển `worker.rs` ra khỏi `value.rs` (bắt buộc trước mọi dòng khác)

| Tệp | Việc | Dòng |
|---|---|---|
| `value.rs:1499-1500` | bỏ `include!("rt/worker.rs")` + `pub use worker::TaskId;`; thay bằng `pub use crate::rt::worker;` + `pub use crate::rt::worker::TaskId;` (giữ đường dẫn test cũ) | −2/+2, vẫn đúng 1.500 (R1 cho phép `≤1500`) |
| `rt.rs` | thêm `pub mod worker;` (cạnh `mod sched;`) | +1 |
| `rt/worker.rs` | không đổi nội dung; chỉ đổi vị trí | 0 |

Gate riêng cho bước 0 (không đổi hành vi), rồi mới P1.

### 6.2 P1 — khung `ExecState` + W=1 (interp, non-frame)

| Tệp / hàm | Việc | Ước dòng |
|---|---|---|
| `rt/exec.rs` (mới) | `WorkerMode`, `ExecState`, `TaskProxy`, `TaskRunner`, `TaskJob/TaskError`, `spawn/await/join` dispatch, `to_send` args, `worker_snapshot` (Arc payload), map lỗi | 220–300 |
| `rt.rs:465` (`Scheduler`) | thêm `pub(crate) exec: ExecState` (`Default`; **`Debug` viết tay** vì `WorkerPool` không `Debug`) | +8 |
| `rt.rs:1143` (`await_task`) | 5–8 dòng dispatch đầu hàm | +8 |
| `rt.rs:1239` (`join_all`) | nếu `exec.on` ⇒ `exec.join(host)`; ngược lại nguyên văn | +5 |
| `interp.rs:1109` | thay `self.sched...spawn` bằng `rt::exec::spawn(self, ...)` (1 call) | +2/−3 |
| `interp.rs` struct/`new` | thêm `top_syms: Vec<u32>` + khởi tạo từ `Hir.stmts` | +12 |
| `host.rs` | `impl Host::worker_snapshot` cho `Interp`; `TaskOutput` dùng lại | +20–30 |
| `interp/worker.rs` (mới) | `call_task(sym, args, env)` — dựng env từ snapshot, **không** chạy init top-level, trả value | +30–40 |
| `builtin.rs:664` (`$spawn`) | gọi `exec::spawn` thay `sched.spawn` | +2/−3 |
| `lib.rs` | `run_workers_*` ẩn (dựng `HirImage`, cài mode, chạy non-frame) | +40–60 |
| test `vs3_worker_eval.rs` | W=1 = default, surface API, lỗi cấu hình | 200–300 |

W=1 **không cần** đổi `worker.rs` (không dùng pool). Tổng P1: **~250–350 production**.

### 6.3 P2 — W>1 interp (pool + runner HIR)

| Tệp / hàm | Việc | Ước dòng |
|---|---|---|
| `rt/worker.rs` | `TaskError`; đổi `TaskFn` trả `Result<SendValue, TaskError>` (spawn cũ bọc String thành `TaskError(RT, span 0)`); `TaskOutcome.error: Option<TaskError>`; `Clone`; `WorkerCtx::{snapshot,cancel_flag,budget_handle,mark_dirty}`; **budget lease lazy**; `Value::into_send` (move, cho args); `spawn_eval`/`spawn_shared_eval`; `OutcomeBoard` (publish/try_take/wait/close/in_flight/done) gắn pool; panic `catch_unwind(AssertUnwindSafe(..))`; stack 16 MiB; `join_outcomes`; đăng ký cancel; cờ `stopping` cho Drop/trap | +190–250 |
| `rt/exec.rs` | `Parallel`: tạo pool lười, submit job, chờ outcome, flush, CrossWrite, final join, map lỗi | +100–150 |
| `interp/worker.rs` (mới) | runner HIR: `Interp::new_for_task(image, env)`, charge lease, cancel tick, `out`; helper cho hook dirty; map `Ret→Ok`, `Trap→TaskError`, `Pending→VS0501` (như `await_task`) | +80–120 |
| `frame/core.rs:129` (`Interp::charge`) | nhánh `budget_lease` (worker) trước thread-local; giữ nguyên default | +10–15 |
| `interp.rs` | hook dirty/cancel: `Var`, `Assign` (Var/Field/Index), tick mỗi 1.024 bước | +15–20 |
| `host.rs` | chuyển `TaskError` → `Trap`/`VS0506`; `TaskOutput` | +20 |
| `rt.rs` | `Scheduler::exec_pump` (cho P4 dùng lại), `exec.cancel` | +30 |
| test `vs3_worker_eval.rs` | W=4: output, cross, budget, cancel, nested, payload, 10k | +400–500 |
| bench `examples/vs3_eval_bench.rs` | CPU/payload matrix, W1/W4 | 150–250 |

Tổng P2: **~430–590 production**.

### 6.4 P3 — VM parity

| Tệp / hàm | Việc | Ước dòng |
|---|---|---|
| `vm_worker.rs` (mới) | `ModuleImage`; runner VM; `init_globals` + snapshot; dirty `Assign/StoreLocal(slot>=GLOBAL_SLOT_BASE)`; charge lease; cancel tick; `goi_than` không init top-level; map `Ret→Ok`/`Trap→TaskError` | 220–320 |
| `vm.rs` | 1 dòng field `worker_budget: Option<BudgetLease>` + 1 dòng gọi `worker_tick` (cancel/budget mỗi 1.024 bước) + `impl Host::worker_snapshot` (gọi hàm trong `vm_worker.rs`); `goi_than` giữ nguyên | +8–12 |
| `lib.rs` (vis-vm) | `run_workers_*` ẩn, dựng `Arc<Module>` | +20–30 |
| test `vs3_worker_eval_vm.rs` | parity interp/VM: output, result, lỗi, cross, budget, cancel | 250–350 |

Tổng P3: **~250–350 production**.

### 6.5 P4 — frame suspend/resume (interp + VM)

| Tệp / hàm | Việc | Ước dòng |
|---|---|---|
| `rt/sched.rs` | `exec_pump(block)` (gọi board, `frame_task_done`, wake); `in_flight`; hook cancel frame → `exec.cancel` | +40–60 |
| `frame/expr.rs:816` (`Callee::Spawn`) | `it.worker_snapshot_from_env(&frame.cur().env)` + `exec::spawn_with_snapshot`; nếu `it.sched.frame.current != 0` ⇒ local | +8–12 |
| `rt/exec.rs` | `spawn_with_snapshot`/`snapshot_from_env` (dùng env frame, không dùng `it.env`) | +20–30 |
| `frame/worker.rs` (mới) | `next_with_pump(it, need_join)`: thay **toàn bộ** khối fallback `Moi` 1022–1046, lọc task có proxy, gọi pump/block khi `next=None` + in-flight | 80–120 |
| `frame.rs:1022-1046` | thay khối bằng **một** call `worker::next_with_pump(...)` ⇒ **giảm** dòng (bù chỗ cho hook khác) | −20/+2 |
| `rt/sched.rs:485` (`frame_final_join`) | gọi `exec.final_join` trước khi quét `kq` | +10–15 |
| `rt/sched.rs:380` (`frame_stuck_trap`) | bỏ qua khi `exec.in_flight() > 0` | +3 |
| `vm_frame.rs:701-764` | mirror 3 hook trên (dùng chung `exec_pump`) — còn ~500 dòng headroom | +30–45 |
| test `vs3_worker_frame.rs` | main suspend khi await, resume đúng, task treo không bị chạy lần hai, final join, deadlock không báo oan | 200–300 |

Tổng P4: **~165–250 production** (frame.rs **không tăng**; phần lớn nằm trong module mới +
`sched.rs`/`vm_frame.rs`/`frame/expr.rs` còn headroom). Đây là pha **rủi ro cao nhất** dù ít dòng.

### 6.6 P5 — opt-in CLI + soak + tài liệu

| Việc | Ước dòng |
|---|---|
| `VIS_WORKERS` (hoặc flag) + kiểm giá trị + fallback/err rõ; chỉ chọn `run_workers_*` khi bật | 60–100 |
| Cảnh báo/kiểm chương trình dùng `chan(0)`/host khi W>1 (nếu CLI chọn non-frame) | 20–40 |
| Soak 10 phút, 100 vòng 10k, đo fd/RSS; cập nhật `VS3-TIEN-DO`, JOURNAL | 0 code + log |

Tổng P5: **~80–140 production**.

**Tổng toàn bộ**: ~1.000–1.700 dòng production, ~1.000–1.400 dòng test/bench.

---

## 7. Mã lỗi, thông điệp, exit

| Tình huống | Mã / hành vi | Exit |
|---|---|---|
| Đối số/const/var cấp tệp không Sendable **tại spawn** | `VS0508` (main-side), message nêu tên biến/giá trị | 1 |
| Kết quả task không Sendable | task trả `TaskError{code: VS0508}`; main bọc `VS0506` giữ mã gốc | 1 |
| Thân task trap (bất kỳ mã) | `VS0506` giữ mã gốc + span lúc tạo (như hiện tại) | 1 |
| Ghi top-level var ở W>1 | P2: `VS0501` + message `ghi chéo worker: biến … bởi tác vụ …`; **chờ A0 cấp `VS0509`** (Q1); main env **không đổi** | 1 |
| Budget cạn | `TaskError` mã `VS0501-BUDGET`, message chứa `VS0501-BUDGET`; bọc `VS0506` | 1 |
| Task bị huỷ trước kết quả | `VS0506` (“tác vụ bị huỷ”) | 1 |
| Worker panic | bắt `catch_unwind`; `TaskError` `VS0501` “worker panic: …”; task khác vẫn chạy | 1 |
| Pool đóng bất ngờ (`ChannelClosed`) | `VS0501` “worker pool đóng bất ngờ” (không phải lỗi thân task) | 1 |
| `InvalidWorkers` (0/>64) | lỗi cấu hình API/CLI, **không** phải trap chương trình | 2 |
| Hết frame + còn worker in-flight | **không** trap; pump/block tới khi có outcome (P4) | — |
| Hết frame + worker không bao giờ xong | không preempt; budget/cancel là hợp tác; ghi nợ | — |

- Không thêm mã mới trong P1/P2 (theo B1). Nếu A0 cấp `VS0509`, thêm vào
  `vis_diag::RUNTIME_CODES` + snapshot 3 ngôn ngữ + `explain` (một lát riêng, không trộn P2).
- `WorkerError` nội bộ **không** lộ ra chương trình; `exec` map sang `Trap`/`R<Value>`.

---

## 8. Pha, LOC, thời lượng, rủi ro

| Pha | Nội dung | LOC prod (ước) | Thời lượng | Rủi ro | Điều kiện xong |
|---|---|---|---|---|---|
| **P0** | tài liệu này | 0 | 1,5 h | — | commit, không code |
| **P1** | move worker.rs + `ExecState` + W=1 + API ẩn | 250–350 | 2–3 ngày | **thấp** | W=1 byte-identical non-frame default; gate; R1 |
| **P2** | W>1 interp non-frame + pool outcome + cross/budget/cancel + 10k | 430–590 | 3–5 ngày | **cao** | bench CPU ≥2,4×; output/result parity; 10k không tràn/RSS phẳng |
| **P3** | VM parity | 250–350 | 2–3 ngày | **trung bình** | interp/VM cùng stdout/exit/diag cho ma trận test |
| **P4** | frame suspend/resume | 165–250 | 2–3 ngày | **rất cao** | main treo/đánh thức đúng; không chạy task lần hai; không deadlock oan |
| **P5** | opt-in CLI + soak + docs | 80–140 | 1–2 ngày | trung bình | gate + Windows + soak; A0 duyệt semantics |

**MVP nên merge trước**: P1+P2 (interp, non-frame) — mở khoá “spawn/await thật trên worker cho
task CPU”, không đụng frame/VM. P3 là parity bắt buộc trước khi coi là tính năng; P4 chỉ cần khi
muốn main tiếp tục I/O trong lúc worker chạy.

---

## 9. Kế hoạch test

### 9.1 Nguyên tắc

- **Không assert thời gian/thứ tự hoàn thành**; chỉ assert stdout/exit/kết quả/tập id.
- Mọi test W>1 chạy **≥20 lần** (hoặc 3 lần ở CI nhanh) để bắt race; so byte-for-byte với W=1.
- Test không dùng `sleep` làm đồng bộ (trừ test cancel có vòng kiểm tra); dùng barrier/kênh nếu cần.
- Mọi test chạy cả `--test-threads=1` và mặc định (song song) để bắt rò trạng thái toàn cục.
- Task CPU trong test phải có việc thật (vòng lặp/tính toán) — chống “xanh giả” như hiện tại.

### 9.2 Ma trận ca (tối thiểu)

| # | Ca | Interp | VM | Kỳ vọng |
|---|---|---|---|---|
| 1 | `spawn` + `await` trả giá trị | ✓ | ✓ | đúng giá trị; `Show` task = `<tác vụ>`; await lần hai = cache |
| 2 | `await` chính task đang chạy / await không phải Task | ✓ | ✓ | `VS0501` (giữ nguyên) |
| 3 | Print W=1 vs W=4, 8/64/1000 task | ✓ | ✓ | **byte-identical**; cũng bằng non-frame default |
| 4 | Await lệch thứ tự + print | ✓ | ✓ | flush theo thứ tự await; bằng default non-frame |
| 5 | Main in xen giữa spawn/await | ✓ | ✓ | thứ tự main/task đúng như default |
| 6 | Task không ai await (final join) | ✓ | ✓ | chạy, output theo spawn order, lỗi surfaced `VS0506` |
| 7 | Budget cạn ở worker | ✓ | ✓ | lỗi có `VS0501-BUDGET`, exit 1, `used+remaining==limit` sau join, không treo |
| 8 | Cancel trước/giữa khi chạy (test-hook) | ✓ | ✓ | `cancelled`, output đã in được flush, `VS0506`, join không treo |
| 9 | Cross-write W=1 vs W=4 | ✓ | ✓ | W=1 cộng dồn/merge; W=4 `CrossWrite` (P2 dùng `VS0501`), env main không đổi, exit 1 |
| 10 | Snapshot tại spawn (main đổi sau spawn) | ✓ | ✓ | W=1 thấy giá trị mới; W=4 thấy giá trị lúc spawn |
| 11 | Nested spawn + await trong task | ✓ | ✓ | đúng kết quả; output gộp vào task cha; cross từ nested báo task cha |
| 12 | Nested sâu >256 | ✓ | ✓ | `VS0501` bọc `VS0506`, không panic/tràn stack |
| 13 | Payload map1k/list10k (16 task) | ✓ | ✓ | kết quả đúng, output identical; đo thời gian |
| 14 | Payload qua const/top-level (snapshot Arc) | ✓ | ✓ | main **không** copy per-spawn; worker tự dựng env cục bộ; snapshot read-only |
| 15 | 10k task | ✓ | ưu tiên interp | đủ kết quả, print đủ, RSS/fd phẳng, không tràn stack; so single-thread |
| 16 | Non-sendable args (closure/chan/cap/Task) | ✓ | ✓ | `VS0508` tại spawn |
| 17 | Non-sendable return | ✓ | ✓ | `VS0508` bọc `VS0506` |
| 18 | Task gọi fs/net/db/exec/cap khi W>1 | ✓ | ✓ | `VS0604` (không host) bọc `VS0506`, không treo |
| 19 | Worker panic (test-hook panic) | ✓ | ✓ | `VS0501` của task, worker khác vẫn chạy, join không treo |
| 20 | Chương trình sync (không async) bật worker | ✓ | ✓ | output/exit y hệt default; không tạo thread nếu không spawn |
| 21 | Frame: main await worker rồi in tiếp (P4) | ✓ | ✓ | main resume đúng, output đúng thứ tự, không chạy task lần hai |
| 22 | Frame: 2 frame cùng chờ 2 worker | ✓ | ✓ | cả hai wake, không deadlock oan, final join đúng |
| 23 | Frame: worker xong trước khi driver tới await | ✓ | ✓ | cache, không `Pending` thừa |
| 24 | `vis test` TAP với test fn async dùng worker | ✓ | — | join cuối khối, `ok/not ok` đúng; không rò task giữa ca |
| 25 | 100 vòng × 10k spawn (soak ngắn) | ✓ | — | fd không tăng, RSS phẳng, output tổng ổn định |
| 26 | Main trap khi còn 10k job chưa chạy | ✓ | — | exit 1 **<1 s**, không chạy nốt hàng đợi, không in output task sau trap, không treo |

### 9.3 Lệnh

```bash
# nhanh, trong lúc phát triển
cargo test -p vis-cli --test vs3_worker_eval -- --test-threads=1
cargo test -p vis-cli --test vs3_worker_eval_vm -- --test-threads=1
cargo test -p vis-interp --lib

# cổng bắt buộc trước merge (P1/P2 tối thiểu)
./vis-dev check
./vis-dev gate --fast
./vis test test/sem            # 525/525 như baseline, cả interp/VM
CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w5 cargo test -p vis-cli --release \
  --test vs3_worker --test vs3_worker_eval --test vs3_worker_eval_vm

# nightly/tuỳ chọn
./vis-dev nightly --fuzz-minutes 2
```

- So sánh golden: chạy cùng bộ `test/sem` ở non-frame default và worker-mode W=1; stdout phải trùng.
- Cache: `vis run --vm` warm/cold trước/sau khi thêm hook — stdout/exit trùng; không đổi key.
- VERSION: `VBC1` corpus byte-identical; `vis build`/`--emit` (nếu có) không đổi.

---

## 10. Hiệu năng — kỳ vọng và cách đo

### 10.1 Số đã có (từ W5, chỉ tham chiếu)

- CPU-only 64 task, ~200 µs/task: W=4 **2,69×** (PASS ngưỡng 2,4×; máy hybrid 2P+8E).
- Payload: `list10k` producer 1,93× / worker-copy 3,42×; `map1k` producer 0,77× /
  worker-copy 3,12×; `spawn_shared` map1k 5,62 ms vs producer 24,63 ms; list10k 5,25 vs 16,19.
- Gate B7: CPU-only + payload matrix; load/topology phải ghi kèm; RSS chỉ tham khảo.

### 10.2 Rủi ro hiệu năng của tích hợp evaluator

| Nguồn | Vì sao | Cách đo / giảm |
|---|---|---|
| Dựng `Interp`/`Vm` **mỗi task** | `Interp::new` quét HIR + `collect_sym_tys` + `MethodRet`; VM `Vm::new` + `init_globals`; 10k task có thể nghẽn | Đo riêng “empty task”; nếu >10–15% thời gian, P2b: **tái dùng evaluator theo worker** với `reset_for_task` (env/out/sched/steps/closure_cells), test cách ly 2 task |
| Snapshot env mỗi spawn | `Arc` COW phía main; worker vẫn dựng `Value` (copy) | Đo với 10k task, 4 biến; nếu lớn, snapshot theo **tập biến hàm dùng** (phân tích tĩnh) + tái dùng env/worker |
| `to_send` đối số | map1k/list10k clone | Move-convert (tiêu thụ `Value` thay vì clone) ở spawn; đo 2 biến thể |
| Buffer output 10k task | giữ đến await/join | Đo RSS; nếu cao, flush sớm hơn hoặc bounded board (backpressure) |
| Job channel đầy (64) | spawn 10k bị chặn | Chấp nhận backpressure; đo wall time spawn |
| Thread stack 16 MiB × N | RSS ảo | Chỉ commit khi cần; đo `VmRSS` + `VmHWM` |

### 10.3 Benchmark đề xuất (example, không phải test)

- `examples/vs3_eval_bench.rs`: cùng 64 task, CPU ~200 µs/task, charge 100/task, chunk 256;
  payload unit/list1k/list10k/map1k/record/enum/str1MiB; W=1/W=4; median 3–5 lần; in load,
  calibration, `speedup`, `used/remaining/limit`, `print_bytes`, `fd`, RSS.
- So sánh 3 đường: default single-thread non-frame, W=1 worker (phải **bằng** default), W=4.
- 10k task: đo wall time + RSS + fd; không đặt ngưỡng cứng nếu máy hybrid/load cao, nhưng phải
  **không** chậm hơn single-thread quá 1,2× cho task rỗng (nếu chậm hơn ⇒ P2b bắt buộc).

---

## 11. Rủi ro và blocker

| # | Rủi ro | Mức | Giảm thiểu |
|---|---|---|---|
| R1 | `Hir`/`Module` không `Send+Sync` do một `Rc` ẩn | Cao | test `assert_send_sync` ở P1; nếu vướng, phương án B: worker tự parse/lower từ `Arc<str>` (đắt, chậm) hoặc hoãn W>1 |
| R2 | Dựng evaluator mỗi task quá đắt (10k) | Cao | P2b tái dùng theo worker + reset; đo trước khi tối ưu |
| R3 | Frame: task worker bị driver chạy lần hai / final join bỏ main đang treo | Rất cao | 3 hook P4 (skip `Moi` có proxy, pump khi `next=None`, final join chờ); test 21–23 |
| R4 | In lệch thứ tự so default | Cao | D6 flush tại await; test 3–5; A0 chốt Q2 |
| R5 | Ghi chéo không bị chặn (env worker ghi vào snapshot?) | Cao | env worker là bản sao `Value`; dirty hook **mọi đường ghi** (Var/Assign/Field/Index); test 9–10 |
| R6 | Deadlock do task chờ task khác worker | Trung bình | handle không Sendable ⇒ không thể chờ chéo; nested chỉ cục bộ; test 11–12 |
| R7 | Cancel không tới safe point | Trung bình | kiểm mỗi 1.024 bước + charge; test vòng lặp dài; ghi rõ không preempt |
| R8 | Budget rò chunk/`used+remaining` sai | Trung bình | lease drop refund; test 7; chunk 256; không reset |
| R9 | R1 vượt 1.500 (value.rs/vm.rs/frame.rs) | Cao | bước 0 move worker.rs; module mới; giữ hook ≤10 dòng |
| R10 | Windows khác timing/stack | Trung bình | test không assert thời gian; stack 16 MiB; Tier 2 trên psvn trước merge P2/P4 |
| R11 | CLI opt-in làm mất `chan(0)`/async net | Trung bình | P5 chỉ chọn worker khi chương trình “thuần”; nếu không, fallback default + cảnh báo; A0 chốt Q3/Q5 |
| R12 | ABI/cache đổi ngoài ý | Cao | không đổi `Value`/`Task`/`VERSION`; test `VBC1` corpus + cache warm/cold; review diff |
| R13 | Thread panic làm mất worker | Trung bình | `catch_unwind`; test 19; `ChannelClosed` map rõ |
| R14 | Ảnh chương trình `Arc<Hir>` nhân bản HIR gây RAM | Thấp | một bản/lần chạy; chỉ bật khi opt-in |
| R15 | Main trap nhưng pool chạy nốt hàng đợi/treo lúc thoát | Trung bình | cờ `stopping` + cancel-all trong `Drop`; test 26; giữ channel ≤64/job |

**Blocker thật sự**: (a) A0 chưa cấp mã CrossWrite (B1); (b) chưa có quyết định “worker mode
được phép bỏ frame/chan(0)/host hay không”; (c) `Send+Sync` của ảnh chương trình chưa có test
trong repo. (a)/(b) là quyết định, (c) là việc P1.

---

## 12. Điều kiện merge

### 12.1 Bắt buộc cho mọi pha

1. `./vis-dev check` PASS: fmt, clippy `-D warnings`, R1–R4, self-test, build, doc-examples.
2. `./vis-dev gate --fast` PASS; số test **không giảm**, 0 fail; ghi số thực tế vào JOURNAL.
3. Không đổi `Value`/`Task`/`VERSION`/cache key/`Show`; có test chứng minh.
4. R1: mọi tệp ≤1.500 dòng; R2: không `unsafe`; không dependency mới.
5. `cargo fmt --all -- --check`; `cargo clippy -p vis-cli --all-targets -- -D warnings`.
6. Test mới chạy ổn định ≥20 lần (W>1); `--test-threads=1` và mặc định.
7. `./vis test test/sem` 525/525 (hoặc số baseline hiện hành) cả interp/VM.
8. Không sửa `crates/vis-net`/`pkg`/`async_net*` trong P1/P2; P4 chỉ `frame/**`, `rt/sched.rs`,
   `vm_frame.rs`.

### 12.2 Riêng P2 (W>1)

- Benchmark CPU-only ≥2,4× (median, ghi topology/load); payload matrix báo cáo, không nhất thiết
  pass mọi payload.
- 10k task: không tràn stack, RSS phẳng qua 100 vòng, fd không tăng; wall time không tệ hơn
  single-thread quá 1,2× với task rỗng (nếu tệ hơn ⇒ P2b).
- Cross-write/cancel/budget/panic: đủ 4 nhóm test âm.
- **Windows Tier 2** (không GitHub Actions): `cargo test` trên psvn/Windows target, 0 fail;
  nếu chưa có host Windows, chỉ merge P1 (không thread) và giữ P2 trên nhánh.

### 12.3 Riêng P4 (frame)

- Test 21–23 + 2 ca deadlock giả; `frame_stuck_trap` không báo oan khi còn in-flight.
- Không chạy thân task lần hai: đếm số lần side-effect (in/biến đếm) = 1.
- Final join không bỏ main đang treo; exit/value đúng.
- Windows Tier 2 + soak 10 phút (async h2/chan hiện có không hồi quy).

### 12.4 Quy trình

- `vs3/w5-spike` chỉ commit; **không push/merge/tag**; A0/W1 review rồi merge.
- Mỗi pha một commit `[VS3][W5] …` + JOURNAL cập nhật `VS3-TIEN-DO.md` dòng VS3-3b.
- Trước merge: `df` >5G; dọn target cũ nếu cần; không chạy GitHub Actions.

---

## 13. Câu hỏi cho A0 (kèm khuyến nghị)

| # | Câu hỏi | Lựa chọn | Khuyến nghị |
|---|---|---|---|
| Q1 | Mã cho ghi chéo worker | (a) `VS0501` + message rõ ở P2; (b) cấp `VS0509` riêng (thêm bảng diag + snapshot 3 ngôn ngữ + `explain`) | **(a) cho P2**, (b) sau khi nghiệm thu; đúng B1 “chưa cấp mã mới” |
| Q2 | Thứ tự in khi await lệch | (a) flush theo **thứ tự tiêu thụ** (bằng default non-frame); (b) prefix-barrier để tuyệt đối spawn order (đổi ngữ nghĩa await) | **(a)**; ghi rõ “spawn order ở final join” |
| Q3 | Tác vụ worker được dùng host/chan/cap? | (a) thuần only, vi phạm `VS0508`/`VS0604`; (b) host riêng từng worker (net reactor riêng — đắt); (c) không hạn chế (sai) | **(a)** cho P2/P3; (b) là lát riêng nếu có nhu cầu |
| Q4 | Frame trong cùng gate? | (a) non-frame trước, P4 sau; (b) làm frame ngay | **(a)**; P4 rủi ro cao, không cần cho gate CPU |
| Q5 | Bề mặt opt-in | (a) API ẩn test-only; (b) `VIS_WORKERS`; (c) flag CLI | **(a) trước**, (b) sau nghiệm thu; không đổi default |
| Q6 | Evaluator mỗi task hay mỗi worker | (a) mỗi task (đúng, đơn giản); (b) mỗi worker + reset (nhanh, rủi ro rò trạng thái) | **(a) P2**, (b) chỉ khi benchmark 10k bắt buộc |
| Q7 | In của task lỗi/cancel | (a) giữ output đã in, flush trước trap; (b) bỏ output | **(a)** đúng B3 |

---

## 14. Kiểm chứng trong phiên này

- Chỉ đọc mã + viết tài liệu; **không** `cargo build/test`, không tạo target mới (tiết kiệm CPU/đĩa).
- `git merge --ff-only main` trên `vs3/w5-spike` → `f75b246`; không push/merge/tag.
- `df /home`: 12G trống (>5G). Không đụng `target/` (19G hiện có).
- Commit: xem `git log -1` (tệp này nằm trong cùng commit); không push/merge/tag.

---

## 15. Tham chiếu

| Tài liệu / mã | Dùng cho |
|---|---|
| `agent/DECISIONS.md` VD-160, VD-178, VD-271 (B1–B7) | ràng buộc Send/heap/snapshot/opt-in |
| `agent/VS3-TIEN-DO.md` §3b | chốt A0 cho VS3-3b |
| `agent/JOURNAL/vs3-w5-spike.md` | số đo biên copy/move, lựa chọn (a) affine |
| `agent/JOURNAL/vs3-w5-worker-pool.md` | trạng thái pool + hook W1 còn thiếu |
| `agent/JOURNAL/vs3-w5-spawn-shared.md` | `spawn_shared`, số đo payload |
| `crates/vis-interp/src/rt.rs:37-73,465-600,1143-1245` | Task/Scheduler/await/join |
| `crates/vis-interp/src/rt/worker.rs:60-100,395-541,704-1014` | SendValue/Error/Board/Pool |
| `crates/vis-interp/src/frame.rs:767-1100`, `rt/sched.rs:54-503` | frame driver, wait/final join/stuck |
| `crates/vis-vm/src/vm.rs:133-184,511-582`, `vm_frame.rs:388-437,701-764` | VM task/handle/frame |
| `crates/vis-interp/src/interp.rs:1058-1110`, `builtin.rs:639-671` | spawn/await hai engine |
| `crates/vis-interp/src/frame/params.rs:13-93` | env cấp tệp/tham số/default |
| `crates/vis-bytecode/src/format.rs:114-552` | Module/Instr/global slot |
| `bin/vis-dev:200-260` | R1/R2 |

---

## Phụ lục A — pseudocode `await_task` (sau tích hợp)

```rust
pub fn await_task(host: &mut dyn Host, t: &Rc<Task>, span: Span) -> R<Value> {
    // Copy thông tin proxy ra trước (không giữ Ref của Scheduler khi chờ).
    let probe = host.sched().borrow().exec.probe(t.id);
    if let Some(p) = probe {
        if p.parallel {
            return host.sched().borrow_mut().exec.await_parallel(host, t, span, p.pool_id);
        }
    }
    // ---- nguyên văn đường cũ ----
    match t.trang_thai.get() { ... }
}

fn await_parallel(host, t, span, id: TaskId) -> R<Value> {
    match t.trang_thai.get() {
        Xong => return cache(t, span),
        DangChay => return trap(RT, "await chính tác vụ đang chạy", span),
        Huy => return cache_or_cancel(t, span),
        _ => {}
    }
    if host.sched().borrow().frame.enabled {
        return Err(Abort::Pending(Pending { task: Some(t.clone()), span, wait: None }));
    }
    let outcome = loop {
        match board.wait_timeout(id, Duration::from_millis(50)) {
            Ok(o) => break o,
            Err(Timeout) => continue,          // không deadline sản phẩm
            Err(Closed) => return trap(RT, "worker pool đóng bất ngờ", span),
        }
    };
    host.out().push_str(&outcome.output);      // flush tại điểm tiêu thụ
    if !outcome.dirty.is_empty() {
        return cross_write_trap(outcome.dirty, t, span);
    }
    let value = match outcome.value {
        Ok(v) => v.to_value(),
        Err(e) => {
            let goc = Trap::new(e.code, e.msg, e.span);
            let wrapped = wrap_task_error(&t.ten, goc, t.span);
            *t.kq.borrow_mut() = Some(Err(wrapped.clone()));
            t.trang_thai.set(Xong);
            return Err(Abort::Trap(wrapped));
        }
    };
    *t.kq.borrow_mut() = Some(Ok(value.clone()));
    t.trang_thai.set(Xong);
    Ok(value)
}
```

## Phụ lục B — pseudocode pump frame (P4)

```rust
// trong run_frame_driver, sau khi có `next`:
let woke = it.sched.borrow_mut().exec_pump(false);
let mut next = it.sched.borrow_mut().frame_next();
if next.is_none() && it.sched.borrow().exec.in_flight() > 0 {
    let _ = it.sched.borrow_mut().exec_pump(true);   // chờ 1 outcome
    next = it.sched.borrow_mut().frame_next();
}
// fallback Moi: lọc `!exec.has_proxy(t.id)` (task worker không dựng frame)
// frame_stuck_trap: if exec.in_flight() > 0 { return None; }
// frame_final_join: exec.final_join() trước khi quét kq
```
