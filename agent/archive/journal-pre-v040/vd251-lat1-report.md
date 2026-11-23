# ST-267 / VD-251 — LÁT 1: nối frame scheduler vào interp & VM thật

- Ngày: 2026-11-20
- Phạm vi: **chỉ đường test/debug nội bộ**. Không commit. Không đụng `agent/DECISIONS.md`,
  `agent/QUEUE.md`, `agent/JOURNAL/vd251-charging-short.md` (đã do Agent 0 ghi).
- Không CLI flag, không API ngôn ngữ, không reactor/socket nonblocking, không đổi
  bytecode/opcode/VERSION/cache-key, không dependency, không `unsafe` mới.

## 0. Khẳng định biên (đọc đầu)

**Runtime mặc định, bytecode phát hành, `Cargo.toml` VERSION, cache key và API ngôn ngữ
KHÔNG đổi.** Mọi mã mới nằm ở (a) các hàm/hàm `#[doc(hidden)]` trong `vis-interp::rt`,
(b) module `vis_interp::frame`, (c) module `vis_vm::vm_frame`, (d) một hook 1 dòng trong
nhánh `CallBuiltin` của VM, (e) một nhánh `Abort::Pending` trong `match` lỗi. Khi
`Scheduler.frame.enabled == false` (mặc định), mọi nhánh cũ chạy **nguyên văn** như trước.
`vis-dev check` xanh (fmt/clippy/R1..R6/self-test/build/doc-examples) là bằng chứng build
và fixture không regression. B1/B2 giữ nguyên baseline (§5).

**Đây không phải Lát 0 lặp lại.** Lát 0 (`lat0_suspend.rs`) là model trong integration test
với `V`/`IFrame` tự dựng. Lát 1 dùng `Interp`/`Vm` **thật**: parse → lower → check → HIR/bytecode
thật → `Interp`/`Vm` thật → `builtin::call`/`ops::bin`/`Value`/`Trap`/`Scheduler` thật.
`lat0_suspend.rs` không bị sửa và vẫn 10/10 xanh.

## 1. Criteria → evidence table

Cả 8 test nằm trong `crates/vis-cli/tests/lat1_frame.rs` (`chay()` ở dòng 44 so sánh
**byte** stdout, `(mã, message, dòng, cột)` chẩn đoán, trace `log`, và `DiagSnapshot`
giữa hai engine). Sources P1/P2/P3 đi qua `parse`→`lower`→`check` thật (`tien_trinh`,
dòng 15).

