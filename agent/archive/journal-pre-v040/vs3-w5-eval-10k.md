<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-3b/W5 — evaluator 10k task + soak + P1 implementation-ready

**Trạng thái**: **PROTOTYPE PASS** — chỉ mở rộng example + journal; **không** sửa
production/ABI/default, không push/merge/tag.
**Nhánh**: `vs3/w5-spike`, đã `git merge main` → **`629782a`** (main đã merge prototype
`8850678`; gate 3538/3 theo mốc user).
**Mã đo**: `crates/vis-cli/examples/vs3_worker_eval_proto.rs` (689 dòng, example chạy tay —
không nằm trong test suite).
**Log**: `agent/JOURNAL/vs3-w5-eval-10k.log`.
**Timebox**: 2 h. `df /home`: **11G** trống trước/sau (>5G).
**Máy**: i5-1235U (2 P-core + 8 E-core), `nproc=12`, Linux; load **10–13** trong phần lớn
phép đo 10k (ghi từng lần trong log) — wall/speedup chỉ tham khảo, **không** dùng làm ngưỡng.

---

## 1. Kết luận ngắn

1. **10.000 job, W=1 vs W=4: checksum byte-identical 3/3 lần** (và output byte-identical khi
   N ≤ 256). Không rò thread/fd; RSS sau 10k ~**14,0–14,3 MB**, HWM ~**15 MB**.
2. Thời gian 10k (job cực nhỏ, `VS3_EVAL_ITERS=1`, cả interp + VM mỗi job): W=1
   **543–1.714 ms**, W=4 **302–894 ms**, speedup **1,57–2,12×** (median 1,80×). Đây là
   **overhead task/pool**, không phải benchmark CPU: task quá nhỏ nên speedup thấp hơn
   prototype CPU-heavy trước (2,7–4,4×).
3. **Chi phí dựng evaluator/task rất nhỏ so với thân task micro**: `Vm::new` **1,0–4,9 µs**;
   task VM mới **16–79 µs**; task interp mới **30–105 µs**. Tỉ lệ dựng VM ≈ **5–6%** thời gian
   task. Tái dùng một `Vm` cho nhiều task chỉ **1,00–1,37×** (median ~1,16×) và **chỉ đúng**
   cho chương trình không `spawn`/`chan` (xem caveat §4) — chưa đủ để P1/P2 đổi thiết kế.
4. **Soak 10 phút W=4, 10k job/batch**: **1.307 batch / 13.070.000 job**, RSS
   **14.228–14.552 kB** (biên độ **324 kB**, plateau), HWM **15.556 kB**, threads **1**, fd
   **4**, checksum đúng **mọi batch**, 0 lỗi, exit 0.
5. **P1 implementation-ready** ở §6: runner W=1 cụ thể `chay_than_local`, dispatch
   `spawn_local/await_local/join_local`, API ẩn `run_workers_local*`, test và file đụng tới;
   **không** cần sửa `value.rs`/`frame.rs`/`vm.rs` cho P1 W=1.

---

## 2. Thay đổi example (env-gated)

| Biến | Mặc định | Ý nghĩa |
|---|---:|---|
| `VS3_EVAL_N` | 64 | số job mỗi lần chạy; đo 10k: `10000` |
| `VS3_EVAL_W` | 4 | W dùng cho soak (1/4); phần so sánh **luôn** chạy cả W=1 và W=4 |
| `VS3_EVAL_ITERS` | 2000 (tự 1 khi N≥1000) | số vòng `work(n)`; 10k dùng 1 để đo overhead |
| `VS3_EVAL_SOAK_SECS` | 0 | soak W=`VS3_EVAL_W` trong bấy nhiêu giây; 0 = tắt |

- Mỗi job vẫn dựng evaluator **riêng** từ cùng `Arc<Anh>` (`Hir`/`Symbols`/`TypeTable` +
  `Arc<Module>`) và chạy **cả hai engine** (interp + VM), kiểm hai engine khớp nhau.
- Job trả `SendValue::Int(checksum)`; `checksum = FNV-1a(stdout, exit, steps_hir, steps_vm,
  version)`. N ≤ 256 mới `ctx.print` để kiểm merge output theo spawn order; N lớn **không** in
  10k dòng.
