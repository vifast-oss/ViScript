<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — verify `4d016c8` + đề xuất Windows async net backend (mio)

> **Người thực hiện**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Main verify**: `4d016c8` (W1g async net production `mio`, `cfg(unix)`).
> **Kết quả**: exact main đỏ **3 ca test-only** (async net production chưa có backend Windows);
> candidate gate test `8b6ad76` xanh **0 failed**. Không sửa sản phẩm; không tag/push/merge;
> không GitHub Actions.

## 0. TL;DR

| Mốc | `cargo test --workspace` | Gate | Host | Targets |
|---|---|---|---|---|
| **`4d016c8`** (main) | 349 block; **3454 passed / 3 failed** / 3 ignored | PASS | 54/54 | 18/18 + 17/1 |
| **`8b6ad76`** (gated test-only) | 349 block; **3453 passed / 0 failed** / 3 ignored | PASS | 54/54 | 18/18 + 17/1 |

3 ca đỏ exact main: `vs3_default::{cli_async_net_read_loopback_hai_engine,
cli_async_net_read_timeout_loopback_hai_engine, cli_async_net_accept_loopback_hai_engine}` —
`VS0505 deadlock` vì Windows chưa có backend async net. Đây là **test-only/portability** (backend
`async_net.rs` hiện `#![cfg(unix)]`); sync net và các mảng khác không hỏng.

## 1. Exact `4d016c8` — kết quả

- `psvn` checkout đúng `4d016c8`, cây sạch.
- `cargo test --workspace --no-fail-fast`: **3454 passed / 3 failed / 3 ignored**; 349 block,
  1 target đỏ (`vis-cli --test vs3_default`).
- 3 ca đỏ đều panic:
  ```text
  error VS0505: deadlock: scheduler hết frame sẵn sàng nhưng còn tác vụ/frame chờ sự kiện
  không bao giờ tới
  ```
  trên cả interp (`vm=false`) và VM (`vm=true`).
- `bin/vis-dev check`: **PASS**; `bin/vis-host-suite`: **54/54**;
  `bin/vis-targets`: core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED**, exit 0.

### Nguyên nhân / phân loại

- `crates/vis-cli/src/async_net.rs` mở đầu `#![cfg(unix)]`, dùng `mio::unix::SourceFd` +
  `std::os::fd::AsRawFd`; `main.rs` bọc `AsyncNet` trong `#[cfg(unix)]`.
- Trên Windows, chương trình async net rơi vào fallback `run_async_*` với host `net` sync; frame
  scheduler kỳ vọng reactor nhưng không có backend ⇒ deadlock `VS0505`.
- Các test `cli_async_net_*` không được gate theo `cfg(unix)` ⇒ exact main đỏ trên Windows.
- **Không phải regression sản phẩm Windows**: sync net/HTTP/TLS/H2/worker vẫn xanh; async net
  production vốn được tài liệu ghi rõ **Linux/Unix trước; Windows/wasm dùng đường cũ**.

## 2. Candidate test-only `8b6ad76`

- Commit `8b6ad76` — `[VS3][W4-fix] vs3_default: gate async net production tests Windows
  (chua co backend)`: thêm `#[cfg(unix)]` cho 4 test `cli_async_net_*` + helper/const chỉ chúng
  dùng (`server_mot_lan`, `server_dem`, `ASYNC_NET_*`) — **13 dòng**, chỉ 1 tệp test.
- Chạy lại: **3453 passed / 0 failed / 3 ignored**; `vs3_default` 9/9 (Windows; 4 ca async net
  được gate); gate PASS; host 54/54; targets 18/18 + 17/1.
- Code sản phẩm `4d016c8..8b6ad76` **không đổi** (diff code rỗng; chỉ test gating).
- Đề xuất A0: cherry-pick `8b6ad76` (hoặc gate tương đương trong main) để Windows exact main xanh;
  nếu không, ghi rõ 3 ca test-only đỏ do backend async net Windows chưa có.

## 3. Đề xuất Windows async net backend (mio) — **chưa code**

### 3.1. Ownership / trạng thái

- `crates/vis-cli/src/async_net.rs` do **W1** sở hữu (commit gần nhất `0be0bb0`); W1 đang làm async
  TLS. W4 **không sửa** file này trong đợt — chỉ đề xuất.
- `crates/vis-cli/src/net.rs` (sync) và `pkg.rs`/`frame/**`/`value.rs` vẫn ngoài phạm vi W4.
- Điều kiện bắt đầu: W1 land async TLS/refactor `async_net`, xác nhận bàn giao file; sau đó W4 mở
  lát riêng (branch/worktree) và chạy checklist Windows.

### 3.2. Thiết kế tối thiểu (giữ một module)

1. **cfg**: đổi `#![cfg(unix)]` → `#![cfg(any(unix, windows))]`; `main.rs` đổi các
   `#[cfg(unix)]` quanh `AsyncNet`/`async_host`/`async_guard`/`async_backend` thành
   `#[cfg(any(unix, windows))]`; wasm giữ fallback `#[cfg(not(any(unix, windows)))]`.
2. **Đăng ký readiness cross-platform**: bỏ `std::os::fd::AsRawFd` + `mio::unix::SourceFd`; dùng
   `mio::event::Source` trực tiếp trên `MioStream`/`MioListener`:
   ```rust
   self.poll.registry().register(s, Token(token), interest)?;
   // deregister tương tự
   ```
   `mio::net::{TcpStream,TcpListener}` implement `Source` trên cả Unix và Windows (IOCP).
