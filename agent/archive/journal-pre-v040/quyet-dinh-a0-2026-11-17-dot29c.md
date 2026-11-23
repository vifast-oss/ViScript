<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Nguyên văn quyết định Agent 0 (VD-167) — Agent 1 dán, không sửa chữ. -->
# QUYẾT ĐỊNH AGENT 0 · VD-167 · Tài liệu công thái học ViScript + bổ sung VD-165

1. Bổ sung VD-165 mục 4b: tách repo phải chuyển cả docs/viscript/, không chỉ thư mục mã viscript/.

2. Mở ST-172 (Agent 1, chỉ đọc, 0 dòng mã), xếp sau ST-169: soạn JOURNAL/vs-ergo.md, mỗi mục gồm hiện trạng (file:dòng), đề xuất, ca đỏ-trước, ảnh hưởng tới mã và test hiện có:
   a. str + str: cho phép, hoặc giữ lỗi nhưng thông báo gợi ý "{a}{b}".
   b. xs[i] trên list trả T?; s[i], s[a:b] trên str vẫn không có.
   c. pattern nhận cả Some(v) lẫn some(v); none giữ nguyên.
   d. giá trị mặc định của type tự áp khi dựng record (nợ, không phải thiết kế mới).
   e. print nhận nhiều đối số.
   f. mọi hàm std.str.f(s, ...) gọi được dạng s.f(...).
   g. format spec "{n:.2}".

3. Kiểm và báo: VS0412 có đang dùng cho 2 lỗi khác nhau (gán lại let, str + str) không. Nếu có, đề xuất tách mã lỗi.

4. Cheat sheet: đổi nhãn "VS0 chưa có" thành "chưa có tại <commit>".

5. Không sửa mã. Agent 0 duyệt vs-ergo.md cùng đợt với 6 tài liệu VS3.
