<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-4b3 — h2 connection reuse (tuần tự) cho cùng authority (2026-11-21)

> **Baseline**: `main` `ec29204`; nhánh `vs3/w2-tls` **default-off** (feature `http2`).
> **Commit default-on thử** được giữ ở nhánh riêng `vs3/w2-http2-default-on` (commit `959d1aa`
> + merge main `07a71e1`); không merge default-on trước tag v0.2.0.
> **Không** push/merge/tag.

## 1. Thiết kế reuse

- `vis-cli/src/net/h2.rs`:
  - `PhienH2` giữ **một** `SendRequest` + `Connection` h2 đã handshake, authority, deadline
    (`Arc<Mutex<Instant>>`) cho adapter.
  - `PhienH2::moi` mở TLS + ALPN; nếu server không chọn h2, trả **cùng** `KenhTls` cho h1
    (fallback không mở thêm kết nối).
  - `PhienH2::request` gửi request tuần tự, poll connection + response/body; connection sống tiếp.
- `vis-cli/src/net.rs`:
  - `h2_phien: HashMap<authority, PhienH2>`, **tối đa 4 authority**; request cùng authority dùng
    lại connection.
  - Request lỗi/GOAWAY/timeout ⇒ bỏ phiên, thử mở phiên mới **một lần** (GET/HEAD idempotent).
  - Server chọn h1 ⇒ `h2_biet=false` + `h2_http1` giữ kết nối TLS cho HTTP/1.1 (hành vi 4b2).
  - `cap[Net]` vẫn kiểm trong `NetGuard::http2_get` trước khi host mở TLS.
- **Không multiplex**: một stream tuần tự tại một thời điểm; không h2 push/trailers; không pool idle
  riêng (phiên sống tới khi lỗi/Net drop/giới hạn 4 authority). H1 pool/cookie/auth giữ nguyên.

## 2. Test

- `crates/vis-cli/tests/vs3_http2.rs` (feature `http2`):
  - **mới** `h2_tai_su_dung_ket_noi`: 2 GET cùng authority, server đếm **1 connection**;
  - `h2_get_va_head`, `h2_redirect_va_cookie`, `h2_basic_auth`;
  - `fallback_http1_khi_khong_alpn` (TLS server không ALPN, cùng connection);
  - `h2_timeout_ro_rang`;
  - `registry_https_thuong_luong_h2` (`vis pkg add` qua registry TLS h2, index+tarball).
- Server test `serve_h2` giờ nhận **nhiều request trên một connection** (trước đây một request rồi
  chờ close), phù hợp reuse; fallback h1 server giữ nguyên.

## 3. Lệnh + kết quả

```text
cargo test -p vis-cli --features http2 --test vs3_http2      7 passed / 0 failed
cargo test -p vis-cli --features http2 --test vs3_tls       12 passed / 0 failed
cargo test -p vis-cli --test vs3_tls                        12 passed / 0 failed
cargo test -p vis-cli --test http                           22 passed / 0 failed
cargo test -p vis-cli --test vs3_keepalive                   8 passed / 0 failed
cargo check -p vis-cli (default/http2/wasm-http2)            Finished
cargo clippy ... -- -D warnings                             Finished
./vis-dev rules                                             OK
```

Log: `agent/JOURNAL/vs3-h2-reuse.log`.

## 4. Khuyến nghị default-on

- Bản default-on thử `959d1aa` (branch `vs3/w2-http2-default-on`) vẫn xanh và +~0,73 MB release;
  **không merge** trước v0.2.0. Nếu A0 duyệt sau v0.2.0, cherry-pick/merge nhánh đó (hoặc đổi một
  dòng `default` trong `Cargo.toml`); nếu không, feature `http2` giữ `--features http2`.
- Windows native h2 vẫn chưa đo; cross-check Windows dừng ở `ring`/libsqlite (#156), không phải h2.

## 5. Blocker / còn lại

- Chưa **multiplex** nhiều request đồng thời trên một connection; chỉ reuse tuần tự.
- Chưa h2 push/trailers, chưa idle-timeout riêng cho phiên h2 (giới hạn 4 authority + drop/error).
- Không dependency mới ngoài `h2` đã duyệt; không đụng `frame/**`, `value.rs`, `pkg.rs`.
- Đĩa còn >5G; không push/merge/tag.
