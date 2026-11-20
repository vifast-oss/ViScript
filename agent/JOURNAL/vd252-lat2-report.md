<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-268 / VD-252 — LÁT 2: reactor giả + timer monotonic trong đường frame test

- Ngày: 2026-11-20
- Phạm vi: **chỉ đường test/debug nội bộ** của `vis-interp` + `vis-vm`. Không commit.
- Không đụng: `agent/DECISIONS.md`, `agent/QUEUE.md`, `agent/JOURNAL/vd252-*`,
  `rust-toolchain.toml`, `Cargo.toml` (và mọi tệp do workstream khác sở hữu). Các tệp đó có
  thay đổi của workstream khác trong lúc làm việc — **không đụng vào**.
- Không CLI flag, không API ngôn ngữ, không dependency mới, không `unsafe` trong mã dự án,
  không opcode/bytecode/VERSION/cache-key thay đổi.

## 0. Khẳng định biên (đọc đầu)

**Runtime mặc định, bytecode phát hành, `Cargo.toml` VERSION, cache key và API ngôn ngữ KHÔNG đổi.**
Mọi mã mới nằm ở: (a) `rt.rs` (kiểu/hàm `#[doc(hidden)]`, `frame.*`), (b) `frame.rs`,
(c) `vm_frame.rs`, (d) hai nhánh `net.read`/`net.accept` **có điều kiện**
`if host.sched().borrow().frame.enabled` trong `stdlib.rs`, (e) test mới. Khi
`frame.enabled == false` (mặc định) mọi nhánh cũ chạy **nguyên văn** như trước.

**Đây là reactor GIẢ (trace-driven), KHÔNG phải mạng thật, KHÔNG phải sleep-loop.**
`read`/`accept` trong đường test **không gọi `NetHost` thật**, không đọc socket, không poll.
Sự kiện do trace (`seq`, `key`, `gen`) cung cấp và do driver fire theo `seq`. B1 chạy qua đường
test dùng **timer monotonic giả lập delay 200 ms của server** (xem §5) — chứng minh overlap
của scheduler, **KHÔNG phải I/O mạng không chặn thật**. Xem §6 (trạng thái reactor thật).

**Về `thread::sleep(5ms)`**: Lát 1 không có sleep trong đường frame test; Lát 2 cũng **không
thêm** busy-poll nào. Hai `sleep(5ms)` có sẵn (`vis-cli/src/net.rs:109` `accept_tren`,
`vis-cli/src/ngoai.rs:154` `try_wait`) nằm ở **đường mặc định** — **giữ nguyên, không đụng**.
Đường test chỉ có **một lần** `thread::sleep` tới deadline monotonic gần nhất trong
`frame_idle()` khi `timer_real_wait == true` (chỉ ca đo B1; mặc định `false`). Đây là timer
wait, không phải vòng lặp poll readiness.

## 1. Criteria → evidence table

13 test mới trong `crates/vis-cli/tests/lat2_reactor.rs`. Hàm `chay()` (:57) so sánh **byte**
stdout, `(mã, message, dòng, cột)` chẩn đoán, `log` scheduler+reactor, `DiagSnapshot`, `exit`
giữa interp và VM trên **cùng source + cùng trace**. Nguồn đi qua `parse → lower_with(std_mods:
["net"]) → check` thật (:13) và bytecode thật (`vis_bytecode::compile`).

