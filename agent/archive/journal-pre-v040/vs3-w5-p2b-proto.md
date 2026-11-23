<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — prototype P2b-1 lazy/batched submit (example-only)

**Trạng thái**: **PROTOTYPE PASS** — chỉ sửa example + journal; **không** sửa
production/ABI/default (`rt/exec.rs`/`rt/worker.rs` giữ nguyên).
**Nhánh**: `vs3/w5-spike`, baseline **`fdbcb64`** (tag v0.3.4 + P2b overhead).
**Mã đo**: `crates/vis-cli/examples/vs3_worker_eval_p2b_overhead.rs` (**439 dòng**; thêm
`P2B_LAZY` vào example cũ, không tạo bản trùng).
**Log**: `agent/JOURNAL/vs3-w5-p2b-proto.log`.
**Máy**: i5-1235U, 12 CPU; release; load 7,17 (lần A) và ~? (lần B, xem log); N=10.000;
baseline reps=3 median.

---

## 1. Mô phỏng lazy/batched

Trong example (không phải production):

1. Tạo 10.000 `Scheduler::tao` + `proxy: HashMap<Task.id, i>` — **chưa gửi pool**;
2. queue `(Task.id, Kind)` 10.000 phần tử;
3. flush **một lần**: `WorkerPool::spawn_shared(Arc<SendValue::Int(0)>, closure)`;
4. `pool.join()` một lần; kiểm 10k outcome + proxy map.

`Kind::Interp`/`Kind::Vm` dựng evaluator **mới mỗi job** từ ảnh `RONG`
(`fn main(): let x=1`) — tức vẫn trả giá constructor mỗi task, không tái dùng evaluator.

**Đây chỉ là mô phỏng**: chưa có `spawn_parallel` snapshot/args, `TaskProxy` thật,
`OutcomeBoard`, runner P2/P3, await/cache/dirty. Nó đo **queue + Task + WorkerPool +
evaluator**; không đo toàn bộ P2 runner.

---

## 2. Số đo 10k task (release)

### Baseline thật (P2/P3 API)

| W | interp sequential | VM sequential |
|---:|---:|---:|
| 1 | 17,1–65,6 ms | 12,6–48,2 ms |
| 4 | **168,4–200,0 ms** | **128,9–149,6 ms** |

- P2 batch (spawn-all, final join) W=4: **31,4–45,3 ms** interp / **29,2–37,5 ms** VM.
- P2 batch W=1: 10,1–14,2 ms interp / 8,0–12,4 ms VM.

### Lazy/batched example

| W | lazy interp | lazy VM |
|---:|---:|---:|
| 1 | **15,2–18,0 ms** | **9,6–11,7 ms** |
| 2 | **16,0–19,5 ms** | **15,9–17,0 ms** |
| 4 | **14,6–17,2 ms** | **14,0–15,0 ms** |

- **Mục tiêu batch W=4 ≤22–30 ms: ĐẠT trong mô phỏng** (14,6–17,2 / 14,0–15,0 ms).
- So P2 batch W=4 hiện tại (31,4–45,3 / 29,2–37,5): lazy mô phỏng nhanh **~2–2,6×**.
- Lazy W=4 **xấp xỉ W=1** (W1 15,2–18,0/9,6–11,7); task quá nhỏ nên không có speedup
  đáng kể, nhưng mục tiêu tránh phạt W>1 đã đạt.
- So W1 baseline thật: lazy W4 ~14–17 ms tương đương W1 (17–65 ms do load), nhưng **không**
  vượt; mục tiêu chỉ là tránh phạt W>1.

---

## 3. Diễn giải / khoảng cách còn lại

- Queue + `Scheduler::tao` + proxy map + `WorkerPool` + evaluator mới/task:
  **~15–17 ms/10k** ở W=4.
- P2 batch W=4 **~31–45 ms** ⇒ còn **~16–28 ms** cho:
  `spawn_parallel` (snapshot top-level, args `to_send`, tao/proxy), runner HIR/VM
  (`chay_than_worker`), `OutcomeBoard`/outcome clone, `await`/cache/final join, Task/ordering.
