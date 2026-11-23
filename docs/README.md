<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Tài liệu ViScript

| Tệp | Nội dung | Ai nên đọc |
|---|---|---|
| [`SPEC-lex.md`](SPEC-lex.md) | Đặc tả từ vựng: token, literal, escape, nội suy, mã lỗi `VS01xx` | người viết lexer/parser, người cần biết cú pháp chính xác |
| [`SPEC.md`](SPEC.md) | Đặc tả ngôn ngữ đầy đủ: cú pháp, kiểu, quyền, CLI, ABI plugin, nền tảng (ST-309) | người dùng/người viết công cụ cần hành vi chuẩn tắc |
| [`ViScript-thiet-ke.md`](ViScript-thiet-ke.md) | Thiết kế ngôn ngữ: kiểu, đơn vị đo, hàm, module, lỗi, thư viện chuẩn | người đóng góp, người muốn hiểu “vì sao” |
| [`doc-examples-allow.txt`](doc-examples-allow.txt) | Danh sách khối ví dụ trong thiết kế **chưa chạy được** (có lý do từng dòng) | người bảo trì `doc-examples` |

Quy ước: mọi khối ```vis trong `ViScript-thiet-ke.md` phải **chạy được** (kiểm bằng `./vis-dev doc-examples`), trừ khối có tên trong `doc-examples-allow.txt`.

Tài liệu điều khiển dự án (QUEUE, quyết định, báo cáo từng đợt) nằm ở [`../agent/`](../agent/).
