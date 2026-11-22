<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-285 (VD-272) — R0b: tách `show_eq`, `cau`, call branch (0 đổi hành vi)

**Trạng thái**: DONE (chờ Gatekeeper nghiệm thu độc lập).
**Nhánh**: `plan/w1-runtime` (đã merge `main` @ `84915aa`, version 0.3.6).
**Phạm vi**: chỉ **di chuyển mã**; không đổi golden/bytecode/ABI/cache/API công khai; không `unsafe`.

## 1. Số dòng trước → sau

| Tệp lõi | Trước | Sau | Ngưỡng ST-285 |
|---|---:|---:|---:|
| `crates/vis-interp/src/value.rs` | 1.500 | **1.186** | ≤ 1.250 |
| `crates/vis-bytecode/src/codegen.rs` | 1.499 | **1.045** | ≤ 1.250 |
| `crates/vis-types/src/tyck/expr.rs` | 1.486 | **1.064** | ≤ 1.250 |
| `crates/vis-interp/src/interp/goi.rs` | 1.479 | **1.155** | ≤ 1.250 |

Cả 6 tệp lõi của R0a/R0b nay ≤ 1.250: `vm.rs` 1240, `frame.rs` 1042, `value.rs` 1186,
`codegen.rs` 1045, `tyck/expr.rs` 1064, `interp/goi.rs` 1155.

Tệp mới (nội dung chuyển nguyên văn):

| Tệp | Dòng | Nội dung |
|---|---:|---|
| `crates/vis-interp/src/value/show_eq.rs` | 326 | `Value::show`, `eq_value`, `cmp_value` + helper băm/so map (`khoa_vo_huong`, `bam_*`, `ChiMucKhoa`, `gop_vo_huong`, `eq_map_*`, `show_map_key`) |
| `crates/vis-bytecode/src/codegen/cau.rs` | 464 | `Emit::cau` (phát lệnh câu lệnh) + `khoi`/`cond`/`slot_cua`/`net_stack`/`trap_no_arm` |
| `crates/vis-types/src/tyck/expr/goi.rs` | 437 | `Checker::kiem_call`, `kiem_args`, `kiem_method` |
| `crates/vis-interp/src/interp/goi_ham.rs` | 333 | `Interp::eval_call`, `eval_args`, `call_value`, `call_closure`/`call_closure_trong`, `call_hfn`/`call_hfn_trong` |

## 2. Đường API / module giữ nguyên

- `value.rs`: `mod show_eq;` (mô-đun con của `value`) — `Value::show`/`eq_value`/`cmp_value`
  vẫn là phương thức `pub` như cũ; helper private của cha (`show_float`, `show_size`, …) vẫn
  được gọi chéo vì `show_eq` là con của `value`.
- `codegen.rs`: `mod cau;` — 6 phương thức chuyển thành `pub(super)` (trước là `fn` private
  trong `impl Emit` ở `codegen`), nên `Emit` ở tệp cha và các mô-đun `than`/`pattern`/`gan_long`/
  `gia_tri`/`if_expr` gọi như cũ.
- `tyck/expr.rs`: `mod goi;` — `kiem_call` `pub(super)`, `kiem_args`/`kiem_method` `pub(crate)`
  (trước `pub(super)`; cần `pub(crate)` vì `tyck/tuy_chon.rs` gọi trực tiếp). Không đổi chữ ký.
- `interp/goi.rs`: `mod goi_ham;` — các phương thức giữ nguyên `pub(crate)`, riêng
  `call_closure_trong`/`call_hfn_trong` vẫn private trong cùng mô-đun con. `exec_for` và phần
  còn lại vẫn ở `goi.rs`.

## 3. Bằng chứng 0 đổi hành vi

1. **So khớp nguyên văn**: từng khối chuyển so với `git show HEAD:<tệp>` — **EXACT** sau khi
   chuẩn hoá đúng các từ khoá visibility nêu trên (`show/eq methods`, `map-eq helpers`,
   `cau+helpers`, `tyck call branch`, `interp eval_call`, `interp call rest`).
2. `./vis-dev gate --fast` ⇒ **PASS** (exit 0): fmt/clippy/R1..R6/self-test/build/doc-examples OK;
   L0 **3576 passed / 3 skipped** (đúng kỳ vọng; log `agent/JOURNAL/st285-gate-fast.log`).
3. `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` ⇒
   `DIFF tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=200.5`
   (log `agent/JOURNAL/st285-diff-100k.log`).
4. Không đụng `format.rs`/`VERSION`/`PHIEN_BAN`/cache/golden/`API-CODES.txt`; không thêm `unsafe`.

## 4. Lệnh tái lập

```bash
export CARGO_TARGET_DIR=$PWD/target        # target riêng của worktree
./vis-dev gate --fast
VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff
wc -l crates/vis-interp/src/value.rs crates/vis-bytecode/src/codegen.rs \
      crates/vis-types/src/tyck/expr.rs crates/vis-interp/src/interp/goi.rs
```
