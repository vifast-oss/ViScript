<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-322a — R1 headroom: tách helper ra khỏi `cli/run_cmd.rs`

- **Ngày**: 2026-11-27 · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st322a`
- **Nền**: `main` @ `304cb6107ea27d5050cb024875203462d0168998`
- **Worktree**: `/home/ps/viswt/st322a` — **không** chạm repo chính (2 agent đang re-run remote gate trên `main`).
- **Trạng thái**: mã + test + cổng **PASS**; chờ Gatekeeper audit; **không merge/push**.
- **Nguồn đề xuất**: `/home/ps/vislogs/r1-headroom-20261126T224448Z.md` (sha `108a8153…`), mục 3.5/§4.5 Giai đoạn A.

## 0. TL;DR

`run_cmd.rs` đúng **1000 dòng** (điểm mềm R1 của phiên cha). Tách **nguyên văn** 6 helper
(`vi_tri_runtime`, `xuat_stdout`, `dong_chan_doan`, `kiem_cu_phap_probe`, `do_luong_chay`,
`hir_co_async`) sang `cli/run_cmd/ho_tro.rs`, rồi re-export `pub(crate)` tại `run_cmd.rs`.

| | Trước | Sau |
|---|---:|---:|
| `crates/vis-cli/src/cli/run_cmd.rs` | 1000 | **872** |
| `crates/vis-cli/src/cli/run_cmd/ho_tro.rs` | — | 138 |

Không đổi hành vi, CLI, stdout/stderr/exit, không đổi `main.rs`/`check.rs`/`fmt.rs`.

## 1. Thay đổi

- **`cli/run_cmd/ho_tro.rs`** (mới, 138 dòng): dòng 14–135 + 970–979 của file cũ, chép **byte-for-byte**
  (đã so sánh `difflib` với `git show HEAD:…`). Gồm doc comment đầy đủ của từng helper.
- **`cli/run_cmd.rs`**: xoá 132 dòng đó, thêm:
  ```rust
  mod ho_tro;
  use ho_tro::do_luong_chay;
  pub(crate) use ho_tro::{
      dong_chan_doan, hir_co_async, kiem_cu_phap_probe, vi_tri_runtime, xuat_stdout,
  };
  ```
- **Điểm khác duy nhất so với nguyên văn**: `fn do_luong_chay` → `pub(super) fn do_luong_chay`.
  Bắt buộc vì quyền riêng của Rust là một chiều (module cha **không** import được item `fn` riêng
  của module con); `pub(super)` giữ hàm chỉ thấy trong `run_cmd`, không lọt ra crate. Thân hàm,
  chữ ký và ngữ nghĩa không đổi.

## 2. API / path ngoài không đổi

- `main.rs:14`: `pub(crate) use cli::run_cmd::{dong_chan_doan, xuat_stdout};` → vẫn phân giải qua
  re-export trong `run_cmd.rs`; không sửa `main.rs`.
- `check.rs` / `fmt.rs`: dùng `use crate::*` → vẫn thấy 2 hàm qua root re-export; không sửa 2 tệp.
- `main.rs:190`: `cli::run_cmd::run_run(...)` không đổi.
- `st238_tests` giữ nguyên trong `run_cmd.rs`; `use super::*` thấy `kiem_cu_phap_probe` qua re-export.

## 3. Kiểm chứng

Mọi lệnh chạy trong worktree, `TMPDIR=/home/ps/vislogs/tmp` (**đĩa**, không tmpfs), `CARGO_BUILD_JOBS=2`.
Disk `/` lúc chạy: **20 G** available (> ngưỡng dừng 5 G) — không bao giờ chạm ngưỡng.

| Lệnh | Kết quả | Log |
|---|---|---|
| `cargo test -p vis-cli` | **PASS** — 1814 passed / 0 failed (11 binary + doc-test) | `st322a-cargo-test.log` |
| `./vis-dev gate --fast` | **PASS rc 0** — fmt/clippy/R1..R10/self-test/build/doc-examples + L0 **3942/3942** | `st322a-gate-fast.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS rc 0** (21.31 s, 0 warning) | `st322a-clippy.log` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **lech=0** — 100.000 chương trình, 7 test PASS (79.6 s) | `st322a-vis-diff.log` |
| `./bin/vis test test/conformance` | **1..573 — 573 ok / 0 not ok**, rc 0 | `st322a-conformance.log` |

Ghi chú: đề bài nêu corpus "568/568"; số thực tại nền `304cb610` là **573/573** (corpus đã lớn thêm
5 ca `vd270_strlen`) — không có ca nào fail. `vis-diff` chạy dù đây là di chuyển thuần tuý (không
chạm runtime semantics) để đối chiếu trực tiếp; `lech=0` xác nhận stdout/stderr/exit khớp 2 engine.

## 4. Luật

- **R1**: 0 tệp > 1500 dòng (toàn cây 258.458 dòng). `run_cmd.rs` 872 < 1000 (điểm mềm của phiên cha).
- **R8**: tổng `.rs` cấp 1 trong `crates/*/tests/` + `vis-host-linux/tests/` = **39/40**; riêng
  `crates/vis-cli/tests/` = **8/15**. Không thêm tệp test nào (helper nằm dưới `src/`).
- `git status` trước commit: `M cli/run_cmd.rs`, `?? cli/run_cmd/ho_tro.rs`, `??` journal — chỉ 3 tệp.

## 5. Kết luận

Di chuyển thuần tuý, 0 đổi hành vi/CLI; cổng máy tái lập được **PASS**. Không merge, không push,
không đụng `agent/QUEUE.md`/`STATE.md`/`DECISIONS.md`. Chờ Gatekeeper (Agent 1) audit rồi A0 duyệt.