- Mỗi pool có `SharedBudget` limit `N+1000`, mỗi job charge 1 ⇒ `used=N`, `remaining=1000`.
- Cross-write vẫn kiểm riêng 4 job: W=1 shared env, W=4 `CrossWrite`.
- `/proc/self/task` đọc qua `dem_thread_on_dinh` (10 mẫu cách 2 ms, lấy min) vì procfs có thể
  còn entry thoáng qua sau `join`; nếu không, lần đầu 10k báo rò giả `1->2`.

---

## 3. Kết quả 10.000 job (3 lần sạch, log đầy đủ ở `.log`)

Chương trình: `work(1)` + `print(work(1)*2)` ⇒ oracle `stdout="0\n"`,
`steps_hir=34`, `steps_vm=39`, checksum `-478856010891637171`.
Mỗi lần example tự chạy **3 vòng** W=1/W=4; bảng lấy median 3 vòng của từng lần.

| Lần | load đầu | W=1 median | W=4 median | speedup | RSS sau / HWM | thread/fd |
|---|---|---:|---:|---:|---:|---|
| 1 | 10,26 | 1.714,3 ms | 808,1 ms | 2,12× | 14.252 / 15.140 kB | 1/4 |
| 2 | 12,99 | 1.405,6 ms | 894,3 ms | 1,57× | 14.076 / 14.844 kB | 1/4 |
| 3 | 13,23 | 542,9 ms | 302,2 ms | 1,80× | 14.140 / 14.976 kB | 1/4 |

- **Checksum**: `-478856010891637171` ở **mọi** job/vòng/lần; W=1 = W=4.
- **Output**: N=64 (mặc định) output 384 byte, W=1 = W=4 = oracle×64; N=10k không echo.
- **Thread/fd**: sau mỗi pool và cuối cùng đều `1` thread, `4` fd; rò = 0.
- **Wall**: dao động lớn vì load 10–13 và task quá nhỏ; **không** assert wall.
- **RSS**: tăng từ ~5 MB (trước) lên ~14 MB (sau 10k, giữ 10k `TaskOutcome`/result tạm),
  HWM ~15 MB; không tăng theo số vòng.

---

## 4. Chi phí evaluator/task và tái dùng (example, không production)

Đo trên cùng ảnh, `N=10.000`, chương trình `work(1)` (mỗi job ~16–105 µs):

| Lần | `Vm::new` (µs/task) | VM mới + run (µs/task) | VM tái dùng (µs/task) | reuse speedup | `vis_interp::run` mới (µs/task) |
|---|---:|---:|---:|---:|---:|
| 1 | 2,97 | 60,01 | 51,94 | 1,16× | 93,98 |
| 2 | 4,85 | 79,11 | 57,70 | 1,37× | 105,09 |
| 3 | 0,99 | 16,03 | 16,06 | 1,00× | 29,94 |

- **Dựng VM** ~1–5 µs/task, tức **~5–6%** của task VM mới; 10k task ⇒ ~10–50 ms cộng dồn
  trên một luồng. Không phải nút cổ chai của task micro.
- `vis_interp::run` (dựng `Interp` + chạy) ~30–105 µs/task; **không tách được** riêng
  constructor vì `Interp::new` là `pub(crate)`. P1/P2 phải đo `Interp::new_for_task` sau khi
  có API trong crate, đừng lấy số này làm chi phí constructor.
- **Tái dùng `Vm`**: tiết kiệm ~0–21 µs/task (**0–27%** trong lần nhiễu), median ~14%. Chỉ đúng
  cho chương trình **không** `spawn`/`chan`: `Vm::run` reset `steps`/`globals`/`stdout` nhưng
  **không reset `Scheduler.tasks`/`kenh`**; chương trình async sẽ tích luỹ task qua các lần
  chạy. Vì vậy example **không** tuyên bố tái dùng evaluator là hướng tối ưu production; P2 chỉ
  làm `reset_for_task` nếu đo thấy >10–15% (đúng plan R2).
- **Không tái dùng `Interp`** trong example: không có constructor/reset công khai.

---

## 5. Soak 10 phút W=4 (N=10k, `VS3_EVAL_ITERS=1`)

