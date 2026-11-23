<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — P4 frame suspend/resume: kế hoạch thiết kế

**Trạng thái**: **DESIGN-ONLY** — không sửa production/test/default trong phiên này.
**Nhánh**: `vs3/w5-spike`, đã `git merge main` → **`3f89401`** (tag **`v0.3.4`** @ `8fa1f96`).
**Mốc hiện hành**: evaluator P1/P2/P3 opt-in, gate Linux **3571/3**; P3 VM parity + soak 2h.
**Không** push/merge/tag; không bật default; không đổi ABI/`VERSION`/cache key.
**Timebox**: 2 h. **Kết luận ngắn**: **P4a khả thi** (main frame await worker task, pump/final
join); **P4 đầy đủ** (chan(0)/host/async I/O trong frame W>1) **chưa nên gộp một lát** — cần
A0 chốt chính sách và refactor R1 trước.

---

## 1. Hiện trạng (đọc mã, không suy đoán)

| Thành phần | Vị trí | Hành vi |
|---|---|---|
| Frame driver interp | `frame.rs:767` | vòng turn; `frames: HashMap<u32,IFrame>`; `frame_next` chọn frame; `step_frame` chạy heap work-stack |
| Frame driver VM | `vm_frame.rs:539` | cùng cấu trúc với `Th`/`Vm` |
| `frame_await` | `rt/sched.rs:54` | task chưa `Xong` ⇒ `Waiter{Task(id)}`; đã `Xong` ⇒ `None` (cache) |
| `frame_next` | `rt/sched.rs:93` | FIFO ready + event/reactor; không biết worker pool |
| `frame_task_done` | `rt/sched.rs:444` | ghi cache, `joined`, wake mọi `Wait::Task(id)` |
| `frame_final_join` | `rt/sched.rs:485` | chỉ quét `Task.kq` đã có, **không chờ** task chưa xong |
| `frame_stuck_trap` | `rt/sched.rs:380` | hết frame + còn task/wait/reactor ⇒ `VS0505` |
| Fallback driver | `frame.rs:1021-1046` | khi `frame_next=None` đẩy **mọi task `Moi`** (trừ external) vào ready |
| Final driver | `frame.rs:1121-1128` | `frame_final_join` rồi `frame_stuck_trap` |
| Await interp | `rt/exec.rs:207` | `await_parallel` **block** board tới khi outcome; chưa biết frame |
| Join interp | `rt/exec.rs:293` | `pool.join`, flush task chưa await; chưa có pump/frame hook |
| Spawn frame | `frame/expr.rs:816-824` | `Callee::Spawn` gọi thẳng `Scheduler::spawn` — **không** qua P2 `exec::spawn_parallel` |
| VM await/join | `vm/worker.rs:202/273` | cùng vấn đề: block, chưa frame-aware |
| Worker task state | P2/P3 | `Scheduler::tao` + proxy; **không có frame** để `step_frame`; state `Moi` tới khi pool xong |
| `vm.rs` | 1.500 dòng | **không còn headroom R1** |
| `frame.rs` | 1.494 dòng | chỉ còn 6 dòng headroom |

**Hệ quả**: nếu bật frame + W>1 hiện tại:
1. fallback thấy task worker `Moi` ⇒ dựng `IFrame` bằng `build_task_frame` ⇒ **chạy thân lần hai
   trên main** (task cũng đang chạy trong pool) — sai/race;
2. `await` task worker trong frame gọi `await_parallel` ⇒ **block main**, phá suspend/resume;
3. `frame_final_join` không chờ pool; `frame_stuck_trap` báo deadlock oan khi pool còn in-flight;
4. spawn trong frame (`frame/expr.rs:816`) không đẩy pool ⇒ W>1 frame không song song.

Vì vậy P4 phải là **một cơ chế pump + frame-aware await**, không chỉ sửa `frame_final_join`.

---

## 2. Phạm vi và ma trận chế độ

| Chế độ | Frame? | Ai chạy task | P4 |
|---|---|---|---|
| `Off` default | sync/non-frame | main scheduler | **không đổi** |
| W=1 `Local` | không frame | caller | **không đổi** (P1) |
| W=2/4 `Parallel` | **không frame** | pool | **không đổi** (P2/P3) |
| W=2/4 `Parallel` | **frame** (`run_async_*`) | main driver + pool CPU task | **P4 mới** |
| W=1 `Local` | frame | main driver/caller | **baseline**, phải byte-identical |