| # | Tiêu chí / bất biến | Test | Bằng chứng (cả hai engine) |
|---|---|---|---|
| 1 | `read` chưa sẵn ⇒ wait + `Pending`; task khác chạy; resume đúng chỗ | `r1_read_pending_other_runs_then_resume` | stdout `R:pre\nO:run\nOk(DATA)\n99\n5\n`; log `reg:0:C1:Read…` → `wait:1:Reactor…` → `wake:1`; `R:pre` đúng 1 lần (không re-run). |
| 2 | `accept` cũng đi reactor | `r2_accept_pending_and_resume_parity` | log `reg:0:L1:Accept…`, stdout `Ok(C9)`; parity tuyệt đối. |
| 3 | Duplicate/stale wake không tạo lượt chạy trùng | `r3_duplicate_and_stale_wake_do_not_rerun`, `r3b_duplicate_fire_direct` | end-to-end: `rstale:0`, `wake:1` đúng 1 lần, giá trị vẫn `DATA`; trực tiếp trên `Scheduler`: `rdup:0`, `wake:1` đúng 1 lần (khi đăng ký còn sống). |
| 4 | Thức giả → WouldBlock → re-register, **không hot spin** | `r4_spurious_wake_reregisters_without_spin` | `rspurious:0:1` → `rstale:0:0` → `reg:0:C1:Read:1…` (gen mới) → `Ok(REAL)`. Guard 200.000 bước không bị chạm. |
| 5 | Close-while-waiting vô hiệu event cũ, gỡ đúng 1 lần | `r5_close_while_waiting_invalidates_old_events` | `rclose:0`, gen tăng; event cũ `rstale`; retry trả `Result.Err` đúng 1 lần; không treo (VM không dính guard). |
| 6 | EOF gỡ waiter đúng 1 lần | `r6_eof_removes_waiter_exactly_once` | `eof:0`; `Ok()` (chuỗi rỗng); snapshot baseline. |
| 7 | Tay mở lại (`handle` dùng lại) không wake nhầm task | `r7_reused_handle_does_not_wake_wrong_task` | Close key0 → đăng ký mới **key1**; event key0 cũ `rstale`; chỉ `Ok(NEW)`. (`key` là số đăng ký, KHÔNG phải fd/tên tay; không dùng sentinel 0 cho timer.) |
| 8 | Nhiều waiter cùng lúc, thứ tự do trace (`seq`) | `r8_multiple_waiters_fifo_trace_order` | 2 đăng ký key0/key1; event `seq0=BB(key1)` rồi `seq1=AA(key0)` ⇒ stdout `Ok(BB)\nOk(AA)\n1\n2\n`; log parity. |
| 9 | `&&`/`\|\|` (Vis: `and`/`or`) bị **từ chối sớm** | `r9_and_or_short_circuit_rejected_clearly` | interp frame `exit=1`, chẩn đoán `short-circuit`, **stdout rỗng** (vế phải không eval); default runtime vẫn `false\n`. |
| 10 | Hằng cấp tệp bị từ chối đúng lý do | `r10_file_const_rejected_clearly` | interp frame `exit=1`, message `hằng cấp tệp`; default `7\n`. |
| 11 | Closure capture bị từ chối rõ | `r11_captured_task_rejected_clearly` | module bytecode crafted có `Instr::LoadCaptured(0)` ⇒ `task_requires_capture()==true`, `run_frame_for_test` `exit=1`, message `capture`. |
| 12 | Timer monotonic, nearest-deadline, overlap (B1) | `b1_overlap_via_monotonic_timer` | 20 timer `deadline_ms=200`, log `timer:<key>:200` chọn một lần; 3 lần đo ~0,201 s (xem §5). |
| 13 | Không hồi quy Lát 1 | `lat1_frame.rs` (8+1) | `8/8` + `1/1` vẫn xanh sau khi struct literal thêm field `..Default::default()`. |

Tổng: `cargo test -p vis-cli --test lat2_reactor` → **13 passed / 0 failed**.
`cargo test -p vis-interp -p vis-cli` → **1020 passed / 0 failed** (1007 trước + 13 mới).
`./vis-dev check` → **PASS** (fmt / clippy -D warnings / R1..R6 / self-test 13/13 /
build / doc-examples 24 khối — 19 xanh + 5 ngoại lệ đỏ đúng).

## 2. Đường interp thật được đổi thế nào

1. `rt.rs` — thêm `ReactorOp`, `ReactorOutcome`, `ReactorEvent`, `ReactorReg`, `ReactorState`;
   `Wait::Reactor{key,gen}`; `Pending` đổi `task: Option<Rc<Task>>` + `wait: Option<Wait>`
   (reactor không cần task trung gian); `FrameTest.reactor/current/timer_real_wait`;
   `DiagSnapshot` thêm `regs`/`timers`.
2. Các hàm `#[doc(hidden)]` trên `Scheduler`: `frame_reactor_load_for_test`,
   `frame_reactor_poll`, `frame_reactor_fire`, `frame_idle`, `frame_reactor_shutdown`;
   mở rộng `frame_next`/`frame_drain_events`/`frame_reclaim`/`diag_snapshot`.
   `frame_reactor_fire` là nơi duy nhất quyết stale (`gen` lệch / không còn đăng ký),
   duplicate (`fired[key]==gen`), EOF/error/close/Spurious — mọi nhánh ghi log tất định.
   `frame_idle` chọn **deadline nhỏ nhất** (`min_by_key`, tie-break key), ngủ một lần khi
   đo B1, rồi fire event theo `seq` cho tới khi có frame sẵn sàng (event stale không dừng sớm).
