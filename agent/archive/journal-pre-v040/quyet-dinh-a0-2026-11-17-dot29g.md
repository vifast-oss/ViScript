<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Nguyên văn quyết định Agent 0 (VD-172) — Agent 1 dán, không sửa chữ. -->
# QUYẾT ĐỊNH AGENT 0 · VD-172 · Cú pháp còn lại (bổ sung VD-171)
Thêm vào phạm vi ST-172b. Mỗi mục: hiện trạng (file:dòng, probe ở HEAD), đặc tả, ca đỏ-trước, ảnh hưởng mã/test, chia task <= 3 h.

GIỮ NGUYÊN: thụt lề + ":", elif, if là biểu thức, biểu thức cuối khối là giá trị trả về, closure "=>" một dòng và fn(...) nhiều dòng, generic [T], .. / ..=, and/or/not, match xét hết, with, derives, set{}, test fn, #/##, ^ten, | complete, extern cmd.

R11. Mọi hàm dựng là hàm: record, biến thể enum, kiểu bọc gọi theo vị trí hoặc theo tên (Circle(2.0), Circle(r: 2.0)); đối số có tên đứng sau đối số vị trí; cùng luật với fn.
R12. Gộp khai báo kiểu: "type X = T" thay cho alias; "type Email(str)" thay cho newtype, dựng bằng Email("..."), lấy giá trị bằng let Email(s) = e. Bỏ từ khoá alias, newtype.
R13. Biến thể enum: không cần tiền tố khi tên duy nhất; trùng tên giữa 2 enum thì bắt buộc Shape.Empty (dùng "."), lỗi biên dịch có gợi ý. Some/None/Ok/Err luôn trong prelude.
R14. Method: "impl P:" cho method riêng; fn f(self) chỉ đọc; fn f(var self) sửa được, chỉ gọi trên biến var; Self viết hoa. Báo hiện trạng impl P: và cách method sửa self hiện nay.
R15. Xuống dòng: tự nối dòng trong (), [], {} và khi dòng mới bắt đầu bằng ".", "|", "??", "and", "or". Dấu phẩy cuối được phép mọi nơi. Báo quy tắc hiện tại.
R16. let được che tên trong cùng phạm vi (bỏ VS0302 cho let); var không được khai báo lại.
R17. Điều kiện if/while chỉ nhận bool; "if xs:" báo lỗi kèm gợi ý xs.is_empty(). Báo hiện trạng.
R18. Pattern: literal, khoảng 1..=5, tuple, record User(name, age: a), gắn tên bằng "as"; thêm while let. Báo pattern nào đã có.
R19. Số nguyên: literal 0o755; tràn i64 lúc chạy thì panic (báo hiện trạng); phép bit là method std: bit_or, bit_and, bit_xor, shl, shr.
R20. assert cond, "thông điệp"; use std.fs as f; use std.fs.{read, write}; list thêm enumerate, zip, any, all, find, sort_by, is_empty; range thêm .step(n).
R21. Pipeline "|" chỉ dùng trong chế độ shell và khối cmd {}; code thường dùng chuỗi method; cả hai dùng chung hàm std. Ghi vào thiết kế.

Kiểm thêm: gọi generic tường minh f[int](x) có xung đột với chỉ số xs[i](...) trong parser không.
Thứ tự thực thi bổ sung vào VD-171: ... → R5 → R11 → R12 → R13 → R14 → R15 → R16 → R17 → R18 → R7 → R19 → R10 → R20 → R21 (chỉ sửa tài liệu).
Cheat sheet cập nhật theo toàn bộ R1-R21 sau khi thực thi; mọi ví dụ là doc-test (VD-170 P0 mục 3).
