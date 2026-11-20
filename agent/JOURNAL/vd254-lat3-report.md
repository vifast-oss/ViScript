<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-271 / VD-254 — LÁT 3: readiness THẬT (`mio` epoll) trên đường frame test

- Ngày: 2026-11-20
- Phạm vi: **chỉ đường frame test nội bộ** của `vis-interp` + `vis-vm`, trên Linux.
  **Không commit.** Không CLI flag, không API/VERSION/cache-key/bytecode/opcode mới,
  không `unsafe` trong mã dự án.
- Không đụng: `agent/DECISIONS.md`, `agent/QUEUE.md`, `agent/JOURNAL/vd252-*`,
  `rust-toolchain.toml`, `Cargo.toml` gốc, docs/tệp do workstream khác sở hữu.
  (Các tệp `agent/*` đang có thay đổi của workstream khác trong lúc làm việc — **không đụng vào**;
  tệp `agent/JOURNAL/vd254-third-party-notice.md` xuất hiện trong lúc làm việc cũng **không đụng**.)

## 0. Khẳng định biên (đọc đầu)

**Runtime mặc định (`vis run`, `vis run --vm`), bytecode phát hành, `Cargo.toml` VERSION,
cache key và API ngôn ngữ KHÔNG đổi.** Adapter Lát 3 chỉ tồn tại khi
`FrameConfig.backend` (trường `#[doc(hidden)]`, test-only) được cài. Khi `backend == None`
(mặc định, gồm cả toàn bộ 13 test parity Lát 2) đường reactor giả chạy **nguyên văn**.

Đây là **read/accept nonblocking trên đường frame test**, **KHÔNG phải async mạng hoàn chỉnh**.
`net.connect`/`net.write` vẫn đồng bộ, có thể chặn scheduler (ở cả runtime mặc định lẫn đường test).
`mio` chỉ là nguồn readiness (epoll); I/O thật được adapter thực hiện bằng `std::io::Read::read`
trên socket **nonblocking**, readiness là **gợi ý retry**, không phải dữ liệu bảo đảm.

Adapter được đặt trong **integration test** `crates/vis-cli/tests/lat3_real.rs` (đường frame
test-only), không lọt vào artifact `vis` phát hành. `mio` khai ở
`[target.'cfg(target_os = "linux")'.dev-dependencies]` của **`vis-cli`** — crate duy nhất sở hữu
adapter; core crate (wasm32) không bao giờ thấy dependency này.

## 1. Criteria → evidence