**Chính sách P4a**: chỉ frame **main** (`frame.current == 0`) được đẩy `spawn` lên pool.
Spawn phát sinh trong frame task (current != 0) chạy **local trong driver** (như task frame cũ),
tránh worker task await worker task và giảm rủi ro thứ tự in. P4b mới mở rộng nếu cần.

---

## 3. Thiết kế P4a — suspend/resume main frame

### 3.1 Luồng `await` task worker trong frame

```
step_frame(frame main) gặp Work::ApplyAwait
  -> await_task(host, t, span)
     -> mode Parallel, t chưa Xong, frame.enabled
        -> KHÔNG chờ board
        -> Err(Abort::Pending { task: Some(t), span, wait: None })
  -> driver thấy Pending:
        t của current frame = Pending
        w = frame_await(current=0, t)  // Waiter::Task(t.id)
        frame_register(w)              // frame 0 ra khỏi ready
  -> vòng sau: pump worker outcome
        exec::pump(host, block=false/true)
          - lấy TaskOutcome (không flush output)
          - set proxy.output/dirty, t.kq, t.trang_thai = Xong
          - frame_task_done(t.id, kq) -> wake frame 0
  -> driver chạy frame 0; ApplyAwait retry:
        await_task(host,t,span)
          - thấy t Xong + proxy chưa flush
          - flush proxy.output vào host.out()
          - dirty ⇒ Trap VS0501; cancelled ⇒ VS0506
          - trả cache kết quả
```

- `exec::await_parallel` phải có nhánh **frame-aware**; nếu `frame.enabled` và task chưa xong thì
  trả `Pending` **trước** khi chạm `OutcomeBoard`.
- Khi resume, `await_parallel` phải đọc **cache/proxy**, không chờ board lần hai (outcome đã bị
  pump lấy). Thứ tự kiểm: `t.trang_thai`/`t.kq` trước, rồi proxy output/dirty.
- `frame_await` đã trả `Wait::Task(id)` đúng; không cần `Wait` mới.

### 3.2 `TaskProxy` mở rộng

```rust
pub(crate) struct TaskProxy {
    pub(crate) task: Rc<Task>,
    pub(crate) pool_id: TaskId,
    pub(crate) handle: TaskHandle,
    pub(crate) done: bool,          // outcome đã được pump lấy
    pub(crate) flushed: bool,       // output đã append vào host.out
    pub(crate) output: String,      // giữ tới await/final join
    pub(crate) dirty: Vec<String>,  // cross-write chờ báo
}
```

- Pump **không** append output; chỉ lưu. Output được flush:
  - tại `await` (resume) — thứ tự tiêu thụ;
  - tại final join — thứ tự spawn cho task không ai await.
- `TaskOutcome.cancelled`/`dirty` được chuyển thành trap đúng lúc flush/resume.

### 3.3 Pump và board

Thêm vào `rt/worker.rs` (module `value::worker`, R1 hiện 1.123 dòng):

```rust
impl OutcomeBoard {
    /// Chờ một trong `ids`; trả (TaskId, TaskOutcome) hoặc Timeout/Closed.
    pub fn wait_any_timeout(
        &self,
        ids: &[TaskId],
        timeout: Duration,
    ) -> Result<(TaskId, TaskOutcome), WaitError>;
}
```

`rt/exec.rs` (376 dòng hiện tại):

```rust
pub(crate) struct PumpInfo {
    pub woken: usize,
    pub in_flight: bool,
    pub closed: bool,
}

/// Rút mọi outcome đã sẵn sàng; block=false ⇒ không ngủ; block=true ⇒ chờ tối đa ~50 ms
/// rồi trả để driver còn fire reactor/timer.
pub(crate) fn pump(host: &mut dyn Host, block: bool) -> R<PumpInfo>;
pub(crate) fn in_flight(s: &Scheduler) -> bool;
pub(crate) fn is_worker_task(s: &Scheduler, id: u32) -> bool;
pub(crate) fn cancel_worker(s: &Scheduler, id: u32); // handle.cancel()
pub(crate) fn final_join(host: &mut dyn Host) -> R<()>;
```

`pump`:
1. lấy `board`/`pool_id` ra khỏi `Scheduler` **trước** khi chờ (không giữ `RefCell` borrow);
2. `wait_any_timeout`/`try_take` cho các proxy chưa `done`;
3. với mỗi outcome: set `proxy.done/output/dirty`; nếu task chưa `Huy`, set `t.kq` + `Xong`;
   gọi `frame_task_done(id, kq)`; **không** flush output;
