<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# STATE — ViScript (chỉ mục trạng thái)

> **2026-11-23 (ST-296d)** — `STATE.md` cũ (2.256 dòng, cập nhật cuối 22/11/2026, mốc `v0.3.6`)
> đã được **giữ nguyên văn** tại [`archive/state/STATE-2026-11-22.md`](archive/state/STATE-2026-11-22.md)
> theo `VD-142` (không sửa/xoá lịch sử; chỉ `git mv`). Tệp này là **chỉ mục** thay thế.

**Đọc trạng thái hiện hành theo thứ tự**:
1. [`ACTIVE.md`](ACTIVE.md) — bảng điều khiển nhanh (≤ 100 dòng), tag và luồng đang chạy.
2. [`ROADMAP.md`](ROADMAP.md) — 8 hạng mục × mức thật.
3. [`reports/`](reports/) — báo cáo vòng mới nhất đã chốt (`R36`, `v0.5.0`) và khung Phase C (`R37`, `v0.6.0`); [`QUEUE.md`](QUEUE.md) — hàng việc.
4. [`DECISIONS.md`](DECISIONS.md) / [`DECISIONS-INDEX.md`](DECISIONS-INDEX.md) — quyết định và chỉ mục.

> **Chế độ hiện hành (2026-11-24, `VD-289`)**: **Phát triển Kiến trúc Tối ưu** (đã hủy/tạm dừng vô thời hạn kế hoạch phát hành `v0.6.0`/`v1.0.0-rc1` và soak 24h; `ST-309` → `BLOCKED`).
> **Sóng 0–3 xong 2026-11-25** (`ST-312a..d` `65ea2a0`..`4e6f810`; `ST-313a/b` `aa816e4`/`c04fee9`; `ST-313c/d` `47245b0`..`1f5e0f46`): 5 tầng DAG xong, `#193`/`#196` đóng, `vis-conformance` 420 ca + 384 test API dời khỏi CLI. Chi tiết `JOURNAL/st312-songs01.md`, `st313-song2.md`, `st313-song3.md`.
> Tập trung hoàn tất **Kiến trúc Đích 5 Tầng DAG** (`ST-312a..ST-313d`: `vis-hir`, `vis-http`, `vis-runtime`, `vis-host-linux` vào workspace, stdlib `StdId` 1 nguồn sự thật + chia miền, `vis-conformance` dữ liệu).
> Nền lịch sử đã đạt: 3-OS Tier 1 (`v0.4.5`/`v0.5.0`), kiến trúc 4 tầng (`v0.5.5`), bảo mật `#190`/`#194`/vá `#195` (`ST-311a..f`). Số liệu đo chi tiết nằm trong `JOURNAL/` và `ACTIVE.md`.
