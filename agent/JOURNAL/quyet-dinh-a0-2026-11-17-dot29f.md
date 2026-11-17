<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Nguyên văn quyết định Agent 0 (VD-171) — Agent 1 dán, không sửa chữ. -->
# QUYẾT ĐỊNH AGENT 0 · VD-171 · Bộ quy tắc công thái học ViScript
Thay thế mục 7, 8, 10, 11, 13 của VD-170. Sửa VD-19, Q2, §1.6. Các mục khác của VD-170 giữ nguyên.
ST-172b soạn đặc tả chi tiết + ca đỏ-trước + bảng chuyển đổi mã cũ cho từng quy tắc; không bàn lại hướng.

R1. Chữ hoa: kiểu/trait/biến thể enum UpperCamel (Some, None, Ok, Err, Circle); biến/hàm/module/trường snake_case; hằng UPPER_SNAKE; literal nguyên thuỷ true/false viết thường. Dựng và pattern viết giống hệt nhau. Pattern viết thường: cảnh báo + vis fmt tự sửa.

R2. Option/Result là enum std: enum Option[T]: Some(T), None; enum Result[T, E]: Ok(T), Err(E). T? là viết tắt Option[T].
 - Bỏ từ khoá none; None là biến thể.
 - Cho phép T??. Tự gói: giá trị T gán vào T?/T??/... được gói đủ số lớp; None luôn là lớp ngoài cùng; lớp trong viết Some(None).
 - Ca đỏ-trước bắt buộc: fn first[T](xs: list[T]) -> T? với T = int?; map[str, int?].get phân biệt được không có khoá và khoá giá trị None.

R3. Xử lý vắng mặt/lỗi:
 - ? đẩy lỗi lên; ?? giá trị mặc định; if let, let ... else, match xử lý tại chỗ.
 - let x = expr else e: <khối> ; khối bắt buộc thoát (return/break/continue/panic); với Option không có biến e.
 - Không có dạng biểu thức "f() else 0".

R4. Lỗi:
 - Mọi hàm std trả Result[T, Error], Error là record cài trait Error (message, source, span), thay cho str.
 - ? tự chuyển E cài trait Error sang AnyError (VD-20).
 - json.decode trả Result[Json, Error]; Json là enum (Null, Bool, Num, Str, Arr, Obj); bỏ kiểu V.
 - main có thể trả Result[(), AnyError]; lỗi thì in lỗi và thoát mã 1.
 - Ca chuẩn: đọc tệp + decode JSON viết được trong 2 dòng bằng ?.

R5. Gọi hàm:
 - Đối số có tên f(1, b: 5); --b chỉ dùng trong chế độ lệnh và cmd.
 - Dựng record User(name: "An", age: 20); trường có mặc định được bỏ.
 - {...} chỉ là map; bỏ dựng record bằng {name: ...}.
 - x.f(a) tương đương mod.f(x, a) khi tham số đầu của f có kiểu của x; áp cho toàn bộ std. Báo xung đột tên giữa module std.str, kiểu str, builtin str(x) nếu có.

R6. Chuỗi: p"..." không xử lý escape, có nội suy; str + str nối chuỗi; {expr:spec} chạy ở runtime; print nhận nhiều đối số, cách nhau một dấu cách.

R7. Số: chỉ int -> float là chuyển ngầm; / luôn trả float; // chia lấy phần nguyên; % khớp với //. const suy kiểu khi vế phải là literal. Sửa văn bản Q2 cho khớp. Liệt kê mọi test hiện dựa vào int / int = int.

R8. Chỉ số: giữ Q5/VD-18. Xác minh xs[i]? và xs.get(i) đã chạy chưa.

R9. Thông báo lỗi: mỗi mã một nghĩa (tách VS0412, VS0501, VS0201, VS0301, VS0604, VS0701; đề xuất số mã và bảng chuyển đổi); lỗi cú pháp hay gặp có dòng "gợi ý:"; bỏ chữ "VS0"; từ khoá chưa làm (newtype, ...) báo lỗi tại chỗ dùng.

R10. Đơn vị: in dạng chuẩn hoá (1h30m, 1.5mb); xác minh trộn đơn vị khác nhau báo lỗi.

Thứ tự thực thi (sau khi Agent 0 duyệt vs-ergo.md): P0 của VD-170 → R9 → R6 → R1 + R2 → R3 + R4 → R5 → R7 → R10. Tất cả xong TRƯỚC VS3.
ST-172b: viết lại 3 script mục 7 theo bộ quy tắc mới (dạng sẽ có sau khi làm), ghi số dòng so với Python.
