<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — điều tra overhead P2b (10k task cực nhỏ)

**Trạng thái**: **EXAMPLE/TEST-ONLY** — không sửa production/ABI/default.
**Nhánh**: `vs3/w5-spike`, đã `git merge main` → **`c962715`**.
**Mã đo**: `crates/vis-cli/examples/vs3_worker_eval_p2b_overhead.rs` (**401 dòng**).
**Log**: `agent/JOURNAL/vs3-w5-p2b-overhead.log` (3 lần N=10000; lần cuối có batch/sched).
**Máy**: i5-1235U, 12 CPU; load 4,3–7,1 (ghi trong log); release, reps=3, median.
**Không** GitHub Actions; không push/merge/tag.

---

## 1. Baseline 10k `spawn`/`await` (interp P2 / VM P3)

Nguồn `SPAWN_10K`: 10k task `t(n)=n+1`, main `spawn` + `await` **tuần tự** từng task.

| W | interp | VM | interp task/s | VM task/s |
|---:|---:|---:|---:|---:|
| 1 | **18,2–21,5 ms** | **14,3–17,1 ms** | ~525–550k | ~586–700k |
| 2 | **164,7–171,5 ms** | **147,4–157,2 ms** | ~58k | ~64k |
| 4 | **172,2–176,8 ms** | **150,2–160,9 ms** | ~57k | ~62–66k |

- W=2/4 chậm **~8–9×** W=1 cho task cực nhỏ. Đây là **latency spawn→await→wake**,
  không phải evaluator/pool throughput.
- W=1 task ~1,8–2,1 µs; W=4 task ~15–17,6 µs; W=2 không cải thiện W=4 (round-trip là chính).

### Batch (spawn 10k, không await từng task, final join một lần)

| W | interp | VM |
|---:|---:|---:|
| 1 | 11,5–14,4 ms | 12,2–12,3 ms |
| 4 | **44,9–47,1 ms** | **44,2–46,4 ms** |

- Batch W=4 nhanh hơn sequential W=4 **~3,5–3,8×**, nhưng vẫn **~4× chậm hơn** W=1 batch.
- Tức: nếu chương trình spawn nhiều rồi await, P2/P3 đã dùng final join batch; phần còn lại là
  overhead **tạo Task/proxy + gửi pool + runner + outcome**.

---

## 2. Tách chi phí (lần N=10000, load 4,34)

| Thành phần | Số đo | Ghi chú |
|---|---:|---|
| `Scheduler::tao` 10k | **1,0 ms** (~0,10 µs/task) | tạo `Task`/ctor, không chạy |
| `OutcomeBoard` publish + try_take 10k | **6,4 ms** (~0,64 µs/task) | board + clone outcome |
| Pool thuần W=4, không evaluator | **20,4 ms** (~2,0 µs/task) | spawn/queue/join |
| Pool W=4 + outcome board | **22,7 ms** (+**2,3 ms**) | clone outcome/board |
| Pool W=4 + `ctx.print` 10k | **18,5 ms** | in không phải nút cổ chai |
| Pool W=4 + `spawn_shared(Arc)` | **24,6 ms** | +~4 ms vs `spawn` |
| Pool W=4 + `Vm::new`+run mỗi task | **15,9 ms** | evaluator VM mới/task |
| Pool W=4 + `vis_interp::run` (Interp mới) mỗi task | **17,2 ms** | evaluator interp mới/task |
| `WorkerPool::new(4)` | **0,1 µs** | lazy pool không đáng kể |
| `Vm::new` | **0,23 µs/task** | constructor |
| VM mới + run (program rỗng) | **0,93 µs/task** | ~0,70 µs run |
| VM **tái dùng** cùng thread | **0,69 µs/task** | **1,27–1,34×** so với mới |
| `vis_interp::run` (Interp mới + rỗng) | **1,27–1,39 µs/task** | constructor + run tối thiểu |

**Suy ra** (xấp xỉ, trừ direct pool):

- Pool/evaluator/scheduler/board: **~20–25 ms/10k**.
- P2/P3 sequential W=4 **~150–177 ms/10k** ⇒ **~130–150 ms** là **latency round-trip
  spawn→worker→board→await**, ~13–15 µs/task. Đây là phần lớn nhất.