3. `stdlib.rs` — hai nhánh `net.read`/`net.accept` chỉ khi `frame.enabled`: validate kiểu/cap
   y hệt đường cũ rồi gọi `frame_reactor_poll`. **Không** gọi `host.net()` ⇒ không thể fallback
   im lặng sang socket thật. `net.write`/`net.connect` vẫn đồng bộ (xem §8).
4. `frame.rs` — driver đọc `p.wait` để dựng `Waiter`; đặt `frame.current` mỗi bước; nạp trace
   từ `FrameConfig.reactors`; gộp `reactor.log` vào log trả về; `Work::Redispatch` lưu đối số
   **đã eval** của builtin khi reactor báo `Pending` ⇒ resume không eval lại operand (không lặp
   side-effect). `classify_callee` nhận diện thêm tên `std.*` (`vis_types::stdlib::sig_of`) để
   `net.read`/`accept` (HIR `Field` sau `lower_with`) đi tới `builtin::call`.

## 3. Đường VM thật được đổi thế nào

1. `vm_frame.rs` — driver đọc `p.wait`, đặt `frame.current`, nạp trace, gộp log, shutdown.
2. `call_builtin_frame()` (:28) vẫn là hook 1 dòng ở `vm.rs:860`: khi `$await`/builtin báo
   `Abort::Pending` nó đẩy lại đối số `th.st` và trừ `pc` đi 1 ⇒ instruction chạy lại lúc resume.
   Với `net.read`/`accept`, `builtin::call → stdlib::call → frame_reactor_poll` trả Pending ⇒
   `Th` được cất nguyên vẹn. Không opcode/bytecode/VERSION/cache-key đổi.
3. `task_th()` (:250) giờ trả `Result<Th, Abort>` và **từ chối rõ** nếu thân task dùng
   `Instr::LoadCaptured` (`vis_bytecode::verify::captured_used > 0`) — thay vì âm thầm dựng
   `captured = Vec::new()`/`writeback = None`.

## 4. Reactor invariants → cơ chế thực thi

| Bất biến | Cơ chế | Test |
|---|---|---|
| Readiness chỉ là **gợi ý retry**, không phải thành công | Event `Ready` chỉ stash kết quả; `frame_reactor_poll` mới trả khi retry; `Spurious` không stash ⇒ retry gặp WouldBlock | r1, r4 |
| Retry gặp WouldBlock ⇒ đăng ký lại, **không hot spin** | `Spurious` tăng `reg.gen`, wake; retry đăng ký wait với gen mới; event gen cũ thành stale | r4 |
| Duplicate wake ≠ duplicate run | `fired[key]==gen` chặn (`rdup`); `frame_wake_one` gỡ waiter + dedup `ready` | r3, r3b |
| Close / timeout / re-register vô hiệu event cũ | Close **tăng gen** rồi giữ reg 1 chu kỳ để retry tiêu thụ `Close` rồi gỡ; gen lệch ⇒ `rstale` | r5, r7 |
| FD/handle dùng lại không wake nhầm task | `key` = số đăng ký tăng dần (`next_key`), không phải tên/fd; reg mới ⇒ key mới | r7 |
| EOF/error/timeout gỡ waiter/timer **đúng một lần** | `results.remove(key)` + `regs.retain` + `frame_wake_one` (mỗi cái một lần) | r5, r6 |
| Không đệ quy scheduler, không blocking read giả Pending | `frame_reactor_poll` chỉ đọc `ReactorState` (Vec/HashMap), không gọi `NetHost`, không vòng lặp chờ; driver là vòng `loop` phẳng | r1–r8 |

**Lifecycle**: `frame_final_join()` → `frame_reclaim()` dọn reg/result của task đã thu hồi;
`frame_reactor_shutdown()` xoá regs/results/fired/events, đặt lại cờ — `diag_snapshot()` về
`baseline()` (`regs=0, timers=0, waiters=0, …`) ở **cả hai engine** mà **không cần đóng tay còn
handle hợp lệ** (r1, r4, r5, r6, r7, r8, b1 đều assert baseline).