| Mục | Số đo |
|---|---|
| Thời gian | **600 s** (exit 0) |
| Batch / job | **1.307 batch** × 10.000 = **13.070.000 job** (~21.800 job/s) |
| RSS sau warm-up | min **14.228 kB**, max **14.552 kB**, biên độ **324 kB** |
| RSS lúc bắt đầu / kết thúc | 14.352 kB (mẫu 5 s) / 14.500 kB (mẫu 597,8 s) |
| VmHWM | **15.556 kB** |
| Threads / fd | **1 / 4** sau mỗi batch và cuối |
| Checksum | `-478856010891637171` ở **mọi** batch |
| Lỗi | **0**; dừng ngay nếu checksum sai (assert trong batch) |

- Lần chạy soak cũng tự chạy 3 vòng W=1/W=4 trước đó: W=1 median **1.253,6 ms**,
  W=4 median **363,1 ms**, speedup **3,45×** — cùng checksum, cùng thread/fd.
- RSS biên độ 324 kB trong 10 phút ⇒ **plateau**, không có dấu hiệu rò evaluator/task;
  thread/fd không tăng qua 1.307 lần tạo/join pool.

- Kiểm mỗi batch: **checksum toàn bộ 10k job = oracle**; sai ⇒ panic/dừng ngay.
- Kiểm thread sau mỗi batch ≤ baseline; fd ≤ baseline+4; cuối cùng thread=1, fd=4.
- RSS plateau: sau warm-up 30 s, `max-min ≤ 64 MB`; số thực tế ghi ở bảng.
- Không có GitHub Actions; chạy local trên worktree.

---

## 6. P1 implementation-ready (W=1, trong crate)

> Phần này **chỉ thiết kế**, không thêm dòng production nào trong phiên này. Mục tiêu: P1
> W=1 là **oracle** — cùng output/exit/diagnostic/steps với default, **không** tạo thread,
> không snapshot, không đổi `Value`/`Task`/`VERSION`.

### 6.1 Runner W=1 — tên hàm/signature cụ thể

Tạo `crates/vis-interp/src/interp/worker.rs` (mới, ~60–90 dòng):

```rust
// SPDX-License-Identifier: MIT OR Apache-2.0
use vis_syntax::Span;
use crate::builtin::Host;
use crate::interp::{Abort, R, RT, Trap};
use crate::value::Value;

/// W=1 (Local): chạy **thân** tác vụ trên caller, dùng `Host::goi_than` (shared `env`/`out`),
/// không snapshot/buffer/thread. Lỗi thân bọc `VS0506` giữ mã gốc + span lúc tạo task.
pub(crate) fn chay_than_local(
    host: &mut dyn Host,
    than: u32,
    args: Vec<Value>,
    ten: &str,
    span: Span,
) -> R<Value> {
    match host.goi_than(than, args, span) {
        Ok(v) => Ok(v),
        Err(Abort::Ret(v)) => Ok(v),
        Err(Abort::Trap(goc)) => Err(Abort::Trap(crate::rt::wrap_task_error(ten, goc, span))),
        Err(Abort::Pending(_)) => Err(Abort::Trap(Trap::new(
            RT, "pending trong thân tác vụ local (W=1)", span,
        ))),
    }
}
```

- `wrap_task_error` trong `crates/vis-interp/src/rt.rs` đổi `fn` → `pub(crate) fn` (+0 dòng).
- `await_task_off` (thân cũ của `await_task`) gọi `chay_than_local` thay vì trực tiếp
  `host.goi_than`; hành vi W=1 **giống hệt** hiện tại (cùng cache/`VS0501`/`VS0506`).

### 6.2 Dispatch `rt/exec.rs` (mới) + hook `rt.rs`/`interp.rs`

`crates/vis-interp/src/rt/exec.rs` (mới, P1 ~140–200 dòng; P2 sẽ thêm pool):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WorkerMode {
    #[default]
    Off,
    Local,
    // P2 mới thêm `Parallel(usize)` cùng pool/board để P1 không có variant chết.
}

#[derive(Debug, Default)]
pub(crate) struct ExecState {
    pub(crate) mode: WorkerMode,
    /// Task.id theo thứ tự spawn — P2 join/final flush dùng; P1 chỉ ghi + test.
    #[allow(dead_code)]
    pub(crate) order: Vec<u32>,
}

/// W=1: tạo task đúng `Scheduler::spawn` cũ + ghi order; trả `Value::Task`.
pub(crate) fn spawn_local(
    host: &mut dyn Host,
    than: u32,
    args: Vec<Value>,
    ten: String,
    span: Span,
) -> R<Value>;

