<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W2 — spike HTTP/2 **reactor-native** (90 phút, default-off)

**Trạng thái**: **SPIKE THÀNH CÔNG một phần** — chứng minh lõi session poll-driven + true
multiplex 4 stream/1 connection; **chưa nối** `FrameReactorBackend`/`async_http.rs` (gap analysis
§6). Không đổi API sync, không bật default, không dependency mới, **không** push/merge/tag,
không GitHub Actions.
**Mốc**: main `e833af0`; tag `v0.3.3` = `bbf3484`.
**Nhánh**: `vs3/w2-tls`, `git merge main` fast-forward `2183a48 → 5e61898` (worktree
`/home/ps/.local/share/opencode/worktree/cd45e5/vs3-w2-tls`).
**Log đầy đủ**: [`vs3-w2-async-h2-spike.log`](vs3-w2-async-h2-spike.log).

## 1. Kết quả ngắn

| Câu hỏi spike | Kết quả |
|---|---|
| Lái `h2::client` bằng reactor (fd readiness) không executor/thread? | **Có** — `IoH2` trả `Pending` + ghi chiều cần chờ; `H2Async::buoc` chỉ poll, `tai` chờ `mio`; **không** `yield_now`, không `cho()` |
| 4 stream đồng thời trên **một** connection (true multiplex)? | **Có** — server chờ đủ 4 stream **trước khi** trả bất kỳ response; client nhận đúng 4 body; `connection=1`, `stream=4`, **40,96–41,11 ms** |
| GET/HEAD? | **Có** — GET body đúng; HEAD chỉ header, body rỗng |
| Cap body? | **Có** — 8 MiB + 1 byte ⇒ `Err` "vượt giới hạn 8388608 byte (8 MiB)", **278,1–285,0 ms**, không OOM/treo |
| GOAWAY? | **Có** — `abrupt_shutdown(INTERNAL_ERROR)` trước khi accept stream ⇒ `GOAWAY HTTP/2 khi kết nối: …`, không chờ hết hạn |
| Timeout? | **Có** — 400 ms ⇒ `quá thời gian chờ phản hồi HTTP/2` sau **400,9–401,3 ms**; RST stream + bỏ phiên |
| Không rò? | **Có** — 20 vòng timeout, `/proc/self/fd`: `sau <= trước + 4` |
| Dùng lại framing/HPACK? | **Có** — cùng crate `h2 0.4.20`, `Request`/`Response`/`RecvStream`, `release_capacity`; không copy code framing |
| Nối được production trong 90 phút? | **Không** — xem gap §6; ước lượng **700–1100 dòng** còn lại |

**Kết luận**: khả thi về mặt kỹ thuật; rào cản còn lại là **tích hợp scheduler/state machine**,
không phải HTTP/2. Giữ feature `h2-async-spike` **default-off** trên nhánh spike.

## 2. Thay đổi

| Tệp | Thay đổi |
|---|---|
| `crates/vis-cli/src/net/h2_async.rs` (mới, 780 dòng) | `H2Async` (session nhiều stream), `Song` (state machine từng stream), `IoH2` (AsyncRead/AsyncWrite reactor-native trên rustls nonblocking), `h2_get_async` + `h2_get_async_voi_goc` + `phien_voi_goc` (`#[doc(hidden)]`) |
| `crates/vis-cli/src/net.rs` | khai báo `pub mod h2_async` sau cfg feature |
| `crates/vis-cli/Cargo.toml` | feature `h2-async-spike = ["http2", "tls"]`, **không** nằm trong `default`; không dep mới |
| `crates/vis-cli/src/net/h2.rs` | chỉ đổi visibility: `MAX_HEADER`, `loi_h2` → `pub(super)` để **dùng lại** (không copy) |
| `crates/vis-cli/tests/vs3_http2.rs` | +307 dòng, module `#[cfg(feature = "h2-async-spike")] h2_async_spike`: 6 test + server TLS/h2 local |

**API tối thiểu**:

```rust
pub fn h2_get_async(url: &str, timeout_ms: u64) -> Result<KetQuaHttp2, String>
```

Trả `status/headers/body` (`KetQuaHttp2` sẵn có) hoặc `Err` rõ (TLS/ALPN/GOAWAY/RST/cap/timeout).
Chỉ `https://` + ALPN h2; không h2c, không redirect/cookie (spike).

## 3. Thiết kế lõi (đã chạy)

```text
H2Async
  send: h2::client::SendRequest<Bytes>          // HPACK/framing của h2, không copy
  conn: Pin<Box<h2::client::Connection<IoH2, Bytes>>>
  songs: HashMap<u64, Song>                     // nhiều stream song song
  co: Rc<Cell<u8>>                              // MUON_DOC | MUON_GHI lần poll gần nhất
  fd: RawFd                                     // fd của KenhTls nonblocking

gui(method,target,headers) -> id                  // gửi TRƯỚC, không chờ
buoc() -> bool                                    // poll conn + mọi Song một lượt
tai(ms)                                           // lặp buoc; chờ fd bằng mio khi Pending
ket_qua(id) -> Result<KetQuaHttp2, String>
```

