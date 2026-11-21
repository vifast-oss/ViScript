<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — thiết kế port async net/TLS backend sang Windows (`crates/vis-cli/src/async_net.rs`)

> **Người làm**: subagent VS3-W4, 2026-11-21.
> **Mốc đọc code**: `vs3/w4-windows` @ `e272007` (code trùng `73af17b`; hai commit sau chỉ docs/journal).
> **Branch thiết kế**: `vs3/w4-windows` (không commit vào `main`).
> **Spike biên dịch/chạy thử**: branch `spike/w4-win-async-check` @ `4319b5d` — **KHÔNG merge**.
> **Kết luận ngắn**: khả thi về mặt kỹ thuật; đã chứng minh bằng compile + test thật trên Windows
> (MSVC native) ở nhánh spike. **Chưa phải hỗ trợ Windows async**: không merge, không đổi `main`,
> không tuyên bố production-ready. Async HTTP/2 **vẫn WIP của W2**, ngoài phạm vi port này.

## 0. TL;DR

| Hạng mục | Kết luận |
|---|---|
| Hiện trạng | `async_net.rs` `#![cfg(unix)]`; Windows async rơi vào fallback **không backend** ⇒ `net.read`/`accept` deadlock `VS0505`, `http.get` async trả lỗi rõ; async net Windows **chưa được hỗ trợ** |
| Khả thi port | **CÓ** — mio 1.2.5 implement `event::Source` cho `TcpStream`/`TcpListener` trên Windows; rustls nonblocking không phụ thuộc epoll |
| Thay đổi cốt lõi | bỏ `SourceFd`/`AsRawFd` → đăng ký trực tiếp đối tượng mio; `register`/`reregister`/`deregister` theo **đối tượng**, không theo fd; xử lý `host_poll` tách ownership |
| Rủi ro lớn nhất | IOCP/AFD ≠ epoll: một đối tượng mio chỉ thuộc **một** `Poll`; `reregister` bỏ qua `Registry` truyền vào; edge-trigger mô phỏng phải re-arm qua `WouldBlock` |
| Spike | `cargo check` + `--all-targets` **PASS** (MSVC); `vs3_default` async net **4/4**, async HTTP **9/9**, `vs3_tls` **21/21**, 3 vòng liên tiếp 0 fail; `--no-default-features` **PASS** |
| Ước lượng | ~**450–650 LOC** (gồm test) · **3–4,5 ngày công** + 1 ngày soak Windows 10 vòng |
| Blocker | **Không có blocker kỹ thuật**. `BLOCKED-ENV` cục bộ: không có target `x86_64-pc-windows-gnu`/mingw — đã dùng **MSVC native trên `psvn`** thay thế |

## 1. Hiện trạng (đọc từ `e272007`)

### 1.1. Điểm vào và cfg

| Tệp | Dòng | Nội dung |
|---|---|---|
| `crates/vis-cli/src/lib.rs` | 10–11 | `#[cfg(unix)] pub mod async_net;` |
| `crates/vis-cli/src/async_net.rs` | 15 | `#![cfg(unix)]` (cả module rỗng trên Windows) |
| `crates/vis-cli/src/main.rs` | 751–767 | `async_rc`/`async_host`/`async_guard`/`async_backend` bọc `#[cfg(unix)]` |
| `crates/vis-cli/src/main.rs` | 806/820, 850/866, 900/916 | nhánh async interp/VM/`.vish`: Unix dùng `*_backend`, non-Unix dùng `run_async_*` **không backend** |
| `crates/vis-cli/Cargo.toml` | 154–157 | `mio` chỉ trong `[target.'cfg(unix)'.dependencies]` |
| `crates/vis-cli/tests/vs3_default.rs` | 13–19, 308–535+ | 4 ca net + 9 ca HTTP async bị gate `#[cfg(unix)]` |
| `crates/vis-cli/tests/vs3_tls.rs` | 507–893 | toàn bộ ca async TLS bị gate `#[cfg(unix)]` |
| `crates/vis-cli/tests/lat256_net.rs` | 13 | backend test-only **Linux** (`#![cfg(target_os = "linux")]`), không dùng `AsyncNet` |

### 1.2. Cấu trúc `AsyncNet`

