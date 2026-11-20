<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Đề xuất PR: thêm ngôn ngữ ViScript

**Repo dự án**: https://github.com/vifast-oss/ViScript
**Grammar**: https://github.com/<org>/tree-sitter-viscript (sẽ tạo)
**Extension**: `.vis` · **Alias fence**: `vis` · **Scope**: `source.viscript`

## Tóm tắt

ViScript là ngôn ngữ lập trình dễ học bằng tiếng Việt (cú pháp thụt lề kiểu Python,
kiểu tĩnh kiểu Rust). Hiện README phải gắn `python title="ViScript"` để GitHub tô màu
vì `vis` chưa được Linguist biết. Đề xuất thêm language + grammar Tree-sitter để
` ```vis ` và các tệp `.vis` được tô màu đúng.

## Thay đổi

- `lib/linguist/languages.yml`: entry `ViScript` (alias `vis`, extension `.vis`).
- `grammars.yml`: trỏ `source.viscript` tới repo grammar ở trên.
- `samples/ViScript/hello.vis`: mẫu nhận diện.
- `test/test_blob.rb`: ca test cho extension + alias.

## Kiểm chứng

- [ ] `npx tree-sitter generate` và `npx tree-sitter test` trong repo grammar xanh.
- [ ] `bundle exec rake` trong Linguist xanh.
- [ ] `script/update-ids` đã chạy; `language_id` không để placeholder.
- [ ] `script/grammar-build` / CI của Linguist nhận grammar submodules.

## Ghi chú

Grammar hỗ trợ identifier Unicode (tên biến tiếng Việt có dấu). Thụt lề là ngữ nghĩa
của ngôn ngữ; external scanner `INDENT`/`DEDENT` sẽ được bổ sung nếu cần parse cây đúng
(riêng mục đích highlight thì không bắt buộc).
