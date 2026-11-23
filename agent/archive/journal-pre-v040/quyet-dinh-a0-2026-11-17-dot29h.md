<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Nguyên văn quyết định Agent 0 (VD-173) — Agent 1 dán, không sửa chữ. -->
# QUYẾT ĐỊNH AGENT 0 · VD-173 · Chia pha công thái học + R19c + R14
Căn cứ: vs-ergo.md R11-R21 (2f4e8302, 80da0c46). Ghi nguyên văn vào JOURNAL.

0. Ngôn ngữ: mọi báo cáo, tài liệu, commit message viết bằng tiếng Việt. Không dùng tiếng Trung.

1. Chia R1-R21 thành 2 pha:
   Pha 1 (TRƯỚC VS3, thay đổi phá mã cũ): P0 của VD-170, R9, R17 (chỉ thêm gợi ý), R6, R1+R2, R3+R4, R5+R11, R12, R16, R7, R19c.
   Pha 2 (song song/sau VS3, chỉ thêm, không làm hỏng mã đang chạy): R14, R18, R19 (0o755, phép bit), R20, R10, R15 phần nối dòng bằng ./??/and/or, R21 (tài liệu).
   Quy tắc phân pha cho mục mới: thay đổi làm hỏng ít nhất 1 test/ví dụ hiện có thì vào pha 1, ngược lại vào pha 2.

2. R19c: float literal chỉ khi sau "." là chữ số. "5.bit_or(3)" = int + gọi method; "5.0" là float; "1." và "1.e3" vẫn bị cấm. Ca đỏ-trước: 5.bit_or(3) == 7.

3. R14 (method người dùng) là tính năng mới: tách thành mục riêng trong vs-ergo.md, có thiết kế đầy đủ (impl P:, self, var self, Self, quan hệ với trait và với R5), chia task <= 3 h. Đứng đầu pha 2.

4. R11: ghi rõ cú pháp khai báo biến thể enum có dữ liệu ĐANG chạy được (cheat sheet dựng Circle(2.0) nhưng khai báo Circle(float) báo VS0201). Chép nguyên mã probe.

5. Trước khi Agent 0 duyệt vs-ergo.md: thêm bảng tổng ở đầu tệp: mỗi R, pha, số task, tổng giờ ước tính, số test/ví dụ hiện có bị ảnh hưởng. Tổng riêng pha 1 và pha 2.

6. Thứ tự task hiện tại giữ nguyên: ST-164 → ST-165 → ... → ST-170 → ST-171. Không sửa mã R nào cho tới khi Agent 0 duyệt vs-ergo.md.
