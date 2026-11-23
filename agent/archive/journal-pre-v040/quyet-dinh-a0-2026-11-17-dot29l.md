<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Nguyên văn quyết định Agent 0 (VD-176) — Agent 1 dán, không sửa chữ. -->
# QUYẾT ĐỊNH AGENT 0 · VD-176 · #146: vá (A) + chứng minh tất định
Ghi nguyên văn vào JOURNAL, thêm dòng "APPROVED <ngày> Agent 0" trong QUEUE. Gỡ BLOCKED-CHO-A0 của #146.

1. Chọn (A): quyen.rs:472 đổi std::fs::metadata -> std::fs::symlink_metadata. Không đổi thiết kế, không đổi kỳ vọng.
   Loại (B) (F_GETPATH/libc/unsafe) và (C) (tách kỳ vọng theo nền).

2. Chứng minh BẮT BUỘC bằng ca tất định, không dựa vào số vòng:
   - Thêm điểm chèn chỉ dùng khi test (#[cfg(test)], không có trong bản build thường) giữa canonicalize (quyen.rs:465) và bước kiểm (quyen.rs:472).
   - Ca đỏ-trước: điểm chèn đổi root/tep thành symlink ra ngoài root ⇒ trên mã cũ đọc được nội dung ngoài (đỏ 100%, chạy 100 lần, ghi 100/100).
   - Ca thứ hai: điểm chèn đổi symlink về tệp thật ⇒ kết quả đúng như đặc tả.
   - Sau khi vá: cả hai ca xanh 100/100 trên mac và ps.
   - Ca âm cho điểm chèn: bỏ điểm chèn thì ca tất định phải báo "không chạy được điểm chèn", không được xanh im lặng.
   - Bằng chứng phụ: 6 triệu vòng ngẫu nhiên trên mac sau khi vá, ghi số đỏ.

3. Ghi giới hạn đã biết vào HOST-BAO-DAM.md: hardlink của tệp ngoài vào trong root được coi là nằm trong root (giới hạn chính sách, không phải lỗi đua).

4. Hướng dài hạn (đưa vào VS3 cùng #140/#109b, không làm bây giờ): mở từng thành phần đường dẫn bằng openat + O_NOFOLLOW qua rustix (Linux có thể dùng openat2 RESOLVE_BENEATH), bỏ hẳn kiểm theo đường dẫn. Đường đọc và đường ghi dùng chung một cơ chế. Agent 1 thêm mục này vào vs3-qd tương ứng.

5. Làm thành task ST-174 (≤ 3 h), xếp sau ST-173, trước ST-170. Đóng #146 khi mục 2 đạt; gate mac 3 lần liên tiếp PASS không cần chạy lại.
   R29 ghi: số lần gate đỏ do #146 trước khi vá (theo st170-dem-gate-do-t146.md) và trạng thái sau khi vá.

Thứ tự: ST-167 → ST-173 → ST-174 → ST-170 (R29) → ST-171.
