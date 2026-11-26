<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# `ST-318g` (W7 · Sóng 7 đợt 2) — R1 mềm `vis-loader` + `vis-runtime`

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st318g`
- **Worktree**: `/home/ps/viswt/st318g` · **Nền**: `main` `baa2dbd3` (`VD-297`) · **Không** chạm repo chính
- **Phạm vi**: move **nguyên văn**, 0 đổi hành vi/API; 0 `unsafe`; không sửa SPEC/QUEUE/DECISIONS/ACTIVE;
  không chạm `crates/vis-interp/src/frame.rs`.

## 1. Kết quả

| Tệp trước | Dòng | Tệp sau | Dòng |
|---|---:|---|---:|
| `crates/vis-loader/src/modules.rs` | **1043** | `modules/mod.rs` (entry + `pub use`) | 81 |
| | | `modules/kieu.rs` (`FileUnit`/`Loaded`/`LoadError`) | 68 |
| | | `modules/duong_dan.rs` (đường dẫn, `vis.lock`, gói, `dòng:cột`) | 212 |
| | | `modules/state.rs` (overlay LSP + cache, `LoadState`) | 155 |
| | | `modules/nap.rs` (`Nap` + hậu duyệt DFS + `load_full_voi`) | 555 |
| `crates/vis-runtime/src/builtin/mod.rs` | **1006** | `builtin/mod.rs` (docs + `mod` + `pub use` + `trap`/`sap_xep_theo_khoa`) | 128 |
| | | `builtin/host.rs` (trait `Host`) | 117 |
| | | `builtin/toan_tu.rs` (`bin`/`arith`/`un`) | 287 |
| | | `builtin/goi.rs` (`call` + helper `exec`) | 509 |

- Mọi tệp sau khi tách **< 700** (trần mục tiêu), tệp lớn nhất `modules/nap.rs` 555.
- API công khai giữ nguyên: `modules::{load, load_with, load_full, load_full_voi_text,
  load_full_voi_text_trang_thai, load_full_stdin, LoadState, FileUnit, Loaded, LoadError, vi_tri,
  dump_deps, dat_noi_dung_mo, xoa_noi_dung_mo, xoa_cache_phu}`;
  `builtin::{Host, call, bin, un, SIGS, sig, builtin_name, BuiltinSig, sap_xep_theo_khoa, KieuKhoa, …}`.
- Test cũ chạy **nguyên chỗ** (`modules/state.rs::tests_overlay`); **0** tệp test cấp 1 mới (R8 giữ **39/40**,
  `vis-cli/tests` **8/15**).

## 2. Commit

| Commit | Nội dung |
|---|---|
| `71efb3f3` | `[ST-318g]` tách `vis-loader/src/modules.rs` 1043 → `modules/{mod,kieu,duong_dan,state,nap}.rs` |
| `4f614921` | `[ST-318g]` tách `vis-runtime/src/builtin/mod.rs` 1006 → `{mod,host,toan_tu,goi}.rs` |

## 3. Cổng (log `/home/ps/vislogs/st318g-*.log`)

| Cổng | Kết quả |
|---|---|
| `cargo test -p vis-loader -p vis-runtime` (`st318g-test-loader-runtime.log`) | **PASS** — 24 + 5 + 290, 0 failed |
| `./vis-dev gate --fast` (`st318g-gate.log`) | **PASS** — fmt/clippy/R1..R10/DAG/build/doc-examples/L0 **3902/3902** |
| `cargo clippy --workspace --all-targets -- -D warnings` (`st318g-clippy.log`) | **exit 0** |
| `VIS_DIFF_N=100000 ./bin/vis-diff` (`st318g-diff.log`) | **`lech=0`** (100.000; 160,4 s) |

## 4. Việc còn lại

- Chờ Gatekeeper/A0 nghiệm thu `REVIEW`; dọn worktree/nhánh trong 24 h sau merge (`AGENTS.md` §6).
- `vis-interp/src/frame.rs` **1030** vẫn để `ST-318h` (sau khi `ST-318a` hợp nhất) — **không** chạm.
