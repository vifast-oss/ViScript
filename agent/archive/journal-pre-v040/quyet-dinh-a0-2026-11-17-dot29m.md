<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Nguyên văn quyết định Agent 0 (VD-177) — Agent 1 dán, không sửa chữ. -->
# QUYẾT ĐỊNH AGENT 0 · VD-177 · ST-167, nợ #147, cây ps
Ghi nguyên văn vào JOURNAL, thêm dòng "APPROVED <ngày> Agent 0" trong QUEUE.

1. ST-167: công nhận DONE (10/10 cả hai nền, cùng HEAD 795df046, cây viscript 092fd5ab). Số mac 1696 s ghi là CẬN TRÊN, không dùng làm số chuẩn.

2. Cấp nợ #147 "mac chậm hơn ps" ngay, không chờ đo lại. Bước điều tra theo thứ tự, không sửa mã, không đổi ngưỡng:
   a. Đo lại soak mac khi load < 2 và KHÔNG có việc của dòng OS chạy (xem mục 3). Ghi load trước/sau.
   b. Nếu tỉ lệ <= 3×: đóng #147, ghi số mới làm số chuẩn.
   c. Nếu vẫn > 3×: đo thời gian từng suite trong gate trên cả hai nền, liệt kê 10 suite chênh lớn nhất (tỉ lệ mac/ps). Kiểm riêng các ca chạy nhiều vòng (vd. #146 50 000 vòng) và các ca tạo tiến trình.
   Xếp sau ST-174, trước ST-170 (R29 cần số soak hợp lệ). Ngân sách 2 h.

3. Khoá máy dùng chung giữa hai dòng (phối hợp với Agent 1 dòng OS):
   - Mọi phép đo thời gian/hiệu năng (soak, gate có tính giờ, ngưỡng thời gian) phải giữ một tệp khoá chung trên máy, vd. ~/.vifast-measure.lock (dùng flock).
   - Dòng còn lại không build/không chạy QEMU khi khoá đang bị giữ; chờ hoặc làm việc không tốn CPU.
   - Log đo ghi: load trước/sau, có giữ khoá không.
   Phép đo không giữ khoá: chỉ được ghi là "cận trên", không dùng làm số chuẩn.

4. Cây ps bị xoá hàng nghìn tệp tracked: điều tra TRƯỚC ST-170.
   - Ghi: đường dẫn repo trên ps, user đang chạy (ps hay root), git status --short | wc -l, các tệp bị xoá thuộc thư mục nào (đếm theo thư mục cấp 1).
   - Trả lời: các lần gate ps đã dùng (ST-161, ST-166, ST-167) chạy trên cây nào; tree hash thư mục viscript lúc đó có khớp commit ghi trong log không.
   - Từ nay gate ps chạy trên một clone/worktree sạch (git status rỗng ngoài thư mục log bị .gitignore). Kiểm bằng máy: vis-remote từ chối chạy nếu cây không sạch.
   - Không khôi phục hay xoá gì trong cây cũ cho tới khi Agent 0 đọc kết quả điều tra.
   - Sau ST-171, ps và psvn clone mới từ repo ViScript, không dùng lại cây cũ.

5. Thứ tự: ST-173 (đang chạy) → ST-174 → mục 4 (cây ps, ≤ 1 h) → #147 bước a-c → ST-170 (R29) → ST-171 (chỉ sau khi Agent 0 duyệt 6 tài liệu VS3 + st169 + vs-ergo.md).
