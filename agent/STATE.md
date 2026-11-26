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

> **Chế độ hiện hành (2026-11-27, `VD-289`/`VD-302`)**: **Phát triển Kiến trúc Tối ưu** (tiếp tục tạm dừng vô thời hạn kế hoạch phát hành `v0.6.0`/`v1.0.0-rc1` và soak 24h; `ST-309` = `BLOCKED`). Không soak 4h/24h, không fuzz 8h, không 3× full lặp, không tag mới.

## 1. Sóng 5, 6, 7 — `DONE`

> **Sóng 5 (`ST-316a`/`ST-316b`/`ST-316d`) — `DONE` (`VD-293`, 2026-11-26, tích hợp `882b7ffa`)**: `vis_runtime::value_ops` một nguồn + true O(1) CoW `Rc::ptr_eq` cho `Map`/`List` (0 đổi VBC1), release `vis-bench` **writebench ~240×** / **listbench 13–21×**; đóng `#199` (25k `const` 1,604 → **0,196 s**), `#48`, `#7`, `#186`, `#145`, `#47`; DAG cấm `vis-host-native → vis-runtime` (+self-test, 4 cạnh cấm), đóng `#198` (0 binary R8 mới), tách `host_parity.rs` 1.457 → 4 tệp < 500 dòng. Tích hợp: gate **3.880/3.880 (4 skip)**, clippy exit 0, `vis-diff 100k lech=0` (60,2 s), DAG **23 crate/5 tầng**, **R8 39/40**.
>
> **Sóng 6 (`ST-317a`/`ST-317b`/`ST-317c`) — `DONE` 3/3 (`VD-294` + `VD-295`, `main` @ `4e03a19c`)**: `Value` **64 → 32 B** (`Record`/`Enum` bọc `Rc`, `Cap` bọc `Box`), CoW `Record` in-place, singleton `{}`, `size_of::<Value>() == 32`; worker orchestration **một nguồn** `vis-runtime/src/worker/orchestrate.rs` 135 dòng + `value/mod.rs` 748 + `sqlite/mod.rs` 282 + `vis-lsp/lib.rs` 236; StdId registry **167** hàm, R10 `API-CODES.txt` ↔ `SPEC.md` `#75`, `AnalysisHost` `#81`/`#66`; gate **3906/3906 (5 ignored)**, diff 100k `lech=0`, `lsp_it` **158/158**, R8 **39/40**.
>
> **Sóng 7 (`ST-318a..i`) — `DONE` 9/9 (`VD-296`..`VD-299`, `main` @ `2cda3a68`)**:
> - **`ST-318a` (W1, `VD-299`)**: đóng **`#203`** (frame W=1 interp ghi `1` như VM, test `w11_*`/`w18`) + **`#204`** (non-frame `ap_va_flush` surface `VS0506` cho W=2/4, test `w12_*`); F1 perf `nap` + flush stdout trước trap, F2 dirty-set `DongBo`, F3 prune + chỉ dirty `push`; perf var-heavy ~**1,03–1,05×** main, write-heavy **0,98×**; audit F3 PASS `5e79bf54`, `st317b` **20/20**.
> - **`ST-318h` (W8, `VD-299`)**: `vis-interp/src/frame.rs` **1030 → 159** + 4 tệp (`helper` 366, `state` 112, `step` 99, `work` 344; `worker.rs` +166), giữ API `Frame`/`step_frame_owned`; audit PASS `03c1dc1f`, gate **3906**, diff 0.
> - **R1 mềm (`ST-318c/d/e/f/g`)**: tách 10 tệp > 1.000 dòng của `vis-types`, `vis-hir`, `vis-diag`, `vis-bytecode`, `vis-cli`, `vis-syntax`, `vis-loader`, `vis-runtime` → **0 tệp > 1.000** (max **1000**, `vis-cli/src/cli/run_cmd.rs`; `frame.rs` 159). **`ST-318i`**: 4 rustdoc link `vis-hir` sạch (`cargo doc -D warnings` exit 0). **`ST-318b`**: `#202` đóng, ca `host_parity::net_http` **20/20**.
> - **Cổng cuối `main` `2cda3a68`**: `GATE_EXIT=0` — **3912/3912 (4 skip)**; `DIFF_EXIT=0` 100k `lech=0` (84,9 s); `CLIPPY_EXIT=0`; `RULES_EXIT=0`; **R8 39/40**, 0 `unsafe`; 0 tệp > 1.500.

## 2. Trục hiện hành — Sóng 8 (b DONE, a/c REVIEW, c2/d/e đang chạy) + `ST-316c` `BLOCKED`

