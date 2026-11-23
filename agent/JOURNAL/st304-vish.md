<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-304 — `crates/vish-edit` ViShell MVP phần A

> **Người làm**: W3, 2026-11-23 · nhánh **`plan/w3-st304-vish`** từ main **`700bb66`**.
> **Local only**; không ssh `mps`; không merge/push/tag trước `v0.5.0`.
> `CARGO_BUILD_JOBS=3`, `CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/tmp`, target riêng.

## 1. Crate + API

`crates/vish-edit` (workspace member, `#![forbid(unsafe_code)]`):

| Thành phần | Nội dung |
|---|---|
| `editor.rs` | `Editor`/`Action`: buffer, cursor UTF-8, `Ctrl-J` xuống dòng + tự thụt lề theo dòng hiện tại (thêm 4 nếu dòng kết thúc `:`), `Tab`, history Up/Down, `Ctrl-R` tìm mờ, `Ctrl-C` → `Action::Cancel` |
| `history.rs` | `History`: nạp/append file **append-only**, `fuzzy()` subsequence có ưu tiên khớp liền |
| `render.rs` | `render_cst()`: tô màu token bằng **CST `vis-syntax`** (keyword/string/number/comment/type), giữ nguyên byte khi bỏ ANSI |
| `key.rs` | `Key`/`parse_keys()`/`read_key()`: UTF-8, arrow, Home/End/Delete, Ctrl-C/Ctrl-R, word-left/right |
| `term.rs` | raw-mode guard: **`rustix::termios` Unix/macOS**, **`crossterm` Windows** (lý do: rustix termios không có Windows; crossterm đã duyệt VD-283, 0 unsafe trong mã ta); `Drop` khôi phục termios kể cả panic |
| `run.rs` | `read_line(prompt, History) -> Outcome` cho `vis`/ViShell; prompt ra stderr, `Interrupted` cho Ctrl-C (caller exit 130) |

## 2. Test

`cargo test -p vish-edit` → **8/8 pass**:
- `parse_keys_arrows_utf8_ctrlc`
- `pty_gia_doc_byte_qua_read_key` — PTY giả Linux/macOS (Cursor byte → `read_key`)
- `nhieu_dong_va_thut_le`
- `ctrl_c_huy`
- `history_append_only_va_fuzzy`
- `fuzzy_chon_bang_ctrl_r`
- `render_cst_co_mau_va_giu_byte`
- `panic_giu_editor_va_history`
- `windows_parse_ctrl_c_va_arrow` (`cfg(windows)`, chưa chạy local)

## 3. Cổng + warm

| Hạng mục | Kết quả |
|---|---|
| `./vis-dev gate --fast` | **PASS — 3679 passed / 4 skipped** (99 binary; wall **70,96 s** ở lần cuối) |
| fmt / clippy / R1..R7 | **PASS** |
| 0 `unsafe` | Có (`#![forbid(unsafe_code)]`; không có `unsafe`) |
| Warm release (20 lần) | **median 50 µs**, min 44 µs, max 146 µs (**< 50 ms**) |

Log gate: `/home/ps/vislogs/st304-gate.log`
sha256 `31475dbfd87302bd60776f143d90032892ddad86b4745a40d3fbffd4f9553d7c`.

## 4. License

- `cargo metadata --format-version 1` → `/home/ps/vislogs/st304-metadata.json`
  sha256 `093796174bac8bcf2822fbf0c620b9b130c54593f56edf917f1dd65e76862cfe`.
- Log: `agent/JOURNAL/st304-license.log` — **12 package mới** (vish-edit + crossterm chain) đều
  MIT/Apache-2.0; **0 package mới ngoài allowlist**. Các license cũ (`ring`/`rustls`/`subtle`/
  `webpki-roots`…) đã được duyệt ở VD trước, không do ST-304 thêm.
- Quyết định: dùng **rustix termios** cho Unix/macOS; **crossterm =0.29.0** chỉ Windows vì
  rustix termios không hỗ trợ Windows; crossterm đã được duyệt có điều kiện VD-283.

## 5. Còn lại / blocker

- **macOS/Windows quick để sau khi W2 merge `ST-300m1`** (không ssh `mps` khi W2 đang đo).
- `ST-304b` (tab completion/job control) **chưa mở** — ngoài thời gian lát này, không blocker.
- Không merge/push/tag. Đĩa còn **11 G** (> 10 G) trước khi dọn target.
