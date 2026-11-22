<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-301 — Pha 2a: lambda param/return annotation + `T??` + đóng #192/#155

**Nhánh**: `plan/w1-st301-pha2a` từ `main` **`9af3976`** (VD-283). Không merge/push/tag.
`CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/home/ps/tmp`; target riêng
`/home/ps/ViScript/target/vs3-w1-st301`. 0 `unsafe`; R1/R7 giữ.

## 1. Bước 0 — đóng `#192` (P4c VM reactor/cancel)

- `run_async_workers_with_full_hosts_caps` (VM) nhận thêm `FrameReactorBackend`; CLI
  `--vm --workers > 1` truyền `async_backend.clone()` ⇒ main frame async net không còn
  `VS0505` vì thiếu reactor ( #192 phần backend).
- Thêm entry test-only `vm_frame::run_frame_workers_for_test` (frame + `FrameConfig` đầy đủ)
  và ca **`vm_p4b_cancel_worker_qua_trigger_frame`**: trigger `frame_cancel_on_seq` huỷ
  worker task tại safe point — `V:post` không chạy, log có `cancel:1`, exit 1, không treo.
- W=1/2/4 frame parity giữ: `vs3_worker_frame_vm` 7/7, `vs3_worker_frame` 12/12.

## 2. Pha 2a — `#155` lambda annotation

- Parser (`vis-syntax/src/parse/expr.rs`): `at_lambda` nhận `(...)` khi sau `)` là `=>`
  hoặc `-> <type> =>` (scan cân bằng, LL(k) theo VD-283); nhánh `(...)` nhận
  `['->' type]` trước `=>`.
- Lower đã có `lambda_param := IDENT [':' type]` + `HExpr::Lambda.ret`; auto-lift:
  - interp: `Closure.param_depths` (mọi điểm dựng closure) + `opt_lift_maybe` khi bind tham số
    (`interp/goi_ham.rs` và `frame/expr.rs`).
  - VM: `Emit::lift_params_opt(&c.params)` cho thân closure (`codegen.rs`).
- `T??`/`T???`: 2 ca mới trong bảng (let/lambda) + `T??` generic/record/list/map; formatter
  bất động (`fmt(a)==fmt(fmt(a))` trong test).
- Docs: EBNF `docs/ViScript-thiet-ke.md` §A.3 cập nhật `lambda_params ['->' type] '=>' expr`,
  `lambda_param := IDENT [':' type]`; DOC_BLOCKS dịch +1 dòng cho khối sau §A.3.

## 3. Test mới

- `crates/vis-cli/tests/vd283_lambda_annot.rs`: **22 ca bảng** parity interp↔VM + 1 ca formatter
  bất động (2 test function).
- `crates/vis-cli/tests/vs3_worker_frame_vm.rs`: +1 ca cancel worker qua trigger frame
  (**7/7**).
- Tổng gate: **3663 passed / 4 skipped** (trước 3660; +3 test function).

## 4. Bằng chứng

- `./vis-dev gate --fast` ⇒ **PASS 3663/4**; fmt/clippy/R1..R7/self-test/build/doc-examples OK.
  Log `/home/ps/vislogs/st301-gate.log` sha256 `07bc6b17…62adae0d3`.
- Fuzz parser **3600 s**, seed `20261126`:
  `383.008.776 ca / 8.692.738.668 token / 0 vi phạm — PASS`
  (`/home/ps/vislogs/st301-fuzz-3600.log` sha256 `24907c57…16a38`).
- `VIS_DIFF_N=100000` seed `20261115` ⇒ `tong=100000 … **lech=0**` (75 s;
  `/home/ps/vislogs/st301-diff-100k.log` sha256 `4444ccb9…35be3`).

## 5. Trạng thái nợ

- **#192 đóng** (backend + ca cancel worker).
- **#155 đóng** (parser + auto-lift + 22 ca).
- `agent/SO-NO.md` §4/§8 cập nhật: 85 mở / 89 đóng / 17 VS3 / 2 không làm / 1 điều hướng = 194.

## 6. Còn lại

- `ST-302` (Pha 2b) **chưa bắt đầu** trong time-box — nhánh `plan/w1-st302-pha2b` chưa mở.
- `T → T??` tự nâng 2 lớp **không** làm (đúng hợp đồng: auto-lift `T → T?`; gọi `int??`
  với `int` vẫn `VS0412`).

## 7. Rebase lên `main` `7a3bf6b` (v0.4.5 prep + R35)

- `git rebase main` **không xung đột** (branch không chạm `Cargo.toml`/lock/main.rs của đợt version).
- HEAD rebase: `99a91ed` → amend journal thành commit cuối; workspace `version = 0.4.5`.
- `cargo check --workspace` xanh (vis-loader/vis-lsp/vis-vm/vis-cli 0.4.5).
- Gate sau rebase: **PASS 3663/4** (158,9 s), fmt/clippy/R1..R7/doc-examples OK.
  Log `/home/ps/vislogs/st301-gate-v045.log` sha256 `acfc0682…dafb6e`.