- **Sóng 8 — “Đóng nợ parity/rustdoc hậu Sóng 7” (`VD-300`..`VD-303`, `main` @ `ee8f252d`)**: `ST-319b` `DONE`; `ST-319c` `DONE (chẩn đoán)`; `ST-319a` `DONE` (merge `07a5e2b3`, PASS `b2033ebd`; `#206` đóng); `ST-319c2` `DONE` (merge `1ec1105a`, PASS `afed0250`; `#209` đóng); `ST-319d` `DONE` (merge `ee8f252d`, PASS `64e84b25`; `#208` đóng); `ST-319e` `DONE` (merge `a9c02abb`, PASS `4baa4d2d`; `#210` đóng); lane không xung đột tệp:
  - **`ST-319a` (W1, `#206`, `DONE` — merge `07a5e2b3`, PASS `b2033ebd`; closure by-value + cache bump, repro `0,0`, 5 test)** — closure cấp tệp: giữ **by-value** (`SPEC` §4.7), **sửa VM** (không sửa interp/SPEC); lane `vis-bytecode/codegen/**`, `vis-vm/vm/**` (trừ `worker.rs`), test conformance; KPI repro **`0/0`** hai engine + closure ghi global parity + `vis-diff 100k lech=0`; có thể trả branch chưa merge.
  - **`ST-319b` (W2, `#207` `DONE`)** — merge `eb7f5830`, PASS `4dff8b31`; rustdoc `vis-interp` link `[`Interp`]` (`frame.rs:5`); chỉ doc comment; KPI `RUSTDOCFLAGS="-D warnings" cargo doc -p vis-interp --no-deps` exit 0.
  - **`ST-319c` (W3, `DONE (chẩn đoán)` — journal `b1249172`, merge `5408290a`)** — **`#208` vẫn `mở`**: root cause **`vis-bytecode/src/codegen/**` thiếu tập `mutable`** (push ở biểu thức không phát `Assign`; ngoài lane frame/worker); chưa sửa mã.
  - **`ST-319c2` (W3b, `#209`, `DONE` — merge `1ec1105a`, PASS `afed0250`)** — frame closure by-value; P2-1 `nap_vao_env` vá `0caa9184`; test w19–w24, 26/26, gate 3921, w24 `3\n3`.
  - **`ST-319d` (W4, `#208`, `DONE` — merge `ee8f252d`, PASS `64e84b25`; codegen mutable, 32 test, gate 3929)** — codegen tập `mutable` theo thiết kế `/home/ps/vislogs/design-208-codegen.md`: xoá đặc cách `push` (`cau.rs`), thêm `Gen.mutables` + writeback `CallMethod; Assign; LoadLocal` (`expr.rs`), **không đổi VBC1**; lane `vis-bytecode/codegen/**`; test 6 ca `st317b_worker_orchestrate.rs`; gate + diff 100k + clippy.
  - **`ST-319e` (W5, `#210`, `DONE` — merge `a9c02abb`, PASS `4baa4d2d`; env module, A/B/C2/E/P5, gate 3922)** — closure gọi **hàm tên** cấp tệp: **interp non-frame** thừa hưởng env snapshot closure, phải dùng **env module**; **VM/frame đã đúng**; lane `vis-interp/src/interp/**`; test **A/B/C2/E/P5** + diff generator; gate + diff 100k + clippy.

- **Đã xong trên `ps`**: `vis-targets --wasm` + WASI **398×2** (371 `.out` + 27 `.trap`), `vis-wasm-sem` **390/390**, `vis-wasm-suite` **28/28** @`d97773de` (log `/home/ps/vislogs/st316c-wasm-final-20261126T1958Z.log:125–127`, sha256 `c7d90f60…da82957`).
- **Chờ máy**: chain 0-LLM `/home/ps/vislogs/peak1-2026-11-27-chain.sh` (18 bước, `--dry-run` exit 0, **chưa chạy**) cho `mps` @`311d2288` + `psvn` @`fe3446d2`; còn `cargo clippy --workspace --all-targets -- -D warnings` + `bin/vis-gate-macos --full` và đối chuẩn `bin/vis-bench` 3-OS. **Không** tuyên bố 3-OS khi chưa có log `mps`. **Checklist `ps`**: verify WASI trên `main` cuối **`d97773de`** (không phải task DONE).

## 3. Nợ và quyết định

- **Đóng trong Sóng 7**: **`#202` đóng ở `ST-318b`** (merge `fb725479`, PASS `001a9ffb`, 20/20); **`#203`/`#204` đóng ở `ST-318a`** (audit F3 PASS `5e79bf54`, merge `2cda3a68`).
- **Nợ Sóng 8**: **`#207` đóng ở `ST-319b`**; **`#206` đóng ở `ST-319a`**; **`#208` đóng ở `ST-319d`** (merge `ee8f252d`, PASS `64e84b25`); **`#209` đóng ở `ST-319c2`** (merge `1ec1105a`, PASS `afed0250`); **`#210` đóng ở `ST-319e`** (merge `a9c02abb`, PASS `4baa4d2d`).
- Sổ nợ hiện tại: **210 = 82 mở + 109 đóng + 16 VS3 + 2 không làm + 1 điều hướng** (`SO-NO.md` §8; R5 xanh; dải 1..210, thiếu `#205`).
- Quyết định gần nhất: [`JOURNAL/quyet-dinh-a0-2026-11-27-vd300.md`](JOURNAL/quyet-dinh-a0-2026-11-27-vd300.md); trước đó `VD-299` (chốt Sóng 7), `VD-298` (đợt 3), `VD-297` (đợt 2), `VD-296` (đợt 1), `VD-295` (đóng Sóng 6), `VD-294` (Sóng 6 phần 1), `VD-293` (Sóng 5), `VD-292` (Sóng 4).

## 4. Nền lịch sử

3-OS Tier 1 (`v0.4.5`/`v0.5.0`), kiến trúc 5 tầng DAG (`VD-289`/`VD-290`), kiến trúc 4 tầng `v0.5.5` (`3d85bd2`), bảo mật `#190`/`#194`/vá `#195` (`ST-311a..f`). Số liệu đo chi tiết nằm trong `JOURNAL/`, `ACTIVE.md` và `archive/state/` (không sửa lịch sử).
