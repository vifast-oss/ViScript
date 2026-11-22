<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-292 (VD-275/VD-280/VD-281/VD-282) — P4a/P4b: frame + evaluator worker

**Trạng thái**: DONE phần **P4a + P4b + SharedBudget + CLI**; **P4c chưa làm** (frame log/golden,
Windows verify — W2/W4 sẽ chạy).
**Nhánh**: `plan/w1-runtime`, merge `main` **`8cc3588`**. Không push/merge/tag; không đổi default
(`--workers` mặc định 1 = đường cũ), không đổi ABI/`Value`/cache, không `unsafe`.
**Target riêng**: `CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w1`,
`CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/home/ps/tmp`.

## 1. Đã làm

### Hạ tầng pump / hook

- `OutcomeBoard::wait_any_timeout(ids, timeout)` (`rt/worker.rs`) — chờ **một trong** nhiều task,
  dùng cho pump frame không giữ borrow `RefCell`.
- 5 hook default no-op trên `Host` (`builtin.rs`): `worker_is_task`, `worker_in_flight`,
  `worker_pump(block)` (50 ms), `worker_final_join`, `worker_cancel`; override cho `Interp`
  (`host.rs`) và `Vm` (`vm/host.rs`).
- `rt/exec.rs`: `TaskProxy` thêm `done/output/dirty/cancelled`; `pump`, `in_flight`,
  `is_worker_task`, `cancel_worker`, `final_join`; `await_parallel` frame-aware:
  task chưa xong ⇒ `Abort::Pending` (suspend), resume đọc cache + flush output **một lần**.
  Cross-write ⇒ **`VS0509`**; cancel ⇒ `VS0506`; lỗi worker vẫn bọc `VS0506` + giữ mã gốc
  (`VS0510` trong message) đúng hợp đồng task-error VD-95/VD-256.
- `vm/worker.rs`: mirror đầy đủ (`pump`/`final_join`/`in_flight`/`is_worker_task`/`cancel`),
  `Proxy` thêm output/dirty/done/cancelled; `join_tasks` Off vẫn `join_all` cũ.
- Driver frame (`frame/worker.rs`, `vm_frame.rs`): fallback **lọc `worker_is_task`** (không chạy
  thân lần hai), khi `frame_next=None` + pool in-flight ⇒ pump 50 ms/lần rồi thử lại; final gọi
  `worker_final_join` trước `frame_final_join`; `frame_stuck_trap` chỉ khi pool hết in-flight;
  cancel của scheduler chuyển tới `TaskHandle.cancel()` (idempotent).
- Policy **P4a**: chỉ frame **main** (`frame.current == 0`) đẩy `spawn` lên pool; spawn trong
  frame task chạy local trong driver (`frame/expr.rs` + VM `spawn_task`).

### CLI + ngân sách

- `vis run --workers <N>`: mặc định 1 (đường cũ); `0`/không phải số ⇒ exit 2. N>1 chọn
  runner `Parallel` cho cả interp/VM, sync và frame; W=1 giữ nguyên.
- N>1: `SharedBudget::new(gioi_han_phien(), 256)`; `--unlimited-budget` ⇒ `None`; budget default
  40M giữ nguyên. Kiểm khoá: `--workers 4 --max-budget 5000` và W=1 **cùng exit 101**, cùng
  `VS0506` + message chứa `VS0510` (log `st292-budget.log`).

## 2. Test

- `crates/vis-cli/tests/vs3_worker_frame.rs` — **12/12**: main await 1 task, 2 task, final join
  spawn order, task chỉ chạy một lần, cross-write `VS0509`, await lệch thứ tự (chỉ khoá giá trị
  theo plan §3.7), 200 task, nested spawn local, `chan(0)` worker `VS0505`, 20 vòng W=2/4
  byte-identical, CLI `--workers 0` exit 2, CLI budget W1 vs W4.
- `crates/vis-cli/tests/vs3_worker_frame_vm.rs` — **6/6**: parity interp↔VM các ca trên cho
  VM; ca 20 vòng W=2/4.
- P1/P2/P3 giữ: `vs3_worker`, `vs3_worker_eval_local/parallel/vm`, `vs3_worker_soak`,
  `lat1_frame`, `lat257_c_lifecycle`, `vd259_frame2` xanh.

## 3. Cổng / bằng chứng

- `./vis-dev gate --fast` ⇒ **PASS**: **3656 passed / 4 skipped** (log `st292-gate.log`).
- `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` ⇒ `tong=100000 … **lech=0**`
  (129,4 s; `st292-diff-100k.log`).
- Soak 10 phút (`VS3_SOAK_SECS=600 vs3_worker_eval_p2_soak`) ⇒ **797 batch**, 23.931.519 task,
  RSS sau warm **16.368–17.796 KB** (Δ 1.428 KB), threads **1**, fd **4**, hwm 22.448 KB
  (log `st292-soak.log`).
- R1: mọi tệp sửa ≤ 1.500 dòng — `rt/exec.rs` 576, `rt/worker.rs` 1.338,
  `frame/worker.rs` 501, `frame/expr.rs` 886, `main.rs` 1.418, `vm/worker.rs` 589,
  `vm_frame.rs` 1.128.

## 4. Chưa làm / giới hạn đã biết

- **P4c**: golden frame log, Windows verify, soak 10 phút riêng cho frame (soak trên là P2
  non-frame; 20 vòng frame đã có trong test VM). W2/W4 chạy Windows.
- CLI VM async + `--workers` chưa truyền `FrameReactorBackend` (entry VM mới không nhận backend)
  ⇒ main frame dùng async net dưới `--workers` sẽ thiếu reactor (`VS0505`) thay vì chạy; chưa có
  test. Cần P4c/W2.
- Cancel worker đã nối hook + driver, nhưng chưa có ca test chuyên biệt qua trigger frame
  (các ca cancel hiện có vẫn xanh).
- Await **lệch thứ tự** không bảo đảm byte-identical thứ tự in (plan §3.7); khoá giá trị/exit.

## 5. Lệnh tái lập

```bash
export PATH=$HOME/.cargo/bin:$PATH \
  CARGO_TARGET_DIR=/home/ps/ViScript/target/vs3-w1 \
  CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/home/ps/tmp
cargo test -p vis-cli --test vs3_worker_frame --test vs3_worker_frame_vm
./vis-dev gate --fast
VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff
VS3_SOAK_SECS=600 cargo run --release -p vis-cli --example vs3_worker_eval_p2_soak
./target/debug/vis run --workers 4 --max-budget 5000 /path/prog.vis
```

## Log lớn ngoài repo (R7)

`st292-gate.log` → `/home/ps/vislogs/st292/st292-gate.log`
c493e25f2e6100b4c0a1d3b45711b661b526852cc54a017475e49ea73aeb650d  st292-gate.log
