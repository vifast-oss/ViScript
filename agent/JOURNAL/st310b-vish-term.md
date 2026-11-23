<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-310b — Đấu nối ViShell: `vish-term` ← `vish-edit` ← `vis-cli`

**Nhánh**: `plan/st310b-terminal`, tạo từ **`4746799`** (`ST-310a`, chưa merge). **Không**
merge/push/tag; không ssh `mps` (đang soak `ST-300m3`). Worktree riêng
`/home/ps/tmp/st310b-wt` (không checkout `/home/ps/ViScript`).
Môi trường: Rust **1.100.0**, `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/home/ps/tmp`.

## 1. Phạm vi và kết quả

| Hạng mục (`VD-285` ST-310b) | Kết quả |
|---|---|
| `vish-term` là tầng Terminal/Render **duy nhất** | Có: `vish-term/src/raw.rs` giữ raw mode; `vish-edit`/`vis-interp`/`vis-cli` không còn tự phát SGR/CSI hay tự hạ bảng |
| Bỏ trùng lặp render ở `vis-interp` | `vis-interp/src/render.rs` chỉ còn **chuyển `Value` → `Table`/`Record`**; bố cục cột/độ rộng/NFC do `vish-term`; bỏ dep `unicode-normalization` trực tiếp |
| Bỏ trùng lặp render ở `vish-edit` | `render.rs` dùng `AnsiPosix` + `Style`; `editor.rs` dùng `render::dim`; `run.rs` dùng `Terminal::clear_line`/`write_line`; `term.rs` chỉ re-export `vish_term::RawMode` |
| Nối `vish-edit` → `vish-term` | Có (`Cargo.toml` + API); `rustix`/`crossterm` chuyển từ `vish-edit` sang `vish-term` (dep đã duyệt, không package mới) |
| `vis-cli` → `vish-edit` + `vish-term` cho `vis repl` | Có: `crates/vis-cli/src/repl.rs` dùng chung lib/bin; TTY thật dùng editor, non-TTY đọc ống; `--format text|viterm` |
| PTY giả + `vs3_viterm` 3 OS | Linux **PASS**; Windows **PASS** (mục 4); macOS **BLOCKED** (không ssh `mps` — m3 đang soak) |
| `./vis-dev gate --fast` ≥ 3754/4 | **PASS — 3766 passed / 4 skipped** (nextest, 69,6 s) |
| 0 `unsafe` / fmt / clippy / R1..R7 | **PASS** (gate `--- clippy: OK`, `--- luật R1..R7: OK`) |

## 2. Thay đổi chính

### 2.1 `vish-term` — một tầng Terminal/Render

- Thêm `src/raw.rs` (`RawMode`): chuyển **nguyên văn** từ `vish-edit/src/term.rs`
  (`rustix::termios` Unix/macOS, `crossterm` Windows, guard `Drop`, no-op khi stdin không TTY).
  `Cargo.toml` thêm hai dep theo target — **không** thêm package mới vào `Cargo.lock`.
- `vish_term::RawMode` re-export; `vish-edit::term::RawMode` giữ đường cũ bằng re-export.
- Không đổi chữ ký `Terminal`/`AnsiPosix`/`ViTermFramed`; snapshot `vs3_viterm` giữ nguyên.

### 2.2 `vish-edit` — render qua `vish-term`

- `render.rs`: bỏ 7 hằng SGR viết tay; `Kind` → `vish_term::Style` (`Kw` xanh dương, chuỗi vàng,
  số hồng, chú thích xanh lá, `Ident` hoa cyan) rồi ghi qua `AnsiPosix<Vec<u8>>` (ANSI16).
  Thêm `dim()` cho chỉ báo tìm `Ctrl-R`; `editor.rs` dùng nó thay `\x1b[2m…`.
- `run.rs`: thêm `read_line_from(reader, writer, prompt, &mut History)` (giữ lịch sử nhiều dòng);
  `read_line` chỉ còn là vỏ stdin/stderr. Prompt/echo vẫn ra **stderr**; con trỏ/xoá dòng qua
  `Terminal::clear_line`/`write_line` thay `\r\x1b[2K`/`eprintln!`.