| # | Tiêu chí | Test (dòng) | Bằng chứng của **cả hai engine** |
|---|---|---|---|
| 1 | A Pending → B chạy → A resume đúng chỗ | `c1_a_pending_b_runs_a_resumes` (:98) | stdout `A:pre\nB:run\nA:post\n48\n5\n`; log `wait:1:Event{key:1,gen:0}` trước `wake:1` rồi `wake:0`. Interp: `rt.rs:564` trả `Abort::Pending`, work stack + operand được khôi phục (`frame.rs:600-624`); VM: `vm_frame.rs:28-54` khôi phục `pc -= 1` + đối số `$await`, giữ `Th`. |
| 2 | Local/capture giữ giá trị; không lặp side effect | `c2_locals_survive_and_no_repeated_side_effect` (:115) | `48 = add1(7 + 40)`: local `y = 7` sống qua suspend. `A:pre`/`B:run`/`A:post` mỗi cái đúng 1 lần. VM dùng `Frame.locals`/`captured`/`st_cu` nguyên văn (`vm.rs:124-135`); interp dùng `CallFrame.env` trên heap (`frame.rs:84-92`). |
| 3 | Gọi hàm lồng **và** await trong biểu thức lồng | `c3_nested_call_and_await_inside_nested_expression` (:125) | `print(add1(await a))`: `await` là đối số của hàm `add1` (frame con thật trong interp `frame.rs:694`; `Instr::Call` thật trong VM); log parity. |
| 4 | Lỗi thật qua Trap/VS0506; await lần 2 trả cache | `c4_real_error_vs0506_and_cached_on_second_await` (:152) | `1 % 0` ⇒ `VS0502` gốc; hai task `waiter` cùng `await` task `bad` đó ⇒ `bad:run` in **1 lần**, chẩn đoán duy nhất `VS0506` có chứa `VS0502`/`bad` (bọc bởi `rt.rs:469` → `wrap_task_error`). Cache là nhánh `TrangThai::Xong` ở `rt.rs:544` (chạy trước nhánh frame). |
| 5 | Task không await vẫn final-join; không auto-cancel | `c5_unawaited_task_still_final_joined` (:184) | stdout `main\nlonely\n` (P3): main kết thúc nhưng driver tiếp tục xếp task `Moi` vào đuôi `frame.ready` (interp `frame.rs:275-283`, VM `vm_frame.rs:180-188`), chạy xong mới dừng. |
| 6 | Duplicate/stale wake không làm task chạy sai | `c6_duplicate_and_stale_wake_do_not_rerun` (:194) | Trace có event `(seq1,key1,gen0)` trùng + `(seq2,key1,gen99)` stale ⇒ log `duplicate:1`, `stale:1`; `wake:1` đúng 1 lần, kết quả vẫn `48`. Chặn tại `rt.rs:339-380` (`fired` map + so `gen`; wake nối đuôi, loại trùng ready). |
| 7 | Thu hồi theo ownership, kiểm bằng diag_snapshot | `c7_reclaim_verified_by_diag_snapshot` (:216) | `DiagSnapshot{live_tasks:0,frames:0,waiters:0,ready:0}` ở cả hai engine; log có đúng 3 dòng `reclaim:` (task 1,2,3). `rt.rs:432-455`: điều kiện = `Xong` ∧ `joined` ∧ `Rc::strong_count == 1`; `ready`/`waiters`/`external` cũng bị dọn. `diag_snapshot()` ở `rt.rs:457` (chỉ test/debug). |
| 8 | Fixture cũ **phân loại**, không đổi golden | `c8_existing_fixtures_still_pass_on_default_path` (:238) | 3 fixture `test/sem/async/*.vis` + `.out` đọc nguyên văn, chạy qua đường **mặc định** (`vis_interp::run` / `vis_vm::run`) — stdout khớp golden. `git status` không có tệp `test/**` hay golden nào bị sửa. |

Tổng: `cargo test -p vis-cli --test lat1_frame` → **8 passed / 0 failed**.
Ngoài ra `cargo test -p vis-interp --test lat1_frame` → **1 passed** (smoke interp đơn).
`cargo test -p vis-interp -p vis-cli` → **1007 passed / 0 failed** (đã gom mọi `test result:`).
`./vis-dev check` → **PASS**.

## 2. Đường interp thật được đổi thế nào

1. `crates/vis-interp/src/interp.rs:70-75` — thêm biến thể `Abort::Pending(Pending)`.
   Runtime mặc định không bao giờ phát ra vì nhánh phát Pending nằm sau
   `if host.sched().borrow().frame.enabled` (`rt.rs:564`).
2. `crates/vis-interp/src/rt.rs` — `TrangThai::Pending` (:37) + `FrameTest` (:75-150) nằm
   trong `Scheduler.frame` (:~168). Các hàm test-only: `frame_enable_for_test` (:259),
   `frame_rearm_for_test` (:274), `frame_await` (:289), `frame_register` (:308),
   `frame_next` (:325, FIFO + fire event theo `seq` recorded trong trace), `frame_fire`
   (:339), `frame_wake_one` (:384), `frame_task_done` (:397), `frame_drain_events` (:411),
   `frame_final_join` (:420), `frame_reclaim` (:432), `diag_snapshot` (:457).
   `tao`/`spawn` (:~200-240) nhận diện nguồn giả **theo tên hàm** — `await f()` dùng `tao`,
   `spawn f()` dùng `spawn` — cùng đi một cơ chế. `await_task` giữ nguyên nhánh cache
   `Xong` đứng trước; chỉ khi `frame.enabled` mới trả `Abort::Pending` thay vì chạy thân.