- `streams: HashMap<String, MioStream>` (`C<n>`, `N:<addr>`), `listeners: HashMap<String, MioListener>`
  (`L<n>`), `tls: HashMap<String, TlsChan>` (`C<n>`), `tls_connects` (`T:<addr>`), `writes` (`W:<tay>`).
- `poll: Poll` + `events` cho reactor scheduler; `host_poll` + `host_events` cho lời gọi host
  đồng bộ (`connect_tls`/`read`/`write` TLS) — **hai selector độc lập**.
- `register`/`deregister` của `FrameReactorBackend` tìm **fd** trong các map rồi bọc `SourceFd`.
- `cho_san_sang(poll, events, fd: RawFd, ...)` đăng ký/gỡ `SourceFd` mỗi lần chờ.
- Connect nonblocking (`try_connect` + `N:`), accept (`try_accept`), write offset (`WriteState`),
  TLS handshake từng bước (`bap_tay`, `bap_tay_reactor`), TLS đọc/ghi (`doc_tls`, `ghi_tls`,
  `try_read_tls`, `try_write_tls`).

### 1.3. Hành vi Windows hiện tại (không phải "chạy chậm")

- `main.rs` non-Unix gọi `run_async_with_full_hosts_caps(..., Some(&mut net), ...)` với `backend = None`.
- `net.connect` khi `backend.is_none()` rơi về host sync (blocking) — **chạy được**.
- `net.read`/`net.accept` khi `frame.enabled` đi `frame_reactor_poll_io` dù không có backend ⇒
  chờ event không bao giờ tới ⇒ **`VS0505 deadlock`** (đúng 3 ca từng đỏ ở `4d016c8`).
- `http.get/head` async khi không backend trả lỗi rõ
  `"async HTTP cần reactor backend thật (không fallback blocking)"`.
- Vì vậy: **không được tuyên bố Windows async net/TLS/HTTP đã hỗ trợ**; port này là điều kiện cần.

## 2. Thay đổi chính xác cần làm

### 2.1. Mở cfg và nối backend (P1)

1. `lib.rs`: `#[cfg(any(unix, windows))] pub mod async_net;`
2. `async_net.rs`: `#![cfg(any(unix, windows))]` (wasm vẫn ngoài).
3. `main.rs`: 7 gate `#[cfg(unix)]` quanh `AsyncNet`/`async_host`/`async_guard`/`async_backend` và
   3 nhánh async → `#[cfg(any(unix, windows))]`; 3 fallback → `#[cfg(not(any(unix, windows)))]`.
4. `Cargo.toml`: `[target.'cfg(any(unix, windows))'.dependencies] mio = ...` (giữ `=1.2.5`,
   `default-features = false`, `os-poll`, `net`; `os-ext` chỉ cần nếu dùng `NamedPipe`, có thể bỏ).
5. Giữ nguyên đường fallback cho wasm/nền khác; **không xoá** fallback sync trong cùng commit.

### 2.2. Bỏ `SourceFd`/`AsRawFd`: đăng ký theo **đối tượng** mio (P1)

Trên Windows, trạng thái đăng ký nằm trong chính `IoSource` của đối tượng mio
(`InternalState { selector, token, interests, sock_state }`), không có bảng fd toàn cục như epoll.
Đăng ký bằng fd thô là **không làm được** (`mio::windows` không export `SourceSocket`).

Thiết kế đã kiểm chứng trong spike:

```rust
enum Nguon<'a> {
    Stream(&'a mut MioStream),
    Listen(&'a mut MioListener),
}

impl Nguon<'_> {
    fn dang_ky(&mut self, poll: &Poll, token: u64, interest: Interest) -> io::Result<()> { ... }
    fn dang_ky_lai(&mut self, poll: &Poll, token: u64, interest: Interest) -> io::Result<()> { ... }
    fn huy(&mut self, poll: &Poll) -> io::Result<()> { ... }
}

fn nguon_mut<'a>(streams, listeners, [tls, tls_connects,] key) -> Option<Nguon<'a>> {
    // ưu tiên TLS (C<n>), TLS connect (T:), stream, listener
}
```

- `register`: `register(src, token, interest)`; nếu `AlreadyExists` → **`reregister`**
  (không `deregister` + `register` như bản Unix hiện tại).
- Lý do: khi `try_connect` chuyển socket từ `streams["N:…"]` sang `streams["C<n>"]`, đối tượng
  **mang theo** đăng ký cũ; `register` lần nữa trả `AlreadyExists`; `reregister` cập nhật token/
  interest của đúng selector đã đăng ký.
