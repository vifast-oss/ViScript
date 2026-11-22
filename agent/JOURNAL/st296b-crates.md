<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296b — tách `vis-loader` + `vis-lsp` + `[workspace.dependencies]`

> **Người làm**: W3, 2026-11-23 · **nhánh** `plan/w3-st296b-crates` @ **`f240c95`**
> (`main` đã merge ST-296d-tools; **không** merge `main` trước tag `v0.4.0`, đúng VD-282).
> **Phạm vi**: chỉ tách crate/khai báo phụ thuộc + sửa đường dẫn test đọc mã nguồn; **0 đổi hành vi**
> CLI/LSP. Không sửa `vis-interp`/`vis-vm`/`vis-types`. Không push/merge/tag; không GitHub Actions.

## 1. `crates/vis-loader` (crate thuần, wasm32)

Chuyển nguyên khỏi `vis-cli`:

| Tệp | Nội dung |
|---|---|
| `modules.rs` | nạp đa tệp `use`/`vis.lock`, HIR hợp nhất, overlay LSP, `dump_deps` (986 dòng) |
| `cache.rs` | khoá cache v2 + đọc/ghi cache (459 dòng) |
| `json_esc.rs` | escaper JSON + fuzz 100k (372 dòng) |
| `audit_span.rs` | audit `span.file` trên toàn fixture (563 dòng; `#[cfg(test)]` như cũ) |
| `lock_meta.rs` | `{bam, chuan_hoa, doc_khoa_cua, GoiKhoa}` + `TepKhoa`/`Khoa`/`doc_lock`; 0 vòng lặp |
| `semver.rs` | `Version`/`VersionReq` (631 dòng) — helper lock cần; `vis-cli::pkg` **re-export** |

- `#![forbid(unsafe_code)]`; không OS/TLS/SQLite. `cargo check -p vis-loader --target
  wasm32-unknown-unknown` **PASS** (và `vis-lsp` cũng PASS).
- `vis-cli` re-export `pub(crate) use vis_loader::{cache, json_esc, modules};` nên mọi
  `crate::modules::…`/`cache::…`/`json_esc::…` giữ nguyên; `pkg` re-export
  `lock_meta::{bam, chuan_hoa, doc_lock, …}` + `semver` nên `gc`/`registry_http`/`resolver`
  không phải đổi ngữ nghĩa.

## 2. `crates/vis-lsp`

- Chuyển **25 tệp** `lsp/**` (6.600 dòng) sang `vis-lsp/src/**`; `mod.rs` → `lib.rs`; API còn
  `pub fn run(&[String]) -> ExitCode`. `vis-cli` gọi `vis_lsp::run` (`vis lsp --stdio` không đổi).
- Chỉ đổi đường dẫn nội bộ: `crate::modules::…` → `vis_loader::modules::…`,
  `crate::lsp::…` → `crate::…`. `#![forbid(unsafe_code)]`.
- **15 tệp `tests/lsp*.rs` vẫn ở `crates/vis-cli/tests/`**: chúng chạy **binary `vis`** qua
  `env!("CARGO_BIN_EXE_vis")` (giao kèo Cargo chỉ cấp cho bin cùng package) và một ca còn gọi
  `vis check`; chuyển sang `vis-lsp/tests` sẽ mất binary/đổi hành vi test. Unit test trong
  `lsp/**` đã chuyển nguyên sang `vis-lsp` (27 test). Toàn bộ 15 tệp tích hợp vẫn chạy trong
  `./vis-dev gate --fast` của workspace.

## 3. `[workspace.dependencies]`

Gom **13 phụ thuộc ngoài** về `Cargo.toml` gốc: `sha2`, `unicode-normalization`, `unicode-ident`,
`rusqlite`, `rustix`, `rustls`, `webpki-roots`, `h2`, `tokio`, `http`, `bytes`, `ring`, `mio`
(cùng `default-features`/`features` cũ; thành viên dùng `{ workspace = true }` + `optional`).
`Cargo.lock` chỉ thêm entry `vis-loader`/`vis-lsp`; **không đổi phiên bản crate ngoài**.
`version.workspace = true` giữ nguyên ở mọi crate.

## 4. Kiểm chứng

| Lệnh | Kết quả |
|---|---|
| `cargo check --workspace --all-targets` | **PASS** |
| `cargo test -p vis-loader` | **29/29** (gồm audit 400+ fixture, fuzz JSON 100k, semver, lock) |
| `cargo test -p vis-lsp` | **27/27** (gồm LSP cache timing chịu tải) |
| `./vis-dev gate --fast` | **PASS — 3638 passed / 4 skipped** (1:52; fmt/clippy/R1..R6/report-check 33/329) |
| `./bin/vis-targets` | core **18/18**; sqlite **16 PASS + 2 EXPECTED_RED** (đúng nợ `#156`) |
| wasm32-unknown-unknown | `vis-loader` **PASS**, `vis-lsp` **PASS** |
| diff 100k seed `202611222300` | **lech=0** (100.000; 6.231 lỗi tĩnh; 93.769 chạy; 146,8 s) |

- Sửa **đường dẫn đọc mã nguồn** trong 3 test canh cache: `vis-cli/src/cache.rs` →
  `vis-loader/src/cache.rs` (`vis-bytecode/tests/lat257_b_compat.rs`,
  `vis-cli/tests/{st246_cache_compat,cache_fastpath}.rs`) + comment `span_tep.rs`.
- Không đổi test/sản phẩm nào khác; 3638 test giữ nguyên tổng (chỉ đổi crate chạy unit test).

## 5. Còn lại / blocker

- **Không blocker**. Phần còn lại (ngoài time-box): thêm `vis-loader`/`vis-lsp` vào ma trận
  `bin/vis-targets` (hiện giữ đúng kỳ vọng **18/18** theo yêu cầu; 2 crate mới đã check wasm riêng),
  và cân nhắc chuyển 15 test tích hợp LSP sang crate riêng nếu sau này `vis-lsp` có binary test.
- **Chưa merge `main` sau `f240c95`**, không push/merge/tag. Nhánh sẵn sàng rebase sau `v0.4.0`.
- Đĩa cuối **15 G** (> 10 G).