| # | Tiêu chí chấp nhận | Bằng chứng |
|---|---|---|
| 1 | Đọc thật: data / EOF / error / timeout | `t1_read_real_data_both_engines` (2 luồng server delay 60 ms → `Ok(PONG)` ×2), `t2_read_eof_removes_waiter_once` (`Ok()` rỗng), `t3_read_error_propagates_as_result_err` (`Err(...)` tay đã đóng), `t4_read_timeout_wakes_once` (80 ms → `Err("quá thời gian…")` **đúng 1 lần**). Cả hai engine, `snapshot == baseline()` sau khi handle drop. |
| 1b | accept: chưa có client rồi client nối; nhiều kết nối đồng thời; nhiều waiter cùng listener | `t5_accept_no_client_then_client_connects` (client nối sau 80 ms), `t6_multiple_simultaneous_accepts` (2 listener, client 60/120 ms), `t12_multiple_waiters_same_listener` (2 task chờ **cùng** `L1`, 2 client → 2 handle, không mất wake). |
| 1c | **Không eval lại đối số / không lặp side-effect khi resume** | `t11_builtin_resume_does_not_reevaluate_args`: hàm `tay()` in `ARG` nằm **trong đối số** `net.read(tay(), …)`; wake thật ⇒ `ARG` in **đúng 1 lần**, stdout `Ok(ONCE)`. Dùng lại `Work::Redispatch` của Lát 2. |
| 1d | Close-while-waiting; stale event; FD/token reuse | `t7_close_while_waiting_invalidates_old_events` (task khác `net.close("C1")` ⇒ retry trả `Err`, baseline). `t9_token_generation_stale_and_reuse_deterministic`: token lạ `999` → `rstale:token:999`, **không wake**; token `0` cũ sau re-arm → stale; tay đóng rồi mở lại ⇒ token `2` mới, event token `1` cũ **không wake nhầm task**. Dùng backend scripted ⇒ tất định, không timing. |
| 1e | Lỗi selector lan rõ | `t8_selector_error_propagates_clearly`: `wait()` trả `Err("epoll giả hỏng")` ⇒ driver trap `exit=1`, message chứa chuỗi lỗi; **không** hiểu là "hết event", **không** rơi vào vòng lặp final-join (đã thêm điểm dừng ở cả 2 driver). |
| 1f | Cleanup về baseline sau test | Mọi test readiness assert `DiagSnapshot` về `baseline()` (`regs=0, timers=0, waiters=0, …`) — gồm cả sau EOF/timeout/error/close. |
| 1g | Giữ parity reactor giả Lát 2 | `cargo test -p vis-cli --test lat2_reactor` → **13/13** (kể cả 3 ca explicit-rejection r9/r10/r11); `lat1_frame` 8/8 (cli) + 1/1 (interp). Log parity byte-with-byte interp↔VM **không đổi**. |
| 2 | B1-real-frame: 1/5/20, nhiều lần, startup/workload riêng, chồng lấp | `t10_b1_real_frame_overlap_measure`, số liệu §5. 20 kết nối × server delay 200 ms → workload ~0,203 s thay vì ~4,0 s. Không yêu cầu cùng thứ tự output giữa các connection độc lập (test chỉ đếm số `Ok(X)`). |
| 3 | B2 / step-cap vẫn mở | Không sửa fairness, không nâng ngưỡng, không thêm gate mới — xem §7. |
| 4 | Từ chối rõ cấu trúc evaluator không hỗ trợ | Giữ nguyên r9 (`and`/`or` short-circuit), r10 (hằng cấp tệp), r11 (closure capture) của Lát 2; **không** fallback sang đường đồng bộ để làm xanh B1. |
| 5 | Gate / test / cross-build | §8. |

Tổng: `cargo test -p vis-cli --test lat3_real` → **12 passed / 0 failed**.
`cargo test -p vis-interp -p vis-cli` → **1032 passed / 0 failed** (1020 của Lát 2 + 12 mới).

## 2. Tích hợp `mio` + Cargo.lock diff

Khai báo (duy nhất, trong `crates/vis-cli/Cargo.toml`):

```toml
[target.'cfg(target_os = "linux")'.dev-dependencies]
mio = { version = "=1.2.5", default-features = false, features = ["os-poll", "net"] }
```

- Chính xác theo A0: ghim tuyệt đối `=1.2.5`, `default-features = false`, chỉ `os-poll` + `net`.
- `cargo metadata`: `mio 1.2.5`, `source = registry+https://github.com/rust-lang/crates.io-index`,
  `license = MIT` — **khớp** proposal; không tự đổi version.
- `Cargo.lock` diff (đã review): thêm đúng 4 package — `mio 1.2.5`
  (checksum `20521a903f02b0a573cd6840e9d0a5a0cc043d361fe924092cd70f8556cc9c4a`, khớp proposal),
  `wasi 0.11.1+wasi-snapshot-preview1`, `windows-sys 0.61.2`, `windows-link 0.2.1`.
  **`log` KHÔNG có** trong lock (đã tắt). **Không có dependency nào khác bị update** (mọi version
  cũ giữ nguyên). Các entry `wasi`/`windows-sys`/`windows-link` chỉ là khai báo đa nền tảng trong
  lockfile; `cargo tree -p vis-cli --target x86_64-unknown-linux-gnu -e normal` cho thấy trên Linux
  chỉ có `mio` + `libc v0.2.191` (đã có từ trước) — `windows-sys` **bị cfg loại**, không vào artifact.
- Dùng `dev-dependencies` thay vì `dependencies` vì adapter nằm hoàn toàn ở test của đường frame
  test-only: vẫn Linux-only, vẫn chỉ `vis-cli`, nhưng không tạo warning "unused dependency" và
  không lọt vào binary `vis` phát hành. (Hình thức table là "such as" theo điều kiện A0.)
