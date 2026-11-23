<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-286 — R0c Refactor `async_net.rs`, `quyen.rs`, `db/sqlite/mod.rs` (0 đổi hành vi)

> **Người làm**: subagent VS3-W2, 2026-11-22.
> **Base**: `1c58ba3` (`[ST-00] quyet-dinh-a0: VD-272..VD-279 + QUEUE ST-284..ST-309`).
> **Phạm vi**: chỉ di chuyển mã nguyên văn sang module con; **0 đổi hành vi/ABI/cache**, không
> `unsafe`, không crate mới, không đổi feature/CLI.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `async_net.rs` | **1490 → 606** dòng (tách `async_net_conn.rs` 913) |
| `quyen.rs` | **1465 → 1190** dòng (tách `quyen_guard.rs` 204, `quyen_path.rs` 104) |
| `db/sqlite/mod.rs` | **1498 → 1245** dòng (tách `schema.rs` 274) |
| Mục tiêu ≤ 1.250 | **đạt** cả 3 tệp (1245 là sát trần, có kiểm R1 của cổng) |
| Test đích | `vs3_http2` **19/19** (http2), **34 + 1 ignored** (async-h2); `vs3_tls` **21/21**; `vs3_default` **22/22**; `quyen` **8/8**; `sqlite` **17/17**; unit `vis` **195/195** |
| `./vis-dev gate --fast` | **PASS** — 3571 passed / 0 failed / 3 ignored |
| Differential 100k | **lech=0** (seed `20261122`, 172,8 s, 93 770 ca chạy) |
| Blocker | **không** |

## 1. Cách tách

Đều dùng module con khai bằng `#[path = "..."] mod …;` (cùng khuôn `async_net_h2.rs` /
`async_net_host.rs` sẵn có), nên module mới vẫn truy cập được item private của module cha:

| Tệp mới | Nội dung chuyển nguyên văn | Điểm vào cũ giữ nguyên |
|---|---|---|
| `crates/vis-cli/src/async_net_conn.rs` | 2 `impl AsyncNet`: cấp handle `C<n>`/`L<n>`, `try_connect` (`N:`), `try_write` (`W:`), TLS host (`T:`, handshake rustls nonblocking, `doc_tls`/`ghi_tls`), `try_*_tls` | method `pub(super)`; `AsyncNet::new`/`moi_voi_goc_tls` vẫn `pub` |
| `crates/vis-cli/src/quyen_guard.rs` | `FsGuard` + `impl FsHost` (mở-rồi-kiểm ST-124/137/174) + `mo_ta_lan_doc` (cfg test) | `pub use guard_impl::FsGuard` |
| `crates/vis-cli/src/quyen_path.rs` | `la_tep_lenh`, `la_tep_thuc_thi`, `ung_vien_path` (Unix/Windows), `phan_giai_lenh`, `chuan_lenh` | `pub use path_impl::{chuan_lenh, phan_giai_lenh}` |
| `crates/vis-cli/src/db/sqlite/schema.rs` | `HOST_PRAGMAS`, `la_bang_trang_tho`, `chinh_sach` (authorizer), `SCHEMA_SCANS`/`schema_scans_for_test`, `kiem_schema_utf8`, `kiem_schema_neu_doi` | `use schema::{…}` trong `mod.rs`; test con `use super::*` không đổi |

- Các hằng/`pub` cũ vẫn nằm đúng đường dùng: `quyen::FsGuard`, `quyen::chuan_lenh`,
  `quyen::phan_giai_lenh`, `sqlite::{HOST_PRAGMAS, chinh_sach, schema_scans_for_test}`
  (tất cả đi qua re-export/import, test hiện hữu không sửa một dòng).
- Không đổi `PHIEN_BAN`, version, khoá cache, feature hay thứ tự khởi tạo field.

## 2. Kiểm chứng “chỉ di chuyển mã”

So **multiset dòng mã** của tệp gốc (`git show HEAD:…`) với tổng (tệp mới + tệp còn lại), sau khi
bỏ dòng `use`/`mod`/`#[path]`/doc-header và chuẩn hoá `pub(super)`:

| Cặp tệp | Khác biệt còn lại |
|---|---|
| `async_net.rs` ↔ `async_net_conn.rs` + phần còn lại | chỉ 3 chữ ký bị `rustfmt` xuống dòng vì thêm `pub(super)` (cùng nội dung) |
| `quyen.rs` ↔ 2 tệp mới + phần còn lại | 0 dòng mã khác; chỉ thêm `#[cfg(all(test, unix))]` cho import |
| `db/sqlite/mod.rs` ↔ `schema.rs` + phần còn lại | 0 dòng mã khác; chỉ thêm `#[cfg(test)]` cho import |

- `grep unsafe` trong 4 tệp mới: **0**.
- `#![forbid(unsafe_code)]` của lib vẫn giữ; binary `vis` không thêm `unsafe`.

## 3. Test đích

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-cli --test vs3_http2 --features http2` | **19 passed / 0 failed** |
| `cargo test -p vis-cli --test vs3_http2 --features async-h2` | **34 passed / 0 failed / 1 ignored** |
| `cargo test -p vis-cli --test vs3_tls` | **21 passed / 0 failed** |
| `cargo test -p vis-cli --test vs3_default` | **22 passed / 0 failed** |
| `cargo test -p vis-cli --test quyen` | **8 passed / 0 failed** |
| `cargo test -p vis-cli --test sqlite` | **17 passed / 0 failed** |
| `cargo test -p vis-cli --bin vis` | **195 passed / 0 failed** (gồm `quyen::tests`, `quyen_st174`, `db::sqlite::tests*`) |

## 4. Cổng

- `./vis-dev gate --fast` (toolchain 1.100.0, `CARGO_TARGET_DIR=target/w2`, `CARGO_INCREMENTAL=0`):
  **PASS**, `exit 0`; L0 **3571 passed / 0 failed / 3 skipped** (68,9 s); fmt/clippy/R1..R6/release/
  self-test/doc-examples/readme-examples đều `OK`.
- Log: `agent/JOURNAL/st286-gate.log` (sha256 `22d2f3799b597ffa18034f9fdf8cfbd47ed45a96de96dca5e6950d3e901a3110`).
- Lần chạy đầu của cổng bị **hết đĩa** (`No space left on device`, `ld` bus error) do target
  workspace đầy; đã xoá **cache tái sinh được** (`~/.cache/opencode/vd270-char-at-target/debug`,
  `target/w2/debug/incremental`) rồi chạy lại xanh. Không xoá mã/nguồn.

## 5. Differential 100k

- `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261122 ./bin/vis-diff`:
  `DIFF tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=172.8 toc_do=579/s seed=20261122`.
- Log: `agent/JOURNAL/st286-diff-100k.log` (sha256 `50ce126922a2a3f350e0a7b334c18aaca31684084f7aae8a006671574875c7b6`).

## 6. Ghi chú

- `cargo clippy -p vis-cli --no-default-features --lib -- -D warnings` còn **2 cảnh báo có sẵn**
  (không do ST-286): `dang_reactor_win` không dùng khi tắt `tls`, và `let mut n` trong
  `dem_tay_dang_mo`; cả hai nằm trong `async_net_nguon.rs`, giữ nguyên theo phạm vi “0 đổi hành vi”.
- Không chạm `agent/QUEUE.md`, `agent/DECISIONS.md`, `SPEC`.
