<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# tree-sitter-viscript (khung)

Grammar Tree-sitter tối thiểu để GitHub Linguist tô màu ViScript. **Chưa sinh parser** —
`src/parser.c`, `src/node-types.json` và `bindings/` sẽ do `tree-sitter generate` tạo ra.

```sh
npm install
npx tree-sitter generate
npx tree-sitter test
npx tree-sitter highlight ../../../README.md   # thử sau khi có parser
```

Lưu ý khi hiện thực hóa:

- `identifier` dùng regex Unicode (`\p{L}`) để nhận tên biến tiếng Việt có dấu.
- Thụt lề là ngữ nghĩa của ViScript: cần **external scanner** sinh `INDENT`/`DEDENT`
  nếu muốn parse cây đúng. Với mục đích highlight thì grammar hiện tại đủ dùng.
- Chuỗi nội suy `"{ten}"` đang được token hóa gộp; có thể tách node riêng để tô màu
  biến trong ngoặc nhọn khác màu chuỗi.
