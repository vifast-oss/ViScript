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

> **Chế độ hiện hành (2026-11-26, `VD-289`/`VD-291`)**: **Phát triển Kiến trúc Tối ưu** (tiếp tục tạm dừng vô thời hạn kế hoạch phát hành `v0.6.0`/`v1.0.0-rc1` và soak 24h; `ST-309` = `BLOCKED`).
> **Đã duyệt `DONE` (`VD-290` + `VD-291`, 2026-11-25..26, nền `6ea63df` / mã `577405f`)**: hoàn tất toàn bộ Kiến trúc 5 Tầng DAG (`ST-312a..ST-313d`) + lát hậu-kiểm & 4 đợt audit `W1..W4` (**`ST-314a` DONE**) — 23 crate/5 tầng (`vis-dag-check` + self-test trong `vis-dev check`), R1..R9 PASS (R8 `39 ≤ 40`, 0 `unsafe`), `StdId` 167/167 compile-time exhaustive, corpus **563/563** (`.trap` + so message), WASI **393×2** + sem **390/390** + suite **28/28**, Linux gate **3817/3817**, Windows full **3.742/0 @`577405f1`**, 7,2M + 1M stdlib + 100k method diff **0 lệch**, 1,54 tỷ+ fuzz **0 vi phạm**; macOS clippy **PASS @`311d2288`** (full gate & clippy hậu-security hoãn khi `mps` bận, bù ở `ST-316c`). Đóng **`#108`**, thu hẹp `#127`, đăng ký **`#197`** (A0 audit: `sandbox.rs:121` tệp đơn `(write)` thiếu `AccessFs::Truncate`). Đồng bộ `docs/SPEC-lex.md`, `docs/SPEC.md` & `docs/ViScript-thiet-ke.md`.
> **Trục hiện hành — Sóng 4 (`ST-315a..ST-315d`, 4 luồng `W1..W4`)**: `ST-315a` **`REVIEW`** (VM `budget_charge` O(1) + bỏ TLS per-event; release `opt-bench` interp **329** / VM **344** ms, trước **610 ms**; `gate --fast` + clippy `-D warnings` + `vis-diff 100k lech=0` — `JOURNAL/st315a-vm-budget.md`), `ST-315b` **`REVIEW`** (`Value::Map(Rc<VisMap>)` CoW + chỉ mục băm lười + Q1–Q3; `opt-bench` release interp **48** / VM **46** ms, `mapbench N=316` **49,5 ms** ~27×, gate 3.836/0 + vis-diff 100k + stdlib_diff 0 lệch — `JOURNAL/st315b-map-cow.md`), `ST-315c` **`REVIEW`** (đã vá `#197` `Truncate` + W3 M2 ống trần 8 MiB/RAII dọn driver + `vis-soak` log ngoài repo; landlock **19/19**, plugin-host **28/28**, `gate --fast` PASS, **Windows full 3.747/0 @`c2b051c9`** — `JOURNAL/st315c-w3.md`), `ST-315d` (mã tĩnh **`VS0304`** chu trình `const` + sửa gợi ý `VS0204` + parity 66 hàm host trong `misc_api_it`, **R8 giữ 39 ≤ 40**; **`REVIEW` 2026-11-26**: VS0304 + Tarjan lặp + parity **66/66**, R-D vá tràn stack/O(N²) — `JOURNAL/st315d-vs0304.md`). Sóng 5 (`ST-316a..ST-316c`, `BLOCKED` chờ Sóng 4): `Value::List` CoW + `bin/vis-bench`, quét nợ `#186/#7/#48/#145`, bù macOS 3-OS & hồ sơ Go/No-Go. Chi tiết: `JOURNAL/quyet-dinh-a0-2026-11-26-vd291.md` & `ACTIVE.md`.
> Nền lịch sử đã đạt: 3-OS Tier 1 (`v0.4.5`/`v0.5.0`), kiến trúc 4 tầng (`v0.5.5`), bảo mật `#190`/`#194`/vá `#195` (`ST-311a..f`). Số liệu đo chi tiết nằm trong `JOURNAL/` và `ACTIVE.md`.
