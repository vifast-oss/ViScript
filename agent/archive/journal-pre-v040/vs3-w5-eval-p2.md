<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — P2 evaluator W>1 (CPU-only, non-frame)

**Trạng thái**: **P2 IMPLEMENTED — gate xanh**, có giới hạn CPU-only + performance ghi rõ.
**Nhánh**: `vs3/w5-spike`, đã `git merge main` → **`a8100ab`** (P1 `54021de` đã merge).
**Không** push/merge/tag; **không** default/CLI; không đổi ABI/`VERSION`/cache key/`Value`/`Task`.
**Timebox**: 4 h. `df /home`: 25G đầu phiên, **13G** sau test, **24G** sau khi xoá `target`
riêng của worktree (xem §7).
**Log**: `agent/JOURNAL/vs3-w5-eval-p2.log`.

---

## 1. Diff / file

| Tệp | Thay đổi | Dòng |
|---|---|---:|
| `crates/vis-interp/src/rt/exec.rs` | `WorkerMode::Parallel(n)`, `HirImage`, `TaskProxy`, pool lazy, `spawn_parallel`, `await_parallel`, `join_parallel`, budget/cancel hook | **376** |
| `crates/vis-interp/src/interp/worker.rs` | `top_syms`, `snapshot_top`, `tao_interp`, `chay_than_worker` (HIR runner), `worker_tick` | **208** |
| `crates/vis-interp/src/rt/worker.rs` | `OutcomeBoard` đầy đủ, `TaskOutcome: Clone`, `WorkerCtx::{cancel_flag, mark_dirty}`, publish outcome | **1.123** (+110/−2) |
| `crates/vis-interp/src/rt.rs` | dispatch `Parallel` cho `await_task`/`join_all` | **1.273** (+10/−6) |
| `crates/vis-interp/src/interp.rs` | `top_syms`, `worker_cancel`, `worker_cancel_deadline`, `worker_budget`; tick 1.024 bước; spawn dispatch 3 chế độ | **1.458** (+36/−9) |
| `crates/vis-interp/src/lib.rs` | `run_workers_parallel`, `_with_budget`, `_cancel_before`, `_cancel_after`; ảnh HIR + budget/cancel trong mode | **526** (+135/−1) |
| `crates/vis-interp/src/frame/core.rs` | `Interp::charge` dùng `BudgetLease` khi là task worker | **154** (+12) |
| `crates/vis-cli/tests/vs3_worker_eval_parallel.rs` | **mới**: 14 test W=2/4 + budget/cross/cancel/host/chan/nested/10k/soak ngắn | **487** |

Tổng production: **+810/−37** (7 tệp); test **+487**. R1 mọi tệp ≤1.500.
**Không đụng** `value.rs` (vẫn 1.500), `frame.rs`, `vm.rs`, `net.rs`; `rt/worker.rs` vốn được
`include!` từ `value.rs` nên `value.rs` không sửa. `frame/core.rs` chỉ là hook charge như plan P2.

---

## 2. Thiết kế P2

- **Mode**: `WorkerMode::{Off, Local, Parallel(n)}`; `Off` mặc định, `Local` P1, `Parallel`
  chỉ non-frame CPU. `run_workers_parallel*` là `#[doc(hidden)]`, không CLI/default.
- **Ảnh**: `run_workers_parallel*` dựng `Arc<HirImage{hIR, symbols, types, text}>` **một lần**;
  worker dựng `Interp` riêng từ ảnh, không gửi `Interp`.
- **Env**: `snapshot_top` chỉ lấy `top_syms` (Var/Let cấp tệp, gồm pattern lồng) →
  `SendValue::Map(Int(sym) → SendValue)`. Worker nạp Map vào `Interp.env` riêng và nạp lại
  `mutable` cho top-level `var`; không chạy lại khởi tạo cấp tệp.
- **Args**: mỗi `Value` → `SendValue::to_send()` trước khi tạo job; lỗi ⇒ `VS0508` ở spawn.
  `Task.args` để rỗng trên main; proxy giữ job/args trong worker.
- **Print**: runner ghi vào `WorkerCtx.out`; `OutcomeBoard` công bố outcome đầy đủ khi task
  xong. `await` flush output tại điểm tiêu thụ; `join_parallel` flush task chưa await theo
  **spawn order**. Test 10k final join so 10.000 dòng byte-identical.