3. `crates/vis-interp/src/frame.rs` (mới) — `run_frame_for_test` (:129) dựng `Interp`
   thật từ HIR thật, rồi điều khiển `IFrame` (:84) gồm `Vec<CallFrame>` + work stack +
   `vals`, tất cả trên heap. `step_frame_owned` (:358) là vòng lặp phẳng (`loop`, không
   đệ quy scheduler). `Work::ApplyAwait` (:600) gọi **chính** `crate::rt::await_task`;
   trên `Abort::Pending` nó đẩy lại `ApplyAwait` + operand vào stack ⇒ resume không eval
   lại operand và không lặp side effect. `dispatch` (:694) gọi `builtin::call` thật, tạo
   frame hàm con thật cho `Call` thường; `call_hfn`/`eval` đệ quy **không bị sửa**.

## 3. Đường VM thật được đổi thế nào

1. Một hook 1 dòng tại `vm.rs:860` (nhánh `Instr::CallBuiltin`): thay
   `builtin::call(...)` bằng `crate::vm_frame::call_builtin_frame(...)`. Hàm này
   (`vm_frame.rs:28`) với `frame.enabled == false` trả **nguyên văn** `builtin::call`;
   với frame mode, khi `$await` báo `Abort::Pending`, nó đẩy lại đối số đã pop và trừ
   `pc` đi 1 ⇒ instruction `$await` chạy lại lúc resume (khi task đã cache). `Th::exec`
   vì thế trả `Err(Abort::Pending)` mà **không làm mất trạng thái** — mọi thứ nằm trong
   `Th { frames, st, iters, loop_ends }` (`vm.rs:617`) vốn đã ở heap.
2. `crates/vis-vm/src/vm_frame.rs` (mới) — `run_frame_for_test` (:56) dựng `Vm` thật,
   verify bytecode thật, `init_globals`, dựng `Th` main (:244) theo đúng logic entry
   của `Vm::chay_chuong_trinh` (VD-156), rồi chạy driver: mỗi `Th` được giữ trong map
   `frames` qua suspend. `task_th` (:232) dựng `Th` cho thân `async fn` từ **chỉ số hàm
   bytecode thật**. Không opcode/bytecode/VERSION/cache-key nào đổi; chỉ visibility
   (`pub(crate)`) của field/phương thức VM để module con dùng.
3. `vm.rs:265` thêm nhánh `Abort::Pending` trong `match` lỗi của `run()` — chỉ để
   exhaustivity; `run()` mặc định không bật frame nên không bao giờ tới đây.

## 4. Test-only vs mặc định; giới hạn phủ

**Chỉ test/debug** (không gọi từ CLI/API ngôn ngữ): `vis_interp::frame::run_frame_for_test`,
`vis_vm::vm_frame::run_frame_for_test`, mọi hàm `frame_*` trong `rt.rs`, `diag_snapshot()`,
`wrap_task_error_for_test`, enum `Waiter`/`Wait`/`FakeEvent`/`DiagSnapshot`, biến thể
`Abort::Pending`, cờ `Scheduler.frame.enabled`. Tất cả `#[doc(hidden)]`, tên `*for_test`.

**Mặc định không đổi**: `vis run` → `run_program`/`Vm::run` → `await_task` run-to-completion
→ `join_all` theo thứ tự tạo; `goi_than` nguyên văn; `SIGS` không thêm builtin; checker
(`vis_types::check`) không đổi tập vị trí `await` chấp nhận.

**Giới hạn phủ (thẳng thắn, không coi là đã xong):**
- Interp frame evaluator (`frame.rs`) chỉ phủ: `Lit`, `Var`, `Bin`, `Call`/`Await`/`Spawn`
  với callee là **tên hàm cấp tệp hoặc builtin**, `Let`/`Var`/`Expr`/`Return`, pattern
  `Bind`/`Wild`. **Chưa phủ**: `&&`/`||`, `vòng lặp`, `if`/`match`, field/index, record/tuple,
  closure/lambda làm callee, hằng cấp tệp, đối số tên, và multi-frame continuation qua
  closure capture. Checker vẫn chấp nhận mọi vị trí đó như trước — chỉ đường test Lát 1
  không mô phỏng được chúng. Đây là analyze gap có chủ đích, không phải regression.
