<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b prototype (W5) — TaskId/proxy, snapshot Env, print merge, budget atomic

**Trạng thái**: **PROTOTYPE — KHÔNG PRODUCTION, KHÔNG PHẢI VS3-3b XONG.**
**Nhánh**: `vs3/w5-spike` · **Worktree**: `/home/ps/.local/share/opencode/worktree/cd45e5/vs3-w5-spike`
· **Baseline**: `2d6ca2b` (đã `git merge main`; không push/merge).
**Căn cứ**: `agent/DECISIONS.md` VD-271 (bổ sung) và `agent/VS3-TIEN-DO.md` §3b (A0 chốt mô
hình affine, `TaskId`/proxy, snapshot `var`, print merge, budget atomic).
**Phạm vi**: chỉ thêm `agent/JOURNAL/vs3-w5-spike/worker_proto.rs` + báo cáo; **không** sửa
`crates/**`, `Cargo.toml`, `frame/**`, không thêm crate/workspace, không đổi runtime/API.

---

## 0. Kết luận ngắn

Prototype **chứng minh được 4 điểm** ở mức mô hình dữ liệu/điều phối:

1. **TaskId/proxy**: `TaskId(u64)` + `Completion` qua channel; không `Rc<Task>`, không
   `Arc<Mutex<Task>>`; completion đến lệch thứ tự (đo được 0–4 lần đảo) nhưng main vẫn join
   đúng theo handle và merge theo thứ tự spawn.
2. **Env snapshot**: task thấy snapshot tại lúc spawn (`base=10`), main đổi sau spawn
   (`base=99`) không ảnh hưởng task; task ghi top-level `var` chỉ vào heap cục bộ. W=1 tuần tự
   được phép (final `shared_counter=8`); W=4 parallel **từ chối rõ** (`PROTO_CROSS_WRITE`),
   main không bị ghi chéo (`shared_counter=0`).
3. **Print merge**: buffer theo task, merge theo **thứ tự spawn**; W=1/W=2/W=4 cho **cùng 334
   byte** trên chương trình mẫu, dù completion đến lệch thứ tự.
4. **Budget atomic**: một `AtomicI64` CAS dùng chung; W=1/2/4 cùng dùng đúng 320.000 đơn vị,
   `used + remaining == limit`; cạn pool còn đúng 0, charge sau cạn vẫn `Err` (**không reset**).
   **Đo được chi phí tranh chấp**: ~12 ns/charge W=1 → ~57 ns/charge W=4 (local baseline
   ~0,4–0,9 ns) ⇒ cần cân nhắc **reserve từng khối** cho worker khi tích hợp W1.

**Chưa chứng minh**: suspend/resume, await lồng, channel data (`Chan`), cancellation, `Value`
thật, tích hợp W1, gate hiệu năng production. **Không tuyên bố worker pool xong.**

---

## 1. Bốn điểm và cách prototype chứng minh

### 1.1 TaskId/proxy completion (không chia sẻ heap)

| Thành phần | Mã | Ý nghĩa |
|---|---|---|
| `TaskId(u64)` | `worker_proto.rs:139-145` | Handle `Copy`; không phải con trỏ/`Rc`. |
| `Job { id, spec }` | `:170-173` | Worker nhận **bản move** của `TaskSpec`; không tham chiếu heap main. |
| `Completion { id, worker, result, out, dirty, charged, ... }` | `:176-188` | Worker gửi kết quả qua `mpsc::Sender<Completion>`; mọi trường Send. |
| `WorkerPool::spawn` | `:439-449` | Trả `TaskId`; đẩy job vào queue worker `(id-1)%W`. |
| `CompletionBoard::wait` | `:373-392` | Nhận completion theo `id`, buffer completion đến trước; không dùng `Rc<Task>`. |
| `WorkerHeap` | `:263-268` | Mỗi worker có `globals` clone riêng; heap không chia sẻ. |
| `assert_send::<...>` | `:990-994` | `Data`, `TaskSpec`, `Job`, `Completion`, `TaskId` đều `Send`. |

**Bằng chứng chạy**: `PROXY handles=8 arrived_out_of_order=0..4` (6 lần chạy); output vẫn
byte-identical. `arrival` là thứ tự completion thực; main merge theo `handles` (spawn order).

