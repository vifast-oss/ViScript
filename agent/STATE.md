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
> **Lát đang `REVIEW` (`ST-314a`, commit `311d2288` + follow-up)**: vá lỗi `NODE_DRIVER` ở `bin/vis-wasm-conformance` (đo thật **299 ca × 2 engine, 0 hỏng**), tường minh `"crates/vis-hir"` ở `Cargo.toml` (**23** member) + R8 `vis-host-linux` (**39** ≤ 40), bỏ wildcard `_ =>` của `StdId` (**167/167**, `path`/`fs_net` nhận `StdId`), vá `DOC_BLOCKS` lệch 1 dòng. Cổng: `gate --fast` **3805/3805 @933dce82 PASS**, `vis-diff 100k lech=0`, `vis-targets` PASS, clippy Linux + Windows PASS; macOS clippy **PASS @311d2288**, full gate bỏ qua theo chỉ đạo người dùng. Chi tiết: `JOURNAL/st314a-hau-kiem.md` (low-peak: DAG 23 crate/5 tầng, self-check bắt `ro` + caller thiếu `--`, core-test 1.804/0, Windows full 3.730/0/4 @933dce82; đề xuất docs cho A0 `JOURNAL/de-xuat-dong-bo-docs-2026-11-25.md`). Cao điểm 2 (`ps`+`psvn`): **67× `vis-diff 100k` lech=0**, fuzz **1.543.483.028 ca/0 vi phạm**, 4× gate 3805/3805, Windows full + final exit 0.
> Nền lịch sử đã đạt: 3-OS Tier 1 (`v0.4.5`/`v0.5.0`), kiến trúc 4 tầng (`v0.5.5`), bảo mật `#190`/`#194`/vá `#195` (`ST-311a..f`). Số liệu đo chi tiết nằm trong `JOURNAL/` và `ACTIVE.md`.
