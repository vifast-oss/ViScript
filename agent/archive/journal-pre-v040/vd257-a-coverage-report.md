<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-257 · Stream A — Báo cáo đóng coverage/path-frame còn lại

**Branch**: `work/vd257-a-coverage` (base `fc297ba`). **Trạng thái**: mã + test xong, commit trên branch,
KHÔNG merge/push. Resource discipline: chỉ chạy test đích `lat257_a_coverage` (+ 2 file test kề để
so regression), `cargo fmt --check`; **không** gate/suite/benchmark, **không** `cargo clean`.

---

## 0. Bất biến cứng — xác nhận

| Bất biến | Kết quả / bằng chứng |
|---|---|
| Checker acceptance không thu hẹp | ✅ Mọi nguồn dương trong test đều `check().is_clean()` (`lat257_a_coverage.rs::tien_trinh:22`); checker không sửa. |
| Không fallback đồng bộ che ô hở | ✅ `cmd` Trap RÕ tại điểm gặp `frame/expr.rs:343`; kênh đầy/rỗng/đóng Trap RÕ `VS0505`/`VS0504` (`b3_...`); bytecode VM hỏng → `exit=1` + "bytecode hỏng" (`e1_...`). Không gọi `Interp::run_program`/`Vm::run` trong frame path. |
| Resume không re-eval / side-effect đúng 1 lần | ✅ `?` short-circuit bỏ vế sau (`a1`, `a3`: `SAU` đếm = 1); `f1` so stdout `"40\n40\n"` (cache) + trace không trùng `(pc,kind,site)`. |
| Runtime mặc định không đổi | ✅ Không sửa `interp.rs`, `rt.rs`, `builtin.rs`, `vm.rs`, `gan.rs`, `fee_shared.rs`; `a1` so `vis_interp::run` stdout bằng frame path; `c1` so thông điệp `cmd` với runtime mặc định. |
| VERSION / bytecode / cache / golden | ✅ Không sửa `Cargo.toml`, `Cargo.lock`, opcode, `VERSION`, fixture hay `.out`/`.err`. `git status test/` sạch. |
| Không implement budget | ✅ `fee.rs` (hook VD-256) không đụng; `f2` bật hook mà trace interp frame vẫn rỗng; `f1` chỉ **dùng** hook có sẵn qua test-hook mới. |

---

## 1. Bảng bằng chứng từng ô (test name + file:line)

| Ô (VD-257 §4) | Trạng thái | Test | Vị trí | Engine |
|---|---|---|---|---|
| 1a. `?` trên `Option`: `Some` bóc, `None` trả sớm + bỏ side-effect sau | **SUPPORTED** | `a1_try_option_unwrap_and_early_return_short_circuit` | `lat257_a_coverage.rs:109`; work `frame.rs:342`, `expr.rs:324`, `proc.rs:971` | I + V |
| 1b. `?` trên `Result`: `Ok` bóc, `Err` trả sớm + fixture `hoi-cham-ok.vis` | **SUPPORTED** | `a3_try_result_early_return_and_fixture_ok` | `lat257_a_coverage.rs:154` | I + V |
| 1c. `xs[i]?` — chỉ số có kiểm tra ⇒ `T?` (VD-178 R8), không trap | **SUPPORTED** | `a2_try_index_checked_returns_option` | `lat257_a_coverage.rs:127`; `proc.rs:1000` | I + V |
| 2a. `chan[T](n)` — tạo kênh (dùng `$kenh` thật, không tự dựng `Value::Chan`) | **SUPPORTED** | `b1_chan_create_send_recv_close`, `b2_chan_async_fixtures_frame_path` | `lat257_a_coverage.rs:185,209`; `expr.rs:336`, `proc.rs:1021` | I + V |
| 2b. `send`/`recv` không chặn (đệm còn chỗ), `close` idempotent, `recv` sau close trả nốt rồi `None` | **SUPPORTED** | `b1_...` | `lat257_a_coverage.rs:185` | I + V |
| 2c. Hai fixture chan thật (`grid-chan-hai-task-kenh`, `grid-kenh-list`) qua frame path = golden | **SUPPORTED** | `b2_...` | `lat257_a_coverage.rs:209` | I + V |
| 2d. `send` khi kênh **đầy** / `recv` khi kênh **rỗng** (chặn) | **EXPLICITLY REJECTED** (`VS0505` deadlock, cùng thông điệp 2 engine, stdout rỗng) | `b3_chan_full_empty_closed_explicit_rejection` | `lat257_a_coverage.rs:239` | I + V |
| 2e. `send` sau `close` | **EXPLICITLY REJECTED** (`VS0504`) | `b3_...` (case `send-after-close`) | `lat257_a_coverage.rs:252` | I + V |
| 3. `cmd { … }` — VS1 | **EXPLICITLY REJECTED** (cùng thông điệp runtime mặc định; VM compile trả `Unsupported("cmd-block")`) | `c1_cmdblock_rejected_clearly_matches_default_runtime` | `lat257_a_coverage.rs:275`; `expr.rs:343` | I (trap) + V (compile) |
| 4. `await` trong **thân closure** | **KHÔNG HOÀN TOÀN** — interp chạy; VM codegen sinh bytecode hỏng ⇒ verify chặn rõ | `e1_closure_body_await_interp_supported_vm_explicit_rejection` | `lat257_a_coverage.rs:360` | I ✅ / V ❌ (xem §4) |
| 5a. Gán `=` trên **trường** (`p.x = …`) và **phần tử** (`xs[i] = …`) | **SUPPORTED** | `d1_field_and_index_assignment_on_frame_path` | `lat257_a_coverage.rs:305` | I + V |
| 5b. Phân biệt method **host** (list/str/chan → `builtin::method`) vs **giá trị hàm** (closure → `call_value`) | **SUPPORTED** | `d2_host_method_vs_closure_value_dispatch` | `lat257_a_coverage.rs:324`; `proc.rs:764-865` | I + V |
| 5c. Gán ghép trên trường/phần tử (`p.x += 1`) | **EXPLICITLY REJECTED bởi PARSER** (`VS0201`) — không bao giờ tới frame path | `d2b_compound_field_assign_rejected_by_parser` | `lat257_a_coverage.rs:345` | parse |
| 5d. `r.f(4)` với `f` là trường chứa closure (user-method qua field) | **EXPLICITLY REJECTED bởi CHECKER** (`VS0403`: "không có phương thức `f`") | ghi chú §4 (không có test dương) | — | tyck |
| 6a. Resume **không double-charge** (VM, hook VD-256) | **VERIFIED** (mỗi `(pc,kind,site)` charge đúng 1 lần; task chạy giữa suspend/resume) | `f1_resume_khong_charge_lai_nhom_cu_vm_budget` | `lat257_a_coverage.rs:398`; test-hook `vm_frame.rs:147` | V |
| 6b. Interp frame path **không tự charge** (không implement budget) | **VERIFIED** (trace rỗng dù hook bật) | `f2_interp_frame_path_khong_tu_charge` | `lat257_a_coverage.rs:444` | I |

