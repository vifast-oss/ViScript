<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-308 — `Terminal` trait & khung hiển thị ViTerm (VD-278/VD-283)

> **Người làm**: W3, 2026-11-23 · nhánh **`plan/w3b-st308-term`** từ main **`c3b7290`**.
> **Local only**: không ssh `mps`/`psvn`, không merge/push/tag, không đụng `QUEUE`/`STATE`/`SO-NO`.
> `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`.

## 1. Crate mới `crates/vish-term`

Crate độc lập ViFastOs (giữ R4), `#![forbid(unsafe_code)]`; chỉ phụ thuộc `unicode-normalization`
(NFC cho độ rộng cột — cùng quy tắc `Render` ST-41; đã có trong workspace/lock).

| Tệp | Nội dung |
|---|---|
| `src/lib.rs` | `Terminal` trait tối thiểu + `NativeTerminal`/`native_terminal()` theo nền |
| `src/caps.rs` | `Capabilities`, `ColorMode`, `Size`; dò `NO_COLOR`/`TERM`/`COLORTERM`, `COLUMNS`/`LINES` |
| `src/style.rs` | `Style`, `Color`, `Attrs`; builder `const fn`, `Style::PLAIN/BOLD/DIM` |
| `src/ansi.rs` | Mã SGR 16/256/truecolor + hạ cấp RGB/256 → 16 màu |
| `src/posix.rs` | `AnsiPosix<W>`: SGR, `LF`, CSI xoá dòng/lên dòng/ẩn-hiện con trỏ |
| `src/windows.rs` | `WinConsole<W>` (`cfg(windows)`): mặc định văn bản thuần + `CRLF`; `with_ansi(true)` cho Windows Terminal |
| `src/render.rs` | `Table`/`Column`/`Align`, `Record`, `Diagnostic`/`Severity`/`Location`; hạ văn bản |
| `src/viterm.rs` | `ViTermFramed<W>`: khung JSON Lines `vish-term/1` |
| `src/width.rs` | `display_width` = số scalar sau NFC |

`Terminal` bắt buộc `caps` + `write`; các method còn lại có default: `write_line`, `write_styled`,
`flush`, `clear_line`, `move_up`, `show_cursor`, và ba hook cấu trúc `table`/`record`/`diagnostic`.
Trait **object-safe** (`&mut dyn Terminal`), đủ cho `render` và `vish-edit` (con trỏ/lịch sử dòng).

## 2. Khung ViTerm & snapshot

- Giao thức **JSON Lines**, mỗi khung một dòng, khoá cố định: `hello`, `text`, `styled`, `table`,
  `record`, `diag`, `clear`, `cursor`, `flush`, `bye`; hằng `viterm::PROTOCOL = "vish-term/1"`.
- `ViTermFramed` ghi đè ba hook cấu trúc ⇒ bảng/bản ghi/chẩn đoán đi **nguyên khung**, không hạ
  thành văn bản; `AnsiPosix`/`WinConsole` dùng bản hạ mặc định.
- Snapshot tại `crates/vish-term/tests/vs3_viterm.rs`: bảng ANSI16/không màu/có kẻ, bản ghi,
  chẩn đoán, NFC↔NFD, ô thiếu/ô thừa, SGR 16/256/truecolor + hạ cấp, con trỏ, toàn bộ khung ViTerm,
  escaping JSON (`"`, `\`, `\n`, `\t`, `\u0001`), `write_line`, object-safe.

## 3. Kiểm chứng

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vish-term` | **19/19 pass** |
| `./vis-dev gate --fast` | **PASS — 3691 passed / 4 skipped** (99 binary; nextest wall **70,76 s**) |
| `cargo clippy -p vish-term --all-targets -- -D warnings` | **PASS** |
| `cargo clippy -p vish-term --target x86_64-pc-windows-msvc --all-targets -- -D warnings` | **PASS** |
| `cargo clippy -p vish-term --target aarch64-apple-darwin --all-targets -- -D warnings` | **PASS** |
| `cargo check -p vish-term --target x86_64-pc-windows-msvc --all-targets` | **PASS** (gồm test `WinConsole`) |
| `cargo check -p vish-term --target aarch64-apple-darwin --all-targets` | **PASS** |
| `cargo check -p vish-term --target wasm32-unknown-unknown --all-targets` | **PASS** |
| `./vis-dev rules` | **PASS** (R1..R7; R1 max trong crate **349** dòng) |
| `cargo tree -p vish-term` | `unicode-normalization` → `tinyvec`; **không** ViFastOs |

Giấy phép dep: `unicode-normalization` = `(MIT OR Apache-2.0) AND Unicode-3.0`; `tinyvec` =
`Zlib OR Apache-2.0 OR MIT` — đều trong allowlist. Không thêm dep mới ngoài workspace.

Log gate ngoài repo: `/home/ps/tmp/st308-gate-final.log`
sha256 `50707451f813e6c25d0efb49a65c8747c1d36ebf59eba3f399c7e48b0f78186e`.

## 4. Ranh giới & còn lại

- **Không sửa** `vis-cli`/`vis-interp`/`vish-edit` (0 đổi hành vi); `vish-edit` chưa có trên base
  `c3b7290`, API đã có `clear_line`/`move_up`/`show_cursor` để nối ở ST-304.
- `WinConsole` mặc định **không ANSI** (an toàn `conhost` cũ); bật `with_ansi(true)` khi VT đã bật.
- Chưa merge/push/tag; chờ nghiệm thu/A0 cập nhật `QUEUE` (ST-308 vẫn `BLOCKED` trong sổ vì phụ
  thuộc `ST-304b`/`ST-305`, nhánh này chỉ hoàn tất phần crate/hợp đồng).