- **Budget**: `ExecState.budget` được truyền vào worker; runner tạo `BudgetLease`/task;
  `Interp::charge` dùng lease thay thread-local. Cạn ⇒ `VS0501-BUDGET`; test
  `used=10.000, remaining=0, limit=10.000`.
- **Cancel**: handle pool `cancel()` ở biên job (test pre-run) + `worker_cancel_deadline`
  (`worker_tick` mỗi 1.024 bước) cho cancel-after. Test task 2M vòng dừng sau 5 ms.
- **Cross-write**: so top-level env trước/sau trong worker; nếu khác ⇒ `WorkerCtx.mark_dirty`
  → `await`/`join` trả `VS0501` + message `ghi chéo worker (W>1): biến …`; env main không đổi.
- **Từ chối**: worker không host/`ngoai`/`fs`/`net`/`db`; `chan(0)` non-frame trả `VS0505`;
  `std.fs` trả `VS0604`; lỗi được bọc `VS0506` với message giữ mã gốc, không treo.

---

## 3. Test mới (14 ca, `vs3_worker_eval_parallel.rs`)

| Test | Kỳ vọng |
|---|---|
| `w2_print_byte_identical`, `w4_print_byte_identical` | 4 task in + await; W=1 == W=2/4 `1\n2\n3\n4\n20\n` |
| `muoi_nghin_task_w2_w4_byte_identical` | 10k spawn/await; output `50005000`, W=1 == W=2/4 |
| `muoi_nghin_task_join_cuoi_in_spawn_order` | 10k task không await; 10k dòng theo spawn order, byte-identical |
| `nested_spawn_await_trong_worker` | task spawn + await task con ⇒ `41` |
| `snapshot_tai_spawn_w1_khac_w2_co_chu_dinh` | W=1 sees `2`, W=2/4 sees snapshot `1` (đúng D4) |
| `budget_atomic_used_cong_remaining_bang_limit` | task 1M vòng, limit 10k ⇒ `VS0501-BUDGET`, `10000+0==10000`, exhausted |
| `cross_write_w1_shared_w2_loi_ro` | W=1 prints `1`; W=2/4 `VS0501` message `ghi chéo worker` |
| `chan0_trong_task_bi_tu_choi_ro` | task `chan(0)` ⇒ `VS0505`, exit 1, không treo |
| `host_fs_trong_task_bi_tu_choi_ro` | `std.fs` trong worker ⇒ `VS0604`, exit 1 |
| `cancel_pre_run_tra_loi_ro` | cancel trước job ⇒ `VS0506`/message `huỷ` |
| `cancel_giua_chung_safe_point` | task 2M vòng, cancel sau 5 ms ⇒ dừng, message `huỷ` |
| `muoi_vong_threads_fd_rss_khong_ro` | 10 vòng W=4: thread/fd/RSS không rò (assert) |
| `cpu_nang_w1_w2_w4_byte_identical` | 4 task × 20k vòng; output byte-identical, đo speedup |

---

## 4. Số đo (máy load 8–15, chỉ tham khảo; không assert wall)

- **CPU nặng — 4 task × 20k vòng**, 3 lần median:
  W=1 **1.400,7 ms**; W=2 **815,9 ms** (1,59×); W=4 **316,3 ms** (**3,75×**).
  Các lần riêng W=2 1,12–3,41×; W=4 2,88–5,58× (nhiễu load).
- **10k spawn/await tuần tự** (mỗi task cực nhỏ): W=1 68–647 ms; W=2 490–2.731 ms;
  W=4 453–1.383 ms ⇒ **0,14–0,69×** — pool/Interp-per-task overhead, **không** phải speedup.
- **10k final join** (task không await, in 10k dòng): W=1 91–286 ms; W=2 300–318 ms;
  W=4 222–254 ms ⇒ **0,34–1,28×**; output byte-identical.
- **10 vòng × 8 task W=4**: ~11 ms, threads **2**, fd **4**, RSS cuối **~19,6 MB**; assert
  `threads ≤ baseline+1`, `fd ≤ baseline+4`, RSS tăng <64 MB.