## 5. B1 / B2 — số đo

### B1-sim — 20 task × 200 ms, QUA ĐƯỜNG TEST (REACTOR GIẢ)

Theo VD-254: đây là **B1-sim**, **không** phải B1-real-frame. B1-real-frame chưa đo, chờ Lát 3.
Chương trình 20 `async fn` mỗi cái `net.read("Ci", 64, 1000)`; 20 event reactor có
`deadline_ms=200` (mô phỏng server delay 200 ms), `timer_real_wait=true`. Delay là timer
monotonic đồng thời — **KHÔNG phải mạng thật**.

| Lần | startup (tạo engine, không reactor wait) | interp workload | VM workload |
|---|---|---|---|
| 1 | 585 µs | 201,384 ms | 201,121 ms |
| 2 | 585 µs | 201,515 ms | 201,193 ms |
| 3 | 585 µs | 201,248 ms | 201,350 ms |

Chồng lấp **nhìn thấy rõ**: ~0,201 s thay vì 20×200 ms = 4,1 s. Mục tiêu ~0,3 s
(số đo, không phải ngưỡng CI) đạt. Không yêu cầu cùng thứ tự output giữa các connection độc
lập trên mạng thật (ở đây không có mạng thật).

**Đối chiếu đường mặc định (không đổi)** — cùng server loopback delay 200 ms,
`vis run` / `vis run --vm --no-cache`, 3 lần mỗi đường: **4,03 / 4,03 / 4,04 s** và
**4,04 / 4,04 / 4,04 s**. Nghĩa là tăng tốc chỉ xảy ra trong đường test (mô phỏng), đúng phạm vi.

### B2 — CPU rồi I/O (đói nguyên vẹn)
`vd251-b2.vis` (500k vòng lặp + `net.read` 200 ms), server loopback delay 200 ms:

| Engine | CPU_DONE | IO_START | IO_DONE | Kết luận |
|---|---|---|---|---|
| interp (default) | 1795157009782 | 1795157009782 | 1795157009983 | `CPU_DONE == IO_START` cùng ms, `IO_DONE−IO_START = 201 ms` |
| VM (default) | 1795157010711 | 1795157010711 | 1795157010913 | cùng hiện tượng, `202 ms` |

**B2 KHÔNG đổi** — starvation vẫn còn. Lát 2 không sửa fairness CPU; đúng như A0 cho phép.

## 6. Reactor thật — đã chứng minh gì / cần gì

**Đã chứng minh (zero-dependency)**: toàn bộ bất biến ở §4 + parity hai engine + lifecycle,
bằng reactor giả trace-driven chạy trên **Interp/Vm thật** (không phải model độc lập). Timer
dùng `std::time::Instant` (monotonic, zero-dep) để chọn deadline gần nhất.

**Chưa chứng minh / KHÔNG tuyên bố**: I/O mạng **thật** không chặn. `std::net` là blocking;
`set_nonblocking(true)` + `read()` chỉ cho readiness *check* (`WouldBlock`), nhưng **std không có
primitives chờ readiness** (`select`/`epoll`/`kqueue`/IOCP không nằm trong std). Không thể xây
reactor thật mà không (a) thêm dependency selector, hoặc (b) dùng `unsafe` FFI trong mã dự án
(bị R2 cấm), hoặc (c) helper thread blocking-read / sleep-loop (bị A0 cấm rõ). Vì vậy Lát 2
**không làm (b)/(c)** và dừng ở mức được phép: adapter giả + timer monotonic.

**Kết luận trung thực**: tích hợp reactor thật (readiness từ kernel) **cần selector dependency,
chờ A0 duyệt**. Tham khảo (chỉ đọc, không sửa/merge) `agent/JOURNAL/vd252-selector-proposal.md`
(`mio 1.2.5` atau `polling 3.11.0`, cfg Linux-only, chưa chọn final). **Không gọi đây là "async
mạng thực tế hoàn chỉnh".**

## 7. Phạm vi Windows / Linux

- Mã Lát 2 dùng `std::time::Instant`, `Vec`, `HashMap`, `VecDeque`, `thread::sleep` — **thuần
  std, cross-platform**. Không cfg Linux-only, không syscall, không FFI. Windows build/test
  không bị ảnh hưởng bởi mã mới (không có branch bị loại trên Windows).
