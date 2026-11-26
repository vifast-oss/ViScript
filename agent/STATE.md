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

> **Chế độ hiện hành (2026-11-27, `VD-289`/`VD-295`)**: **Phát triển Kiến trúc Tối ưu** (tiếp tục tạm dừng vô thời hạn kế hoạch phát hành `v0.6.0`/`v1.0.0-rc1` và soak 24h; `ST-309` = `BLOCKED`). Không soak 4h/24h, không fuzz 8h, không 3× full lặp, không tag mới.

## 1. Sóng 5 và Sóng 6 — `DONE`

> **Sóng 5 (`ST-316a`/`ST-316b`/`ST-316d`) — `DONE` (`VD-293`, 2026-11-26, tích hợp `882b7ffa`)**: `vis_runtime::value_ops` một nguồn + true O(1) CoW `Rc::ptr_eq` cho `Map`/`List` (0 đổi VBC1), release `vis-bench` **writebench ~240×** / **listbench 13–21×**; đóng `#199` (25k `const` 1,604 → **0,196 s**), `#48`, `#7`, `#186`, `#145`, `#47`; DAG cấm `vis-host-native → vis-runtime` (+self-test, 4 cạnh cấm), đóng `#198` (0 binary R8 mới), tách `host_parity.rs` 1.457 → 4 tệp < 500 dòng. Tích hợp: gate **3.880/3.880 (4 skip)**, clippy exit 0, `vis-diff 100k lech=0` (60,2 s), DAG **23 crate/5 tầng**, **R8 39/40**.
>
> **Sóng 6 (`ST-317a`/`ST-317b`/`ST-317c`) — `DONE` 3/3 (`VD-294` + `VD-295`, `main` @ `4e03a19c`)**:
> - **`ST-317a` (W1, `VD-294`)**: nén `Value` **64 → 32 B** (`Record(Rc<VisRecord>)`, `Enum(Rc<VisEnum>)`, `Cap(Box<CapVal>)`), CoW `Record` ghi tại chỗ qua `ptr_rc`/`tach_neu_cung_rc` (test pointer interp+VM; bench `r.f = v` **~34×**, `r = r with` **~30×**), singleton `Rc<VisMap>` cho `{}`; gate **3886/3886 (4 skip)**, `vis-diff 100k lech=0` (60,1 s), clippy PASS, conformance **568/568**, 0 `unsafe`, VBC1 không đổi; `size_of::<Value>() == 32`.
> - **`ST-317b` (W2, `VD-295`)**: tách `vis-pkg/src/pkg.rs` **1151 → max 377** (`f247567c`), `vis-runtime/src/rt/sched.rs` **1189 → max 279** (`987c5585`), `vis-types/src/tyck/pat.rs` **1140 → max 321** (`7ae2f96e`) — audit `e1fe1733` PASS, merge `f8f155d0`; `vis-lsp/src/lib.rs` **1056 → 236** + `server/*` (`29a7b254`, `lsp_it` **158/158**); `vis-core/src/value/mod.rs` **1415 → 748** (+`lit` 403, `ten` 291) (`3dd51026`); `sqlite/mod.rs` **1132 → 282** + 6 tệp (`7a33d996`); **worker orchestration một nguồn** `vis-runtime/src/worker/orchestrate.rs` **135** dòng (`ap_dung_outcome`/`flush_proxy`/`pump`/`ap_va_flush`, `codes::CROSS`), `exec.rs` 461→**368**, `vm/worker.rs` 586→**485**, **10 test W1..W10** (`9fc06efb`); dedup `opt_depth_ty`/`loi_arity`/`loi_khong_phai_ham` (`a92fdb2f`); vá audit `a801ccc6` (merge `main` VD-294), `d56b96f1` (W6 frame `VS0506` W=1/2/4), `7a1d7ea7` (journal P2-B/non-frame W6/P3-B); audit cuối `ef3e6928` + `4e03a19c` **PASS**. Cổng: gate **3906/3906 (5 ignored)** ≡ 3901/3901 + 4 skip, `vis-diff 100k lech=0` (56,5 s), clippy 0 warning, W **10/10**, DAG **23/5/0**, R8 **39/40**, 0 tệp sản phẩm > 1.100 LOC thuộc lane W2.
> - **`ST-317c` (W3, `VD-294`)**: `vis-hir/src/stdlib/registry.rs` là **một nguồn** cho `StdId` + chữ ký + metadata (macro `std_registry!`, **167** hàm, xoá 8 bảng cũ −1.547 dòng, không `_ =>`); đóng **`#75`** bằng R10 `API-CODES.txt` ↔ `SPEC.md` §7.2 (57 mã, fixtures 20/20); đóng **`#81`/`#66`** bằng `vis_lsp::AnalysisHost` + `vis_loader::LoadState` tiêm được (0 `thread_local!` trong `vis-lsp`, `lsp_it` **158/158**). Gate **3885/3885 (4 skip)**, diff 100k `lech=0`, R8 **39/40**.