3. **Nonblocking conversion**:
   - `listen`: `std::net::TcpListener::bind` → `set_nonblocking(true)` → `MioListener::from_std`.
   - `connect` (API blocking `NetHost::connect`): `std::net::TcpStream::connect_timeout` →
     `set_nonblocking(true)` → `MioStream::from_std`; `try_connect` giữ `mio::net::TcpStream::connect`
     (đã nonblocking).
   - `from_std` trên Windows **đòi** socket nonblocking; thiếu bước này sẽ lỗi/không nhận event.
4. **Mã lỗi chờ nối**: bỏ `raw_os_error() == Some(115)` (EINPROGRESS Unix). Dùng
   `ErrorKind::WouldBlock | ErrorKind::NotConnected`, thêm helper `cfg(windows)` nhận
   `WSAEWOULDBLOCK (10035)`/`WSAEINPROGRESS (10036)` nếu cần; test connection-refused/reset.
5. **Cargo**: `mio` chuyển từ `[target.'cfg(unix)'.dependencies]` sang
   `[target.'cfg(any(unix, windows))'.dependencies]` với features `os-poll`, `net`;
   `os-ext` chỉ Unix (hoặc bỏ nếu không dùng). Không thêm crate mới (`mio 1.2.5` đã duyệt).
6. **Thông điệp**: `epoll_wait` → `mio poll` (không đổi semantics).
7. **TLS (W1)**: wrapper TLS async phải bọc `MioStream` Windows như Unix; kiểm
   `vs3_tls` + test async TLS trên Windows trước khi bật mặc định.
8. **Giữ fallback sync** cho tới khi backend Windows xanh; không xoá đường cũ trong cùng commit.

### 3.3. Test plan (Windows)

- Re-enable 4 ca `cli_async_net_*` đã gate; thêm `vs3_async_net_windows` (hoặc mở rộng
  `vs3_default`):
  - connect/listen/accept, read/write, partial-write 300 KB, timeout 50 ms, đóng giữa chừng;
  - hai kết nối đồng thời + thứ tự event; connection refused/reset; loopback IPv4/IPv6.
- Chạy cả interp + VM, byte-identical; `NetGuard`/cap loopback.
- Stress: 100 kết nối × 1 MB, accept loop, không treo/không busy-spin; đo CPU.
- Full psvn: `cargo test --workspace --no-fail-fast` (0 failed), `vis-dev check`, host 54/54,
  `vis-targets` 18/18 + 17/1; log/sha như checklist v0.2.0.
- Linux regression: full gate + test; `async_net` Unix không đổi hành vi.

### 3.4. Rủi ro

| Rủi ro | Giảm thiểu |
|---|---|
| IOCP khác epoll (edge/level, event filter) | dùng `mio::event::Source`, test readiness; không tự làm fd |
| `from_std` thiếu nonblocking trên Windows | set_nonblocking trước; test ngay sau convert |
| Mã lỗi connect khác (10035/10036 vs 115) | helper `cfg(windows)`, test refused/reset |
| `Poll::poll`/timeout semantics | test timeout + cancel/close; không busy-spin |
| Xung đột W1 (async TLS, `async_net.rs`) | chờ W1 land + bàn giao; W4 chỉ sửa sau |
| `mio` feature/`os-ext` chỉ Unix | tách target dependency; CI/targets check |
| wasm bị kéo mio | giữ `cfg(any(unix,windows))`, wasm fallback |
| TLS async trên Windows | test `vs3_tls` + TLS async trước khi bật |

## 4. Kết luận / đề xuất

1. Windows exact `4d016c8` **không xanh** vì 3 test async net chưa gate; **sản phẩm sync không hỏng**.
2. Đưa `8b6ad76` (test-only) vào main để Windows xanh; ghi rõ async net production Windows **chưa
   có backend** cho tới lát W1/W4 sau.
3. W4 sẵn sàng implement backend mio Windows khi W1 bàn giao `async_net.rs`; thiết kế/test plan
   ở §3. Không code trong đợt này.

## 5. Log

| Mốc/Lệnh | Log | sha256 | Exit |
|---|---|---|---|
| `cargo test` @ `4d016c8` | `/tmp/opencode/psvn-4d01-cargo.log` | `d3b99608…4f88` | **101** |
| gate + host @ `4d016c8` | `/tmp/opencode/psvn-4d01-exact-gate.log` | `46935b37…09c2` | 0 |
| `vis-targets` @ `4d016c8` | `/tmp/opencode/psvn-4d01-exact-targets.log` | `976e8fa9…6807` | 0 |
| `cargo test` @ `8b6ad76` | `/tmp/opencode/psvn-4d01b-cargo.log` | `4bf22ca0…532a` | **0** |
| gate + host @ `8b6ad76` | `/tmp/opencode/psvn-4d01b-gate.log` | `caff0985…d487` | 0 |
| `vis-targets` @ `8b6ad76` | `/tmp/opencode/psvn-4d01b-targets.log` | `a46be5cf…7c8c` | 0 |

Log máy đích: `vis-remote-20261121T080819Z.log` (cargo exact), `…T082604Z.log` (gate exact),
`…T082701Z.log` (targets exact), `…T081631Z.log` (cargo candidate), `…T082359Z.log` (gate candidate),
`…T082517Z.log` (targets candidate), `…T082720Z.log` (trạng thái cuối).
