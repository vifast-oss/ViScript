<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Nguyên văn quyết định Agent 0 (VD-169) — Agent 1 dán, không sửa chữ. -->
# QUYẾT ĐỊNH AGENT 0 · VD-169 · Chia ST-172 thành 2 bước

ST-172a (Agent 1, chỉ đọc, 0 dòng mã), làm ngay sau ST-169:
Soạn JOURNAL/vs-ergo-facts.md gửi Agent 0. CHỈ ghi dữ kiện, KHÔNG đề xuất.
Mỗi mục ghi rõ trạng thái: [ĐÃ CÓ] / [THIẾT KẾ CÓ, CHƯA LÀM] / [CHƯA CÓ], kèm file:dòng và commit.
1. Cú pháp
   - Danh sách đầy đủ từ khoá, toán tử, bảng độ ưu tiên toán tử.
   - Tóm tắt ngữ pháp (EBNF hoặc mô tả theo từng hàm của parser).
   - Quy tắc thụt lề, xuống dòng, chú thích, các loại chuỗi (thường, r"...", nhiều dòng), nội suy.
2. Hệ kiểu
   - Kiểu số (int bao nhiêu bit, tràn số xử lý ra sao, trộn int/float), str (Unicode, độ dài tính theo gì).
   - Quy tắc suy kiểu: chỗ nào bắt buộc ghi kiểu, chỗ nào suy được.
   - Optional: hiện có tự gói T thành T? không; ?. ?? if let hoạt động thế nào.
   - Generic, trait, impl, derives: làm được gì, chưa làm được gì.
   - Có chuyển kiểu ngầm nào không.
3. Ngữ nghĩa giá trị
   - list/map/set/record: truyền vào hàm là sao chép hay dùng chung; var bên ngoài có bị đổi không.
   - Closure bắt biến thế nào; có shadowing không.
4. Lỗi
   - Result, ?, else e:, các kiểu lỗi đang có; một hàm gọi cả fs và json thì hiện phải viết thế nào.
   - Toàn bộ danh sách mã lỗi (all_codes): mã, tên, thông điệp. Đánh dấu mã nào dùng cho nhiều lỗi.
   - Văn bản thật của thông báo lỗi (copy nguyên output) cho 10 cái bẫy trong cheat sheet.
5. Thư viện chuẩn
   - Bảng đầy đủ: module, hàm, chữ ký, có gọi được dạng method không.
   - Builtin toàn cục (print, len...): danh sách và chữ ký.
6. Tính năng riêng
   - Literal đơn vị: danh sách đơn vị, phép toán cho phép giữa các đơn vị, cách in ra.
   - cmd: sinh --help, kiểu tham số hỗ trợ.
   - Pipeline: kiểu dữ liệu chảy qua, các lệnh đang có (where, sort_by, select...).
   - Module, import, phạm vi công khai, vis pkg.
7. Trải nghiệm thật
   - 3 script thật 50 đến 200 dòng viết bằng ViScript (vd. đọc log rồi lọc, gọi HTTP rồi ghi JSON, công cụ cmd có cờ), mỗi script kèm bản Python tương đương.
   - Các lỗi Agent 2 hay gặp nhất khi viết test/ví dụ ViScript (đếm từ log nếu có).
8. Quyết định đã chốt
   - Danh sách quyết định thiết kế đã có lý do ghi trong docs (vd. kb = 1000, không có class), và những thứ đã chốt "không làm".

Định dạng: một tệp md duy nhất (JOURNAL/vs-ergo-facts.md), không giới hạn độ dài. Đầu tệp có mục lục đánh số theo 8 nhóm; mỗi nhóm bắt đầu bằng heading "## N. <tên nhóm>" để Agent 0 tra nhanh.

ST-172b: soạn JOURNAL/vs-ergo.md theo phạm vi VD-168, CHỈ bắt đầu sau khi Agent 0 gửi phân tích dựa trên ST-172a.
