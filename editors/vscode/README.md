# ViScript cho VS Code (mỏng)

Extension tối thiểu: TextMate grammar + client LSP gọi `vis lsp --stdio`.

- `package.json`: khai báo language `viscript` (`.vis`, `.vish`), grammar, cấu hình `viscript.serverPath` (mặc định `vis`).
- `syntaxes/viscript.tmLanguage.json`: grammar cơ bản (comment `#`, string, số, keyword, type, tên hàm).
- `extension.js`: `LanguageClient` nối JSON-RPC tới `vis lsp --stdio`.
- Không build/publish trong repo. Muốn chạy thử: `npm install` trong thư mục này (cần `vscode-languageclient`), rồi F5 trong VS Code.
- Server `vis` phải có trên `PATH` hoặc đặt `viscript.serverPath`.