- P2/P3 batch W=4 **~45–47 ms/10k** ⇒ ~25–30 ms trên direct pool/eval: chủ yếu Task/proxy/
  snapshot/runner/outcome (không còn round-trip từng await).
- Outcome clone/board chỉ **~2,3 ms/10k** (~1–2%); print **không đáng kể**; lazy pool đã có.

---

## 3. Các biến thể đã thử trong example

| Biến thể | Kết quả | Khả thi production? |
|---|---|---|
| **Batch final join** (spawn 10k rồi join) | 44–47 ms W=4 vs 150–177 ms sequential | **Có** — đã có cho task không await; API batch rõ hơn cần P2b-1 |
| **Pool thuần** (không evaluator) | 20,4 ms W=4 | Đã có; là sàn overhead |
| **Pool + evaluator mới/task** | 15,9–17,2 ms | Đã có |
| **Outcome board** vs không board | +2,3 ms; publish+try 6,4 ms | Bỏ board cho non-frame nếu có completion slot |
| **`spawn_shared`** | 24,6 ms vs 20,4 ms (`spawn`) | P2 dùng; +~4 ms/10k |
| **Print 10k** | 18,5 vs 20,4 ms (nhiễu) | Không cần tối ưu |
| **VM tái dùng thread-local** | standalone 1,27–1,34×; **pool reuse 0,82–0,98×** | **Không đáng** ở dạng hiện tại |
| **Interp tái dùng** | **Không thể** trong example: `Interp::new` `pub(crate)`, không có `reset_for_task` | Cần API nội bộ; xem P2b-4 |
| **Lazy pool** | tạo pool 0,1 µs | Đã có; không còn gain |
| **Compact proxy / Vec index** | chưa thử được vì proxy private | Đề xuất P2b-2 |
| **Per-task completion slot** | chưa thử được vì runner private | Đề xuất P2b-3 |

- VM reuse: `Vm::run` reset globals/steps/stdout nhưng **không reset Scheduler**; chỉ thử với
  program rỗng không spawn/chan. Pool reuse còn chậm hơn do thread-local/RefCell/contention
  ở task cực nhỏ ⇒ **không phải hướng chính**.
- Interp reuse: `Interp::new` private + borrow HIR; muốn reuse phải đổi ownership/`reset_for_task`
  — không thể làm sạch chỉ trong example.

---

## 4. Đề xuất P2b cụ thể

### P2b-1 (ưu tiên cao) — lazy/batched job submit + batch await

- **Vấn đề**: `spawn_parallel` gửi job ngay; sequential await trả giá round-trip; batch vẫn
  tạo/gửi 10k job lẻ.
- **Đề xuất**: `ExecState` giữ `pending_jobs: Vec<Job>`; `spawn_parallel` chỉ tạo `Task`+proxy+
  snapshot (không gửi pool); `await_parallel`/final join gọi `flush_jobs()` một lần.
  Với batch, 10k job gửi một loạt; với sequential, flush 1 job trước await (không đổi semantics).
- **Signature**:
  ```rust
  // rt/exec.rs
  struct PendingJob { task_id: u32, than: u32, args: Vec<SendValue>,
                      snap: Arc<SendValue>, span: Span }
  pub(crate) fn spawn_parallel_lazy(...) -> R<Value>;
  pub(crate) fn flush_jobs(host: &mut dyn Host) -> R<()>;
  ```
- **File**: `rt/exec.rs` +80–120; `rt/worker.rs` `spawn_shared_batch`/`spawn_many` +40–60.
- **Lợi ích kỳ vọng**: batch W=4 45→~25–30 ms (gần direct pool 20 ms); sequential giữ nguyên
  hoặc giảm nhẹ. **Risk**: thứ tự start task thay đổi, pool channel đầy, `Task` state Moi lâu;
  giữ `spawn` semantics “chưa chạy tới await” (đúng hiện tại).

### P2b-2 (ưu tiên trung bình) — compact proxy/index

