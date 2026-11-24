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

> **Mốc hiện hành (2026-11-24, `VD-288`)**: **3-OS Tier 1 chính thức** (Linux, Windows x86_64 từ `v0.4.5` @ `3daff62`, macOS arm64 từ `v0.5.0` @ `c06d7c6f`) + WASI Preview 1 runtime (`vis-host-wasm`) + kiến trúc 4 tầng (`v0.5.5` @ `3d85bd2c`, 18 crate workspace + 2 host workspace độc lập).
> `ST-311a..f` đã DONE trên `main`; mã nguồn đóng băng cho Phase C (`ST-309`) tại **`H4 = db67eab`** (version **`0.6.0`**, `#194`/`#190` đóng, `#195` chờ watchdog 2h trên `H4`, `#196` mở non-blocking).
> **Chưa tag `v0.6.0` / `v1.0.0-rc1`** — chờ hoàn tất ma trận Phase C trên `H4` + chốt `R37-viscript.{md,json}`.
> Không có số liệu mới trong tệp này — số liệu đo nằm trong `JOURNAL/`/`reports/` và `ACTIVE.md`.
