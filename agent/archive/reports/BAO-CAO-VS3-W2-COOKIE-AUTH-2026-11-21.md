<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W2 — cookie jar + Basic auth cho HTTP (2026-11-21)

> **Baseline**: `main` `8fe0802` (fast-forward từ `cc74dbc`, không conflict).
> **Phạm vi**: VS3-4b còn lại, **không** HTTP/2, **không** dependency mới, không đụng `pkg.rs`/`frame/**`.

## 1. Cookie jar nội bộ

- `vis-interp/src/cookie.rs`: `CookieJar` (Rust nội bộ, không phải API ngôn ngữ) — parse
  `Set-Cookie` **tối thiểu RFC 6265**:
  - `name=value` (bỏ quote đơn giản), tên/giá trị có ký tự điều khiển hoặc `;` bị từ chối;
  - `Path` (default-path RFC), `Domain` (domain-match, IP chỉ khớp chính xác; bỏ `.` đầu),
    `Secure`, `HttpOnly` (ghi nhận, không ảnh hưởng vì không có JS);
  - `Max-Age` (âm/0 = xoá; thắng `Expires`), `Expires` IMF-fixdate
    `Wed, 21 Oct 2015 07:28:00 GMT` (không crate; `SystemTime`).
- Giới hạn: tối đa **64** cookie/jar, tên+giá trị ≤ **4096** byte; jar nằm trong `Net`, không ghi đĩa.
- Gửi header `Cookie` theo **host/path**, chỉ gửi `Secure` khi URL https; **không** gửi sang host
  khác. Cookie nhận ở hop redirect được lưu **trước** khi theo `Location` ⇒ hop sau cùng host thấy.
- `NetHost::cookie_jar` (default `None`) để host khác (fake/VM) không có cookie; `NetGuard` chuyển
  tiếp; `Net` (CLI) giữ jar. Không có `http.cookie*`/cờ người dùng.

## 2. Basic auth

- URL dạng `scheme://user:pass@host[:port]/…`:
  - `tach_url` tách userinfo khỏi authority, **không** đưa vào `Host`/`authority`/log;
  - header `Authorization: Basic base64(user:pass)` (base64 nội bộ, không crate);
  - userinfo thiếu `:` ⇒ lỗi rõ `user:pass@host`; ký tự điều khiển ⇒ lỗi, không header injection.
- Redirect:
  - `Location` tương đối cùng origin ⇒ giữ userinfo (auth tiếp tục);
  - `Location` tuyệt đối **không** có userinfo ⇒ **không** chuyển Authorization sang host khác;
  - `Location` có userinfo ⇒ dùng đúng userinfo mới.
- **Không** log credential: `--explain-perms` chỉ in `NET http GET authority…`; lỗi URL đi qua
  `an_toan` (thay userinfo bằng `***`). Không có API header/`401` challenge tự động.

## 3. Test hermetic (`crates/vis-cli/tests/vs3_cookie_auth.rs`)

| Ca | Nội dung |
|---|---|
| `set_cookie_qua_redirect_duoc_gui_lai` | 302 + `Set-Cookie` ở `/a` → `/b` gửi `Cookie` |
| `cookie_giu_giua_hai_loi_goi` | jar trong `Net` giữ cookie qua 2 `http_get` |
| `secure_khong_gui_qua_http` | `Secure` không gửi trên http |
| `cookie_path_va_cross_host` | `Path=/a` không gửi `/b`; host `localhost` không thấy cookie `127.0.0.1`; quay lại host cũ vẫn gửi |
| `cookie_het_han_bi_bo` | `Max-Age=0` xoá cookie |
| `basic_auth_dung_va_khong_lo_credential` | header Basic đúng; `vis run --explain-perms` không chứa `user:pass`/base64 |
| `auth_khong_di_theo_redirect_khac_host` | 302 sang host khác không có Authorization |
| `head_cung_luu_cookie` | `http.head` lưu Set-Cookie, request sau gửi |
| `userinfo_sai_khong_panic_va_khong_lo` | `user@` thiếu mật khẩu ⇒ lỗi rõ, không lộ userinfo |

Unit `vis-interp/src/cookie.rs` (5 ca): path/secure/httpOnly, domain/default-path, Max-Age/Expires,
từ chối domain lạ/ký tự điều khiển, parse ngày. Unit `net.rs` (8 ca) thêm userinfo/Basic auth và
không lộ credential trong lỗi.

## 4. Lệnh + kết quả

```text
cargo test -p vis-cli --test vs3_cookie_auth      9 passed / 0 failed
cargo test -p vis-interp --lib cookie::tests      5 passed / 0 failed
cargo test -p vis-interp --lib net::tests         8 passed / 0 failed
cargo test -p vis-cli --test http                22 passed / 0 failed
cargo test -p vis-cli --test vs3_keepalive        8 passed / 0 failed
cargo test -p vis-cli --test vs3_tls             12 passed / 0 failed
cargo test -p vis-cli --test vs3_registry_https   6 passed / 0 failed
cargo test -p vis-cli --test vs3_registry --test vs3_registry_sig
                                                 11 + 9 passed
cargo test -p vis-cli --no-default-features --test vs3_cookie_auth
                                                  9 passed / 0 failed
cargo test -p vis-cli --no-default-features --test vs3_registry
                                                 11 passed / 0 failed
cargo check -p vis-cli (default/core/wasm-core/win-core/release) -> Finished
cargo clippy ... -- -D warnings                  -> Finished
./vis-dev rules                                  -> OK
./bin/vis-targets                                core 18/18; sqlite 16 + 2 EXPECTED_RED; exit 0
```

Log đầy đủ: `agent/JOURNAL/vs3-cookie-auth.log`.

## 5. Còn lại / giới hạn

- Cookie: **không** SameSite, public-suffix list, cookie prefix (`__Host-`/`__Secure-`), IDNA,
  persistent store, chia sẻ giữa process; `Expires` chỉ IMF-fixdate; không có API xoá/đọc cookie
  từ chương trình.
- Auth: chỉ Basic qua userinfo; **không** xử lý `401 WWW-Authenticate`, không API header tuỳ biến;
  không chuyển auth qua redirect khác host; credential nằm trong source người dùng (không log).
- `http://` vẫn loopback; `https://` host bất kỳ qua `cap[Net]` (với chương trình); không proxy/HTTP2.
- TLS test root hook vẫn **debug-only**; release không biên dịch.
- Không đụng `pkg.rs` (W5) hay `frame/**`.
