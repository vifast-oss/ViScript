<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-2b (W5) — timeout kênh `recv_timeout` / `send_timeout`

**Trạng thái**: mã + test **đường frame test** cả interp/VM; runtime mặc định **chưa hỗ trợ**
(cố ý — W1 1d sẽ flip async default). Không đụng `frame/**`/`interp.rs`/`vm.rs`; không đổi
`Value`, worker, net, pkg.
**Căn cứ**: `DECISIONS.md` VD-271 (bổ sung) mục 2b: `recv_timeout(ms) -> T?` (None = timeout
**hoặc** đóng), `send_timeout(ms, v) -> bool` (false = timeout **hoặc** đóng); `ms < 0` trap `VS0501`.

## 1. Hợp đồng

- `c.recv_timeout(ms)`:
  - `Some(v)` nếu có giá trị;
  - `None` nếu hết `ms` **hoặc** kênh đã `close` (không phân biệt);
  - `ms < 0` ⇒ trap `VS0501`.
- `c.send_timeout(ms, v)`:
  - `true` nếu gửi được;
  - `false` nếu hết `ms` **hoặc** kênh đã `close` (không phân biệt);
  - `ms < 0` ⇒ trap `VS0501`.
- `ms == 0`: thử một lần; không đăng ký timer.
- Ngoài frame test (`frame.enabled == false`): trả `VS0501` với thông điệp **"cần đường frame
  test/async"**, không treo, không im lặng. W1 1d sẽ chuyển async default sang frame.

## 2. Cơ chế

- `tyck/expr/kenh.rs`: thêm chữ ký `recv_timeout(int) -> T?`, `send_timeout(int, T) -> bool`.
- `rt/chan.rs`: timeout dùng **reactor timer hiện có** (`ReactorReg` + `ReactorEvent.deadline_ms`),
  không thêm timer heap/field vào `rt.rs`/`frame/**`.
  - Timer tổng hợp `handle = chan-timeout:<id>:<send>`, `Wait::Reactor`; frame retry qua
    `probe_timeout`.
  - Channel event (send/recv/close) đánh thức timer bằng `ReactorOutcome::Ready(Unit)`; hết hạn
    bằng `ReactorOutcome::Error("chan timeout")`.
  - `chan(0)`: timed sender/receiver vẫn vào `send_waiters`/`recv_waiters`; rendezvous bình thường
    ghép và đánh thức; timeout gỡ waiter đúng một lần.
  - `chan(n≥1)`: timed send/recv không dùng `chay_mot`/busy-drain; full/empty ⇒ timer; send/recv
    bình thường đánh thức timed waiter để retry.
  - `close` đánh thức cả timed waiter (trả None/false).
- `builtin.rs`: chỉ cập nhật doc; dispatch vẫn qua `rt::chan::method`.

## 3. Test

`crates/vis-cli/tests/vs3_chan_timeout.rs` — **17 passed**:
- cap1 recv timeout rỗng/có giá trị; cap1 send timeout đầy/còn chỗ; cap1 chờ sender/receiver;
- cap0 recv/send timeout ghép sender/receiver; cap0 timeout không đối tác;
- close khi timed recv/send đang chờ; close trước timeout; ms=0 (cap0/cap1, có/không đối tác);
- ms<0 trap `VS0501` cả hai engine; runtime mặc định lỗi rõ; timer thật `timer_real_wait=true`
  không treo.

Chạy kèm: `vs3_chan0` **13**, `vs3_cancel` **14**, `vs3_budget` **8**, `vs3_worker` **8**;
`cargo fmt --check` + `clippy -D warnings` PASS.

## 4. Chưa nối / rủi ro

- **W1 1d**: khi flip async default, cần giữ `FrameTest.reactor`/`Wait::Reactor`/`frame.current`
  của đường frame hiện tại (W5 dùng trực tiếp), hoặc port cơ chế synthetic timer sang scheduler
  mới. Nếu W1 thay bằng timer heap thật, `recv_timeout/send_timeout` nên dùng heap đó; hành vi
  API không đổi.
- `timer_real_wait=false` (mặc định frame test) là **thời gian ảo**: timeout nổ ngay khi scheduler
  rảnh, không ngủ đủ `ms`. Test B1/real-time phải bật `timer_real_wait=true`. Runtime thật W1 phải
  dùng deadline monotonic.
- Không có `select` (VS3-2c), không timeout trên `Chan` liên worker (VS3-3b/2a), không đổi
  `Value`/ABI/bytecode/cache.
- `ms` rất lớn: `Instant::checked_add` tránh panic; hành vi có thể bão hoà, chưa test.
- Cancel timed waiter: cancellation hiện gỡ `ReactorReg`/`Wait::Reactor` qua `rt/sched`; chưa có
  ca riêng trong file này (VS3-1b đã phủ reactor cancellation).

## 5. Không tuyên bố

Không "async default" / "VS3-2b xong toàn runtime"; chỉ đường frame test. Không sửa
`frame/**`, `interp.rs`, `vm.rs`, `value.rs`, `rt/worker.rs`, `net.rs`, `pkg.rs`.