> Đây là **proxy**, không phải scheduler hoàn chỉnh: prototype chưa có suspend/resume; task chạy
> trọn trong một lần `execute_job`. Phần suspend/resume + await lồng vẫn thuộc W1/VS3-3b thật.

### 1.2 Per-worker Env snapshot & phát hiện ghi chéo

| Thành phần | Mã | Ý nghĩa |
|---|---|---|
| `RootEnv::snapshot` | `:94-96` | Deep clone top-level `var` tại **thời điểm spawn**. |
| `WorkerHeap.globals` | `:263-268` | Mỗi worker clone root lúc tạo pool; heap riêng. |
| `TaskEnv` | `:104-136` | Task nhận `EnvSnapshot`; `write_global_int` ghi **local** + `dirty`. |
| `TaskKind::Safe` | `:155-158` | Chỉ đọc snapshot; in `base`/`data_len`; không dirty. |
| `TaskKind::CrossWrite` | `:159-160` | Ghi top-level `var`; `dirty` được gửi trong completion. |
| `run_cross_write` | `:610-742` | W=1: tuần tự, áp dirty theo thứ tự spawn (mô phỏng hiện tại). W>1: nếu có dirty ⇒ `RunError::CrossWrite`, không merge output task, main counter giữ nguyên. |

**Kết quả**:
- Safe: task in `base=10`; main sau spawn đổi `base=99`; task không thấy 99; `main:base=99`.
- W=1 cross-write: task in `cw-counter=1..8`, main `shared_counter=8` (tuần tự).
- W=4 cross-write: cả 8 task nhận snapshot 0, ghi local thành 1; main phát hiện `dirty` của
  `TaskId(1..8)`, in `error: PROTO_CROSS_WRITE ...`, `main_counter=0`; **không** task output nào
  được merge vào output chương trình.

> Chính sách production nên từ chối **sớm hơn** (checker/spawn) thay vì chạy rồi mới phát hiện;
> prototype cố ý chạy để chứng minh “kể cả lọt vào worker thì cũng không ghi chéo được”.
> Mã `PROTO_CROSS_WRITE` chỉ là mã prototype, **không phải** mã sản phẩm; A0 phải cấp mã/thông điệp
> chính thức (không dùng `VS0508` — mã đó là “không Sendable”, nghĩa khác).

### 1.3 Print buffer theo task + merge tất định

- Mỗi `Completion` mang `out: String`; `execute_job` chỉ ghi vào buffer của task (`format!` +
  `push_str`), không ghi `Host.out` chung.
- `run_safe` merge: `main:start\n` + **task output theo thứ tự spawn** + `main:end\n` +
  `main:base=...` + `main:shared_counter=...` + `main:results=[...]`.
- W=1 tự nhiên đúng thứ tự; W=2/4 completion đến lệch (0–4 lần đảo) nhưng merge vẫn theo
  `handles`.
- **Đo**: output dài **334 byte**, `W=1 == W=2 == W=4` tuyệt đối (assert trong `main`).
- Output mẫu W=4 (rút gọn):
  ```
  main:start
  t0:base=10
  t0:data_len=1000
  ...
  t7:data_len=1000
  main:end
  main:base=99
  main:shared_counter=0
  main:results=[1000,1001,1002,1003,1004,1005,1006,1007]
  ```
- Chính sách merge: **spawn order**. A0 cần xác nhận có dùng await order hay không; nếu task
  trap giữa chừng, cần chốt có merge phần buffer đã in hay bỏ (prototype hiện bỏ output task khi
  cross-write bị từ chối; task trap thường chưa có ca).

### 1.4 Ngân sách atomic dùng chung

| Thành phần | Mã | Ý nghĩa |
|---|---|---|
| `SharedBudget` | `:190-241` | `limit: i64`, `remaining/used: AtomicI64`, `exhausted: AtomicBool`, `failed_charges: AtomicU64`. |
| `charge(weight)` | `:211-233` | CAS `remaining`; chỉ trừ khi đủ; `used.fetch_add`; không có setter/reset. |
| `snapshot()` | `:235-241` | Đọc cuối sau join; `used + remaining == limit`. |
| `exhaustion_probe` | `:834-875` | Cạn pool; gọi `charge` lần nữa phải `Err`; `remaining` vẫn 0. |
| `run_budget_shared/local` | `:744-832` | Đo shared atomic vs budget local (lower bound, **không** phải pool chung hợp lệ). |