- P2b-1 chỉ **dời thời điểm gửi job** không tự xóa 16–28 ms đó; muốn đạt **22–30 ms** phải
  ghép P2b-1 (lazy flush) + P2b-2 (compact proxy) + P2b-3 (bớt clone/board). Mô phỏng chứng minh
  **phần queue/pool không phải rào cản**; production overhead còn lại là Task/proxy/runner.
- P2 sequential W=4 168–200/129–150 ms: round-trip await vẫn là nút chính; lazy batch không
  giúp chương trình `spawn`+`await` từng task (flush 1 job/await như cũ). Không hứa speedup
  cho shape này; W=1 vẫn tốt nhất.

---

## 4. Phần nào là production P2b-1 (chưa làm)

| Thành phần | File / signature đề xuất | Ước dòng |
|---|---|---:|
| `PendingJob`/queue + `flush_jobs` | `rt/exec.rs`: `spawn_parallel_lazy(...)`, `flush_jobs(host)`, final/await gọi flush | **+80–120** |
| `WorkerPool` batch submit / shared payload | `rt/worker.rs`: `spawn_shared_batch` hoặc `spawn_many` (tránh root clone/job lẻ) | **+40–60** |
| `ExecState` pending + order/proxy | `rt/exec.rs`: `pending: Vec<PendingJob>`, `flushed: bool`; giữ `Task` Moi tới flush | +30–50 |
| Compact proxy (P2b-2, khuyến nghị kèm) | `Vec<Option<TaskProxy>>`/index thay 2 HashMap | +60–90 |
| Per-task slot/no clone (P2b-3, tùy) | `TaskSlot`/`spawn_slot`; bỏ clone board+channel | +60–100 |

**Risk production**:
- `spawn` phải giữ semantics “task chưa chạy tới await/final”; lazy đúng, nhưng task `Moi`
  lâu hơn, `Scheduler.tasks` giữ 10k handle; final join phải flush **trước** khi quét.
- `await` phải `flush` job của task mình (hoặc toàn bộ) trước khi chờ board; tránh deadlock
  khi task chưa gửi.
- Nếu flush một lần 10k job, channel 64 backpressure; đo wall/RSS; worker pool đầy.
- `Task`/proxy/map nhớ; output/dirty/cross-write phải gắn với proxy; `frame` P4 có thể gọi
  flush từ `exec_pump` (P4a) — cần test.
- Không đổi `Value`/`Task`/VERSION/cache; hidden API test-only.

---

## 5. Điều kiện benchmark / ngưỡng

- Release, N=10k, reps=3–5 median + p95; ghi load; warmup.
- Đo 3 shape: sequential `spawn`+`await` từng task, batch spawn-all final join, lazy queue.
- **Ngưỡng P2b-1**:
  - batch W=4 **≤22–30 ms** (mô phỏng đạt 14,6–17,2; production target 22–30);
  - batch W=4 **≤1,5× W=1** batch (hiện P2 ~3–4×; mục tiêu ≤1,5×);
  - sequential W=4 **không tệ hơn** baseline, nhưng không hứa nhanh hơn W=1;
  - gate P1/P2/P3 8/14/10, 20 vòng, RSS/fd phẳng; Windows P2b nếu đụng pool.
- Nếu production P2b-1 + P2b-2 không đạt ≤30 ms, **không** bật default/CLI; giữ W=1 cho
  sequential tiny, W>1 cho task CPU lớn/batch.

---

## 6. Kết luận

- **Khả thi về mặt queue/pool**: mô phỏng lazy + `WorkerPool` + evaluator mới/task đạt
  **14,6–17,2 ms** cho 10k batch W=4, dưới mục tiêu 22–30 ms.
- **Chưa đủ để tuyên bố P2b-1 xong**: production còn Task/proxy/snapshot/runner/board
  (~16–28 ms); cần P2b-1 + P2b-2 (và tùy P2b-3) để đưa P2 batch 31–45 → 22–30.
- **Không có blocker code** cho prototype; production P2b cần A0 duyệt và gate/Windows.
- Sequential tiny vẫn latency-bound: W=1 là lựa chọn; W>1 chỉ có lợi cho task đủ lớn/batch.
- `fmt`/`clippy` PASS; R1 example **439 dòng**; df sau test **21G** (>5G); không GitHub Actions.