4. trả `PumpInfo`; `block=false` dùng 0 ms, `block=true` 50 ms (đủ để driver quay lại reactor).

### 3.4 Hook `Host` cho driver dùng chung interp/VM

`frame.rs`/`vm_frame.rs` không được gọi trực tiếp `Scheduler.exec` (interp) hoặc `Vm.worker` (VM).
Thêm method **default no-op** vào `Host` (không phá implementor ngoài cây):

```rust
fn worker_is_task(&self, _id: u32) -> bool { false }
fn worker_in_flight(&self) -> bool { false }
fn worker_pump(&mut self, _block: bool) -> R<usize> { Ok(0) }
fn worker_final_join(&mut self) -> R<()> { Ok(()) }
fn worker_cancel(&mut self, _id: u32) {}
```

- Interp override đọc `Scheduler.exec`; VM override đọc `WorkerState`.
- Default `false/Ok(0)` giữ nguyên hành vi Off/W1/P2/P3 non-frame.

### 3.5 Driver sửa tối thiểu

Trong `frame.rs` và `vm_frame.rs`:

1. **Fallback `Moi`** (`frame.rs:1028-1046`, `vm_frame.rs:764-782`):
   filter thêm `!it.worker_is_task(t.id)` (task worker không dựng `IFrame`/`Th`).
2. **Khi `frame_next=None` và `worker_in_flight()`**:
   gọi `it.worker_pump(true)`; nếu `woken>0` ⇒ `frame_next()` lại; nếu timeout ⇒ lặp (để
   `frame_next` fire event/reactor/timer), không `break`.
3. **`frame_stuck_trap`**: chỉ gọi khi `!it.worker_in_flight()`; nếu còn in-flight thì pump tiếp.
4. **Final** trước `frame_final_join` (`frame.rs:1121`, `vm_frame.rs:843`):
   `it.worker_final_join()?` — chờ pool, cache mọi task, flush output chưa flush **spawn order**,
   clear proxy/pool; sau đó `frame_final_join` quét cache và surface lỗi.
5. **Reclaim**: `frame_reclaim` chỉ thu hồi `Rc::strong_count==1`; proxy đang giữ `Rc<Task>` nên
   phải clear proxy **trước** `frame_final_join`/reclaim. `final_join` clear `proxy`,
   `pool_to_task`, `outcome_board`, `pool` sau khi cache.

Có thể tách khối fallback sang `frame/worker.rs` (~80–120 dòng) để `frame.rs` **giảm** dòng:
`next_after_idle(it, &frames, need_join) -> R<NextFrame>` với
`enum NextFrame { Run(u32), Pumped, Done }`.

### 3.6 Không chạy task hai lần

- Worker task có proxy ⇒ fallback **không** đưa id vào `frame.ready`.
- Chỉ frame thật (`task` local `tao` không proxy) mới được dựng `IFrame`/`Th`.
- Test bắt buộc: task in đúng **một** lần; `steps`/output đếm; log `purge` không chứa worker id.
- `frame.current` chỉ là frame id; worker task id không bao giờ là `current`.
- Nếu proxy thiếu (bất biến hỏng) ⇒ trap `VS0501` rõ, **không** fallback chạy thân.

### 3.7 Thứ tự in

- Worker task in vào buffer.
- `await` resume: append buffer của task **tại điểm tiêu thụ** (giống run-to-completion W=1).
- Không ai await: final join append theo **spawn order**.
- Main in trực tiếp; output task đã xong vẫn nằm buffer tới await/final ⇒ W=1/W=2/W=4
  byte-identical cho chương trình await đúng thứ tự (và cho final join).
- Nếu await lệch thứ tự, flush theo await; đó là ngữ nghĩa W=1 hiện tại, không phải spawn order
  tuyệt đối. Ghi rõ trong test/limit.
- Output task bị trap/cancel đã in **trước** khi trap vẫn flush (giống P2/P3).

### 3.8 Cancel

