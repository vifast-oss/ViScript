<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# STATE — ViScript (chỉ mục trạng thái)

> **2026-11-23 (ST-296d)** — `STATE.md` cũ (2.256 dòng, cập nhật cuối 22/11/2026, mốc `v0.3.6`)
> đã được **giữ nguyên văn** tại [`archive/state/STATE-2026-11-22.md`](archive/state/STATE-2026-11-22.md)
> theo `VD-142` (không sửa/xoá lịch sử; chỉ `git mv`). Tệp này là **chỉ mục** thay thế.

**Đọc trạng thái hiện hành theo thứ tự**:
1. [`ACTIVE.md`](ACTIVE.md) — bảng điều khiển nhanh (≤ 100 dòng), tag và luồng đang chạy.
2. [`ROADMAP.md`](ROADMAP.md) — 8 hạng mục × mức thật.
3. [`reports/`](reports/) — báo cáo vòng mới nhất (`R35`); [`QUEUE.md`](QUEUE.md) — hàng việc.
4. [`DECISIONS.md`](DECISIONS.md) / [`DECISIONS-INDEX.md`](DECISIONS-INDEX.md) — quyết định và chỉ mục.

> **Mốc (2026-11-23)**: `v0.4.5` đã tag @ `3daff62` (**Windows Tier 1** đã công bố + WASI
> Preview 1), `v0.4.1` @ `90844bc`, `v0.4.0` @ `c2accdf`. **macOS arm64 đã verify `v0.4.5`**
> trên `mps` (gate 3631/0/3, workspace 3636/0/3, host 54/54, WASI 28/28, `vis-wasm-sem`
> 388/390) nhưng **Tier 1 chưa công bố** — chờ `ST-300m3` (3× `--full` + soak 4 h).
> `ST-300m1` đã merge (`c3b7290`); **`ST-300m2`/`ST-300m3` đang chờ**. Các crate
> `vish-edit`, `vish-term`, `vis-plugin-abi`, `vis-plugin-macros` còn ở nhánh riêng, **chưa merge**.
> Không có số liệu mới trong tệp này — số liệu đo nằm trong `JOURNAL/`/`reports/` và `ACTIVE.md`.