**Kết quả (median 6 lần chạy; mỗi lần 3 reps nội bộ; 64 task × 5000 charge = 320.000):**

| Mode | W=1 | W=2 | W=4 |
|---|---:|---:|---:|
| Shared atomic — ns/charge | **12,0** | **42,8** | **57,2** |
| Shared atomic — ms | 3,83 | 13,68 | 18,32 |
| Local baseline — ns/charge | 0,9 | 0,6 | 0,4 |
| Local baseline — ms | 0,28 | 0,21 | 0,12 |

- `used=320.000`, `remaining=10.000`, `limit=330.000` ở mọi W; `used + remaining == limit`.
- **Cạn pool**: `limit=160.000`; `used=160.000`, `remaining=0`, `exhausted=true`; số **lượt charge
  thất bại** 33–35; gọi `charge(1)` sau đó vẫn `Err`, remaining vẫn 0 ⇒ **không reset**.
- Full workload safe dùng 64 × 200 = 12.800 charge; shared W=1/2/4 đều đúng, không cạn.
- **Phát hiện**: shared atomic tăng ~12 → 57 ns/charge khi W=1→4 (tranh chấp CAS), so với local
  ~0,4 ns. Với 10 triệu sự kiện ngân sách, chi phí phụ có thể ~0,1–0,6 s. Đề xuất W1:
  **reserve từng khối** (worker `fetch_sub` một block lớn, charge local, trả lại phần dư) —
  vẫn “một pool logic”, nhưng giảm tranh chấp; **chưa** làm trong prototype.

---

## 2. Đo W=1/2/4, thời gian, RSS

### 2.1 Thời gian workload mẫu

64 task, mỗi task CPU ~50k vòng, snapshot `data` 5.000 phần tử, 200 charge/task; median 3 reps
bên trong mỗi lần chạy, rồi median 6 lần chạy:

| W | `run_ms` | Speedup | `startup_ms` (tạo pool) |
|---:|---:|---:|---:|
| 1 | **6,38** | 1,00 | 0,15–0,55 |
| 2 | **4,17** | **1,53×** | 0,69–1,08 |
| 4 | **2,33** | **2,74×** | 1,56–2,31 |

- W=4 range 2,16–2,66 ms (tùy load); W=2 range 3,69–4,23 ms.
- Startup tăng theo W do tạo thread + clone `WorkerHeap`; workload ngắn (<10 ms) nên startup
  chiếm tỷ lệ đáng kể. Trong runtime thật, pool nên được tạo một lần, không mỗi lần chạy.
- Máy **i5-1235U (2P+8E)**, `nproc=12`; số **chỉ hướng**, không dùng làm gate production.

### 2.2 RSS (tiến trình con sạch)

Mỗi worker có heap pad 50.000 `Data::Int` (~1,6 MB/worker) để thấy chi phí heap theo luồng;
task snapshot 64 × 1.000 phần tử. Đo `VmRSS`/`VmHWM` sau khi chạy:

| W | RSS cuối | **HWM (đỉnh)** |
|---:|---:|---:|
| 1 | 3,5 MB | **6,0 MB** |
| 2 | 3,5 MB | **7,4 MB** |
| 4 | 3,6 MB | **10,4 MB** |

- HWM tăng ~4,4 MB từ W=1 → W=4, phù hợp 3 heap worker bổ sung + thread/queue; RSS cuối giảm vì
  worker pool đã drop trước khi đọc (allocator giữ/trả tùy).
- **Không** kết luận “RSS pool = W × heap” tuyến tính; nhưng đủ chứng minh **heap theo luồng
  là chi phí thật**, cần cap/measure ở 3b-4.
- Task snapshot là bản deep clone top-level `var`; nếu chương trình có `var` lớn, chi phí này
  nhân theo số task — đây là lý do cần snapshot/affinity có kiểm soát.

---

## 3. Kết luận về 4 điểm

