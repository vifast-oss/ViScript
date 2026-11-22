<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Thư mục điều khiển dự án (agent)

Đây là “bộ não” của quá trình phát triển: quyết định, hàng đợi việc, bằng chứng đo, báo cáo.

| Tệp / thư mục | Nội dung | Ai ghi |
|---|---|---|
| [`ACTIVE.md`](ACTIVE.md) | Bảng điều khiển nhanh ≤ 100 dòng: tag, luồng, việc kế | Agent 2/W3 |
| [`QUEUE.md`](QUEUE.md) | Danh sách task `ST-##` + trạng thái `READY/BLOCKED/REVIEW/DONE` + dòng `APPROVED` | Agent 0 (qua người điều phối) |
| [`DECISIONS.md`](DECISIONS.md) | Quyết định thiết kế `VD-##` (nguồn sự thật về “vì sao”) | Agent 0 |
| [`DECISIONS-INDEX.md`](DECISIONS-INDEX.md) | Chỉ mục 1 dòng/`VD` + trạng thái, sinh tự động từ `DECISIONS.md` | ST-296d (sinh máy) |
| [`BLOCKED.md`](BLOCKED.md) | Việc phải chờ Agent 0 quyết (kèm bằng chứng, phương án) | Agent 2 |
| [`STATE.md`](STATE.md) | Chỉ mục trạng thái hiện hành; bản đầy đủ cũ ở `archive/state/` | Agent 2 |
| [`ROADMAP.md`](ROADMAP.md) | 8 hạng mục × mức độ thật (đã có / thiết kế có / chưa có) | Agent 0/1 |
| [`HOST-BAO-DAM.md`](HOST-BAO-DAM.md) | Bảo đảm theo nền tảng (macOS/Linux/Windows) + giới hạn đã biết | Agent 2 |
| [`JOURNAL/`](JOURNAL/) | Nhật ký từng task: log đo, báo cáo, quyết định dán nguyên văn | Agent 2 |
| [`reports/`](reports/) | Báo cáo từng đợt `R##-viscript.{json,md}` (máy kiểm bằng `test/report/check_report.py`) | Agent 2 |
| [`archive/`](archive/README.md) | Lịch sử bất biến: báo cáo/handoff/snapshot/đề xuất cũ + `STATE` cũ; tra `archive/INDEX.md` | ST-296d |

**Người mới**: đọc `ACTIVE.md` → `ROADMAP.md` → báo cáo mới nhất trong `reports/` → `QUEUE.md`;
lịch sử cũ tra `archive/INDEX.md` (không sửa tệp archive — `VD-142`).
