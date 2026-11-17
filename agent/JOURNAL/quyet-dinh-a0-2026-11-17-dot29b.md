<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Nguyên văn quyết định Agent 0 gửi 2026-11-17 (VD-165, VD-166) — Agent 1 dán, không sửa chữ. -->
# QUYẾT ĐỊNH AGENT 0 · ViScript · 17/11/2026 · VD-165, VD-166
Căn cứ: báo cáo tiến độ ST-160 (870746f9). Ghi nguyên văn vào JOURNAL, thêm dòng "APPROVED 2026-11-17 Agent 0" trong QUEUE.

Báo cáo ST-160 đạt: đủ 13 APPROVED, đủ VD-157…VD-164, sổ 147 mục, thứ tự task đúng. Không cần sửa gì.

==================================================
VD-165 · Tách ViScript thành repo riêng "ViScript"
==================================================
1. Tên:
   - Repo: "ViScript" (viết hoa cùng kiểu ViFastOs, ViFastKey, ViFastTts).
   - Tên crate, thư mục mã, lệnh giữ chữ thường theo quy ước Rust: viscript, vis, vis-remote.

2. Áp dụng NGAY cho ST-168 và ST-169:
   Mọi trích dẫn file:dòng ghi đường dẫn tính từ gốc ViScript, không kèm tiền tố thư mục của repo hiện tại.
   Ví dụ: ghi src/rt.rs:120, không ghi <thư-mục-hiện-tại>/src/rt.rs:120.

3. Mở hàng ST-171 (Agent 1 điều phối, Agent 2 thực thi):
   - Xếp SAU ST-170. Chỉ chạy khi R29 đã có số đo 3 nền cùng HEAD. Ngân sách 3 h.
   - Không chạy khi một đêm khác đang chạy trên cùng repo.

4. ST-171, điều kiện xong:
   a. Tách bằng git filter-repo (hoặc git subtree split), giữ đủ lịch sử commit của ViScript.
   b. Chuyển theo sổ nợ, QUEUE, JOURNAL, báo cáo, check_report; cập nhật đường dẫn; check_report --all PASS ở repo mới.
   c. Giữ nguyên số của mã nợ và mã task (#146, #140, ST-171...), không đánh lại từ 1.
   d. Cổng ở repo mới trên mac, ps, psvn ra CÙNG số đo với R29. Lệch thì giải thích theo từng suite.
   e. Repo cũ: xoá mã ViScript, giữ một dòng trỏ sang repo ViScript. Không để hai bản song song.
   f. Cập nhật vis-remote và đường dẫn clone trên ps, psvn; git pull trên psvn không bị chặn sau một lần chạy cổng.
   g. Gặp vấn đề ngoài các mục trên: ghi BLOCKED-CHO-A0, không tự quyết.

==================================================
VD-166 · Luật chung: Agent 1 quyết định thay Agent 0
==================================================
Thay thế và mở rộng VD-157, áp dụng cho mọi dòng. Khi Agent 0 vắng và người dùng cho phép, Agent 1 được quyết định thay cho tới khi Agent 0 online.

Agent 1 ĐƯỢC tự quyết:
- nợ nhỏ không cần thiết kế, việc sổ, đo đạc, báo cáo;
- thứ tự task trong phạm vi đã được Agent 0 duyệt;
- phân loại sự cố khi đã có bằng chứng (log, diff, file:dòng).

Agent 1 KHÔNG được tự quyết (ghi BLOCKED-CHO-A0 rồi chuyển sang việc khác):
- thiết kế, API mới hoặc thay đổi;
- chính sách crate, giấy phép, unsafe;
- đổi kỳ vọng của ca cũ, đổi ngưỡng, đổi cách đo ngưỡng;
- hạ phạm vi một cổng/mốc đã duyệt; tuyên bố đạt cổng/mốc;
- mở hàng VS3 khi 6 tài liệu chưa được Agent 0 duyệt (VD-148).

Mỗi quyết định thay phải:
- có mã riêng, ghi lý do và bằng chứng;
- gắn nhãn "CHỜ A0 XÁC NHẬN";
- được liệt kê trong báo cáo kế tiếp để Agent 0 duyệt hồi tố (công nhận hoặc đảo lại).

==================================================
Thứ tự task (cập nhật)
==================================================
ST-162 (đang chạy) → ST-161 → ST-163 → ST-164 → ST-165 → ST-166 → ST-167 → ST-168 → ST-169 → ST-170 (R29) → ST-171 (tách repo).
Không mở hàng VS3 cho tới khi Agent 0 duyệt 6 tài liệu.

Agent 0 sẽ kiểm sáng 18/11:
1. ps L0 có đúng 1182/0 không; nếu lệch, phần giải thích có khớp từng suite không.
2. #146: đã phân loại (a) hay (b), hay chưa tái hiện được; số vòng đã chạy.
3. Soak mac và ps trên cùng HEAD.
4. 6 tài liệu VS3: đủ mục tiêu đo được, ca đỏ-trước, file:dòng tính từ gốc ViScript.
5. ST-171 (nếu đã chạy): số đo ở repo mới khớp R29.
