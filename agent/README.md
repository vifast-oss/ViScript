<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Thư mục điều khiển dự án (agent)

Đây là “bộ não” của quá trình phát triển: quyết định, hàng đợi việc, bằng chứng đo, báo cáo.

| Tệp / thư mục | Nội dung | Ai ghi |
|---|---|---|
| [`QUEUE.md`](QUEUE.md) | Danh sách task `ST-##` + trạng thái `READY/BLOCKED/REVIEW/DONE` + dòng `APPROVED` | Agent 0 (qua người điều phối) |
| [`DECISIONS.md`](DECISIONS.md) | Quyết định thiết kế `VD-##` (nguồn sự thật về “vì sao”) | Agent 0 |
| [`BLOCKED.md`](BLOCKED.md) | Việc phải chờ Agent 0 quyết (kèm bằng chứng, phương án) | Agent 2 |
| [`STATE.md`](STATE.md) | Ảnh chụp trạng thái gần nhất | Agent 2 |
| [`ROADMAP.md`](ROADMAP.md) | 8 hạng mục × mức độ thật (đã có / thiết kế có / chưa có) | Agent 0/1 |
| [`HOST-BAO-DAM.md`](HOST-BAO-DAM.md) | Bảo đảm theo nền tảng (macOS/Linux/Windows) + giới hạn đã biết | Agent 2 |
| [`JOURNAL/`](JOURNAL/) | Nhật ký từng task: log đo, báo cáo, quyết định dán nguyên văn | Agent 2 |
| [`reports/`](reports/) | Báo cáo từng đợt `R##-viscript.{json,md}` (máy kiểm bằng `test/report/check_report.py`) | Agent 2 |

**Người mới**: đọc `ROADMAP.md` → báo cáo mới nhất trong `reports/` → `QUEUE.md` là đủ để biết đang ở đâu.
