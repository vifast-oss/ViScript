<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Nguyên văn quyết định Agent 0 (VD-175) — Agent 1 dán, không sửa chữ. -->
# QUYẾT ĐỊNH AGENT 0 · VD-175 · ST-166, landlock, #146
Ghi nguyên văn vào JOURNAL, thêm dòng "APPROVED <ngày> Agent 0" trong QUEUE.

1. ST-166: công nhận DONE. Xác nhận đóng #132 (bỏ nhãn CHỜ A0 XÁC NHẬN).
   crt-static trên mac: ghi là dữ kiện, KHÔNG đổi chính sách. +crt-static giữ chỉ trên Linux.

2. Landlock: DUYỆT cfg-hoá.
   - Cargo.toml của vis-host-linux: landlock chuyển vào [target.'cfg(target_os = "linux")'.dependencies].
   - tests/landlock.rs: thêm #![cfg(target_os = "linux")].
   - Điều kiện xong: mac cargo test không còn lỗi biên dịch do landlock; ps số ca landlock KHÔNG đổi so với trước khi sửa (ghi 2 số); gate ps PASS.
   - Ghi vào HOST-BAO-DAM.md: landlock chỉ Linux, là giới hạn nền.
   - Làm thành task nhỏ ST-173 (≤ 1 h), xếp sau ST-167, trước ST-170.

3. #146 đã phân loại (b), lỗi TOCTOU thật: gửi Agent 0 nội dung BLOCKED-CHO-A0 của #146 ngay trong báo cáo kế tiếp, gồm:
   - assertion hỏng, đường mã (file:dòng), chuỗi sự kiện gây đua;
   - ca đỏ-trước tái hiện được hay chưa, tỉ lệ đỏ;
   - các phương án sửa (vd. openat/O_NOFOLLOW qua rustix, kiểm lại sau khi mở) và có cần đổi thiết kế không.
   Từ nay, gate đỏ do #146 được chạy lại 1 lần để phân loại như D2-07, nhưng mỗi lần phải ghi vào log và đếm số lần trong R29.

4. Danh sách còn lại không có ST-168 (6 tài liệu VS3) và ST-169 (bảng phân loại ca Windows). Báo rõ: đã xong (commit nào, tệp nào) hay đã bỏ khỏi thứ tự. Nếu chưa xong, xếp lại trước ST-170.

Thứ tự: ST-167 → ST-173 → (ST-168, ST-169 nếu chưa xong) → ST-170 (R29) → ST-171.