- VM dùng bytecode thật nên phủ rộng hơn; tuy nhiên task frame trong test mode khởi tạo
  `captured = Vec::new()`/`writeback = None` (không đi đường `MakeClosure` của
  `goi_ham_bytecode`) — closure của async task chưa được kiểm chứng ở frame mode.
- **Step-cap parity vẫn MỞ** (VD-249/ST-260): interp đếm bước ngữ nghĩa, VM đếm từng
  lệnh, cùng ngưỡng 10^7 ⇒ 2M vòng lặp: interp exit 0 / VM trap `VS0506→VS0501`. Lát này
  **không** sửa, **không** claim parity cho workload chạm giới hạn; không triển khai ngân
  sách ngữ nghĩa/hạn ngạch CPU (để Lát sau, chờ A0).
- Không reactor, không socket nonblocking, không timer/fd thật: event là cờ/giá trị do
  trace cung cấp. Không hứa parity cho thứ tự hoàn thành mạng thực tế.

## 5. B1/B2 baseline (chạy lại, KHÔNG yêu cầu cải thiện)

Recreate tối giản trong `/tmp/opencode/` (`vd251-server.py` delay 0,2 s; B1 = 20 task
`net.read` 200 ms + MARK; B2 = CPU 500k vòng + I/O có `time.now_ms`). Chạy `vis run`
(interp) và `vis run --vm --no-cache` trên cùng binary `target/debug/vis`.

| Ca | Engine | Kết quả | So với baseline cũ |
|---|---|---|---|
| B1 (20×200 ms) | interp | `MARK` cuối, **wall 4.04 s** | ~4.11 s — giữ nguyên (I/O vẫn chặn) |
| B1 (20×200 ms) | VM | `MARK` cuối, **wall 4.04 s** | ~4.11 s — giữ nguyên |
| B2 (CPU+I/O) | interp | `CPU_DONE 1795154762558`, `IO_START 1795154762558`, `IO_DONE 1795154762759 ok` | `CPU_DONE` = `IO_START` **cùng ms**, `IO_DONE−IO_START = 201 ms` — đói nguyên vẹn |
| B2 (CPU+I/O) | VM | `CPU_DONE 1795154763498`, `IO_START 1795154763498`, `IO_DONE 1795154763699 ok` | cùng hiện tượng, `201 ms` |

Kết luận: Lát 1 **không** đổi B1/B2 (đúng dự kiến). CPU fairness / nonblocking I/O là
việc Lát 2+, không tự động làm ở đây.

## 6. Ghi chú validation

- `./vis-dev check` → `PASS` (fmt OK; clippy OK; R1..R6 OK; self-test 13/13; build OK;
  doc-examples 24 khối — 19 xanh + 5 ngoại lệ đỏ đúng; fixtures PASS).
- `cargo test -p vis-interp -p vis-cli` → **1007 passed / 0 failed**.
- `cargo test -p vis-cli --test lat1_frame` → **8/8**; `cargo test -p vis-interp --test
  lat1_frame` → **1/1**; `lat0_suspend` → **10/10** (không sửa).
- `./vis-dev gate --fast` đã thử nhưng **dừng ở self-test case âm do `Disk quota exceeded`
  khi script copy cây sang `/tmp/tmp.*`** (lỗi môi trường, không phải regression mã).
  `vis-dev check` chạy trực tiếp trên repo xanh; không claim `gate --fast` xanh.
- Lần đầu chạy `cargo test -p vis-interp -p vis-cli` sau `gate --fast` bị lỗi import
  do artifact `target/` bị stale từ bản copy của gate; đã `cargo clean -p ...` và chạy lại
  → 1007/0. Không đổi mã để "sửa" lỗi artifact đó.
