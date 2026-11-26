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

> **Chế độ hiện hành (2026-11-26, `VD-289`/`VD-293`)**: **Phát triển Kiến trúc Tối ưu** (tiếp tục tạm dừng vô thời hạn kế hoạch phát hành `v0.6.0`/`v1.0.0-rc1` và soak 24h; `ST-309` = `BLOCKED`).
> **Đã duyệt `DONE` (`VD-290` + `VD-291` + `VD-292` + `VD-293`, 2026-11-25..26, nền `8ff19b42` / mã `7014fb7b`, tích hợp Sóng 5 `882b7ffa`)**: hoàn tất Kiến trúc 5 Tầng DAG (`ST-312a..ST-313d`) + hậu-kiểm `ST-314a` + trọn bộ **Sóng 4 (`ST-315a..ST-315d DONE`)** + trọn **Sóng 5 (`ST-316a`, `ST-316b`, `ST-316d` DONE)** — `vis_runtime::value_ops` một nguồn + true O(1) CoW `Rc::ptr_eq` cho `Map`/`List` (0 đổi VBC1), release `vis-bench` **writebench ~240×** / **listbench 13–21×**; đóng `#199` (25k `const` 1,604 → **0,196 s**), `#48`, `#7`, `#186`, `#145`, `#47`; DAG cấm `vis-host-native → vis-runtime` (+self-test, 4 cạnh cấm), đóng `#198` (0 binary R8 mới), tách `host_parity.rs` 1.457 → 4 tệp < 500 dòng. Tích hợp: gate **3.880/3.880 (4 skip)**, clippy exit 0, `vis-diff 100k lech=0` (60,2 s), DAG **23 crate/5 tầng**, **R8 39/40**. Windows full gần nhất **`3.779/0/4 @7014fb7b`**; macOS `mps` chưa chạy full (bù ở `ST-316c`). Sổ nợ đăng ký **`#200`** (P2 lệch interp↔VM `main` nhiều tham số `int`) & **`#201`** (P3 `argv` gán vào tham số `int` không kiểm kiểu) — `mở`, VS3/correctness; tổng **202 = 82 mở + 101 đóng + 16 VS3 + 2 không làm + 1 điều hướng**.
> **Trục hiện hành — Sóng 6 (`ST-317a` W1 + `ST-317c` W3 `READY` song song; `ST-317b` W2 `READY` nhưng chờ tệp; `ST-316c` giữ `BLOCKED` vì `mps`)**: `ST-317a` (nén `Value` 64→32 B + `Record` CoW + singleton `{}`; lane `vis-core/src/value/**`), `ST-317c` (bảng `StdId` một nguồn + kiểm máy `API-CODES.txt`↔`SPEC.md` `#75` + `vis-lsp` `AnalysisHost` `#81`/`#66`; lane `vis-lsp/**`), `ST-317b` (khử trùng lặp `vis-interp` sync↔frame/worker + 6 tệp > 1.100 LOC → < 1.000; **cấm chạm `vis-core/src/value/mod.rs` & `vis-lsp/src/lib.rs`** tới khi 317a/317c merge), `ST-316c` (làm trước trên `ps`: `vis-targets --wasm` + hồ sơ Go/No-Go; phần `mps` chờ máy rảnh). Chi tiết: `JOURNAL/quyet-dinh-a0-2026-11-26-vd293.md` & `ACTIVE.md`.
> Nền lịch sử đã đạt: 3-OS Tier 1 (`v0.4.5`/`v0.5.0`), kiến trúc 4 tầng (`v0.5.5`), bảo mật `#190`/`#194`/vá `#195` (`ST-311a..f`). Số liệu đo chi tiết nằm trong `JOURNAL/` và `ACTIVE.md`.
