<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-4b4 — h2 queue reuse + idle timeout (2026-11-21)

> **Baseline**: `main` `a26ec4b` (HTTP/2 default-on, v0.3.0 candidate); nhánh `vs3/w2-tls`
> fast-forward, không conflict. **Không** push/merge/tag.
> **Phạm vi**: connection h2 reuse; **không** dependency mới; không đụng `pkg.rs`/`frame/**`/`value.rs`.

## 1. Thiết kế

- `PhienH2` giữ `SendRequest` + `Connection` đã handshake; `Net` cache tối đa **4 authority**.
- **Hàng đợi tuần tự** trên một connection: mỗi `http_get/head` gửi một request, đọc hết response,
  giữ connection cho request kế tiếp. API ngôn ngữ **đồng bộ** nên chưa thể có nhiều stream
  in-flight từ một chương trình; đây là lựa chọn “queue trên một connection” được nêu trong yêu cầu.
- **Idle timeout 15 s**: `PhienH2.luc_dung` cập nhật sau mỗi response; `Net::http2_get` dọn **lazy**
  mọi phiên quá 15 s trước khi tái dùng (đóng connection/TLS ngay khi drop).
- Lỗi/GOAWAY/timeout ⇒ bỏ phiên, thử mở phiên mới **một lần** (GET/HEAD idempotent); deadline
  **không reset** trong một request (adapter dùng `han` chung).
- Fallback h1 giữ nguyên: server không ALPN h2 ⇒ dùng lại **cùng kết nối TLS**; h1 pool/cookie/auth
  không đổi; `NetGuard` kiểm `cap[Net]` trước khi mở h2.
- Không h2 push/trailers; không multiplex đồng thời thật (xem blocker).

## 2. Test

`crates/vis-cli/tests/vs3_http2.rs` (8 ca; default-on nên chạy cả trong `cargo test` thường):
- `h2_tai_su_dung_ket_noi`: **4 GET** cùng authority, server đếm **1 connection** (hàng đợi tuần tự).
- **mới** `h2_idle_timeout_dong_phien`: request 1 → nghỉ **15,2 s** → request 2; server đếm **2**
  connection (phiên idle bị đóng).
- Giữ GET/HEAD, redirect+cookie, Basic auth, fallback h1 không ALPN, timeout, registry h2
  (`vis pkg add` index+tarball qua h2).

```text
cargo test -p vis-cli --test vs3_http2       8 passed / 0 failed (15,3 s, có idle test)
cargo test -p vis-cli --test vs3_tls        12 passed / 0 failed
cargo test -p vis-cli --test http           22 passed / 0 failed
cargo test -p vis-cli --test vs3_keepalive   8 passed / 0 failed
cargo check -p vis-cli (default/no-default/wasm-http2)  Finished
cargo clippy ... -D warnings                 Finished
./vis-dev rules                              OK
```

Log: `agent/JOURNAL/vs3-h2-idle.log`.

## 3. Default-on

- `main` `a26ec4b` đã bật `default = ["sqlite","tls","pkg-sig","http2"]`; nhánh này theo main.
- Bản thử default-on trước đây ở nhánh `vs3/w2-http2-default-on` (lịch sử); hiện main là nguồn.
- Release default-on vẫn cần Windows MSVC thật để xác nhận h2; cross-check Linux→MSVC dừng ở
  `ring`/`libsqlite3-sys` (thiếu `lib.exe`, nợ #156), không phải lỗi h2.

## 4. Blocker / còn lại

- **Chưa multiplex đồng thời thật**: API `http_get/head` đồng bộ + `Net` `&mut` một luồng; muốn
  nhiều stream in-flight phải có API async/thread riêng (lát sau), nên lát này dùng hàng đợi tuần tự.
- Idle cleanup **lazy** (khi có request kế tiếp/Net drop), không có thread reaper; connection idle
  không tự đóng ngay sau 15 s nếu chương trình không làm gì.
- Không h2 push/trailers; không h2 keep-alive idle pool riêng ngoài reuse trên.
- Không dependency mới; không push/merge/tag; đĩa còn >5G.
