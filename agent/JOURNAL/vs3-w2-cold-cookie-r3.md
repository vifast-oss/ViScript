<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W2 — cold-start 4 task, Set-Cookie async, rà cancel (feature `async-h2`, default-off)

**Trạng thái**: cold-start **PASS** (không cần request làm ấm), Set-Cookie **PASS**; cancel task
là **nợ chung h1/h2**, ghi rõ và chưa mở. Feature `async-h2` **default-off**, không đổi API sync,
không dependency mới, không push/merge/tag.
**Nhánh**: `vs3/w2-tls`, đã `git merge main` (`9241870`).
**Log**: [`vs3-w2-cold-cookie-r3.log`](vs3-w2-cold-cookie-r3.log).

## 1. Cold-start 4 task cùng authority

### 1.1 Nguyên nhân gốc (khác dự đoán ban đầu)

Race cũ **không** nằm ở re-arm nhiều waiter trên cùng một reactor key; nó nằm ở **handle H
dùng chung**:

1. Trước đây mọi frame nhận cùng `H1`; scheduler coi `(H1, ReadWrite)` là một đăng ký. Khi một
   frame được đánh thức và re-arm, nó **giành** registration (gen/token mới), các frame còn lại
   vẫn chờ `{key, gen}` cũ ⇒ chỉ một frame tiến, số còn lại chết tới deadline.
2. Cách sửa **nhỏ hơn** 80–120 dòng scheduler: cấp **handle riêng từng frame** `H<n>#<frame>`
   (`h2_base()` chuẩn hoá về session `H<n>`), kết hợp `AsyncNet` đã đăng ký epoll **một lần/fd**
   và `wait` phát event cho **mọi** token của fd (fd-wake-all, có từ commit trước). Mỗi frame có
   reactor key riêng nhưng chung connection; không còn tranh registration.

Không sửa `frame_real_wait`/gen: mọi reactor key h2 hiện có đúng **một** waiter, nên nhánh
"multi-waiter re-arm" không còn đường kích hoạt. Nếu sau này muốn nhiều waiter chung một key,
vẫn cần fix scheduler ~80–120 dòng (đã ghi ở mục 4).

### 1.2 Bằng chứng

Test `async_h2_production_bon_stream_cold` (mới):

- `vis run` interp **và** VM, 4 `spawn` `http.get_timeout` cùng authority, **không** warm-up.
- Server `Cho4` chỉ trả sau khi nhận đủ **4 stream trên một connection**; đếm server-side.
- Kết quả: **PASS**, mỗi engine `conns=1`, `streams=4`; tổng `conns=2`, `streams=8`.

Test `async_h2_cold_multiplex_20_vong` (mới, `VS3_W2_COLD_ROUNDS`, mặc định **20**):

- 20 vòng, xen kẽ interp/VM, mỗi vòng 4 task cold-start.
- Assert mỗi vòng đúng output 4 path, < 5 s, `conns == rounds`, `streams == rounds × 4`.
- Chạy thêm **3 lần × 100 vòng** (`VS3_W2_COLD_ROUNDS=100`): **300/300 vòng PASS**,
  1 connection + 4 stream mỗi vòng, không treo.
- Chạy riêng lẻ (không song song test khác) bản 20 vòng từng kèm kiểm `/proc/self/fd` và pass;
  trong full suite đã bỏ kiểm fd vì fd là process-wide (test khác mở socket song song), thay bằng
  test fd 20-timeout chạy tiến trình con (fd **5 → 5**).

## 2. Set-Cookie response async

- `AsyncNetHost` có **cookie jar riêng** (`CookieJar`), `NetHost::cookie_jar` trả `Some(&mut jar)`;
  `main.rs` khởi tạo. Trước đây async host trả `None` ⇒ h1/h2 async không lưu/không gửi cookie.
- `async_http.rs` thêm `luu_set_cookie(host, url, headers)`:
  - **h2**: gọi trước HEAD/redirect/trả body, nên Set-Cookie của response redirect được lưu.
  - **h1**: gọi ngay sau khi parse header (một lần/response), không đổi luồng đọc byte.
- Gửi lại dùng đúng `thong_tin` cũ (`CookieJar::header` theo host/path/secure) qua
  `header_h2`/request h1 ⇒ parity sync.
- Test production `async_h2_production_cookie_redirect`:
  - `/redir-cookie` → 302 + `Set-Cookie: sid=xyz; Path=/` → `/echo-cookie` trả cookie đã gửi.
  - `/set-cookie` → `sid=abc`, request sau thấy `sid=abc`.
  - Output đúng: `sid=xyz\nsid=xyz\nset\nsid=abc\n`; < 3 s.
- **Không hồi quy h1**: `vs3_tls` **21/21** (gồm 7 async https h1), `vs3_default async_http`
  **9/9**.

## 3. Cancel task giữa request

- **Không có API người dùng để cancel** task đang chạy; `frame_cancel_*` hiện là đường
  **test-only** của scheduler. Không có safe-point hook nào gọi `HttpAsync::dong` khi frame bị huỷ.
- Đây là **nợ chung HTTP/1.1 + HTTP/2**: `DANG_CHAY` giữ handle/session tới khi frame id bị tái
  dùng (lần gọi HTTP kế tiếp mới `dong`). Sửa đúng cần hook scheduler → `net` (đóng handle +
  `frame_reactor_cancel_handle`) và cho cả h1, ước lượng **80–120 dòng** + test lifecycle.
- Phần **đã có**: timeout giữa request gọi `frame_reactor_h2_cancel` (RST_STREAM) rồi close;
  test 20 timeout chạy tiến trình con fd **5 → 5**, không rò stream/fd.
- Theo cho phép của A0: **ghi rõ và bỏ qua** trong đợt này, không thêm nửa hook chỉ cho h2.

## 4. Còn mở

| Việc | Trạng thái |
|---|---|
| Multi-waiter **cùng một** reactor key (nếu sau này bỏ handle `#frame`) | chưa cần; nếu cần: cập nhật mọi `Wait::Reactor{key,gen}` sang gen mới khi re-arm (~80–120 dòng + test) |
| Cancel task giữa request (h1+h2) | nợ chung, xem §3 |
| Cookie: `Set-Cookie`/`SameSite`/`Secure` | dùng `CookieJar` sync (`nhan_set_cookie`), không viết lại parser |
| Windows | `async-h2` vẫn `#![cfg(unix)]` |

## 5. Test / lệnh

```text
cargo test -p vis-cli --test vs3_http2                                19 passed
cargo test -p vis-cli --features async-h2 --test vs3_http2            32 passed, 1 ignored
cargo test ... async_h2_cold_multiplex_20_vong (mặc định 20 vòng)     PASS
VS3_W2_COLD_ROUNDS=100 ... (3 lần)                                    300/300 PASS
cargo test -p vis-cli --features async-h2 --test vs3_tls              21 passed
cargo test -p vis-cli --features async-h2 --test vs3_default async_http 9 passed
cargo fmt --check                                                     OK
cargo clippy --workspace --all-targets -- -D warnings                 OK
cargo clippy -p vis-cli --all-targets --features async-h2 -- -D warnings OK
./vis-dev check                                                       PASS
df                                                                    13 G trống
```

Test file `vs3_http2.rs` tách module sang `tests/support/h2_async_spike.rs` để giữ R1
(884 + 656 dòng); `async_net.rs` 1477 dòng.
