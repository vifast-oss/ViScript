<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-305 — LSP hover generic/completion `std.*` + extension VS Code (PARTIAL)

> **Trạng thái: TẠM DỪNG theo chỉ đạo giờ cao điểm.** Nhánh `plan/w4-st305-lsp-vscode` từ
> `700bb66` (VD-284). **Chưa chạy gate/test** (không build/test mới khi tạm dừng); không ssh mps;
> không merge/push/tag. Tiếp tục **13:00–15:00** sau khi W2 xong `m1/m2`.

## 1. Đã làm (chưa nghiệm thu)

- `crates/vis-lsp/src/phan_tich.rs`: `chu_ky_fn` in **tham số kiểu generic** `[T, U]` cho hàm và
  phương thức (hover/`detail`), không đổi API.
- `crates/vis-lsp/src/goi_y.rs`: completion sau **`std.`** gợi ý **module** (`fs`, `str`, `list`,
  …) từ `FUNCS` (kind 9), vẫn giữ đường `fs.`/`sqlite.` cũ.
- Test mới (chưa chạy):
  - `lsp_it/lsp_hover_method.rs`: `hover_ham_generic_neu_tham_so_kieu`,
    `hover_phuong_thuc_generic_neu_tham_so_kieu`.
  - `lsp_it/lsp_extra.rs`: `lsp_completion_sau_std_lieu_module`.
- `editors/vscode/`: `package.json` (language `.vis/.vish`, grammar, `viscript.serverPath`),
  `syntaxes/viscript.tmLanguage.json`, `language-configuration.json`, `extension.js`
  (`LanguageClient` → `vis lsp --stdio`), `README.md`; sidecar `.license` cho JSON; **không**
  build/publish.
- `agent/reports/R36-viscript.md`: **khung** cho `v0.5.0`; chưa có JSON; QA 3-OS/WASI ghi rõ sẽ
  điền sau; không tuyên bố phát hành.

## 2. Chưa làm / rủi ro

- **Chưa chạy** `cargo test` LSP, `./vis-dev gate --fast`, fmt/clippy/R1..R7 — kỳ vọng gate
  **3671/4** nhưng **chưa xác nhận**.
- Test generic method có thể cần chỉnh theo `TypeTable::show` thực tế (hover `[T]`).
- R36 mới là khung; **không** tạo `R36-viscript.json` (W3b/A0 chốt).

## 3. Việc tiếp theo (13:00–15:00)

1. Chạy `cargo test -p vis-cli --test lsp_it` + targeted LSP; sửa test nếu lệch.
2. `./vis-dev gate --fast` (kỳ vọng 3671/4), fmt/clippy/R1..R7.
3. Hoàn thiện R36 khung nếu cần; **không** merge/push/tag.
4. Sau khi A0 báo `m1` merge + `m2` xong mới chạy `ST-300m3` trên mps (3× full + soak 4h +
   `vis-wasm-sem` ≥388/390) — **chưa làm bây giờ**.

## 4. Blocker

- Không blocker kỹ thuật; tạm dừng do giờ cao điểm. Không tag/push/merge; không GitHub Actions.