/// W=1: `await_task_off` (chạy `chay_than_local` trên caller, cache như cũ).
pub(crate) fn await_local(
    host: &mut dyn Host,
    t: &Rc<Task>,
    span: Span,
) -> R<Value>;

/// W=1: `join_all_off` theo thứ tự `order`, không thread/buffer.
pub(crate) fn join_local(host: &mut dyn Host) -> R<()>;

// Trong `rt.rs`: thân cũ đổi tên, `pub(crate)` để `exec` gọi lại.
pub(crate) fn await_task_off(host: &mut dyn Host, t: &Rc<Task>, span: Span) -> R<Value>;
pub(crate) fn join_all_off(host: &mut dyn Host) -> R<()>;
```

Hook tối thiểu:

| Tệp | Thay đổi | Dòng |
|---|---|---:|
| `crates/vis-interp/src/rt.rs` (1.246/1.500; `Scheduler` 465, `await_task` 1143, `join_all` 1239) | `pub(crate) mod exec;`; `Scheduler.exec: exec::ExecState`; `await_task` → dispatcher Off/Local; `join_all` → dispatcher; đổi `wrap_task_error` thành `pub(crate)`; tách `await_task_off`/`join_all_off` | +14/−0 |
| `crates/vis-interp/src/interp.rs` (1.423/1.500; `HExpr::Spawn` 1073–1110) | `#[path = "interp/worker.rs"] mod worker;`; `HExpr::Spawn` gọi `rt::exec::spawn_local(self, f.sym, pos, f.name.clone(), *span)` | +2/−3 |
| `crates/vis-interp/src/interp/worker.rs` (mới) | `chay_than_local` + `#[cfg(test)]` nhỏ | +60–90 |
| `crates/vis-interp/src/rt/exec.rs` (mới) | `WorkerMode`, `ExecState`, `spawn_local`, `await_local`, `join_local` | +140–200 |
| `crates/vis-interp/src/lib.rs` (313) | `run_workers_local*` ẩn; refactor `run_with_hosts_caps_mode(..., mode)` để không lặp 70 dòng | +50–70 |
| `crates/vis-interp/src/host.rs` (104) | (tuỳ chọn P1) stub `worker_snapshot` trả `"host không hỗ trợ worker"`; P2 điền | +18–25 |
| `crates/vis-cli/tests/vs3_worker_eval_local.rs` (mới) | test P1 (xem 6.4) | +250–300 |

**Không đụng P1**: `value.rs` (1.500/1.500), `frame.rs` (1.494), `vm.rs` (1.484),
`rt/worker.rs` (chỉ P2), `net.rs`, `Cargo.toml`, `VERSION`, SPEC.

### 6.3 API ẩn cho test/CLI (lib.rs)

```rust
#[doc(hidden)]
pub fn run_workers_local(
    text: &str, hir: &Hir, symbols: &Symbols, types: &TypeTable, args: &[String],
) -> RunOutput;

#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn run_workers_local_with_hosts_caps<'a>(
    text: &'a str, hir: &'a Hir, symbols: &'a Symbols, types: &'a TypeTable,
    args: &[String],
    ngoai: Option<&'a mut dyn crate::builtin::Ngoai>,
    fs: Option<&'a mut dyn crate::builtin::FsHost>,
    net: Option<&'a mut dyn crate::builtin::NetHost>,
    db: Option<&'a mut dyn crate::builtin::DbHost>,
    caps: &[(u32, Value)],
) -> RunOutput;
```

Cả hai set `it.sched.borrow_mut().exec.mode = WorkerMode::Local` **sau** `Interp::new` và trước
`run_program`; `run_with_hosts_caps` hiện tại truyền `WorkerMode::Off` nên **default không đổi**.
Không thêm env/CLI; W=1 chỉ dùng trong test/example. P1 dùng entry **non-frame** như
`run_with_hosts_caps`, nên `chan(0)` vẫn `VS0505` như default non-frame — parity frame async là
P4, không phải P1.

### 6.4 Test cần thêm

`crates/vis-cli/tests/vs3_worker_eval_local.rs` (mới, ~250–300 dòng):