- Frame cancel request (`frame_cancel_process_for_test`, `rt/sched.rs:597`) hiện đặt task `Huy`,
  wake awaiter, không đóng handle. Với worker task:
  1. driver sau `frame_next` gọi `it.worker_cancel(id)` cho id trong `frame.cancelled` (idempotent)
     ⇒ `TaskHandle.cancel()`;
  2. worker outcome `cancelled` (hoặc task đã `Huy`) ⇒ `proxy.done`; **không** ghi đè `Huy`;
  3. resume `await` thấy `Huy`/`cancelled` ⇒ `VS0506` "tác vụ bị huỷ" (không cache kết quả mới);
  4. output đã in trước cancel vẫn flush nếu outcome đã có; nếu chưa có, final join flush.
- Cancel **sau khi có kết quả** ⇒ cache thắng (`await` lần hai trả cache).
- Không thêm mã mới; không preempt task đang chặn host/chan.
- P4a chỉ cần test cancel một task worker đang chờ; P4b mở rộng cancel nhiều task/nested.

### 3.9 `chan(0)`, host, async I/O

- Worker task không host/`chan(0)` (P2/P3): `chan(0)` `VS0505`, host `VS0604`, bọc `VS0506`.
- Frame main W=1 Local vẫn dùng `chan(0)`/host như cũ; P4 **không** làm mất.
- W>1 frame:
  - worker task không host/chan; task dùng các đường đó lỗi rõ khi await, không treo.
  - main frame dùng `chan(0)` với task worker **không có peer** ⇒ unsupported; phải trap/no-hang
    (pump + `frame_stuck_trap` sau khi pool xong) — test âm bắt buộc.
  - main frame host/reactor vẫn chạy trên main; pump `block=true` chỉ 50 ms, driver quay lại
    `frame_next` để fire timer/reactor, không bỏ đói event.
- Async I/O trong worker bị từ chối (`VS0604`); async I/O trong main frame dùng reactor như cũ.
- P4a **không** hứa `chan(0)` giữa main frame và worker task; P4b/A0 mới quyết định có wire
  channel qua pool hay không.

### 3.10 VM mirror

- `vm/worker.rs` thêm `pump`/`await_frame`/`final_join`/`in_flight`/`is_worker_task`/`cancel`
  tương tự `rt/exec.rs`; `TaskProxy` VM thêm output/dirty/done.
- `vm_frame.rs` driver dùng cùng `Host` hook; fallback/final/stuck như interp.
- VM `spawn_task` chỉ đẩy pool khi `vm.sched.frame.current == 0` (P4a policy); frame task spawn
  local.
- `vm.rs` **1.500 dòng**: P4 phải làm **R0 refactor** trước: chuyển `impl Host for Vm` (~80 dòng)
  sang `vm/host.rs`, giữ `impl Vm` tại chỗ; `vm.rs` còn headroom cho hook. Không đổi hành vi.
- `frame.rs` 1.494 dòng: chuyển fallback sang `frame/worker.rs` để **giảm** dòng.

---

## 4. Hook/file/signature ước lượng

| Tệp | Hook / signature | Ước dòng |
|---|---|---:|
| `rt/worker.rs` | `OutcomeBoard::wait_any_timeout(&[TaskId], Duration)`; publish/outcome đã có | +20–30 |
| `rt/exec.rs` | `PumpInfo`, `pump(host, block)`, `in_flight`, `is_worker_task`, `cancel_worker`, `final_join`; `TaskProxy` output/dirty/done; `await_parallel` frame-aware | +150–220 |
| `builtin.rs` | 5 method default `worker_is_task/in_flight/pump/final_join/cancel` | +18–25 |
| `host.rs` | override 5 method cho `Interp` | +20–30 |
| `frame/expr.rs` | `Callee::Spawn` rẽ nhánh `current==0 && Parallel` → `exec::spawn_parallel` | +5–8 |
| `frame.rs` | thay fallback bằng `frame::worker::next_after_idle`; final gọi `worker_final_join`; stuck guard | −20/+2 |
| `frame/worker.rs` (mới) | `next_after_idle`, `pump_if_idle`, filter `Moi`; ~80–120 | +80–120 |
| `rt/sched.rs` | (nếu cần) helper cancel idempotent; không đổi `frame_final_join` | +10–20 |
| `vm/worker.rs` | `PumpInfo`, pump/await/final/inflight/is_task/cancel; proxy output/dirty | +100–150 |
| `vm_frame.rs` | fallback filter/pump/final/stuck; +30–50 | +30–50 |
| `vm/host.rs` (mới) | **R0**: chuyển `impl Host for Vm` khỏi `vm.rs` | 80–90 (move) |
| `vm.rs` | R0 còn `mod host`/field; P4 hook | −80/+5 |
| test `vs3_worker_frame.rs` | interp: main await worker, 2 task, print, final, cancel, chan0/host, 10k, no double-run | +250–350 |
| test `vs3_worker_frame_vm.rs` | parity interp/VM W=1/2/4, same cases | +180–280 |

