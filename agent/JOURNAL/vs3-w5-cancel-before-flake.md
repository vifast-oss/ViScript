<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — flake cancel-before-run: điều tra + fix

**Trạng thái**: **FIXED trên nhánh** — race thật trong `cancel_before`; fix tối thiểu production
`rt/worker.rs` + `rt/exec.rs`; không đổi default/ABI/`Value`/`Task`.
**Nhánh**: `vs3/w5-spike`, baseline **`d24379e`** (tag `v0.3.4` @ `8fa1f96`).
**Flake gốc**: `vs3_worker_eval_parallel::cancel_pre_run_tra_loi_ro` — `exit=0`, kỳ vọng `1`;
1 fail ở soak run 2 / gate-3 (load ~9,9).
**Không** GitHub Actions; **không** push/merge/tag; tag `v0.3.4` giữ nguyên (cần `v0.3.5` nếu
duyệt merge).

---

## 1. Root cause

`run_workers_parallel_cancel_before` đặt `ExecState.cancel_before = true`. Trong
`spawn_parallel` (P2):

```rust
let handle = pool.spawn_shared(snap, ...)?; // job đã vào queue
if cancel_before {
    handle.cancel();                       // ← set sau khi worker có thể đã check
}
```

`WorkerPool::spawn` tạo `cancel = Arc<AtomicBool>::new(false)`, gửi `Job` vào channel; worker
`worker_loop`/W=1 `join` kiểm `cancel.load(...)` **trước** khi chạy task. Race:

1. main gửi job (`cancel=false`);
2. worker nhận job, `cancel.load() == false`;
3. main mới `handle.cancel()`;
4. worker chạy task ⇒ kết quả bình thường, `exit=0`.

Vì vậy “cancel-before-run” chỉ là best-effort, không đảm bảo task chưa chạy. Flake xảy ra khi
worker được schedule/dưới tải trước main.

---

## 2. Fix (tối thiểu)

`WorkerPool`:

- `spawn` → `spawn_inner(f, cancelled=false)` (hành vi cũ).
- Thêm `spawn_cancelled` → `spawn_inner(f, true)`.
- `spawn_shared` giữ cũ (false); thêm `spawn_shared_cancelled` và
  `spawn_shared_with_cancel(payload, f, cancelled: bool)`.
- `spawn_inner` khởi tạo `AtomicBool::new(cancelled)` **trước** khi đưa job vào `queue`/`txs`.

`spawn_parallel`:

```rust
pool.spawn_shared_with_cancel(snap, closure, cancel_before)?;
// bỏ handle.cancel() sau spawn
```

- `cancel_before=true` ⇒ worker/W=1 join thấy `true` ngay lần check đầu, không chạy thân;
  outcome `cancelled`, `await` trả `VS0506` / exit 1.
- `cancel_after`/cancel giữa chừng **không đổi**: vẫn dùng `TaskHandle.cancel` sau khi job
  đã chạy; đây là cancel hợp tác, có thể thắng/thua theo safe point.
- `spawn` thường (`cancel_before=false`) giữ nguyên `AtomicBool(false)` và cùng đường.

**File/dòng**:

| Tệp | Thay đổi | Dòng |
|---|---|---:|
| `crates/vis-interp/src/rt/worker.rs` | `spawn_inner`, `spawn_cancelled`, `spawn_shared_cancelled`, `spawn_shared_with_cancel` | **+44/−?** (1.165 dòng) |
| `crates/vis-interp/src/rt/exec.rs` | `spawn_parallel` dùng `spawn_shared_with_cancel(..., cancel_before)`, bỏ `handle.cancel` | **+17/−?** (377 dòng) |

Không đụng `value.rs`/`frame.rs`/`vm.rs`/`net.rs`; không `unsafe`; không default/CLI/ABI.

---

## 3. Tái hiện / kiểm chứng

Temporary test (không commit; test-only):
- 16 thread × vòng lặp gọi `run_workers_parallel_cancel_before(..., workers=2)` trên chương
  trình 2 task, đếm `exit != 1`.

| Giai đoạn | Số lần | Kết quả |
|---|---:|---|
| Before (8 thread) | 1.000 | **0 fail** (cửa sổ race hẹp, local không trúng) |
| Before (16 thread) | 20.000 | **0 fail** local; nhưng soak gate đã trúng **1 fail** run 2 |
| After (16 thread) | 20.000 | **0 fail** |
| After P2 | 10 × 14 test | **140/140 pass** |
| After P2 release | 14/14 | **PASS** (1,85 s) |
| P1 / P3 / worker | 8/8 / 10/10 / 14/14 | **PASS** |
| Gate `./vis-dev gate --fast` | 3571/3 | **PASS** (97,308 s) |

- Local repro trước fix **không trúng** 1.000/20.000 vì race phụ thuộc schedule/tải; bằng chứng
  trúng là soak run 2 (`cancel_pre_run_tra_loi_ro`, exit 0) và đọc code xác nhận thứ tự
  `spawn` → `cancel`. Sau fix, 20.000 lần + 10 vòng P2 đều xanh.
- `cargo fmt --all -- --check`, `cargo clippy -p vis-interp -p vis-cli --all-targets -- -D warnings`:
  **PASS**.
- R1: `rt/worker.rs` 1.165, `rt/exec.rs` 377; không đổi ABI/VERSION/cache.
- `df /home`: 21G trước test; target riêng được xoá sau chốt (nếu còn).

---

## 4. Quyết định

- **Đây là race thật**, không phải test sai: API test tên `cancel_before` hứa “huỷ trước khi chạy”,
  nhưng code cũ set cờ **sau** khi enqueue. Sửa production tối thiểu (không đổi default).
- Giữ tag `v0.3.4`; nếu duyệt, cherry-pick commit này vào `main` và phát hành **v0.3.5** (không
  tag trong phiên này).
- Không thêm test vào gate (test hiện có `cancel_pre_run_tra_loi_ro` đã phủ; temporary 20k chỉ
  dùng điều tra, đã xoá). Gate vẫn **3571/3**.
- Blocker: cần quyết định release patch (`v0.3.5`) và merge; không có blocker code còn lại.
- P4/P2b không bị ảnh hưởng; soak 4h đã dừng theo ưu tiên flake.
