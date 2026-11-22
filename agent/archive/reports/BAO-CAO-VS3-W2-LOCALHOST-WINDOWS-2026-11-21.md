<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W2 — fix loopback/`localhost` Windows (2026-11-21)

> **Baseline**: `main` `5dec89b` (fast-forward từ `096ea2c`, không conflict).
> **Blocker**: Windows phân giải `localhost` thành `::1` trước, server/test bind `127.0.0.1` ⇒
> `WSAECONNREFUSED` 10061. Sửa **product** `vis-cli/src/net.rs`, không đổi API/dependency.

## 1. Sửa

- `chi_loopback` trả **Vec<SocketAddr>** thay vì địa chỉ đầu tiên:
  - tách `host:port`/`[IPv6]:port` bằng `tach_dia_chi`;
  - chỉ nhận host `localhost` hoặc IP `is_loopback()` (`127.0.0.0/8`, `::1`);
  - phân giải **tất cả** địa chỉ, **lọc** chỉ địa chỉ loopback;
  - `localhost` luôn có fallback `127.0.0.1:port` (kể cả resolver chỉ trả `::1`);
  - **sắp IPv4 trước**, nên không chờ `::1` bị từ chối trên Windows.
- `Net::connect` thử **lần lượt mọi** địa chỉ trong **cùng deadline** `timeout_ms`; lỗi cuối được
  giữ nguyên thông điệp (timeout/refused/khác), không panic.
- `Net::listen` bind địa chỉ đầu tiên (IPv4 nếu có).
- TLS `tls::noi` cũng sắp IPv4 trước (giữ nguyên danh sách, thử lần lượt) — không đổi policy
  `https://` host bất kỳ.
- `vis-interp::tach_url`: `http://` chấp nhận host loopback `localhost`/`127.0.0.0/8`/`::1`
  (trước chỉ `127.0.0.1`/`localhost`) ⇒ `http://[::1]` chạy; host khác vẫn bị từ chối.
- `quyen::kiem_net`: loopback nhận diện bằng `IpAddr::is_loopback()` (vẫn chỉ auto-allow khi có
  `cap[Net]`; strict + không cap vẫn `VS0604`). Không nới host ngoài loopback.

## 2. Test

- Unit `vis-cli lib`: `chi_loopback_tra_moi_dia_chi_va_chan_remote` — `localhost` có `127.0.0.1`,
  IPv4 đứng trước, `[::1]`/`127.0.0.2` hợp lệ, `example.com`/`10.0.0.1`/`[::2]` bị chặn.
- Unit `vis-interp net`: `http://[::1]`/`127.0.0.2` parse được; `10.0.0.1` bị từ chối.
- Integration `vs3_cookie_auth`:
  - `localhost_bind_127_va_ipv6`: server bind `127.0.0.1`, gọi `http://localhost:...` chạy; server
    bind `[::1]`, gọi `http://[::1]:...` chạy (bỏ qua có ghi chú nếu nền không bật IPv6);
    `http://example.com`/`http://10.0.0.1` bị từ chối.
  - `strict_caps_kiem_localhost`: `--strict-caps` + `cap[Net] @ "localhost"` chạy; không khai cap
    ⇒ `VS0604`, cap vẫn kiểm.
- Các ca cũ (HTTP, net, keep-alive, TLS, cookie/auth, registry) giữ nguyên.

## 3. Lệnh + kết quả

```text
cargo test -p vis-cli --test vs3_cookie_auth      11 passed / 0 failed
cargo test -p vis-cli --test http                22 passed / 0 failed
cargo test -p vis-cli --test net                  9 passed / 0 failed
cargo test -p vis-cli --test vs3_keepalive        8 passed / 0 failed
cargo test -p vis-cli --test vs3_tls             12 passed / 0 failed
cargo test -p vis-interp --lib net::tests         8 passed / 0 failed
cargo test -p vis-cli --lib                       6 passed / 0 failed
cargo test -p vis-cli --test cap --test strict_caps --test quyen
                                                  4 + 7 + 8 passed
cargo test -p vis-cli --bin vis net_quyen::tests  3 passed / 0 failed
cargo check -p vis-cli (default/core/wasm-core/win-core/release) -> Finished
cargo clippy ... -- -D warnings                   -> Finished
./vis-dev rules                                   -> OK
./bin/vis-targets                                 core 18/18; sqlite 16 + 2 EXPECTED_RED; exit 0
```

Log: `agent/JOURNAL/vs3-localhost-windows.log`.

## 4. Còn lại

- Windows **runtime** chưa chạy trên máy Windows thật trong phiên này; fix dựa trên mô tả blocker +
  test localhost đa địa chỉ. Ca Windows 10061 cần xác nhận lại trên runner Windows.
- `localhost` vẫn chỉ loopback; không DNS remote; `[::1]` cần IPv6 loopback của nền (test tự bỏ qua
  có ghi chú nếu bind thất bại).
- Không proxy/cookie/HTTP2 thay đổi; TLS/keep-alive/cookie/auth giữ nguyên.
- Không đụng `pkg.rs` (W5) / `frame/**`; không dependency mới; không push/merge.