- Không dùng `mio::unix::SourceFd` — tính năng đó nằm sau feature `os-ext` **không được A0 duyệt**.
  Adapter dùng `mio::net::{TcpStream, TcpListener}::from_std` (thuộc feature `net` **đã duyệt**),
  đăng ký qua `Registry::register/deregister` — mọi API gọi là safe.
- **Unsafe**: mã dự án vẫn **0 unsafe** (`git diff` chỉ có 2 dòng comment nhắc đến unsafe trong dependency).
  `mio` có FFI epoll bên trong crate — đây là unsafe của dependency (đánh giá riêng trong exception
  report của workstream khác), không phải unsafe dự án.

## 3. Kiến trúc và bất biến adapter → cơ chế thực thi

Core (`vis-interp::rt`, zero-dep, cross-platform) định nghĩa trait `FrameReactorBackend` +
`ReactorIoOutcome`/`ReactorRawEvent` (toàn `#[doc(hidden)]`). Core **không** biết `mio`/fd/Windows;
adapter Linux cài trait này ở test. Không có implementation mặc định ⇒ **không có fallback im lặng**
sang socket blocking.

| Bất biến | Cơ chế | Test |
|---|---|---|
| Readiness là gợi ý retry, `WouldBlock` re-register đúng, không hot-spin, không mất wake | `frame_real_poll` thử I/O **trước**; `WouldBlock` ⇒ register/re-arm + `Pending` trả về scheduler. Không vòng lặp poll, không sleep readiness; mỗi wake chỉ đẩy frame vào `ready` đúng 1 lần (`frame_wake_one` dedup). | t1, t4, t9 |
| Token phân biệt **generation**, fd/tay dùng lại không wake nhầm | `next_token` tăng dần, **không tái sử dụng**; `token_map: token → (key, gen)`; re-arm mint token mới + gen+1; event token cũ không còn trong map ⇒ `rstale:token:*`, event gen lệch ⇒ `rstale:key:gen`. Adapter dereg bằng handle hiện hành nên fd reused không bị gỡ nhầm. | t9 (scripted, tất định), t7 |
| EOF/error/timeout gỡ waiter đúng **một lần** | `frame_real_done` (idempotent) = dereg token + xoá `token_map` + remove reg; gọi đúng 1 chỗ trên mỗi nhánh `Data`/`Accepted`/`Err`/timeout. `frame_reactor_shutdown` dọn nốt + log `rshutdown`. | t2, t3, t4, t7, t9 |
| Nhiều waiter cùng socket/listener — không wake tùy tiện làm mất progress | Cùng `(handle, op)` **chia sẻ một reg/key/gen**. Waiter mới join (`fired[key] != gen`) → **không re-arm**, chỉ xếp wait thêm; event đánh dấu `fired[key] = gen` rồi `wake_reactor` đánh thức **mọi** waiter; mỗi frame retry I/O thật, kẻ được dữ liệu thì `done`, kẻ vẫn `WouldBlock` thì re-arm gen mới. (Bug này đã bị phát hiện và sửa trong quá trình làm — log `rjoin`.) | t6, t12 |
| Close/timeout/dereg vô hiệu event cũ | `net.close` trên đường frame gọi `frame_reactor_cancel_handle` **trước** `backend.close`: dereg token + remove reg + remove token_map + wake waiter → retry thấy tay mất ⇒ `Err`. | t7, t9 |
| Lỗi selector là lỗi, không phải "hết event" | `wait()` trả `Err` ⇒ lưu vào `frame.selector_error`; `frame_idle` dừng ngay; cả 2 driver `break` rồi biến thành trap (`exit=1`, message gốc). | t8 |
| Có việc ⇒ chờ selector theo deadline gần nhất; **không** chờ timeout 0 | `frame_idle_real`: `deadline = min(deadline riêng của các reg)`, `timeout = min(deadline-now, CAP=5s)`; nếu đã quá hạn → 1 ms (không 0); nếu **không có reg nào** → trả `false` ngay (không ngủ vô ích). Selector `poll(timeout)` thật — **không** phải sleep-poll. | t4, t10 |
| Không re-evaluate operand khi resume | Dùng `Pending` + `Work::Redispatch` của Lát 2; đối số đã eval được lưu và đẩy lại nguyên vẹn. | t11 |