- Reactor **thật** (khi có dependency) mới cần cfg `target_os = "linux"` theo proposal; hiện
  **chưa thêm**, nên không có rủi ro Windows. Runtime mặc định Windows giữ nguyên.
- `timer_real_wait` mặc định `false` ⇒ test parity **không** phụ thuộc wall-clock; chỉ ca B1 bật
  `true` để đo.

**`connect`/`write` vẫn đồng bộ (tạm thời)**: chỉ `read`/`accept` được nối vào reactor giả.
`net.connect`/`net.write` trong đường frame test vẫn gọi thẳng `NetHost` như đường mặc định và
**có thể chặn scheduler** — đây là nợ được ghi rõ, không được hiểu là "async mạng hoàn chỉnh".

## 8. Coverage / explicit rejection (bổ sung theo yêu cầu A0)

Ba chỗ từ ma trận `vd252-coverage-matrix.md` đã sửa trong đường test (không thu hẹp checker,
không đổi runtime mặc định):

1. **`and`/`or` (`&&`/`||`), `??`, `|>`** — `frame.rs` `Work::Eval/HExpr::Bin` (:553) từ chối
   **trước khi đẩy operand**, có `Span`, message `short-circuit`. Trước đây cả hai vế bị eval
   rồi mới lỗi (side-effect có thể chạy). Ca âm: `r9` — stdout RỖNG + `exit=1`. VM bytecode thật
   hạ `and` thành `JumpIfNotTrue` nên chạy được (**không** dùng để tuyên bố evaluator parity).
2. **Hằng cấp tệp** — `run_frame_for_test` thu thập `HItem::Const` (sym + tên); `HExpr::Var`
   rơi xuống `it.env` (rỗng vì frame path không chạy vòng const của `run_program`) giờ báo
   `hằng cấp tệp … chưa được đường frame test hỗ trợ` thay vì `biến … chưa định nghĩa`. Ca âm:
   `r10`. VM bytecode thật hạ const thành lệnh `Const` ⇒ vẫn đúng (`7\n`), **không fallback**.
3. **Closure capture** — `vm_frame.rs::task_th` guard `captured_used(code) > 0` ⇒ Trap rõ
   `cần capture biến tự do chưa được đường frame test hỗ trợ`, thay vì âm thầm dựng capture rỗng
   (kết quả sai mà vẫn pass). Ca âm: `r11` (module crafted có `LoadCaptured(0)`).

Mỗi dòng "KHÔNG" liên quan đã có ca âm tương ứng; checker/ngôn ngữ không bị thu hẹp (cùng
nguồn vẫn chạy đúng qua runtime mặc định — `r9`/`r10` assert golden).

**Hạn chế còn lại (thẳng thắn, không coi là đã phủ)**: evaluator interp frame vẫn không mô
phỏng `Un`, `if`/`match`, vòng lặp, field/index, record/tuple/closure-làm-callee, đối số tên,
multi-frame continuation qua capture — các chỗ đó đã có trap tường minh từ Lát 1 (xem
`vd251-lat1-report.md` §4 và ma trận). B2 starvation vẫn mở. Reactor thật chờ A0 duyệt dependency.

## 9. Validation — số liệu chính xác

| Lệnh | Kết quả |
|---|---|
| `./vis-dev check` | **PASS** (fmt OK; clippy `-D warnings` OK; R1..R6 OK; self-test 13/13; build OK; doc-examples 19 xanh + 5 ngoại lệ đỏ đúng) |
| `cargo test -p vis-cli --test lat2_reactor` | **13 passed / 0 failed** |
| `cargo test -p vis-interp -p vis-cli` | **1020 passed / 0 failed** |
| `cargo test -p vis-cli --test lat1_frame` | **8/8** (không hồi quy) |
| `cargo test -p vis-interp --test lat1_frame` | **1/1** (không hồi quy) |

Ghi chú: toolchain đã được pin `1.100.0` bởi workstream khác (không phải tôi); `cargo fmt --all`
chỉ định dạng tệp Rust tôi sở hữu — không đụng `Cargo.toml`/`rust-toolchain.toml` (các thay đổi
thấy trong `git status` ở hai tệp đó là của workstream khác). Không có `cargo clean` nào được
thực hiện; không có lỗi artifact ngoại lai.