- `IoH2::poll_read/poll_write/poll_flush` trả `Pending` đúng hợp đồng tokio; byte đã nhận vào
  rustls **không mất** (poll_write trả `Ready(n)` sau khi buffer, flush riêng).
- `buoc` trả `true` khi có tiến triển ⇒ `tai` chạy tiếp ngay, không chờ event đã tiêu thụ; chỉ
  khi thật sự `Pending` mới đăng ký fd với `mio` (`Interest` suy từ `co`).
- Khi connection đóng/lỗi: poll nốt các stream để h2 trả lỗi **theo stream** (GOAWAY/RST) trước,
  chỉ stream còn treo mới nhận lỗi connection.
- Timeout có hiệu lực **cả trong vòng còn tiến triển** (server bơm liên tục), không chỉ lúc
  `WouldBlock`. Hết hạn: `RST_STREAM(CANCEL)` mọi stream còn mở, đánh dấu phiên đóng.

## 4. Test (feature `h2-async-spike`)

Thêm vào `tests/vs3_http2.rs` để đúng lệnh yêu cầu; 19 test sync phía trên **không đổi**.

| # | Test | Kỳ vọng | Kết quả |
|---|---|---|---|
| 1 | `async_h2_get_va_head` | GET body `hello`; HEAD status 200/body rỗng; 2 stream | PASS |
| 2 | `async_h2_bon_stream_mot_connection` | 4 stream gửi trước, server đủ 4 mới trả (ngược thứ tự); đúng body; 1 connection | PASS — 40,96 ms (log) |
| 3 | `async_h2_goaway_loi_ro` | GOAWAY trước accept ⇒ `Err` nêu GOAWAY, < 3 s | PASS |
| 4 | `async_h2_timeout_khong_treo` | server treo, 400 ms ⇒ `Err` "thời gian", < 3 s | PASS — 400,9 ms (log) |
| 5 | `async_h2_cap_body` | 8 MiB + 1 ⇒ `Err` cap; request sau vẫn chạy | PASS — 278,1 ms (log) |
| 6 | `async_h2_timeout_lap_khong_ro_fd` (Linux) | 20 timeout, fd không tăng > 4; đếm fd **process-wide** nên chạy trong tiến trình con (`..._con`, `#[ignore]`) | PASS — fd 5 → 5 |

Chạy lặp 3 lần liên tiếp đều xanh (5 test song song + test fd trong tiến trình con). Test fd
đếm **process-wide**; nếu chạy chung 25 test song song sẽ đếm fd của test khác, nên tách tiến
trình con để không flaky. Chạy riêng: **fd 5 → 5** sau 20 timeout.

**Full `--test vs3_http2` có spike**: **25 pass, 1 ignored** (19 sync + 5 spike song song + wrapper
fd; test fd chạy tiến trình con) — 15,7–15,9 s song song, 40,98 s `--test-threads=1`.
**Không spike** (default): **19/19 pass**, 15,54–16,14 s ⇒ không hồi quy.

## 5. Đo

Máy i5-1235U, `nproc=12`, Linux, debug build; load ngoài không kiểm soát.

| Ca | Số đo |
|---|---|
| 4 stream/1 connection (gửi 4 trước, server đợi đủ 4) | **40,96 ms** (log); connection=1, stream=4 |
| Body cap 8 MiB + 1 | **278,1 ms** tới `Err` |
| Timeout 400 ms | **400,9 ms** (sai số hạn 0,9 ms) |
| 20 × timeout + connect/handshake (tiến trình con) | **3,04 s**; fd **5 → 5**, không tăng |
| Full có spike | 15,7–15,9 s song song (25 pass, 1 ignored) / 40,98 s threads=1 |
| `cargo fmt --check` | OK |
| `cargo clippy --workspace --all-targets --features h2-async-spike -- -D warnings` | OK |
| `./vis-dev check` (fmt/clippy/rules/self-test/build/doc/readme) | **PASS** |

## 6. Gap analysis — phần sync/private cần tách, ước lượng

### 6.1 `PhienH2` hiện tại (sync, private)

- `pub(super) struct PhienH2` nằm trong module **private** `net::h2`; field `send`, `conn`, `han`,
  `authority`, `luc_dung` đều private.
- `conn` bị **đóng kiểu** `Connection<DongBo, Bytes>` — adapter sync + executor `cho()`
  (`thread::yield_now`). Không thể poll nó bằng adapter reactor khác; muốn chia sẻ phải
  **re-handshake** hoặc refactor kiểu IO.
- `request`/`doc_phan_hoi` là vòng lặp **blocking sở hữu state response**; không tách được state
  per-stream. `gui_nhieu` chỉ `#[cfg(test)]`, không phải API production.
- **Cần tách**: đưa `Song` + `IoH2` + `H2Async` (của spike) vào `net/h2_core.rs`; `PhienH2`
  thành wrapper mỏng (`gui` + `tai_trong_han` + `ket_qua`). Sync giữ nguyên hành vi, chạy lại
  19 test `vs3_http2`. **Ước lượng 150–220 dòng**, không đổi API sync.