- `deregister`: tìm đối tượng theo token → `huy`. Nếu đối tượng đã bị drop (close), `Drop` của
  `IoSourceState` gọi `mark_delete()` — AFD poll đang chờ được huỷ; trả `Ok` như cũ.
- Unix dùng **cùng** đường này (mio net types cũng impl `Source` trên Unix); hành vi epoll tương
  đương, đã chạy lại 42 ca local + 24 ca Windows ở spike.

**Cấm**: `poll2.registry().reregister(&mut sock, …)` khi `sock` đang thuộc `poll1`. Trên Windows
`reregister` **bỏ qua** tham số `Registry` và sửa selector đã lưu trong đối tượng.

### 2.3. `host_poll` và ràng buộc "một socket — một Poll" (P2)

Windows: mỗi đối tượng mio chỉ giữ **một** `InternalState`; đăng ký với `Poll` thứ hai trả
`AlreadyExists`. `deregister` rồi đăng ký lại được, nhưng AFD poll cũ phải bị huỷ bất đồng bộ.

Hiện trường `host_poll` chỉ dùng cho TLS host **đồng bộ** (`connect_tls`/`read`/`write` của
`NetHost`) — trong production frame mode các lời gọi này **không** chạy (net.* đi backend,
http.* đi `async_http`). Vì vậy chọn **hai Poll + ownership tách biệt**:

1. **Bất biến**: socket do host TLS tạo ra chỉ đăng ký với `host_poll`; socket của reactor chỉ
   đăng ký với `poll`. Không bao giờ cùng một handle.
2. Thêm theo dõi ownership (ví dụ `HashSet<u64>` token host hoặc cờ trên `TlsChan`) và
   `debug_assert!`/trả lỗi rõ khi vi phạm. Trên Windows, nếu reactor `register` gặp `AlreadyExists`
   **và** đối tượng đang thuộc `host_poll` ⇒ trả `Err("handle đang thuộc host_poll")`, **không**
   `reregister` (sẽ cướp selector).
3. `cho_san_sang` tiếp tục đăng ký/gỡ từng lần chờ (đơn giản, đúng); chấp nhận chi phí
   register/deregister AFD. Tối ưu sau: giữ đăng ký suốt đời socket + `reregister` interest.
4. Phương án dự phòng nếu về sau cần dùng chung socket: **một** `Poll` duy nhất với namespace
   token (bit cao = host), nhưng phải tự hàng đợi event của reactor khi host chờ — **không khuyến
   nghị** cho phase đầu.

### 2.4. TLS nonblocking (P2)

- Giữ nguyên rustls `ClientConnection` + một hạn monotonic cho TCP connect + handshake.
- Mọi I/O socket vẫn qua `mio::net::TcpStream` (`Read`/`Write`), nên khi `WouldBlock` mio tự
  re-arm AFD (`IoSourceState::do_io`). **Không** đọc/ghi bằng fd thô hay `std::net` trên cùng socket.
- `cho_san_sang` đổi tham số `fd: RawFd` → `sock: &mut MioStream`; 4 call site
  (`ket_noi_tls`, `bap_tay`, `doc_tls`, `ghi_tls`) truyền socket trực tiếp. Trong `doc_tls`/
  `ghi_tls`, mượn rời `&mut self.host_poll` / `&mut self.host_events` rồi `&mut t.sock` để tránh
  xung đột borrow.
- Timeout handshake: một `Instant` duy nhất; `Poll::poll(events, Some(remaining))`; Windows trả
  `Ok(())` + events rỗng khi hết hạn (đã xử lý).
- Partial `write_tls`: giữ vòng `wants_write`; sau handshake còn Finished/alert treo thì đẩy ở
  I/O kế cận (như hiện tại).
- Chứng chỉ/kho gốc: không đổi (`webpki-roots`, hook test `VIS_TLS_TEST_ROOT_DER` chỉ debug).

### 2.5. connect / accept / read / write / timeout (P2)

