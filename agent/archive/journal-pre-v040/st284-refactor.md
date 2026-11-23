<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-284 (VD-272) — R0a: tách `vm/host.rs` & `frame/worker.rs` (0 đổi hành vi)

**Trạng thái**: DONE (chờ Gatekeeper nghiệm thu độc lập).
**Nhánh**: `plan/w1-runtime`, nền `8f50af1` (`7a5a85e` merge main; giữ nguyên commit journal fuzz cũ).
**Phạm vi**: chỉ **di chuyển mã**; không đổi bytecode/ABI/cache/ngưỡng cổng; không thêm `unsafe`.

## 1. Số dòng trước → sau

| Tệp | Trước | Sau | Ngưỡng ST-284 |
|---|---:|---:|---:|
| `crates/vis-vm/src/vm.rs` | 1.500 | **1.240** | ≤ 1.250 |
| `crates/vis-interp/src/frame.rs` | 1.494 | **1.042** | ≤ 1.250 |

Tệp mới (cùng nội dung đã chuyển, không sửa logic):

| Tệp | Dòng | Nội dung |
|---|---:|---|
| `crates/vis-vm/src/vm/host.rs` | 205 | `run_with_fs`/`run_with_hosts`/`run_with_hosts_caps`/`run_with_full_hosts_caps`, 4 setter host, `impl Host for Vm` |
| `crates/vis-vm/src/vm/tests.rs` | 76 | nguyên `mod tests` cũ (cùng module path `vis_vm::vm::tests`) |
| `crates/vis-interp/src/frame/worker.rs` | 464 | `run_frame_driver` + `frame_output_early` + `build_task_frame` + `step_frame` |

`frame_output_early`/`build_task_frame`/`step_frame` chỉ được driver gọi ⇒ chuyển kèm để
không sinh `dead_code` (clippy `-D warnings`), vẫn là di chuyển nguyên văn.

## 2. Đường API giữ nguyên

- `vm.rs` khai báo `mod host;` và `pub use host::{run_with_fs, run_with_full_hosts_caps, run_with_hosts, run_with_hosts_caps};`
  ⇒ `vis_vm::run_with_*` và `vis_vm::vm::run_with_*` không đổi.
- `frame.rs` khai báo `mod worker;` + `pub(crate) use worker::run_frame_driver;`
  ⇒ `vis_interp::lib.rs` vẫn gọi `frame::run_frame_driver` y nguyên.
- `vm/tests.rs` giữ `use super::*` nên vẫn chạm helper private như cũ; tên test không đổi.

## 3. Bằng chứng 0 đổi hành vi

1. **So khớp nguyên văn**: từng khối chuyển được so với `git show HEAD:<tệp>` bằng kiểm
   substring — tất cả **EXACT** (`host:run_with_*`, `host:setters+Host`, `worker:driver`,
   `worker:early`, `worker:build`, `worker:step`, `tests:body`). Chỉ bỏ thụt lề 4 dấu của
   thân `mod tests` khi thành tệp riêng.
2. `./vis-dev gate --fast` ⇒ **PASS** (exit 0): fmt OK, clippy OK, luật R1..R6 OK,
   self-test case âm OK, build OK, doc-examples OK (24 khối), L0 **3571/3571 test PASS**
   (log `agent/JOURNAL/st284-gate-fast.log`).
3. `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` ⇒
   `DIFF tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=158.4`
   (log `agent/JOURNAL/st284-diff-100k.log`).
4. Không đụng `crates/vis-bytecode`, `PHIEN_BAN`, cache, golden, `API-CODES.txt`; `git diff`
   chỉ có 2 tệp sửa + 3 tệp mới.

## 4. Lệnh tái lập

```bash
export CARGO_TARGET_DIR=$PWD/target        # target riêng của worktree
./vis-dev gate --fast
VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff
wc -l crates/vis-vm/src/vm.rs crates/vis-interp/src/frame.rs
```