| Điểm | Kết luận prototype | Còn mở |
|---|---|---|
| **TaskId/proxy** | Khả thi; handle `u64` + channel completion đủ để await đúng khi completion lệch thứ tự; không chia sẻ heap. | Suspend/resume, await lồng, cancellation, task handle trong `Value`. |
| **Env snapshot** | Khả thi; task chỉ thấy snapshot; ghi top-level `var` bị phát hiện qua `dirty`; W>1 từ chối rõ, main không bị ghi chéo. | Chốt mã/thông điệp; chốt snapshot cho W=1; kiểm tra/ từ chối sớm ở checker/spawn. |
| **Print merge** | Khả thi; buffer task + merge spawn order cho W=1/W=2/W=4 byte-identical, dù completion lệch thứ tự. | Chốt spawn order vs await order; policy output khi trap/cancel. |
| **Budget atomic** | Khả thi về accounting/no-reset; nhưng **tranh chấp CAS đắt** (12→57 ns/charge). | Chunk reservation; chính sách task nào trap khi cạn; tích hợp W1. |

---

## 4. Phần cần W1 tích hợp (sau VS3-2a)

> **Không** làm trong prototype này. `VS3-2a` (wait-set/`chan(0)`) phải ổn định trước vì nó sửa
> `rt.rs`/`builtin.rs`/scheduler — vùng W1 sẽ đụng khi làm worker pool.

| Vùng | Hiện trạng (main `2d6ca2b`) | Prototype | Việc W1 |
|---|---|---|---|
| `crates/vis-interp/src/rt.rs` | `Task` chứa `args`, `kq: RefCell`; `Scheduler` một luồng; `spawn`/`await_task`/final join dùng `Rc<Task>` | `TaskId` + `Job`/`Completion` + per-worker queue | Tách **handle ngoài** (`TaskId`) khỏi task nội bộ worker; proxy completion; final join theo id; giữ cache/`VS0506`. |
| `crates/vis-interp/src/value.rs` | `Value::Task(Rc<Task>)`; `Show` `<tác vụ>` | Không dùng `Rc` | Chốt shape `Value::Task` = handle `TaskId`/opaque; ảnh hưởng `Show`/`eq`/API; A0 duyệt. |
| `interp.rs` / `interp/goi.rs` | `spawn` eval args; `goi_than` chạy thân với `self.env` chung | `TaskSpec.snapshot` | Snapshot top-level `var` tại spawn; task env riêng; ghi top-level `var` theo policy; test fixture async. |
| `crates/vis-vm/src/vm.rs`, `vm_frame.rs` | `Th`/`Frame` heap; VM mượn `Scheduler` | Worker/VM riêng | Mỗi worker một `Vm`/`Scheduler`; `Frame` không di trú; snapshot/`TaskId` cùng hợp đồng interp. |
| `crates/vis-interp/src/frame/fee.rs` | `thread_local! BUDGET: RefCell<Budget>` | `AtomicI64` shared | Thay bằng budget handle `Arc<SharedBudget>`/`AtomicI64`; giữ default off/test-hook; **chunk reservation** để giảm tranh chấp; chốt hợp đồng VS3-1a (BUDGET/Q/default). |
| `crates/vis-interp/src/host.rs` | `out() -> &mut String` chung | Task `out: String` | Host/buffer theo task; merge ở final join; `print` trong task không ghi trực tiếp output chung. |
| `rt.rs` cancellation (VS3-1b) | `Huy`, safe point, deregister | Chưa có | Completion proxy phải xử lý task huỷ/lỗi; không treo; giữ `VS0506`/cache. |
| `Chan` (VS3-2a) | `Rc<RefCell<Chan>>`, `chan(0)` chưa wait-set | Chỉ completion channel | `Arc<Mutex<ChanBuf<Wire>>>` + `Condvar`; encode/decode; close/timeout/select — **sau** VS3-2a. |
| Runtime mặc định | single-thread | Prototype | Worker pool **opt-in/test-only**; default giữ single-thread tới nghiệm thu + VERSION/cache. |

**Thứ tự đề xuất**: VS3-2a xong → 3b-0 hợp đồng/test (TaskId, snapshot, print, budget atomic) →
3b-1 pool skeleton W=1 parity → 3b-2 cross-worker spawn/await + proxy → 3b-3 channel data → 3b-4
perf/RSS gate. Không gộp channel data vào 3b-1.