| Đường | Windows | Ghi chú |
|---|---|---|
| `mio::net::TcpStream::connect` | nonblocking, `WSAEWOULDBLOCK` được mio coi là `Ok` | giữ nguyên `try_connect` |
| chờ connect | `Interest::WRITABLE`, rồi `take_error()` + `peer_addr()` | AFD `POLL_CONNECT_FAIL` → `Event::is_error()`; scheduler wake mọi event nên retry vẫn chạy |
| lỗi chờ | `ErrorKind::WouldBlock/NotConnected` + mã `10035`, `10036`, `10057` | bỏ phụ thuộc `115` (EINPROGRESS Linux) hoặc gate `cfg(unix)` |
| `listen`/`accept` | listener phải **nonblocking**; accepted socket thừa hưởng nonblocking | `TcpListener::bind` → `set_nonblocking(true)` → `MioListener::from_std` |
| `connect` host (blocking API) | `connect_timeout` → `set_nonblocking(true)` → `MioStream::from_std` | **hiện thiếu** `set_nonblocking` (cả Unix); sửa trong P2 |
| read/write | `WouldBlock`/`Interrupted` → chờ; `WSAECONNRESET (10054)` → lỗi dữ liệu | partial write giữ offset |
| timeout | deadline scheduler; `frame_idle_real` cap 5 s; host loop sleep 2 ms | Windows timer ~15,6 ms khi `Poll` timeout rất nhỏ — test phải có biên |
| cancel/close | bỏ đăng ký token + drop socket; `mark_delete()` huỷ AFD poll | kiểm tra không còn event stale trong `token_map` |
| đổi tên log | `"epoll_wait thất bại"` → `"poll thất bại"` | không đổi semantics |

### 2.6. Bảng mã lỗi Windows

| Mã | Tên | Ý nghĩa trong `try_connect`/I/O |
|---:|---|---|
| 10035 | `WSAEWOULDBLOCK` | đang chờ — `WouldBlock` |
| 10036 | `WSAEINPROGRESS` | connect đang chạy — `WouldBlock` |
| 10057 | `WSAENOTCONN` | chưa nối xong — `WouldBlock` |
| 10061 | `WSAECONNREFUSED` | nối bị từ chối — lỗi dữ liệu |
| 10054 | `WSAECONNRESET` | peer reset — lỗi dữ liệu (không flake test server: xem `0bd3a64`) |
| 10053 | `WSAECONNABORTED` | accept/read bị huỷ — lỗi dữ liệu |
| 995 | `ERROR_OPERATION_ABORTED` | huỷ AFD poll khi close — **không** trả cho người dùng |

## 3. Rủi ro IOCP/AFD vs epoll và cách giảm

| # | Rủi ro | Ảnh hưởng | Giảm thiểu |
|---|---|---|---|
| R1 | Một đối tượng mio chỉ thuộc một `Poll` | `host_poll` + reactor trên cùng socket ⇒ `AlreadyExists`/cướp selector | ownership invariant §2.3; lỗi rõ thay vì `reregister` mù |
| R2 | `reregister` bỏ qua `Registry` trên Windows | cập nhật nhầm selector | chỉ gọi trên đúng `Poll` sở hữu; test 2 Poll riêng |
| R3 | Edge-trigger mô phỏng: chỉ re-arm khi gặp `WouldBlock` qua `do_io` | đọc/ghi ngoài `mio` làm mất wake | mọi I/O qua `MioStream`; không dùng fd thô |
| R4 | `register` = "AlreadyExists" ở mức đối tượng, không phải fd | token cũ/stale sau khi move `N:`→`C<n>` | `reregister` khi `AlreadyExists`; test connect rồi read ngay |
| R5 | AFD dùng `SIO_BASE_HANDLE`; LSP/VPN/proxy chen giữa có thể phá readiness | test treo trên máy có antivirus/LSP lạ | ghi nhận môi trường; chạy `psvn` sạch; test refused/reset |
| R6 | `Poll::poll` không cho poll lồng nhau (`is_polling` assert) | host wait trong lúc scheduler đang poll | host TLS chỉ chạy ngoài `frame_idle_real` (đúng thiết kế hiện tại) |
| R7 | Timeout Windows thô (ms, timer ~15,6 ms) | test 50 ms có thể lệch | biên rộng, assert "có timeout" thay vì đo chính xác |
| R8 | `from_std` **không** tự set nonblocking (Unix lẫn Windows) | host accept/read chặn | `set_nonblocking(true)` trước `from_std` (§2.5) |
| R9 | `os-ext`/NamedPipe không dùng | tăng bề mặt biên dịch Windows | bỏ `os-ext` nếu không cần; giữ `os-poll`+`net` |
| R10 | wasm bị kéo `mio` | vỡ target wasm | giữ `cfg(any(unix, windows))`, wasm fallback |
| R11 | Async h2 chưa tích hợp (W2) | dễ tuyên bố quá mức | test h2 async chỉ sau khi W2 land; ghi rõ N/A |

