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

> **Chế độ hiện hành (2026-11-25, `VD-289`/`VD-290`)**: **Phát triển Kiến trúc Tối ưu** (tiếp tục tạm dừng vô thời hạn kế hoạch phát hành `v0.6.0`/`v1.0.0-rc1` và soak 24h; `ST-309` = `BLOCKED`).
> **Sóng 0–3 ĐÃ DUYỆT `DONE` (`VD-290`, 2026-11-25, code freeze `90fd24de`)**: `ST-312a..d` (`65ea2a0`..`4e6f810`), `ST-313a/b` (`aa816e4`/`c04fee9`+`8855bc3c`), `ST-313c/d` (`64855d7`..`90fd24de`, `47245b0`..`157c73b`) — 5 tầng DAG (**23 crate**, `vis-dag-check` OK), `#193`/`#196` đóng, `vis-conformance` 420 ca + 384 test API dời khỏi CLI, `worker_leak_it.rs` tách process riêng (fallback **3.809/0**, Windows full **3.729/0**), đồng bộ `docs/ViScript-thiet-ke.md` & `docs/SPEC.md`.
> **Việc kế tiếp (`ST-314a` `READY`)**: vá lỗi `NODE_DRIVER` ở `bin/vis-wasm-conformance` (chạy thật 2 engine), tường minh `"crates/vis-hir"` ở `Cargo.toml` + R8 `vis-host-linux`, bỏ wildcard `_ =>` của `StdId`, và chạy bù clippy macOS `mps` khi rảnh. Chi tiết: `JOURNAL/quyet-dinh-a0-2026-11-25-vd290.md`.
> Nền lịch sử đã đạt: 3-OS Tier 1 (`v0.4.5`/`v0.5.0`), kiến trúc 4 tầng (`v0.5.5`), bảo mật `#190`/`#194`/vá `#195` (`ST-311a..f`). Số liệu đo chi tiết nằm trong `JOURNAL/` và `ACTIVE.md`.