## 2. Trục hiện hành — `ST-316c` (W4) `BLOCKED`

- **Đã xong trên `ps`**: `vis-targets --wasm` + WASI **395×2** (368 `.out` + 27 `.trap`), `vis-wasm-sem` **390/390**, `vis-wasm-suite` **28/28** @`29a7b254` (log `/home/ps/vislogs/st316c-wasm-20261126T1613Z.log`).
- **Chờ máy**: chain 0-LLM `/home/ps/vislogs/peak1-2026-11-27-chain.sh` (18 bước, `--dry-run` exit 0, **chưa chạy**) cho `mps` @`311d2288` + `psvn` @`fe3446d2`; còn `cargo clippy --workspace --all-targets -- -D warnings` + `bin/vis-gate-macos --full` và đối chuẩn `bin/vis-bench` 3-OS. **Không** tuyên bố 3-OS khi chưa có log `mps`.
- **Cổng tích hợp cuối `main` `4e03a19c`** (log `/home/ps/vislogs/final-main-20261126T1747Z.log`, sha256 `b0044754…f75dd`): `GATE_EXIT=0` (L0 **3901/3901**, 4 skipped), `DIFF_EXIT=0` (100.000, `lech=0`, 61,9 s), `CLIPPY_EXIT=0`, `RULES_EXIT=0`; **R8 39/40**, `vis-cli/tests` **8/15**.

## 3. Nợ và quyết định

- **`VD-295`** đăng ký **`#203`** (`mở`, W2 frame W=1 lệch interp↔VM: stdout `0` vs `1`, exit 0) và **`#204`** (`mở`, non-frame `run_workers_parallel*`/`_vm*` W≥2 nuốt lỗi task không await, cả 2 engine) — giao **VS3/correctness**; phát hiện từ audit `ST-317b`, không chặn `DONE`.
- **`ST-318a`** (Builder, `plan/st318a`, 2026-11-27): đóng **kỹ thuật** `#203` (frame W=1 interp in `1` như VM; test parity `w11_*`) + `#204` (`ap_va_flush` surface `VS0506` cho non-frame W=2/4 cả 2 engine; test `w12_*`); gate **3904/3904** (4 skip), clippy 0, R8 **39/40**. Chờ A0 cập nhật `SO-NO.md`/`QUEUE` (Builder không sửa).
- Sổ nợ hiện tại: **205 = 85 mở + 101 đóng + 16 VS3 + 2 không làm + 1 điều hướng** (`SO-NO.md` §8; R5 xanh).
- Quyết định gần nhất: [`JOURNAL/quyet-dinh-a0-2026-11-27-vd295.md`](JOURNAL/quyet-dinh-a0-2026-11-27-vd295.md); trước đó `VD-294` (Sóng 6 phần 1), `VD-293` (Sóng 5), `VD-292` (Sóng 4).

## 4. Nền lịch sử

3-OS Tier 1 (`v0.4.5`/`v0.5.0`), kiến trúc 5 tầng DAG (`VD-289`/`VD-290`), kiến trúc 4 tầng `v0.5.5` (`3d85bd2`), bảo mật `#190`/`#194`/vá `#195` (`ST-311a..f`). Số liệu đo chi tiết nằm trong `JOURNAL/`, `ACTIVE.md` và `archive/state/` (không sửa lịch sử).