- `proxy: HashMap<u32, TaskProxy>` + `pool_to_task: HashMap<TaskId,u32>` → `Vec<Option<TaskProxy>>`
  (Task.id tuần tự) + `Vec<TaskId>`; bỏ hash/alloc mỗi task.
- `rt/exec.rs` +60–90, `vm/worker.rs` +60–90; `TaskProxy` output/dirty/done.
- **Lợi ích**: ~0,5–1,5 µs/task trong batch (10k ~5–15 ms); **risk** thấp, cần test id/reclaim.

### P2b-3 (ưu tiên trung bình) — per-task completion slot, bỏ clone outcome

- Worker publish thẳng `TaskOutcome` vào slot `Arc<TaskSlot>` của task; await đọc slot; final
  join một completion channel (không clone outcome sang cả board + channel).
- `rt/worker.rs` +60–100 (`TaskSlot`, `spawn_slot`), `rt/exec.rs` +40–60.
- **Lợi ích**: board delta 2,3 ms/10k + publish/try 6,4 ms (~1–2%); giảm lock toàn cục.
- **Risk**: ownership/reclaim, output/dirty; phải giữ non-frame parity.

### P2b-4 (thấp) — snapshot COW + evaluator construction

- Snapshot top-level COW/generation (`ExecState.snapshot: Arc`, invalidate khi main ghi top-level):
  +60–90 dòng `rt/exec.rs`/`interp.rs`/`vm.rs`; chỉ có lợi khi top-level payload lớn, **không**
  giải quyết 10k task rỗng.
- Interp constructor 1,3 µs/task; cache `InterpTemplate`/precompute maps ~+60–100 chỉ giảm
  ~1 µs/task; **không ưu tiên** so với round-trip.
- VM reuse: standalone 1,27–1,34× nhưng pool 0,82–0,98×; **không làm** cho đến khi có reset
  per-worker và đo lại.

### P2b-5 — giữ W=1 cho sequential tiny

- 10k task tuần tự là **latency-bound**; W>1 không thể vượt W=1 nếu mỗi await phải chờ worker.
  Giữ khuyến nghị W=1 cho shape này; W>1 cho task CPU đủ lớn hoặc spawn-all-then-await.
- Không hứa “10k task nhỏ nhanh hơn”; P2b chỉ hứa **batch W=4 ≤1.5× W1** và không regression
  sequential.

---

## 5. Điều kiện benchmark / ngưỡng chấp nhận

- Release, N=10k, reps=5 (median + p95), load ghi kèm, warm-up 1 lần; đo cả **sequential await**
  và **spawn-all final join**; interp + VM; W=1/2/4.
- Không assert wall trong gate; benchmark tay/soak như P2/P3.
- **Ngưỡng P2b-1**:
  - batch W=4 ≤ **1,5×** W=1 batch (hiện **~4×**; mục tiêu ≤22 ms khi W1 ~12 ms);
  - sequential W=4 **không tệ hơn** hiện tại (mục tiêu ≤2× nếu làm được, nhưng không hứa);
  - task/s batch tăng ≥ **1,8×** so baseline 45 ms.
- **Ngưỡng giữ**: P1/P2/P3 8/14/10, gate 3571+, 20 vòng, RSS/fd phẳng, no default/ABI.
- Windows P2b nếu chạm pool: debug+release 3528+.

---

## 6. Kết luận

- **Nút cổ chai chính** của 10k task cực nhỏ là **round-trip spawn→await→wake**, không phải
  `Vm::new`/`Interp::new`, outcome clone, print hay lazy pool.
- **Batch final join** đã nhanh hơn sequential 3,5–3,8×; vẫn còn ~25–30 ms overhead Task/proxy/
  runner so direct pool ~20 ms.
- **P2b nên làm P2b-1 + P2b-2**, cân nhắc P2b-3; **không** ưu tiên evaluator reuse (VM lợi ít,
  Interp không có API), **không** hứa W>1 cho sequential tiny.
- Không có blocker code cho điều tra; P2b implementation mới là phần cần A0 duyệt (thay đổi
  `rt/exec.rs`/`rt/worker.rs`, hidden API, không default).
- `vm.rs`/`frame.rs` vẫn ở trần P4; P2b không đụng hai tệp này ở pha đầu.