- `term.rs`: chỉ còn `pub use vish_term::RawMode;`.

### 2.3 `vis-interp` — Render ST-41 dùng `vish-term`

- `bang`/`khoi` chuyển `Value` thành `vish_term::Table`/`Record` rồi hạ văn bản qua
  `AnsiPosix::with_color(ColorMode::None)`. Quy tắc giữ nguyên: cột theo bản ghi đầu, 2 dấu cách,
  cột số căn phải, độ rộng NFC, không khoảng trắng thừa cuối dòng.
- `rong` còn là API mỏng gọi `vish_term::display_width`; bỏ dep `unicode-normalization` trực tiếp.
- 4 unit test mới khoá bảng/bản ghi/`(rỗng)`/NFC.

### 2.4 `vis-cli` — `vis repl` TTY + `--format viterm`

- Chuyển thân repl từ `cli/repl.rs` sang `src/repl.rs`, khai báo ở **cả** `lib.rs` và `main.rs`:
  binary dùng `repl::run`, test dùng `vis_cli::repl::run_with` (PTY giả bằng `Cursor`).
- **TTY thật** (`stdin.is_terminal()`): `vish_edit::read_line_from` + prompt `vis> ` ở stderr;
  `Ctrl-C` ⇒ thoát **130** (khung `bye` vẫn đóng nếu ViTerm).
- **Non-TTY**: đọc dòng từ ống, **không** prompt/lời chào/ANSI trên stdout — `print` chỉ ra kết quả
  chương trình; stderr giữ khuôn chẩn đoán cũ (`mã: dòng: thông điệp`).
- **`--format viterm`** (cả `--format viterm` và `--format=viterm`): `hello` đầu tiên, kết quả là
  khung `text`, chẩn đoán là khung `diag`, `bye` cuối; stdout **chỉ** khung `{"v":1,…}`.
- `--format` lạ ⇒ exit 2, stderr nêu `text|viterm`; usage cấp cao cập nhật.
- 8 test mới trong `crates/vis-cli/tests/cli_it/repl_term.rs`: editor PTY giả (text/viterm),
  giữ trạng thái + rollback, Ctrl-C 130, pipe sạch, viterm qua ống, dạng `=`, format lạ.

## 3. Flake cổng bắt được (sửa nhỏ, không đổi hành vi sản phẩm)

Hai ca có sẵn đỏ do chạy **song song** trong cùng tiến trình test, không liên quan mã ST-310b:

1. `async_it/test_timeout.rs::treo_bi_giet_trong_3s`: `con_cua_minh()` đếm **mọi** tiến trình con
   của test binary, gồm `vis run` của ca khác. Sửa: lọc theo thư mục tạm **riêng** của ca
   (`con_cua_minh(&d)`), vẫn kiểm đúng “không rò con của lượt chạy này”.
2. `stdlib_it/vd270_str_trim.rs`: trùng tiền tố `vis-vd270-st-…` + tag + pid với
   `vd270_str_title.rs` ⇒ ca này xoá thư mục của ca kia. Sửa: tiền tố riêng `vis-vd270-strtrim-…`.

## 4. Kiểm chứng

### 4.1 Linux (local)

```text
$ ./vis-dev gate --fast
  Summary [82.795s] 3766 tests run: 3766 passed (1 slow), 4 skipped
  --- fmt: OK / clippy: OK / luật R1..R7: OK / toolchain: OK / release: OK
  --- self-test case âm: OK / doc-examples: OK / readme-examples: OK
  --- L0 test: OK
  vis-dev gate --fast: PASS
```

Log: `/home/ps/tmp/st310b-gate-final.log`
sha256 `e86adbd0a6a4955a399b8db11173d3231f343a4261257427e502787481468d0e`.

