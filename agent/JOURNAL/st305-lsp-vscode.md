<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-305 — LSP hover generic/completion `std.*` + extension VS Code

> **Trạng thái: DONE (local, chưa merge).** Nhánh `plan/w4-st305-lsp-vscode` từ `700bb66`
> (VD-284); commit `[ST-305] ...`; không merge/push/tag; **không ssh mps** (W2 đang độc quyền
> `m1/m2`). `m3` (3× full + soak 4h + `vis-wasm-sem`) sẽ do A0 báo sau.

## 1. LSP — hover generic & completion `std.*`

- `crates/vis-lsp/src/phan_tich.rs`: `chu_ky_fn` in **tham số kiểu generic** `[T, U]` cho hàm và
  phương thức (hover/`detail`), giữ API.
- `crates/vis-lsp/src/goi_y.rs`: completion sau **`std.`** gợi ý **module** (`fs`, `str`, `list`,
  …) từ `FUNCS` (kind 9); đường `fs.`/`sqlite.` cũ giữ nguyên.
- Test mới:
  - `lsp_it/lsp_hover_method.rs`: `hover_ham_generic_neu_tham_so_kieu`,
    `hover_phuong_thuc_generic_neu_tham_so_kieu`.
  - `lsp_it/lsp_extra.rs`: `lsp_completion_sau_std_lieu_module`.
- Kết quả: `cargo test -p vis-cli --test lsp_it` **155/155 pass** (0,17 s);
  `cargo test -p vis-lsp` **29/29 pass** (0,01 s).

## 2. Extension VS Code (`editors/vscode/`)

- `package.json`: language `viscript` (`.vis`, `.vish`), grammar, `viscript.serverPath` (mặc định
  `vis`); dependency `vscode-languageclient` (không build/publish trong repo).
- `syntaxes/viscript.tmLanguage.json`: TextMate cơ bản (comment `#`, string, số, keyword, type,
  tên hàm).
- `language-configuration.json`: comment/bracket/indent.
- `extension.js`: `LanguageClient` chạy `vis lsp --stdio`.
- `README.md`; sidecar `.license` cho các `.json` (R3).

## 3. Cổng Linux

- `./vis-dev gate --fast`: **PASS — 3674 passed / 0 failed / 4 skipped** (+3 ca ST-305 so baseline
  3671/4); fmt/clippy/R1..R7 OK; `report-check` PASS (34 báo cáo, 333 hàng QUEUE);
  doc-examples 24 khối.
- Disk local `/`: còn **>10 G**.

## 4. R36

- `agent/reports/R36-viscript.md`: **khung** cho `v0.5.0`; **chưa có JSON** (W3b/A0 chốt); QA
  3-OS/WASI ghi rõ sẽ điền sau; không tuyên bố phát hành.

## 5. Việc còn lại

- Merge nhánh (A0) và chạy `ST-300m3` trên mps (3× `bin/vis-gate-macos --full` + soak 4h +
  `vis-wasm-sem` ≥388/390) — **chưa làm**.
- Không tag/push/merge; không GitHub Actions. Không blocker.