### 6.2 `FrameReactorBackend` / `AsyncNet`

- Backend chỉ có `try_read`/`try_write` theo handle byte và `register(token, handle, op)` chọn
  **một** `Interest` từ `ReactorOp`. H2 cần **READABLE|WRITABLE luân phiên theo bước** và cần
  bước session (không chỉ đọc byte).
- **Cần**: `ReactorOp::ReadWrite` hoặc `fn interest(handle)` trên backend; `try_read` cho handle
  `H<n>` gọi `H2Async::buoc` rồi trả `WouldBlock`/meta; map `H<n> -> H2Async`; TLS async đặt
  ALPN `["h2","http/1.1"]` khi feature + https và trả `H<n>` thay `C<n>` khi ALPN=h2.
  **Ước lượng 180–300 dòng** (`async_net.rs`, gồm session map + ALPN + đóng/dọn).
- `AsyncNet` single-thread, `H2Async` có `Rc<Cell<u8>>` ⇒ sống được trong host; không cần `Send`.

### 6.3 Core state machine (`vis-interp/src/net/async_http.rs`)

- Hiện chỉ ghi/đọc HTTP/1.1 byte; h2 trả status/header/body có cấu trúc. Hai lựa chọn:
  (a) thêm stage `H2Send/H2Recv` + backend method meta (sạch, đúng W1 Option A);
  (b) nhồi response h2 thành byte HTTP/1.1 (nhanh nhưng lặp limit/trailer, dễ lệch).
- **Cần**: stage mới + deadline theo request + nhánh handle `H<n>`. **Ước lượng 300–450 dòng**
  (chưa kể test 17 ca W1).

### 6.4 Parity sản phẩm còn thiếu

| Phần | Trạng thái spike | Ước lượng |
|---|---|---:|
| Redirect GET (5 hop), HEAD không theo | chưa | 60–90 |
| Cookie jar / Basic auth | chưa | 60–100 |
| Session pool theo authority + idle 15 s | chưa (mỗi `h2_get_async` một phiên) | 120–180 |
| Cancellation hook scheduler → RST/cleanup | chưa (timeout thì có; huỷ task giữa request chưa) | 50–80 |
| TLS handshake nonblocking dùng chung `async_net` | viết riêng ~90 dòng trong spike | tách helper ~60–90 |
| DNS `to_socket_addrs` | vẫn blocking (như async hiện tại) | ngoài phạm vi |
| Windows (W4) | `#![cfg(unix)]` | theo W4 |

### 6.5 Tổng

**700–1100 dòng production + test** để nối production (khớp ước lượng W1 700–1200). Spike đã
chứng minh phần khó nhất (lái h2 không executor, multiplex thật, cap/GOAWAY/timeout) trong
**~500 dòng code production** (780 dòng kể cả doc/comment).

## 7. Quyết định cần A0/W1

1. **Refactor dùng chung** (khuyến nghị): tách `net/h2_core.rs`, `PhienH2` thành wrapper sync;
   một client h2 duy nhất, không lệch hành vi. Ngược lại: giữ spike riêng và nối trực tiếp
   (nhanh hơn nhưng hai client).
2. **Backend interest**: thêm `ReactorOp::ReadWrite`/`interest(handle)` (khuyến nghị) hay đăng ký
   hai token? Cần trước khi nối scheduler.
3. **Response trong core**: structured meta (khuyến nghị) hay byte HTTP/1.1 tổng hợp?
4. Có cho feature `h2-async-spike` (default-off) vào main như bằng chứng/spike, hay giữ nhánh
   `vs3/w2-tls` tới khi nối xong?

## 8. Lệnh tái lập

```bash
cd /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w2-tls
CARGO_TARGET_DIR=/home/ps/ViScript/target \
  ~/.cargo/bin/cargo test -p vis-cli --features h2-async-spike --test vs3_http2
# test rò fd chạy riêng trong tiến trình con (đếm fd là process-wide)
CARGO_TARGET_DIR=/home/ps/ViScript/target \
  ~/.cargo/bin/cargo test -p vis-cli --features h2-async-spike --test vs3_http2 \
    async_h2_timeout_lap_khong_ro_fd_con -- --ignored --nocapture
CARGO_TARGET_DIR=/home/ps/ViScript/target \
  ~/.cargo/bin/cargo test -p vis-cli --test vs3_http2          # default: 19 sync
~/.cargo/bin/cargo fmt --all -- --check
CARGO_TARGET_DIR=/home/ps/ViScript/target \
  ~/.cargo/bin/cargo clippy --workspace --all-targets --features h2-async-spike -- -D warnings
```

## 9. Blocker

- Không có blocker kỹ thuật cho hướng Option A; chỉ thiếu **thời gian** trong 90 phút để nối
  `AsyncNet` + core + parity (700–1100 dòng).
- Không có push/merge/tag; nhánh spike sẵn sàng cho W1/W2 tiếp.