**Ước lượng**:
- **R0 refactor** (không đổi hành vi): 0 LOC net, 0,5–1 ngày; bắt buộc vì `vm.rs` 1.500,
  `frame.rs` 1.494.
- **P4a** (main frame await worker, pump, final, output, interp+VM): **250–350 production**
  interp + **120–180** VM + **400–600 test**; 2–3 ngày.
- **P4b** (cancel worker, chan0/host policy, nested/frame task): **80–150 production**,
  **150–250 test**; 1–2 ngày.
- **P4c** (frame log/golden, Windows, soak): **40–80 production**, **100–200 test**; 1 ngày.
- Tổng P4: **~500–750 production**, **~650–1.000 test**, **4–6 ngày** — lớn hơn P2/P3 vì
  đụng hai driver + scheduler + R1 refactor.

---

## 5. Rủi ro

| # | Rủi ro | Mức | Giảm |
|---|---|---|---|
| R1 | `vm.rs` 1.500, `frame.rs` 1.494 — không đủ dòng | Cao | R0 move `impl Host for Vm`, fallback `frame/worker.rs`; đo R1 trước P4 |
| R2 | Fallback chạy worker task lần hai | Rất cao | `worker_is_task` filter; test in một lần/log purge |
| R3 | Pump giữ `RefCell` borrow khi chờ ⇒ panic | Cao | lấy board/pool_id ra trước; không giữ borrow |
| R4 | Await resume chờ board lần hai (outcome đã mất) | Cao | resume đọc `t.kq`/proxy; board chỉ pump |
| R5 | Output lệch thứ tự W=1 | Cao | buffer/proxy; flush tại await/final spawn order; test print + final |
| R6 | `frame_stuck_trap` báo deadlock oan khi pool in-flight | Cao | `worker_in_flight` guard; pump timeout 50 ms; test timer+worker |
| R7 | `frame_reclaim` giữ task vì proxy `Rc` | Trung bình | clear proxy trước final/reclaim; snapshot `live_tasks` |
| R8 | Cancel worker không tới pool / ghi đè `Huy` | Cao | `worker_cancel` idempotent; pump không overwrite Huy; `VS0506` |
| R9 | Main trap khi pool còn 10k task, thoát chậm | Trung bình | `Drop` pool/stop; test main trap; đo <1 s |
| R10 | `chan(0)` main chờ worker treo | Cao | policy reject/không hứa; test VS0505/no-hang; P4b/A0 |
| R11 | Host/async I/O trong worker vs reactor main | Trung bình | worker không host; pump timeout để driver fire reactor |
| R12 | VM/interp khác log/step | Trung bình | test parity stdout/exit/codes/value; không so log/step |
| R13 | Windows timing/thread stack | Trung bình | test không assert wall; 20 vòng; Windows P4 debug/release |
| R14 | Đổi default/ABI ngoài ý | Cao | hidden API, Off/W1 giữ nguyên; gate P1/P2/P3 + VBC1/cache |

---

## 6. Test plan

### Interp `vs3_worker_frame.rs` (mới, ~250–350 dòng)

| # | Ca | Kỳ vọng |
|---|---|---|
| 1 | Main `await` một worker task worker CPU rồi in | suspend/resume; task chạy một lần; output W=1==W=4 |
| 2 | Main `await` 2 task, task 2 xong trước task 1 | wake đúng, không chạy lại, thứ tự in như W=1 |
| 3 | Main in trước/sau await | buffer flush đúng điểm tiêu thụ |
| 4 | Task không ai await, main kết thúc | final join chờ pool, output spawn order, lỗi surfaced |
| 5 | 100–10.000 task | không tràn stack, không treo, RSS/fd ổn |
| 6 | Task trap / result không Sendable | `VS0506`/`VS0508`, exit 1, không chạy lại |
| 7 | Cancel worker qua frame cancel | `VS0506` "huỷ", output trước cancel flush, join không treo |
| 8 | `chan(0)` main + worker task | `VS0505`/no-hang theo policy, không false exit 0 |
| 9 | Host/fs trong worker task | `VS0604`, main frame vẫn chạy |
| 10 | Reactor/fake timer + worker in-flight | timer vẫn fire; `frame_stuck_trap` không báo oan |
| 11 | W=1 Local frame (baseline) | output/exit/codes y hệt trước P4 |
| 12 | Task `spawn` trong frame task (current!=0) | P4a chạy local trong driver, không đẩy pool |