Validation thật: `cargo test -p vis-cli --test lat257_a_coverage` → **13 passed / 0 failed**;
`cargo fmt --check` → sạch; regression kề: `lat2_reactor` 13/13, `lat255_coverage` 7/9 (2 ca cũ lỗi thời — §5).

---

## 2. Thay đổi mã (đều nằm trong module frame được giao)

- `crates/vis-interp/src/frame.rs:339-360` — thêm 4 variant `Work`: `ApplyTry`, `ApplyTryIndex`,
  `BuildChan`, `RedispatchMethod`. **Ghi chú trung thực**: `frame.rs` là **gốc của module `frame/**`**;
  enum `Work` nằm ở đây nên không thể thêm variant chỉ bằng `frame/*.rs`. Đây là tệp duy nhất ngoài
  thư mục con `frame/` mà stream A sửa; không stream nào khác đụng tệp này (đã kiểm tra 3 worktree).
- `crates/vis-interp/src/frame/expr.rs:324-352` — `eval_work` xử lý `HExpr::Try` (2 dạng), `HExpr::ChanNew`,
  và cho `HExpr::CmdBlock` trap với thông điệp VS1 của runtime mặc định (thay vì cụm "chưa được đường
  frame test hỗ trợ" gây hiểu lầm).
- `crates/vis-interp/src/frame/proc.rs:809-863, 971-1030, 1034` — `ApplyTry` (trả sớm bằng cách xoá
  `cf.work`/`cf.loops` và đặt `last`, khớp `Instr::Try` của VM), `ApplyTryIndex` (`Value::opt`,
  không panic), `BuildChan` (gọi `$kenh`), `RedispatchMethod` + `method_writeback` — continuation giữ
  **đối số đã eval** nếu method host trả `Abort::Pending` (phòng thủ; xem §4 mục 3 — chỗ chưa có ca dương).
- `crates/vis-vm/src/vm_frame.rs:142-168` — tách thân driver thành `run_frame_with_budget(..., Option<(FeeTable,u64)>)`;
  giữ nguyên `run_frame_for_test` (gọi với `None`); thêm `run_frame_for_test_with_budget` trả kèm trace.
  **Không** sửa `budget_charge`/`call_builtin_frame`/`vm.rs` — stream A không implement/đổi budget.

Không có thay đổi nào với `rt.rs`, `interp.rs`, `stdlib.rs`, `builtin.rs`, `vm.rs`, `vis-bytecode/**`,
`cache.rs`, `net.rs`, manifest/lock, `DECISIONS/QUEUE`, journal/test của stream khác.

---

## 3. Golden

**KHÔNG CÓ golden nào bị đổi.** Không tệp `.out`/`.err` trong `test/sem/` được sửa (`git status test/` sạch).
`b2` và `a3` **đọc** fixture/golden có sẵn rồi so sánh byte-for-byte; `c1`/`b3` giữ stdout rỗng hoặc
dừng đúng điểm bị từ chối. Runtime mặc định vẫn trả đúng golden (đã so ở `a1`, `c1`, và trong `b2`
bằng `chay` + đối chiếu golden).

---

## 4. Danh sách **CHƯA HOÀN TẤT** (không tuyên bố coverage complete)

1. **`await` trong thân closure trên VM** — interp chạy đúng (`e1`: stdout `"42\n"`), nhưng
   `vis_bytecode::compile` sinh bytecode bị `verify` từ chối `"bytecode hỏng: ngăn xếp rỗng"`
   ⇒ `run_v` exit=1. Đây là lỗi **codegen trong `crates/vis-bytecode`**, nằm ngoài file stream A được
   cấp (cấm sửa). Frame path từ chối **rõ** (không fallback), nhưng ô này **chưa supported trên VM**.
   Gợi ý A0: giao cho stream/codegen sửa rồi bật ca dương song song. Biến thể có async lồng nhau
   (`fn() -> await src()` với `src` lại `await`) còn làm interp lặp scheduler (`guard > 200_000`) —
   cũng ghi nhận, không sửa ở stream A.
2. **Blocking `send`/`recv` trên kênh** — frame path trả `VS0505` deadlock (vì `chay_mot` đọc
   `sched.ready` vốn rỗng trên đường frame). Đây là **từ chối rõ đúng bất biến VD-252**, nhưng chưa phải
   "mô phỏng chờ kênh có suspend/resume". Không có ca dương cho "kênh đầy → task khác recv → resume".
   Cần thiết kế wait theo kênh (đụng `rt.rs`) — ngoài phạm vi stream A.
3. **`RedispatchMethod` (proc.rs:839)** chỉ là continuation phòng thủ cho `Abort::Pending` từ method
   host; trên đường frame hiện `chay_mot` không trả Pending nên **chưa có ca dương chạy qua nhánh này**.
4. **`r.f(4)` (user-method qua field)** bị checker chặn `VS0403`; chỉ có nhánh host-method của
   `ApplyMethodOrField` được test dương. Nhánh `field_of → ApplyValueCall` được test gián tiếp qua
   gọi closure trực tiếp (`d2`), không phải qua cú pháp `r.f(...)`.
5. **Gán ghép trên trường/phần tử** (`p.x += 1`) bị parser chặn `VS0201` — ô này "không áp dụng"
   chứ không phải lỗi evaluator.
6. **Set/Map/EnumLit/Interp/let-else** kế thừa từ VD-255 vẫn chưa có test riêng trong file của stream A
   (đã có implement + test ở `lat255_coverage.rs`; stream A không nhân bản).

---

## 5. ⚠️ Test VD-255 cũ cần A0/leader cập nhật khi integrate (stream A KHÔNG được sửa)

Việc `?` và `chan` giờ được hỗ trợ làm **2 assertion cũ trong `crates/vis-cli/tests/lat255_coverage.rs`
lỗi thời** (không phải regression — chúng khẳng định "bị từ chối", giờ đúng là "được hỗ trợ"):

- `z_unsupported_structures_still_rejected_clearly` (`lat255_coverage.rs:355-383`): bỏ/đảo 2 case
  `?` (dòng 358-368) và `chan` (dòng 370-382); gi lại case `HExpr::Error` nếu còn.
- `z_all_async_fixtures_classified_and_run_through_frame_path` (`lat255_coverage.rs:432`):
  `classify()` dòng 426 phải đổi `"grid-chan-hai-task-kenh" | "grid-kenh-list" => "frame"`; nhánh
  `"chan"` (dòng 526-545) đổi sang so stdout với golden `.out` như nhánh `"frame"`.

Tiền lệ: VD-255 đã đổi `lat2_reactor.rs::r9/r10` từ "expect rejection" sang "expect support" khi
`&&`/const được phủ (`vd255-coverage-report.md:111-113`). Stream A không sửa test của task khác; đề
nghị leader áp 2 patch trên (hoặc cho phép stream A sửa trong một lệnh riêng).

---

## 6. Kết luận ngắn

Đã đưa `?` (Option/Result + `xs[i]?`), `chan` (tạo/send/recv/close + closed-state), `cmd` (từ chối
đúng VS1), gán trường/chỉ số và phân biệt host/value method vào đường frame thật trên **cả hai engine**,
và **chứng minh bằng chạy thật** rằng resume không double-charge và interp frame không tự charge.
Phần còn mở (VM closure-await, chờ kênh blocking, 1 nhánh continuation phòng thủ) được liệt kê rõ ở §4;
không fallback âm thầm, không đổi checker/runtime/golden. 13 test đích đều passed.
