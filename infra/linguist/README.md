<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Đăng ký ViScript với GitHub Linguist

README dùng fence ` ```python title="ViScript" ` để GitHub tô màu tạm thời. Muốn GitHub hiểu
thẳng ` ```vis ` và tô màu tệp `.vis`, cần thêm ViScript vào
[github/linguist](https://github.com/github-linguist/linguist). Đây là khung chuẩn bị —
**chưa gửi PR**, vì grammar phải được sinh và test trước.

## Hiện trạng

- `vis` **không** có trong `lib/linguist/languages.yml` của Linguist (kiểm tra bản master
  ngày 2026-11-20: có VHDL, Verilog, Vim script, Visual Basic…, không có ViScript).
- Repo có lexer/parser riêng ở `crates/vis-syntax`, nhưng đó là Rust hand-written,
  **không phải Tree-sitter** — Linguist cần grammar Tree-sitter để tô màu code fence.

## Các bước trước khi gửi PR

1. Tách `tree-sitter-viscript/` thành repo public riêng (giữ nguyên nội dung khung này).
2. Trong repo grammar: `npm install`, `npx tree-sitter generate`, `npx tree-sitter test`.
   Bổ sung ca test cho: `let/var`, `fn`, `if/elif/else`, `for … in`, `while`,
   `type/enum/trait/impl`, `match`, `use`, chuỗi nội suy `"{x}"`, comment `#`,
   số có đơn vị `1kb`, `2h`, toán tử `??`, `?.`, `//`, closure `x => …`.
3. Commit `src/parser.c` đã sinh + `src/node-types.json` + `bindings/` (tree-sitter yêu cầu).
4. Fork `github-linguist`, áp dụng các snippet trong `linguist-patch/`:
   - `languages.yml` — thêm entry `ViScript` (alias `vis`, extension `.vis`).
   - `grammars.yml` — trỏ `source.viscript` tới repo grammar ở bước 1.
   - `samples/ViScript/hello.vis` — mẫu tối thiểu để Linguist nhận diện.
   - `test/test_blob.rb` — thêm ca theo mẫu của các language khác.
5. Chạy `bundle exec rake` trong Linguist; mở PR kèm `PR-BODY.md`.
6. Sau khi PR merge và GitHub deploy: đổi fence trong README từ
   `python title="ViScript"` về `vis`, thêm `*.vis linguist-language=ViScript` vào
   `.gitattributes`, rồi bỏ bước "tô màu tạm" trong README.

## Liên quan trong repo này

- `test/readme-examples.py` — kiểm mọi khối README vẫn `vis check` xanh (chạy trong `vis-dev check`).
- `README.md` mục "Góp ý / phát triển" — giải thích tại sao fence dùng `python`.