- **Budget**: `used=10000`, `remaining=0`, `limit=10000`, `exhausted=true`.
- **Cancel-after**: task 2M vòng dừng sau 5 ms, test 0,01 s.
- **Kết luận hiệu năng**: P2 đúng/hữu ích cho task CPU đủ lớn; **không đạt** mục tiêu plan
  “10k task rỗng không chậm quá 1,2×” — cần P2b tái dùng evaluator/pool theo worker hoặc
  giảm overhead spawn nếu muốn dùng đại trà.

---

## 5. Gate / test

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-cli --test vs3_worker_eval_parallel -- --test-threads=1` | **14/14 PASS** (~4,8 s) |
| `cargo test --release ... vs3_worker_eval_parallel` | **14/14 PASS** (2,18 s; build 50 s) |
| `cargo test -p vis-cli --test vs3_worker_eval_local` | **8/8 PASS** (P1 giữ nguyên) |
| `vs3_default` / `vs3_worker` / `vs3_budget` | **22/22 / 14/14 / 8/8 PASS** |
| `cargo test -p vis-interp --lib` | **303/303 PASS** |
| `./vis-dev gate --fast` | **PASS**; nextest **3561 passed, 3 skipped** (91,978 s) |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --all-targets` | PASS |
| `cargo clippy -p vis-cli --all-targets -- -D warnings` | PASS |

Gate tăng **+14 test** so P1 3547/3 ⇒ **3561/3**; không xoá/nới test.

---

## 6. Giới hạn / phần còn thiếu

- **Chỉ CPU**: worker `Interp` không host/`fs`/`net`/`db`/`ngoai`; `chan(0)` non-frame vẫn
  `VS0505`. Không hỗ trợ frame suspend/resume (P4), VM runner (P3), direct `await f()` không
  qua `spawn` (không có proxy ⇒ lỗi), `Chan`/closure/Task qua biên (VS0508).
- **Cancel**: API test-only `cancel_before`/`cancel_after`; chưa có API sản phẩm để user huỷ
  handle giữa chừng. Safe point mỗi 1.024 bước; task bị chặn trong host/chan không tick.
- **Budget**: lease nằm trong `Interp` worker; `TaskOutcome.budget_used` vẫn 0 (snapshot lấy
  từ `Arc<SharedBudget>` bên ngoài); không cộng dồn steps của worker vào `RunOutput.steps`
  (steps chỉ main).
- **Output/OutcomeBoard**: clone outcome sang board + completion channel ⇒ bộ nhớ gấp đôi cho
  payload lớn; P2b nên chọn một đường completion.
- **Snapshot**: toàn bộ top-level `var`/`let`; một binding không Sendable làm mọi spawn W>1
  báo `VS0508` (bảo thủ như plan D4); chưa COW/cache snapshot giữa các spawn.
- **10k performance**: xem §4; cần P2b nếu muốn dùng rộng.
- **P3/P4**: VM parity và frame chưa làm.

---

## 7. Blocker / đĩa

- **Không có blocker code**: gate xanh, P1 vẫn xanh, default Off byte-identical (gate 3561).
- **Blocker quyết định** vẫn còn: mã cross-write W>1 (`VS0509`?) và chính sách W>1 bỏ
  frame/`chan(0)`/host — P2 dùng `VS0501` + message như user cho phép.
- **Đĩa**: sau test còn **13G**; đã xoá `target` riêng của worktree
  (`/home/ps/.local/share/opencode/worktree/cd45e5/vs3-w5-spike/target`) sau khi chốt ⇒ còn
  **24G**. Không đụng target/cache của repo chính.

---

## 8. Lệnh đã chạy

```bash
~/.cargo/bin/cargo test -p vis-cli --test vs3_worker_eval_parallel -- --test-threads=1
~/.cargo/bin/cargo test --release -p vis-cli --test vs3_worker_eval_parallel -- --test-threads=1
~/.cargo/bin/cargo test -p vis-cli --test vs3_worker_eval_local --test vs3_default --test vs3_worker --test vs3_budget -- --test-threads=1
~/.cargo/bin/cargo test -p vis-interp --lib
./vis-dev gate --fast
~/.cargo/bin/cargo fmt --all -- --check
~/.cargo/bin/cargo check --all-targets
~/.cargo/bin/cargo clippy -p vis-cli --all-targets -- -D warnings
VS3_W5_MEASURE=1 ~/.cargo/bin/cargo test -p vis-cli --test vs3_worker_eval_parallel -- --test-threads=1 --nocapture
```