### VM `vs3_worker_frame_vm.rs` (mới, ~180–280 dòng)

- Cùng 1–10 cho VM; so interp/VM **stdout/exit/codes/value** cho W=1/2/4.
- VM `$spawn`/`$await` frame; host/chan0 reject; final join.
- `vm.rs` R0 refactor xanh trước P4a.

### Chạy

```bash
cargo test -p vis-cli --test vs3_worker_frame -- --test-threads=1
cargo test -p vis-cli --test vs3_worker_frame_vm -- --test-threads=1
cargo test -p vis-cli --test vs3_worker_eval_local --test vs3_worker_eval_parallel \
  --test vs3_worker_eval_vm -- --test-threads=1
./vis-dev gate --fast
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy -p vis-cli --all-targets -- -D warnings
```

- Mọi test W>1 chạy **20 vòng** (hoặc 5 vòng CI nhanh), cả `--test-threads=1` và mặc định.
- Test không assert wall; chỉ stdout/exit/codes/value/tập id.
- Windows: P4 debug + release, `cargo test --workspace --no-fail-fast`.

---

## 7. Điều kiện merge P4

1. `Off`/W=1 Local byte-identical; P1/P2/P3 8/14/10 xanh; gate **3571+** không fail.
2. Main frame await worker task thật; task worker **không** chạy lần hai; output W=1/2/4
   byte-identical cho ca await/final.
3. `frame_final_join` chờ pool, surface lỗi; `frame_stuck_trap` không false positive khi
   in-flight; main trap không treo/không chạy nốt 10k.
4. Cancel worker: `VS0506`, output flush, không ghi đè cache; `chan(0)`/host reject/no-hang.
5. 10k task: không tràn stack, RSS/fd phẳng 20 vòng; soak ngắn 10 phút W=4 frame.
6. R0 refactor đã xanh riêng (`vm.rs` <1.500, `frame.rs` <1.500); R1 mọi tệp; no `unsafe`.
7. Windows P4 debug+release 0 fail; không GitHub Actions.
8. Không đổi `Value`/`Task`/`VERSION`/cache key/`Show`; hidden API/test-only; không default.

---

## 8. Quyết định A0 / blocker

- **Chính sách W>1 frame**: P4a chỉ main frame offload; frame task spawn local. A0 cần chốt có
  mở rộng spawn trong task frame / nested worker không.
- **`chan(0)`/host/async trong frame W>1**: có wire channel/host qua pool không, hay từ chối
  như P2/P3? P4a đề xuất **từ chối/no-hang**, không hứa parity.
- **Mã cross-write**: giữ `VS0501` + message hay cấp `VS0509`; P4 dùng cùng P2/P3.
- **Cancel**: có cần API sản phẩm (`TaskHandle`) hay chỉ hook test? P4a test-only.
- **R1 refactor**: bắt buộc trước P4; nếu A0 không cho move `impl Host for Vm`/fallback
  `frame.rs`, P4 **không thể** thêm hook sạch — blocker kỹ thuật.
- **Hiệu năng 10k**: P2/P3 đã chậm <1×; P4 thêm pump/frame overhead; cần P2b trước khi hứa
  speedup. P4 không tuyên bố nhanh hơn.
- **P4 ước lượng**: P4a 2–3 ngày, P4b 1–2 ngày, P4c 1 ngày; không gộp vào một lát 2 giờ.

---

## 9. Kết luận khả thi

- **Khả thi**: main frame `await` worker task, suspend frame, pump outcome, resume, final join,
  output/cancel cơ bản — với R0 refactor và hook `Host` như trên.
- **Không khả thi sạch trong một patch nhỏ**: `chan(0)`/host/async I/O qua biên worker, nested
  worker spawn, full frame log parity, và R1 `vm.rs`/`frame.rs`. Cần P4a → P4b → P4c.
- **Đề xuất**: làm **R0 refactor + P4a test-only** trước; giữ `chan(0)`/host W>1 là **từ chối
  rõ**; chỉ mở P4b sau khi A0 chốt chính sách và P2b xử lý overhead 10k.