## 4. Test plan Windows

### 4.1. Ca sẵn có — mở gate (đã chạy ở spike)

| Tệp | Ca | Spike Windows |
|---|---|---:|
| `vs3_default.rs` | `cli_async_net_{read,read_timeout,accept,write_partial}_loopback_hai_engine` | **4/4** |
| `vs3_default.rs` | `cli_async_http_*` (get/head/redirect/body lớn/timeout/cap/hai tác vụ/cache) | **9/9** |
| `vs3_tls.rs` | 21 ca TLS (gồm `async_https_*`, `async_net_backend_tls_{doc_ghi,connect}_qua_reactor`) | **21/21** |

Mỗi ca chạy **cả interp và VM** (đúng hợp đồng `hai_engine`).

### 4.2. Ca mới cần thêm

1. **Loopback cơ bản**: connect/listen/accept IPv4 `127.0.0.1`, IPv6 `[::1]`, `localhost`
   (Windows có thể trả `::1` trước — đã sort IPv4-first ở `ket_noi_tls`/`loopback_addr`).
2. **Read/write**: EOF, đóng giữa chừng, partial write 300 KB và 1 MB, write khi peer không đọc
   (timeout), `WSAECONNRESET` (server `shutdown`/RST) trả lỗi dữ liệu không panic.
3. **Connect**: refused (port đóng), timeout 50 ms, huỷ khi đang chờ, hai connect đồng thời.
4. **Accept**: timeout 100 ms, accept sau khi client đến trễ, đóng listener khi đang chờ.
5. **Timeout/cancel**: read 50 ms, write treo, `net.close` từ task khác khi đang chờ;
   không `VS0505`, không busy-spin (đo CPU ~0 khi chờ).
6. **TLS**: handshake OK/sai chứng chỉ/sai tên/timeout; HTTPS body lớn; ALPN; TLS host
   (`AsyncNetHost`) **và** TLS reactor (`T:`/`C<n>`) trên cùng binary.
7. **HTTP/1.1 async**: get/head/redirect/timeout/cap/hai tác vụ cùng host/body lớn/cache warm-cold.
8. **HTTP/2 async**: **chỉ sau khi W2** tích hợp `h2_core`/`AsyncNet`; trước đó ghi `N/A` và giữ
   test h2 sync hiện có (không hồi quy).
9. **Handle/fd leak**: 1 000 vòng connect→write→close; assert `streams/listeners/tls/connects/
   writes/tokens` rỗng; `(Get-Process -Id $PID).HandleCount` không tăng đơn điệu (cho phép ±vài
   handle do runtime); chạy 10 vòng.
10. **10 vòng soak**: mỗi vòng `cargo test --workspace --no-fail-fast` + `bin/vis-host-suite` +
    `bin/vis-targets` + 2 suite async ở trên; ghi thời gian/pass/fail/flake; 0 flake mới đạt.
11. **Hồi quy Unix**: gate đầy đủ (kỳ vọng 3536/3 tại mốc này) + 2 suite async; `lat256_net`
    (Linux-only) vẫn xanh; `async_net` không đổi hành vi.

### 4.3. Bằng chứng spike (đã chạy, không merge)

- Máy: `psvn` Windows `MINGW64_NT-10.0-26300`, 6 CPU, cargo/rustc **1.100.0**, MSVC native.
- Nhánh `spike/w4-win-async-check` @ **`4319b5d`** (6 tệp, **+198/−134**):
  `async_net.rs`, `main.rs`, `lib.rs`, `Cargo.toml`, `vs3_default.rs`, `vs3_tls.rs`.
- Kết quả:
  - `cargo check -p vis-cli` **PASS** (3 s), `--all-targets` **PASS** (19 s),
    `--no-default-features` **PASS** (2,3 s).
  - `vs3_default cli_async_net`: **4/4** (2,23 s); `cli_async_http`: **9/9** (1,74 s);
    `vs3_tls`: **21/21** (6,47 s).
  - Lặp **3 vòng**: mỗi vòng 15 ca async net/http + 21 ca TLS, **0 fail**, 9–10 s/vòng.
  - Local Linux: `cargo check --all-targets` PASS; `vs3_default` 21/21, `vs3_tls` 21/21.
