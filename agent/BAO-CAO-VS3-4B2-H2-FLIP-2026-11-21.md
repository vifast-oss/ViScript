<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-4b2 — kiểm tra default-on `http2` cho v0.3.0 (2026-11-21)

> **Baseline**: `main` `f710e5b` (fast-forward, chứa `77b71fb`); **commit thử** trên `vs3/w2-tls`
> bật `http2` trong `default` của `vis-cli`. **Đây là đề xuất thử, chưa được A0 duyệt flip.**
> **Cập nhật 4b3**: bản thử default-on được giữ ở nhánh riêng `vs3/w2-http2-default-on`
> (`959d1aa` + merge main); `vs3/w2-tls` đã trở lại **default-off** trước tag v0.2.0.**

## 1. Diff

```diff
 [features]
-default = ["sqlite", "tls", "pkg-sig"]
+default = ["sqlite", "tls", "pkg-sig", "http2"]
```

`http2 = ["tls", "dep:h2", "dep:tokio", "dep:http", "dep:bytes"]`; dependency vẫn chỉ target
`cfg(not(target_arch = "wasm32"))`, optional. Không dependency mới ngoài `h2` đã duyệt.

## 2. Kết quả default-on

| Kiểm tra | Kết quả |
|---|---|
| `cargo check -p vis-cli` | **PASS** |
| `cargo check -p vis-cli --no-default-features` | **PASS** (core = không h2) |
| `cargo check -p vis-cli --release` | **PASS** |
| `cargo test -p vis-cli --test vs3_http2` | **6/6** (có registry h2 negotiation) |
| `cargo test -p vis-cli --test vs3_tls` | **12/12** |
| `cargo test -p vis-cli --test http` | **22/22** |
| `cargo test -p vis-cli --test net` | **9/9** |
| `cargo test -p vis-cli --test vs3_keepalive` | **8/8** |
| `cargo test -p vis-cli --test vs3_registry_https` | **6/6** (fallback h1 + h2 client) |
| `cargo test -p vis-interp --lib net::tests` | **8/8** |
| `cargo clippy ... -D warnings` | **PASS** |
| `./vis-dev rules` | **OK** |
| `cargo tree` default Linux | có `h2 0.4.20`, `tokio 1.54.1`, `bytes`, `http` |
| `cargo tree` default wasm | **KHÔNG** có `h2`/`tokio` |
| `cargo tree --target wasm --features http2` | **KHÔNG** có `h2`/`tokio` |

**Registry + h2 thật**: `vs3_http2::registry_https_thuong_luong_h2` chạy `vis pkg add` qua TLS
server h2 (ALPN h2, index + tarball, SHA-256) với root test; **PASS**. Test fallback h1 vẫn chạy
(`fallback_http1_khi_khong_alpn`).

## 3. Kích thước release

| Bản | Byte | Ghi chú |
|---|---:|---|
| default-off (`http2` tắt) | **9.352.896** | baseline commit trước flip |
| default-on (`http2` bật) | **10.079.680** | **+726.784 byte (~7,8%)** |

Build release default-on xanh; thời gian build trong log (máy có cache).

## 4. Windows

- `cargo check --target x86_64-pc-windows-msvc --no-default-features` **PASS**.
- `cargo check --target x86_64-pc-windows-msvc` **FAIL** ở `ring`/`libsqlite3-sys` (thiếu
  `lib.exe` — cùng nợ toolchain C #156, không phải lỗi h2).
- `--no-default-features --features http2` cũng **FAIL** ở `ring` vì `http2` kéo `tls`; **không**
  thể tách h2 khỏi ring trên cross-check Linux. Cần runner Windows MSVC để xác nhận runtime h2.

## 5. Khuyến nghị

**Đề xuất flip default-on cho v0.3.0** nếu A0 chấp nhận:

- +~0,73 MB release (+7,8%) và thời gian build tăng (h2/tokio/bytes/http);
- mọi test h1/h2/registry/TLS xanh trên Linux; wasm không kéo h2/tokio;
- trade-off: mỗi authority HTTPS lần đầu **probe ALPN**; server h1 fallback trên **cùng kết nối**
  (đã test), nên không tăng số kết nối server; h2 vẫn một request/kết nối (không multiplex).
- Windows native chưa đo; nếu A0 muốn Windows Tier 2 cùng v0.3.0, cần runner MSVC trước khi flip.

Nếu A0 chưa duyệt: **revert đúng một dòng** `default = [...]` về `["sqlite","tls","pkg-sig"]`;
feature `http2` vẫn dùng được bằng `--features http2`. Commit thử hiện tại được đánh dấu `[THỬ]`
trong message.

## 6. Log

`agent/JOURNAL/vs3-http2-flip.log` (test + check + tree + Windows cross-check).
