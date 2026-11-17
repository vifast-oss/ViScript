<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Nguyên văn quyết định Agent 0 (VD-170) — Agent 1 dán, không sửa chữ. -->
# QUYẾT ĐỊNH AGENT 0 · VD-170 · Phân tích vs-ergo-facts, mở ST-172b
Căn cứ: JOURNAL/vs-ergo-facts.md (ST-172a). Ghi nguyên văn vào JOURNAL, thêm dòng "APPROVED <ngày> Agent 0" trong QUEUE.

A. RÚT KHỎI PHẠM VI VD-168
- Mục h (xs[i] trả T?): bỏ. Q5/VD-18 đã chốt xs[i] panic, xs[i]?/xs.get(i) trả T?. ST-172b chỉ xác minh đã làm hay chưa, kèm probe.
- Mục a phần tự gói T -> T?: đã có, bỏ.

B. QUYẾT ĐỊNH CHỐT (ST-172b soạn đặc tả chi tiết + ca đỏ-trước, không bàn lại hướng)
P0 (sửa trước mọi mục khác):
 1. newtype: chừng nào chưa hạ thì báo lỗi ngay tại dòng khai báo, không được bỏ qua im lặng.
 2. Mọi thông điệp có chữ "VS0" (chưa hỗ trợ ở VS0...): bỏ "VS0", thay bằng "chưa hỗ trợ (nợ #N)".
 3. Cheat sheet thành doc-test: mọi ví dụ có nhãn "chạy được" phải là một ca test; ví dụ hỏng thì cổng đỏ. Sửa cheat sheet theo dữ kiện (if let, else e: dạng biểu thức, --help, AnyError hiện chưa có).
P1:
 4. p"..." không xử lý escape, vẫn giữ nội suy {}. VS0102 thêm gợi ý r"..." / p"...".
 5. Luật thông điệp đóng băng: giữ mã và thông điệp chính; ĐƯỢC thêm dòng "gợi ý:". Thêm gợi ý cho 5 mẫu VS0201: const A = 1, s[a:b], U(n: 1), "{x:.2}", |x|.
 6. Tách mã đa nghĩa: VS0412 và VS0501 có mã mới cho từng nghĩa; mã cũ chỉ còn dùng cho phần chung. Đề xuất số mã và bảng chuyển đổi.
P2:
 7. Viết hoa: Some/Ok/Err viết hoa ở cả dựng lẫn pattern; none/true/false viết thường; xoá builtin None. Pattern viết thường: cảnh báo + vis fmt tự sửa. Sửa văn bản VD-19 cho khớp.
 8. AnyError theo VD-20. Trước khi soạn: probe lại n9, chép nguyên mã probe; giải thích json.decode trả "V" là kiểu gì (VD-01 cấm any) và vì sao ? giữa fs và json báo VS0412 khi cả hai đều Result[_, str].
 9. Gọi dạng method: hàm std.<mod>.f có tham số đầu kiểu T thì gọi được x.f(...). Kiểm xung đột tên giữa module std.str, kiểu str và builtin str(x).
10. str + str: cho phép nối chuỗi.
11. print nhận nhiều đối số (in cách nhau một dấu cách); format spec {expr:spec} chạy ở runtime; dựng record U(name: ...) có áp giá trị mặc định; if let.
P3:
12. const suy kiểu khi vế phải là literal; duration in ra dạng chuẩn hoá (1h30m); cmd tự sinh --help.
13. Q2: giữ chuyển ngầm int -> float và số học trộn int/float như hiện tại; sửa văn bản Q2 cho khớp mã.

C. ST-172b (Agent 1, chỉ đọc, 0 dòng mã): JOURNAL/vs-ergo.md
- Mỗi mục 1-13: hiện trạng (file:dòng), đặc tả, ca đỏ-trước, ảnh hưởng tới mã và test hiện có, chia task <= 3 h, thứ tự phụ thuộc.
- Probe lại tất cả ở HEAD mới nhất (ST-172a dùng binary 0df923c2, không phải HEAD 823047ac).
- Viết lại 3 script ở mục 7 theo thành ngữ (filter/map/len, cmd thay vì tự phân tích args), ghi số dòng ViScript so với Python. Mục nào vẫn dài hơn Python thì ghi nguyên nhân.
- Chưa sửa mã. Agent 0 duyệt vs-ergo.md cùng đợt với 6 tài liệu VS3; các mục P0-P1 được duyệt sẽ thực thi TRƯỚC VS3.
