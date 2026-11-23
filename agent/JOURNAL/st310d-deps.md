<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-310d — làm mỏng `vis-cli/Cargo.toml`: 0 dep OS/TLS/SQLite trực tiếp

> **Người làm**: subagent ST-310d, 2026-11-23/24. **Nhánh** `plan/st310d-deps` từ **`e9c62cb`**
> (main). **Không merge/push/tag** trước `v0.5.0`. Máy `ps` (Linux x86_64, rustc 1.100.0).
> `CARGO_BUILD_JOBS=3 TMPDIR=/home/ps/tmp`; local only (không ssh `mps`/`psvn`).

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `[dependencies]` của `vis-cli` | **12 crate nội bộ**, **0 dep OS/TLS/SQLite trực tiếp** |
| 7 dep theo yêu cầu | `rusqlite`, `rustls`, `webpki-roots`, `h2`, `tokio`, `mio`, `rustix` **đã rời `[dependencies]`** |
| Feature mapping | giữ nguyên crate chủ: `sqlite`/`tls`/`http2`/`async-h2` → `vis-host-native`; `tls`/`pkg-sig` → `vis-pkg` |
| Feature matrix | **12/12 tổ hợp** `cargo check -p vis-cli` **PASS** |
| `--all-targets` | default **PASS**, `--no-default-features` **PASS** (test vẫn biên dịch) |
| `./vis-dev gate --fast` | **PASS** — **3754 passed / 0 failed / 4 skipped** |
| fmt / clippy / R1..R7 / toolchain / release / doc-examples | **PASS**; **0 `unsafe`** |
| Blocker | **không** |

## 1. Thay đổi

`crates/vis-cli/Cargo.toml` chỉ còn **feature-forwarding**:

```toml
sqlite   = ["vis-host-native/sqlite"]
tls      = ["vis-host-native/tls", "vis-pkg/tls"]
pkg-sig  = ["vis-pkg/pkg-sig"]
http2    = ["tls", "vis-host-native/http2"]
async-h2 = ["http2", "tls", "vis-host-native/async-h2"]
```

`[dependencies]` còn đúng các crate workspace: `vis-diag`, `vis-syntax`, `vis-fmt`,
`vis-types`, `vis-interp`, `vis-bytecode`, `vis-vm`, `vis-loader`, `vis-lsp`,
`vis-host-native`, `vis-pkg`, `vis-plugin-abi` (+ `vis-host-linux` cfg Linux).

### 1.1. Dep đã bỏ / forward

| Dep | Trước | Sau |
|---|---|---|
| `rusqlite` | `[dependencies]` optional, `sqlite` bật `dep:rusqlite` | feature `sqlite` → `vis-host-native/sqlite`; `rusqlite` chỉ còn **dev-dependency** cho fixture WAL của test |
| `rustls` | `[dependencies]` optional non-wasm | `tls` → `vis-host-native/tls` + `vis-pkg/tls`; `rustls` chỉ còn **dev-dependency** non-wasm (server TLS hermetic) |
| `webpki-roots` | `[dependencies]` optional non-wasm | **bỏ hẳn** — chỉ `vis-host-native` dùng; `Cargo.lock` giảm 1 dòng phụ thuộc của `vis-cli` |
| `h2` | `[dependencies]` optional non-wasm | `http2` → `vis-host-native/http2`; `h2` chỉ còn **dev-dependency** non-wasm (server h2 hermetic) |
| `tokio` | `[dependencies]` optional non-wasm | `http2` → `vis-host-native/http2`; `tokio` chỉ còn **dev-dependency** non-wasm (IO traits trong test) |
| `mio` | `[dependencies]` cfg `any(unix,windows)` + dev unix | chỉ còn **dev-dependency unix** (adapter epoll của test async) |
| `rustix` | `[dependencies]` cfg unix | chỉ còn **dev-dependency unix** (`Errno::INPROGRESS` trong test) |

Ngoài 7 dep trên, 3 dep chỉ test dùng trực tiếp cũng hạ xuống `[dev-dependencies]` để
`[dependencies]` sạch hoàn toàn: `ring` (sinh khoá Ed25519 trong `sig_it`), `http`/`bytes`
(kiểu request/response của server h2 trong `http2_it`). Không thêm crate mới, không đổi
phiên bản ghim.

### 1.2. Vì sao vẫn cần dev-dependency?

Các suite tích hợp còn nằm trong `vis-cli` (ST-310e mới chuyển về crate chủ) và dựng
**server TLS/h2 local**, **fixture WAL** và **selector epoll** trực tiếp. Đưa các crate đó
vào `[dev-dependencies]` giữ nguyên 100% hành vi test, nhưng chúng **không vào artifact
`vis`** phát hành. Khi ST-310e chuyển suite, các dev-dependency này sẽ đi theo test.

## 2. Bằng chứng feature matrix (12/12)

Chạy `cargo check -p vis-cli` từng tổ hợp (không `--all-targets`, đúng yêu cầu):

```text
[01] <default>                                                  PASS
[02] --no-default-features                                      PASS
[03] --no-default-features --features tls                       PASS
[04] --no-default-features --features sqlite                    PASS
[05] --no-default-features --features http2                     PASS
[06] --no-default-features --features async-h2                  PASS
[07] --no-default-features --features pkg-sig                   PASS
[08] --no-default-features --features tls,sqlite                PASS
[09] --no-default-features --features tls,pkg-sig               PASS
[10] --no-default-features --features sqlite,pkg-sig            PASS
[11] --no-default-features --features async-h2,pkg-sig          PASS
[12] --no-default-features --features tls,sqlite,pkg-sig        PASS
```

Kiểm thêm đường test: `cargo check -p vis-cli --all-targets` **PASS** (default) và
`cargo check -p vis-cli --all-targets --no-default-features` **PASS**.

## 3. Cổng máy

```text
./vis-dev gate --fast
--- fmt: OK
--- clippy: OK
--- luật R1..R7: OK
--- toolchain: OK
--- release: OK
--- self-test case âm: OK
--- báo cáo: OK
--- build: OK
--- doc-examples: OK (31 khối: 26 xanh + 5 ngoại lệ đỏ đúng)
--- L0 test: OK
Summary [101.109s] 3754 tests run: 3754 passed (1 slow), 4 skipped
vis-dev gate --fast: PASS
```

- **0 `unsafe`**: `vis-cli` giữ `#![forbid(unsafe_code)]`; R2 của cổng PASS.
- Không đổi source/test, không đổi semantics; chỉ đổi chủ sở hữu dependency và feature-forwarding.

## 4. Ghi chú phạm vi

- ST-310a/ST-310c chưa hợp nhất vào `main` tại `e9c62cb`; thay đổi này **độc lập**, chỉ dựa
  trên `vis-host-native`/`vis-pkg` đã có từ ST-300b.
- Không merge/push/tag. Không ssh `mps`/`psvn`; không chạy gate Windows/macOS.
