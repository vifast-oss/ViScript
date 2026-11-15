# ST-112 — kế hoạch sửa `Span` thiếu file-id (nợ correctness)
**Quyết định**: **KHÔNG sửa** trong ST-112 — ước lượng **>150 dòng** (không chỉ `modules.rs`), rủi ro
đổi vị trí lỗi runtime/codegen. Đã tái hiện + chỉ đúng chỗ: `JOURNAL/st112-chan-doan.log`.

## Thiết kế chọn
Thêm **file-id** vào `Span` (`file: u16`, mặc định 0) thay vì cộng base toàn cục: base làm mọi chỗ
`&text[span]` phải trừ base, và `interp::pos`/bytecode spans sẽ lệch.

## Chỗ phải sửa (đếm bằng grep, HEAD hiện tại)
- `Span` + `Span::new`: `crates/vis-syntax/src/lex/token.rs`; **90** lời gọi `Span::new` trong 5 crate
  (`vis-syntax`, `vis-types`, `vis-interp`, `vis-bytecode`, `vis-cli`).
- Cắt văn bản theo span: **13** chỗ (`grep 'span.start..span.end|&text\\['`), gồm `lower.rs:487`,
  `lower/khai.rs` 7 chỗ, `lower/sugar.rs` 2, `vistest.rs`, LSP `goi_y`/`thanh_vien`.
- Map chẩn đoán: `vis-cli/src/modules.rs:795-820` (duyệt `hir.items`, không có stmt/file-id).
- Tính `dòng:cột`: `modules.rs:817` (`vi_tri(&f.text, span)`), `interp::pos` (Trap runtime).
- Codegen/bytecode: `vis-bytecode/src/codegen.rs:71` chép span vào `format::Span`.
- LSP: `lsp/mod.rs`/`chan_doan.rs` chuyển span ↔ line/col; phải truyền `rel`.

## Thứ tự làm
1. `Span { start, end, file }` + `Span::new` giữ file 0; thêm `Span::in_file`.
2. Lexer/parser nhận `file_id`; **90** `Span::new` đổi dần theo module (dùng `file` từ `LexOutput`).
3. Map chẩn đoán theo `(file, span)`; `vi_tri` chọn text theo file; sửa 13 chỗ cắt text.
4. `interp::pos`/codegen/LSP nhận/ghi file-id; bytecode `Span` thêm file (bump `VBC1`? — **không** nếu chỉ
   debug; nếu đổi format phải bump).
5. Ca: top-level a, fn a, module b, cả hai, TAP `a.vis`/`b.vis`; chạy sem/parity/host/diff.

## Rủi ro
- Đổi `Span` lan 5 crate, dễ vỡ **90** constructor; `VBC1` nếu thêm field; runtime `pos` cần text theo file.
- Không được đổi vị trí chẩn đoán hiện có (TAP/`--json` golden) ngoài ca sửa.
- Ngân sách ST-112 3 h **không đủ**; nên task riêng 4–6 h có review.
