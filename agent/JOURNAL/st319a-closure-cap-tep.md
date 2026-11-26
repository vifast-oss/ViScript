<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-319a — Đóng `#206`: closure đọc/ghi biến cấp tệp theo **by-value**

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st319a`
- **Nền**: `main` @ `d97773de` (`VD-299`, Sóng 7 `DONE`)
- **Worktree**: `/home/ps/viswt/st319a` — không chạm repo chính.
- **Trạng thái**: **WIP checkpoint** — phần mã + test đã xanh trên cổng nhanh; chưa tuyên bố `DONE`.

## 0. TL;DR

| Trước | Sau (default interp ↔ VM) |
|---|---|
| `var g = 0` / closure đọc `g`: interp `0,0`, VM `0,1` | cả hai `0,0` |

Hướng: **codegen** chụp biến cấp tệp **có xuất hiện trong thân closure** vào `MakeClosure`
(ô cục bộ riêng + `LoadCaptured`/`Assign` prologue — cùng cơ chế `var` capture ST-60b), nên
closure đọc/ghi **bản snapshot**; **không đổi VBC1**, không đổi `vis-vm`.

## 1. Thay đổi

| Tệp | Nội dung |
|---|---|
| `crates/vis-bytecode/src/codegen/bien_tu_do.rs` (mới) | quét `sym` được tham chiếu trong `HBody`/`HStmt`/`HExpr` (đi xuyên lambda lồng); sym là duy nhất nên không cần miền ràng buộc |
| `crates/vis-bytecode/src/codegen/expr.rs` | `HExpr::Lambda`: sau snapshot `f.locals`, chụp thêm global **được tham chiếu** (dedupe, sắp theo slot ⇒ closure không chạm global giữ bytecode cũ). `HExpr::Var`: ưu tiên ô cục bộ (closure đã chụp) trước kho global |
| `crates/vis-bytecode/src/codegen/cau.rs` | `slot_cua`: `captured` → `f.locals` → `globals` → cấp ô mới (ghi trong closure không xuyên global) |
| `crates/vis-conformance/tests/misc_api_it/st319a_closure_cap_tep.rs` (mới, `#[path]` trong `lib_api_it.rs`) | 3 ca: đọc snapshot, ghi không xuyên + bền qua nhiều lần gọi, lambda lồng giữ snapshot closure ngoài; mỗi ca chạy 4 đường (interp/VM mặc định + W=1 cả hai engine) |
| `crates/vis-vm/tests/diff_100k.rs` | thêm nhóm sinh `ClosureCapTep` (3 mẫu cố định) để `vis-diff` bắt được lớp lỗi này về sau |

## 2. Kết quả cổng (tại checkpoint)

| Cổng | Kết quả |
|---|---|
| `cargo fmt --all` | sạch |
| `cargo check --workspace --all-targets` | PASS (18,74 s) |
| `./vis-dev gate --fast` | **PASS** — nextest **3915/3915** (0 fail), có 3 ca `st319a_closure_cap_tep` |
| `cargo test -p vis-bytecode -p vis-vm -p vis-interp` | PASS (bytecode 103 + …, vm 56 + diff, interp 55) |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | `tong=100000 chay=94425 lech=0` (80,2 s) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `vis test test/sem` / `--vm test/sem` | **525/525** cả hai engine |

## 3. Việc còn lại / phát hiện ngoài lane

- **Đường frame/async (`run_async_*`, tức `--workers`/chương trình có `spawn`-`await`)**: closure
  **ghi** global vẫn lệch — interp frame in `5\n5` (ghi xuyên qua `DongBo::ghi` trong
  `vis-interp/src/frame/worker.rs`), VM in `5\n0`. Đọc thì đã khớp (`0\n0`). Tệp `worker.rs`
  **thuộc lane 319c / bị cấm ở 319a** ⇒ không sửa; đề xuất A0 giao 319c hoặc mở nợ mới.
- Khi hoàn tất: chạy lại `gate --fast` + `vis-diff 100k` sau journal/commit cuối, rồi báo A0
  nghiệm thu (chưa tự tuyên bố `DONE`).