| Test | Kỳ vọng |
|---|---|
| `local_bang_default_sync` | chương trình `print`/`var`/`while`: stdout/exit/steps/diag **bằng** `vis_interp::run` (non-frame) |
| `local_bang_default_async_spawn_await` | `async fn` + `spawn`/`await` (không `chan(0)`): cùng output default non-frame |
| `local_chan0_van_nhu_non_frame` | `chan(0)` cho **cùng** `VS0505` như default non-frame; parity async frame là P4 |
| `local_await_hai_lan_tra_cache` | await lần hai trả cache, không chạy thân lần hai, steps không đổi |
| `local_join_task_khong_await` | task không ai await vẫn chạy + output đúng spawn order; lỗi surfaced `VS0506` |
| `local_loi_than_boc_vs0506` | task trap giữ mã gốc + span lúc tạo, `exit=1` |
| `local_khong_tao_thread` | đếm `/proc/self/task` trước/sau bằng nhau (Linux; skip nền khác) |
| `local_host_fs_net_db_van_chay` | W=1 full host như default (không bị chặn như W>1) |
| `local_khong_doi_abi` | `VBC1` round-trip + `Value::Task` `Show` = `<tác vụ>`; không đổi `VERSION`/cache key |

`crates/vis-interp/src/rt/exec.rs` `#[cfg(test)] mod tests` (~40 dòng):

| Test | Kỳ vọng |
|---|---|
| `mode_mac_dinh_off` | `ExecState::default().mode == Off` |
| `local_spawn_ghi_order` | 3 spawn ⇒ `order = [1,2,3]` |
| `local_await_khong_chay_lai` | cache + `DangChay` giữ `VS0501` như cũ |

P2 mới thêm `crates/vis-cli/tests/vs3_worker_eval_10k.rs` (W=1 vs W=4, cross/budget/cancel,
10k) — example hiện tại là bản chạy tay trước khi có API ẩn.

### 6.5 Ước lượng và thứ tự P1

1. `rt/exec.rs` + `rt.rs` dispatch + `interp/worker.rs` + hook `HExpr::Spawn` (W=1 = oracle).
2. `lib.rs` `run_workers_local*` + refactor mode.
3. Test `vs3_worker_eval_local.rs`; so 4 chương trình với default; đếm thread.
4. Gate: `cargo test -p vis-cli --test vs3_worker_eval_local`, `vs3_default`, `vs3_worker`,
   `./vis-dev check`; R1; không đổi ABI.

Ước lượng P1: **~270–390 dòng production**, **~250–300 dòng test** (khớp plan §6.2
250–350 production; phần `ExecState` P1 nhỏ hơn vì chưa có pool/board/proxy).

---

## 7. Blocker

- **Không có blocker kỹ thuật mới.** 10k + soak chứng minh ảnh chương trình dùng chung,
  evaluator độc lập, checksum tất định, không rò thread/fd.
- Blocker **quyết định** vẫn giữ nguyên từ plan: mã lỗi cross-write W>1 (`VS0509`?) và chính
  sách W>1 bỏ frame/`chan(0)`/host (A0). P1 W=1 **không** cần hai quyết định này.
- `Interp::new` vẫn `pub(crate)`: P1 phải thêm `run_workers_local*`/runner trong crate; example
  ngoài crate chỉ đo được gián tiếp qua `vis_interp::run`.

---

## 8. Lệnh đã chạy

```bash
~/.cargo/bin/cargo check -p vis-cli --example vs3_worker_eval_proto
~/.cargo/bin/cargo run -q -p vis-cli --example vs3_worker_eval_proto          # N=64
VS3_EVAL_N=10000 VS3_EVAL_ITERS=1 \
  ~/.cargo/bin/cargo run -q -p vis-cli --example vs3_worker_eval_proto       # 3 lần
VS3_EVAL_N=10000 VS3_EVAL_ITERS=1 VS3_EVAL_W=4 VS3_EVAL_SOAK_SECS=600 \
  ~/.cargo/bin/cargo run -q -p vis-cli --example vs3_worker_eval_proto
~/.cargo/bin/cargo check --all-targets
~/.cargo/bin/cargo fmt --all -- --check
~/.cargo/bin/cargo clippy -p vis-cli --all-targets -- -D warnings
~/.cargo/bin/cargo test -q -p vis-cli --test vs3_worker -- --test-threads=1
```

- R1: example **689 dòng** ≤ 1.500; không đụng production.
- `cargo check --all-targets`, fmt, clippy `-D warnings`: **PASS**.
- `vs3_worker`: **14/14 PASS**.
- `df /home`: **11G** trống (>5G); không GitHub Actions, không push/merge/tag.
