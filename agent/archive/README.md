<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ARCHIVE — lịch sử tài liệu điều khiển (ST-296d)

> **Chỉ đọc / bất biến.** Mọi tệp trong thư mục này là **bản gốc lịch sử**: chỉ được `git mv` từ
> `agent/` vào đây, **không sửa nội dung**, không xoá. Lịch sử Git đi theo tệp (`git log --follow`).
> Nguyên tắc **VD-142**: báo cáo/snapshot cũ không bị viết lại hay thay số; chỗ nào cần đính chính
> thì tạo tệp mới ở thư mục đang hoạt động và trỏ về bản lưu.

## 1. Cấu trúc

| Thư mục | Nội dung | Ví dụ |
|---|---|---|
| [`reports/`](reports/) | báo cáo/handoff/snapshot/đo lường rời (không phải `agent/reports/R##`) | `BAO-CAO-*`, `BAN-GIAO-A0-*`, `SNAPSHOT-*`, `DEM-*`, `LICH-SANG-*` |
| [`proposals/`](proposals/) | đề xuất/thiết kế/RFC/audit/ghi chú cũ | `DE-XUAT-*`, `KE-HOACH-*`, `RFC-*`, `VISCRIPT-ERGONOMICS-*` |
| [`state/`](state/) | `STATE.md` cũ (bản đầy đủ 2.256 dòng đến 22/11/2026) | `STATE-2026-11-22.md` |

`agent/reports/R##-viscript.{md,json,json.license}` **không** nằm trong archive: đây là bộ báo cáo
vòng đang được `test/report/check_report.py --all` kiểm và R34 sẽ ghi tiếp.

## 2. Cách tra

- [`INDEX.md`](INDEX.md) — bảng **đường dẫn gốc → vị trí lưu** cho toàn bộ tệp đã chuyển.
- Tìm theo tên cũ: `rg "TÊN-CŨ" agent/archive/INDEX.md` hoặc `git log --follow -- <đường-dẫn-mới>`.
- Trạng thái hiện hành: [`../ACTIVE.md`](../ACTIVE.md), [`../STATE.md`](../STATE.md),
  [`../reports/`](../reports/), [`../QUEUE.md`](../QUEUE.md).

## 3. Quy ước

1. Tài liệu mới/đang hiệu lực **không** đặt trong `archive/`.
2. Không sửa tệp archive để “cập nhật số”; tạo bản mới ở thư mục hoạt động và ghi liên kết.
3. Chỉ `git mv` khi quy hoạch tiếp; cập nhật `INDEX.md` cùng commit.
