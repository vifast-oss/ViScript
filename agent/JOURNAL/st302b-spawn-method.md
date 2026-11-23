<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-302b — `spawn obj.m(args)` (method `async` trong `impl`)

**Nhánh**: `plan/w1b-st302-spawn` từ **`d3ea467`** (ST-302 Pha 2b, chưa merge).
**Worktree**: `/home/ps/.local/share/opencode/worktree/cd45e5/plan-w1b-st302-spawn`.
Local only — không ssh `mps`/`psvn`, không merge/push/tag. Target riêng
`/home/ps/tmp/w1b-st302-target`; `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`.

## 1. Phạm vi

Mở `spawn obj.m(args)` / `spawn self.m(args)` cho method `async` trong `impl`, giữ nguyên
`spawn f(args)` cấp tệp. Kết quả `await` là `Task[ret]` như spawn hàm; outcome lỗi/huỷ giữ
`VS0506`. Method **đồng bộ** bị từ chối tĩnh với thông điệp nói rõ “phương thức”.

## 2. Thay đổi

| Tệp | Nội dung |
|---|---|
| `crates/vis-syntax/src/parse/expr.rs` | `spawn` + `IDENT`/`self` nay parse **postfix** (Call/GenericCall) thay vì chỉ `IDENT (`; vẫn là soft keyword nên `s.spawn(1)` không đổi. |
| `crates/vis-types/src/lower/expr.rs` | `SpawnExpr` bọc một `Call`: hạ lời gọi (dispatch tĩnh chèn `self`) rồi tách thành `HExpr::Spawn` — interp/VM dùng chung đường `HFn.is_async`/`$spawn` sẵn có. |
| `crates/vis-types/src/tyck/expr.rs` | Method không `async` ⇒ `VS0412` với thông điệp riêng cho phương thức; hàm cấp tệp giữ thông điệp cũ. |
| `crates/vis-cli/tests/vd284_spawn_method.rs` | **12 ca** mới (liệt kê §4). |
| `crates/vis-fmt/tests/moi.rs` | +1 ca snapshot/idempotent cho `spawn p.lay()`. |
| `CHEAT-SHEET-ViScript.md` | Ghi cú pháp `spawn obj.m(args)` + lỗi `VS0412` cho method đồng bộ. |

Interp/VM **không cần sửa mã**: lowering biến lời gọi phương thức thành
`Call(Var(sym impl), [self, …])`, nên `HExpr::Spawn` nhận đúng `sym` đã có trong `fns`/`fmap`
và cờ `is_async` của method.

## 3. Test đỏ trước

10 ca đầu của `vd284_spawn_method.rs` đỏ đúng lý do **trước** khi sửa parser:

```text
thread 'spawn_method_self' panicked at vd284_spawn_method.rs:16:5:
self: parse [Diagnostic { code: VS0201, message: "mong `(`, thật `.`", ... }]
...
test result: FAILED. 0 passed; 10 failed; 0 ignored
```

## 4. Test mới (`crates/vis-cli/tests/vd284_spawn_method.rs`)

| # | Ca | Kỳ vọng |
|---|---|---|
| 1 | `spawn_method_self` | `spawn c.lay()`, `await` ⇒ `7` |
| 2 | `spawn_ham_cap_tep_van_chay` | `spawn f(1)` cũ vẫn chạy ⇒ `2` |
| 3 | `spawn_method_tra_str` | method async trả `str`, tham số `str` ⇒ `an` |
| 4 | `spawn_method_var_self_khong_ghi_nguoc` | `var self` trong task không ghi ngược `c` của main ⇒ `2`, `1` |
| 5 | `spawn_method_co_tham_so` | tham số `int` ⇒ `12` |
| 6 | `spawn_method_tu_trong_method_khac` | `spawn self.lay()` trong method khác ⇒ `9` |
| 7 | `spawn_hai_method_await_lai_tra_cache` | hai task + `await` lần hai trả cache ⇒ `11`, `12`, `11` |
| 8 | `spawn_method_outcome_loi_la_vs0506` | chia 0 trong task ⇒ `VS0506`, parity mã/thông điệp |
| 9 | `spawn_method_dong_bo_bi_tu_choi_ro` | method đồng bộ ⇒ `VS0412` + “phương thức” |
| 10 | `spawn_method_khong_ton_tai_bao_vs0403` | method lạ ⇒ `VS0403` khi hạ |
| 11 | `spawn_method_worker_local_va_parallel_khop` | local + parallel (W=2), interp ↔ VM đều `5` |
| 12 | `spawn_method_cancel_khop_hai_engine` | huỷ giữa chừng ⇒ `VS0506` “huỷ” ở cả hai engine |

`crates/vis-fmt/tests/moi.rs::spawn_phuong_thuc_async` khoá snapshot + idempotent.

## 5. Cổng và đo

- `./vis-dev gate --fast` ⇒ **PASS — 3689 passed / 4 skipped** (3686 + 3 test function mới;
  log `/home/ps/tmp/st302b-gate-final.log`, sha256 `bdea352894ff9b1d4edc88f774fae58a4a49deb52d06506e9aea85d6eadc5408`).
  `check` (fmt/clippy/R1..R7/self-test/build/doc-examples/readme) PASS trong cùng log.
- `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261123 ./bin/vis-diff` ⇒
  `DIFF tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=82.1 toc_do=1217/s` (log
  `/home/ps/tmp/st302b-diff100k.log`, sha256 `b68a440db71bd873d380eab70648e6ef67f3d6e35538839531f8ee448c0042e8`).
- CLI tay trên tệp mẫu: `vis run`, `vis run --vm --no-cache` đều in `1`, exit 0; `vis check`
  TAP `ok 1`; method đồng bộ in `# VS0412 9:13 … (phương thức này không `async`)`, exit 1.

## 6. Giới hạn / chưa làm

- Chưa hỗ trợ `spawn obj?.m(args)` (receiver `Option`) — cùng giới hạn dispatch tĩnh của lời gọi
  phương thức thường; `obj`/`self` cần kiểu tĩnh (annot, `self` trong `impl`, trường có kiểu).
- Không thêm fixture `test/sem/**` để tránh đụng ba bộ đếm khoá cứng; 12 ca Rust đã chạy thật
  interp + VM (kể cả local/parallel/cancel).
- Không chạy Windows/macOS gate (ngoài phạm vi local). Không merge/push/tag.