---

## 5. Blocker / quyết định cần A0

| # | Vấn đề | Đề xuất |
|---|---|---|
| **B1** | Mã/thông điệp cho **ghi chéo top-level `var`** trong parallel mode | Prototype dùng `PROTO_CROSS_WRITE`; A0 cấp mã chính thức (không dùng `VS0508` — nghĩa khác). Nên từ chối ở checker/spawn khi biết. |
| **B2** | Snapshot top-level `var` cho **W=1** có đổi ngữ nghĩa hiện tại? | Prototype dùng snapshot cho safe W=1; cross-write W=1 mô phỏng tuần tự. Cần A0 chốt: W=1 cũng snapshot hay giữ shared env; test toàn bộ fixture async. |
| **B3** | Thứ tự merge print: spawn order hay await order? Output task khi trap/cancel? | Prototype chọn **spawn order**, bỏ output task khi cross-write bị từ chối. A0 chốt; thêm ca trap/cancel. |
| **B4** | Budget atomic: tranh chấp 12→57 ns/charge; chính sách cạn pool (task nào trap?) | Đề xuất **chunk reservation** (fetch block lớn, charge local, trả dư) để vẫn một pool logic; chốt ai trap/ có deterministic không; gắn VS3-1a. |
| **B5** | `Value::Task(Rc<Task>)` → `TaskId`/handle: shape, `Show`, API, cache/ABI | A0 duyệt cùng W1; không đổi API công khai nếu có thể. |
| **B6** | Tích hợp W1 **sau VS3-2a**; không được đụng `rt.rs`/`Chan` trước | Giữ prototype ngoài `crates/**`; W1 mở lát 3b-0 sau khi 2a ổn định. |
| **B7** | Perf/RSS chỉ là prototype (64 task, CPU 50k; hybrid 2P+8E) | Gate 3b-4 phải CPU-only + payload matrix, ghi topology/load; không dùng số này làm nghiệm thu. |

---

## 6. Giới hạn / không tuyên bố

- **Không** “worker pool xong”, “VS3-3b xong”, “async song song đã bật”.
- **Không** thay `Value`, không đổi `Rc`→`Arc`, không shared heap/GC, không channel data.
- **Không** chứng minh suspend/resume/await lồng; task chạy trọn một lần trong prototype.
- **Không** chứng minh cancellation/timeout; completion proxy chưa có nhánh huỷ.
- **Không** dùng số đo prototype làm gate production; máy hybrid, workload nhỏ.
- **Không** sửa `crates/**`, `frame/**`, `Cargo.toml`; không thêm crate.

---

## 7. Tái lập

```bash
cd /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w5-spike

rustc -O agent/JOURNAL/vs3-w5-spike/worker_proto.rs -o /tmp/vs3w5worker
/tmp/vs3w5worker                       # self-test + đo W=1/2/4 + RSS con
/tmp/vs3w5worker --child 4             # một tiến trình con, in RSS/HWM
```

- `worker_proto.rs`: prototype (SPDX, standalone, không crate).
- `worker_proto-run.txt`: một lần chạy đại diện (run 6).
- `worker_proto-summary.txt`: median 6 lần chạy.
- Không chạy `gate --fast` vì không đổi mã sản phẩm; đây là prototype trong `agent/`.
- Baseline `2d6ca2b`; `git merge main` đã chạy, không push.

---

## 8. Nguồn

- `agent/DECISIONS.md` VD-271 (bổ sung) — mô hình affine, TaskId/proxy, snapshot `var`,
  print merge, budget atomic, worker pool opt-in.
- `agent/VS3-TIEN-DO.md` §3b — cùng nội dung A0 chốt.
- `agent/JOURNAL/vs3-w5-spike.md` — spike VS3-3a (số đo copy/channel/RSS; B1–B6).
- `crates/vis-interp/src/rt.rs` (`Task`, `Scheduler`, `spawn`, `await_task`);
  `crates/vis-interp/src/value.rs` (`Value::Task`, `Env`);
  `crates/vis-interp/src/frame/fee.rs` (`thread_local! BUDGET`);
  `crates/vis-interp/src/host.rs` (`out`);
  `crates/vis-vm/src/vm.rs`, `vm_frame.rs` (`Th`/`Frame`).