```text
$ cargo test -p vish-term -p vish-edit
  vish-term 19/19; vish-edit 8+9/… (không đổi snapshot)
$ cargo test -p vis-cli --test cli_it repl
  10/10 pass (8 ca mới + 2 ca ST-303 cũ)
$ cargo test -p vis-cli --test cli_it render::
  16/16 pass — Render ST-41 byte-identical sau khi chuyển sang vish-term
```

### 4.2 Cross-target (không chạy, chỉ biên dịch)

```text
$ cargo check -p vish-term -p vish-edit -p vis-interp --target x86_64-pc-windows-msvc --all-targets   PASS
$ cargo check -p vish-term -p vish-edit -p vis-interp --target aarch64-apple-darwin --all-targets      PASS
$ cargo check -p vish-term --target wasm32-unknown-unknown --all-targets                              PASS
$ cargo check -p vis-interp --target wasm32-unknown-unknown --all-targets                             PASS
$ cargo test  --manifest-path vis-host-wasm/Cargo.toml                                                PASS (17 + 12)
$ cargo check --manifest-path vis-host-wasm/Cargo.toml --target wasm32-wasip1 --all-targets            PASS
```

### 4.3 Windows (`psvn`, quick có mục tiêu)

```text
$ bin/vis-remote psvn --require-clean -- bash -lc '… git switch --detach st310b-test;
    cargo test -p vish-term && cargo test -p vish-edit &&
    cargo test -p vis-cli --test cli_it repl_term::'
  vish-term: unittests 0; tests/vs3_viterm.rs 21 passed / 0 failed   (gồm 2 ca WinConsole)
  vish-edit: tests/completion_job.rs 8 passed; tests/editor.rs 9 passed; 0 failed
  vis-cli:   tests/cli_it.rs repl_term:: 8 passed / 0 failed (0,94 s)
  vis-remote: [exit=0] host=psvn
```

Log: `/home/ps/vislogs/st310b-win.log`
sha256 `aa3cd16a3e3c0cad8001a5d2e2a185c5e493e01ad59859388e7a6a1665a8dc42`.
Bundle kiểm: `/home/ps/tmp/st310b.bundle` sha256
`21e16eb8c96df7f60bb80cf7df9bc85cbc1e4505cff228c35dca222281081090`; sau đo đã trả `psvn`
về `8cc3588a` (dirty 0) và xoá nhánh tạm `st310b-test`.

### 4.4 macOS (`mps`)

**BLOCKED** — phiên này **không ssh `mps`** theo chỉ đạo (m3 đang soak). Cần chạy quick khi `mps`
rảnh: `cargo test -p vish-term -p vish-edit` + `cargo test -p vis-cli --test cli_it repl_term::`.
Mã đã biên dịch `aarch64-apple-darwin` (mục 4.2).

## 5. Quyết định / lưu ý cho Agent 0

1. **Chiều phụ thuộc mới**: `vis-interp → vish-term` (và `vish-edit → vish-term`, `vis-cli → cả hai`).
   `vish-term` vẫn là lá DAG (chỉ `std` + `unicode-normalization` + `rustix`/`crossterm` theo target),
   không có vòng; `vis-interp` không còn dep `unicode-normalization` trực tiếp.
2. **Raw mode ở `vish-term`**: đây là phần điều khiển terminal, không phải render, nhưng `QUEUE`
   ST-310b đã chỉ định hợp nhất `vish-edit/src/term.rs`; chọn chuyển sang `vish-term/src/raw.rs` để
   “một tầng Terminal” đúng nghĩa. Hai dep OS đã có trong lock/allowlist, **0 package mới**.
3. **Khuôn chẩn đoán text giữ nguyên** (`mã: dòng: thông điệp`, loader có thêm `tệp:dòng:cột`) để
   không đổi đầu ra script; chỉ khung `viterm` dùng `vish_term::Diagnostic`.
4. `vis repl` non-TTY **không còn** lời chào stderr (trước đây luôn in); stdout giữ nguyên tuyệt đối.
   Đây là thay đổi có chủ đích để “pipe sạch”.
5. Không merge/push/tag; không đụng `mps`/`psvn` ngoài quick có mục tiêu.