- Log `psvn` `/c/Users/ps/vislogs/w4-73af17b-winspike-*`; sha256:
  `check3 0be8c2d4…d375`, `asyncnet fe5a1dda…4910`, `asynchttp 3d89053f…a946`,
  `asynctls ae550658…bc0e`, `loop 3f5f4d2b…b0ac`.
- **Đã dọn**: worktree spike trên `psvn` xoá; repo chính `psvn` @ `73af17bd`, dirty 0.

> **Không tuyên bố production support.** Spike chỉ chứng minh: (a) biên dịch được trên MSVC,
> (b) các ca hiện có chạy xanh trên loopback. Chưa có: 10 vòng soak, leak test, h2 async,
> xử lý ownership `host_poll` đầy đủ, code review, gate Linux đầy đủ.

## 5. Phase / LOC / thời gian

| Phase | Nội dung | LOC ước tính (prod/test) | Thời gian |
|---|---|---:|---:|
| **P0** | Spike (đã xong, không merge) | 148/50 | 0,5 ngày |
| **P1** | cfg + `lib.rs`/`main.rs`/Cargo + bỏ `SourceFd`/`AsRawFd` + `Nguon`/`nguon_mut` + `reregister` | 150–190 (30–50 test gate) | 0,5–1 ngày |
| **P2** | `host_poll` ownership + `set_nonblocking` + mã lỗi Windows + `wait` log + timeout/cancel | 80–120 (20–40) | 0,5–1 ngày |
| **P3** | Mở gate 2 suite + ca mới (loopback/refused/reset/cancel/handle leak) | 20–40 (250–350) | 1–1,5 ngày |
| **P4** | 10 vòng `psvn` + Linux gate + log/sha + journal | 20–40 | 1 ngày (+ soak) |
| **Tổng** | | **~450–650** (prod ~250–350) | **3–4,5 ngày** + 1 ngày soak |

Biên rủi ro 30% cho các pha P2/P3 (IOCP edge case, test timing Windows).
Người làm cần: quyền sửa `async_net.rs`/`main.rs` + test `vs3_default`/`vs3_tls`; **không** đụng
`net/h2.rs` (W2) và không đổi ABI/bytecode/cache.

## 6. Kết luận

1. **Khả thi**: port không cần crate mới, không cần viết lại rustls; mio 1.2.5 đã hỗ trợ Windows
   đủ cho `TcpStream`/`TcpListener` + `Poll` IOCP/AFD.
2. **Điều kiện tiên quyết**: chuyển từ "fd + `SourceFd`" sang "đối tượng mio + `Source`"; tôn
   trọng ràng buộc một-socket-một-Poll; set nonblocking trước `from_std`.
3. **Chưa hỗ trợ**: Windows async net hiện vẫn deadlock/không có backend; tài liệu/CLI không được
   nói khác cho tới khi P4 xanh 10 vòng. Async h2 **ngoài phạm vi** (W2 WIP).
4. **Blocker**: không có. `BLOCKED-ENV` duy nhất là thiếu target `x86_64-pc-windows-gnu` và mingw
   trên máy Linux — đã thay bằng MSVC native trên `psvn` (bằng chứng mạnh hơn cho đích thật).

## 7. Tệp tham chiếu

- `crates/vis-cli/src/async_net.rs` (toàn bộ), `crates/vis-cli/src/main.rs:718–930`,
  `crates/vis-cli/src/lib.rs:10–11`, `crates/vis-cli/Cargo.toml:150–160`.
- `crates/vis-cli/tests/vs3_default.rs:308–535`, `crates/vis-cli/tests/vs3_tls.rs:503–893`.
- `agent/JOURNAL/vs3-w4-asyncnet-windows.md` (đề xuất trước, `4d016c8`), `agent/VS3-TIEN-DO.md`
  (W2 async h2 WIP).
- mio 1.2.5: `src/io_source.rs`, `src/sys/windows/mod.rs` (`IoSourceState`), `src/sys/windows/
  selector.rs` (AFD, `register`/`reregister`), `src/sys/windows/event.rs` (`POLL_CONNECT_FAIL`).