**Giới hạn đã ghi rõ (không ngụy biện):** `timeout_ms <= 0` không có hạn riêng ⇒ dùng trần CAP 5 s
(để không block vô hạn và không hot-spin); các waiter chia sẻ reg dùng chung deadline của lần đăng ký
đầu; deadline **không bị reset** bởi wake giả (spurious) ⇒ tổng thời gian chờ vẫn được tôn trọng.

## 4. Đường interp / VM thay đổi thế nào

- `rt.rs`: thêm trait + kiểu Lát 3; `ReactorReg` thêm `token`/`deadline`; `FrameTest` thêm
  `backend`/`token_map`/`next_token`/`selector_error`; `frame_idle` rẽ nhánh sang
  `frame_idle_real` khi có backend; `frame_reactor_shutdown` dereg nốt; thêm 1 impl block với
  `frame_install_backend_for_test`, `frame_reactor_poll_io`, `frame_take_selector_error`,
  `frame_reactor_cancel_handle`, `frame_real_poll`, `frame_idle_real`.
  **Hàm Lát 2 `frame_reactor_poll` giữ nguyên chữ ký và hành vi** (test `r3b` gọi trực tiếp vẫn xanh).
- `stdlib.rs`: hai nhánh `net.read`/`net.accept` có điều kiện gọi `frame_reactor_poll_io` thay vì
  `frame_reactor_poll`; `net.close` ưu tiên adapter khi có backend. Các nhánh mặc định
  (`frame.enabled == false`) **không đổi**.
- `frame.rs` / `vm_frame.rs`: `FrameConfig` thêm trường backend; driver cài backend, dừng ngay khi
  có `selector_error`, và biến lỗi đó thành trap sau vòng lặp. Không opcode/bytecode/VERSION đổi.

## 5. B1-real-frame — số đo (server loopback delay 200 ms, 3 lần mỗi cỡ)

`startup` = bind listener + tạo N client connection + parse/lower/check source (trước khi chạy engine).
`workload` = chỉ thời gian `run_frame_for_test`. "ok" = số `Ok(X)` đếm được (bằng N).

| N | lần | startup interp | workload interp | workload VM | ok |
|---|---|---|---|---|---|
| 1 | 1 / 2 / 3 | 0,80 / 1,20 / 1,47 ms | 201,25 / 201,85 / 201,31 ms | 201,38 / 201,87 / 201,26 ms | 1/1/1 |
| 5 | 1 / 2 / 3 | 3,62 / 3,48 / 3,24 ms | 201,81 / 202,28 / 201,61 ms | 201,63 / 202,00 / 201,86 ms | 5/5/5 |
| 20 | 1 / 2 / 3 | 10,73 / 4,57 / 6,69 ms | 202,66 / 203,47 / 203,58 ms | 202,44 / 203,29 / 201,59 ms | 20/20/20 |

Đọc: workload ~0,20–0,204 s **bất kể N** ⇒ chồng lấp rõ ràng; N×200 ms (20 → ~4,0 s) không xảy ra.
Startup tăng nhẹ theo N (tạo socket), đúng bản chất. ~0,3 s là **mục tiêu quan sát**, không phải
ngưỡng CI; test chỉ có chống-regression thô `workload < N×200/2` cho N≥5 (N=1 không có gì để chồng).
Không assert thứ tự output giữa các connection.

### So sánh B1 (ghi nhãn trung thực)

| Ca | Số liệu | Nguồn / bản chất |
|---|---|---|
| **B1-real-frame** (ca trên) | ~0,201–0,204 s cho 1/5/20 | **Đo mới ở Lát 3**, readiness epoll thật trên socket loopback thật, chạy qua đường frame test. |
| **B1-sim** (Lát 2) | ~0,201 s (3 lần) | **Kế thừa từ báo cáo Lát 2**, **không đo lại ở Lát 3**: timer monotonic *giả lập* delay 200 ms, reactor **giả** trace-driven, **không có socket thật**. |
| **B1-real-default** (Lát 2) | 4,03–4,04 s (interp), 4,04 s (VM) | **Kế thừa từ báo cáo Lát 2**, **không đo lại ở Lát 3**: `vis run` / `vis run --vm --no-cache`, socket thật nhưng chạy **tuần tự** (N×200 ms). Đường mặc định không bị sửa ở Lát 3. |

