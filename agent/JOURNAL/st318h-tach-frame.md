<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-318h — Tách `crates/vis-interp/src/frame.rs` 1.030 → 159 (R1 mềm, `VD-298`)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st318h` · **Nền**: `plan/st318a` @ `640180e0` (đã có `#203`/`#204` + audit PASS gốc).
- **Worktree**: `/home/ps/viswt/st318h` — không chạm repo chính, không chạm `frame/worker.rs`.
- **Phạm vi**: chỉ `crates/vis-interp/src/frame*` (+ journal/STATE). **Move nguyên văn, 0 đổi hành vi/API**, 0 `unsafe`.

---

## 1. LOC trước/sau (`wc -l` thật)

| Trước | Sau | Dòng | Trách nhiệm |
|---|---:|---:|---|
| `src/frame.rs` | **1030** | — | — |
| | `src/frame.rs` | **159** | doc module + `mod` + re-export + 2 test cũ (`closure_moi_lan_goi_env_rieng`, `ten_khong_ton_tai_khong_nhan_argv`) |
| | `src/frame/work.rs` | **344** | work stack: `Work`, `MethodKind`, `Callee`, `LoopKindInit` |
| | `src/frame/state.rs` | **112** | state trên heap: `LoopState`/`LoopKindState`, `CallFrame`, `IFrame` + 6 method |
| | `src/frame/helper.rs` | **366** | helper: `block_works`/`initial_work`/`append_work`, `bind_pat_env`/`bind_all_env`, `field_of`/`index_of`/`type_name`, `if_work`/`args_works`/`run_arm_body`/`pop_n`/`take_call_args_names`/`underflow` |
| | `src/frame/step.rs` | **99** | `step_frame_owned`, `eval_expr_in`, `run_frame_for_test` |

Tệp lớn nhất sau tách: **366** (< 700 mục tiêu, trần 1.500). `frame/worker.rs` (663) **không chạm**. Các tệp `charge/core/expr/params/proc/proc_expr` không chạm.

**API giữ nguyên**: `vis_interp::frame::run_frame_for_test` (`pub use step::run_frame_for_test`), `vis_interp::frame::IFrame` (`pub use state::IFrame`), `pub(crate) frame_charge`/`run_frame_driver`/`FrameConfig`/`FrameOutput`. Nội bộ: item chuyển sang `pub(super)` (đúng phạm vi cũ trong module `frame`), con dùng chung qua `use super::*` như trước.

## 2. Bằng chứng move nguyên văn

Script đối chiếu từ `git show HEAD:crates/vis-interp/src/frame.rs` (bản trước tách) với 4 tệp mới, bỏ header/`use`/khoảng trắng và `pub(super)`:

| Tệp | Kết quả |
|---|---|
| `work.rs` | **TOKEN-IDENTICAL** — 329 dòng khác trắng, không đổi token |
| `state.rs` | token-identical, chỉ `cargo fmt` nối dòng chữ ký `fn new` (thêm dấu phẩy cuối) |
| `helper.rs` | **TOKEN-IDENTICAL** — 347 dòng khác trắng |
| `step.rs` | token-identical, chỉ `cargo fmt` nối dòng chữ ký `step_frame_owned` |

0 dòng logic bị sửa; thứ tự/ngữ nghĩa chẩn đoán không đổi. Test cũ chạy nguyên chỗ trong `frame.rs`, **0 tệp test cấp 1 mới** (R8 giữ 39/40).

## 3. Cổng (số thật)

| Cổng | Kết quả | Log + sha256 |
|---|---|---|
| `cargo test -p vis-interp` | **62 pass** — lib 5 + `interp_it` 55 + `interp_it` (binary phụ) 2; 0 fail | `/home/ps/vislogs/st318h-test-interp2.log` |
| `./vis-dev gate --fast` | **PASS** — nextest **3906/3906 passed, 4 skipped** (86,1 s); `check` PASS (fmt/clippy/R1..R10/DAG/self-test/build/doc-examples) | `/home/ps/vislogs/st318h-gate.log` · `5ac6c106…b4d936` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — 100.000 chương trình (93.768 chạy), 135,3 s, 739/s | `/home/ps/vislogs/st318h-diff.log` · `7380dff6…546f8` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** | `/home/ps/vislogs/st318h-clippy.log` · `95a75904…1cd4b` |
| R8 | **39/40** (`vis-cli/tests` 8/15), R1 mọi tệp ≤ 1.500 | trong `gate --fast` (`luật R1..R10: OK`) |

**Lưu ý**: lần chạy `cargo test -p vis-interp` đầu tiên đỏ `lat0_suspend::baseline_matches_current_vis` vì thiếu `target/debug/vis` (điều kiện môi trường, test cũ); sau `cargo build -p vis-cli` chạy lại **55/55 xanh**.

## 4. Commit

| Commit | Nội dung |
|---|---|
| `8c22a682` | `[ST-318h] tach frame.rs 1030 -> 159 + 4 tep (work/state/helper/step), move nguyen van` |
| (commit journal + STATE) | `[ST-318h] journal + STATE: so lieu cong` |

## 5. Việc còn lại

- Audit F1 của nhánh `plan/st318h` (kèm `plan/st318a` 6 commit) — chờ A0/Gatekeeper.
- Không merge `main` (không cần cho lát này); nhánh hợp nhất cùng `ST-318a`.
- Không còn tệp sản phẩm > 1.000 dòng thuộc lane này; KPI Sóng 7 `13 → 0` chờ `ST-318a`/`ST-318h` hợp nhất.