Không được đọc ba dòng trên là cùng một phép đo; chỉ B1-real-frame là số đo của workstream này.

## 6. Windows / core / wasm32

Toolchain `1.100.0` (đã pin bởi workstream khác). Đã `rustup target add x86_64-pc-windows-msvc wasm32-unknown-unknown`.

| Lệnh | Kết quả |
|---|---|
| `cargo check --target x86_64-pc-windows-msvc -p vis-cli --no-default-features` (core config) | **PASS** |
| `cargo check --target wasm32-unknown-unknown -p vis-cli --no-default-features` (core config) | **PASS** |
| `cargo tree -p vis-cli --target x86_64-pc-windows-msvc / wasm32-unknown-unknown -e normal,dev` | `mio` / `windows-sys` / `wasi` **KHÔNG xuất hiện** — cfg Linux hoạt động |
| `cargo check --target x86_64-pc-windows-msvc -p vis-cli` (default/sqlite) | **EXPECTED RED** — `libsqlite3-sys` cần `lib.exe` (không có trên host Linux), lỗi từ `cc-rs`, **không liên quan `mio`** (đúng nợ #156 trong target matrix) |
| `cargo check --target wasm32-unknown-unknown -p vis-cli` (default/sqlite) | **EXPECTED RED** — same, cần `clang` cho bundled SQLite, **không liên quan `mio`** |

Core crate (`vis-interp` chứa trait/adapter hook, `vis-vm`, `vis-types`, `vis-bytecode`) biên dịch
xanh trên cả Windows-msvc và wasm32 thông qua hai lệnh core config ở trên. cfg là **phòng thủ**,
không phải bằng chứng về runtime Windows; runtime mặc định Windows vẫn giữ nguyên.

## 7. Nợ còn mở (không sửa ở Lát 3)

- **Coverage HIR**: evaluator frame vẫn không phủ `Un`, `if`/`match`, vòng lặp, field/index,
  record/tuple/closure-làm-callee, đối số tên, continuation đa frame qua capture — giữ nguyên từ Lát 1/2.
- **Closure writeback**: `vm_frame.rs::task_th` vẫn **từ chối rõ** task cần `Instr::LoadCaptured`;
  interp frame không mô phỏng capture. Không fallback im lặng.
- **B2 starvation / step-cap parity**: CPU rồi I/O vẫn đói nguyên vẹn; không sửa fairness, không nâng
 /giảm step-cap, không thêm threshold. B2 của Lát 2 (`CPU_DONE == IO_START`, `IO_DONE−IO_START≈201 ms`)
  giữ nguyên, không đo lại.
- **`connect`/`write` vẫn đồng bộ**, có thể chặn scheduler — cả runtime mặc định lẫn đường frame test.
  Đây chưa phải async mạng hoàn chỉnh.
- Báo cáo exception bên thứ ba (unsafe/license của `mio`) do workstream khác sở hữu
  (`agent/JOURNAL/vd254-third-party-notice.md`); không đụng vào.

## 8. Validation tổng hợp

| Lệnh | Kết quả |
|---|---|
| `./vis-dev check` | **PASS** — fmt OK; clippy `-D warnings --all-targets` OK; R1..R6 OK; self-test 13/13; build OK; doc-examples 24 khối (19 xanh + 5 ngoại lệ đỏ đúng) |
| `cargo test -p vis-cli --test lat3_real` | **12 passed / 0 failed** |
| `cargo test -p vis-cli --test lat2_reactor` | **13 passed / 0 failed** (parity + explicit rejection không hồi quy) |
| `cargo test -p vis-cli --test lat1_frame` / `cargo test -p vis-interp --test lat1_frame` | **8/8** + **1/1** |
| `cargo test -p vis-interp -p vis-cli` | **1032 passed / 0 failed** (107 test target) |
| Cross-target Windows/wasm32 core config | **PASS/PASS** (mio bị cfg loại) |

Không `cargo clean`, không artifact ngoại lai, **không commit**. `cargo fmt --all` chỉ định dạng các
tệp Rust thuộc workstream này.
