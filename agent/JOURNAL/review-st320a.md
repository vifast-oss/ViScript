<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-320a — `var self` + `push`: chốt ngữ nghĩa + khoá parity (`#211`)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Gatekeeper (Agent 1)
- **Nhánh**: `plan/st320a` · **Worktree**: `/home/ps/viswt/st320a`
- **HEAD audit**: **`f133aea6`** — `[ST-320a] var self + push: khoa parity 6 duong + red proof
  is_var_self (#211)`; nền `d1254b05` (parent duy nhất, `git log d1254b05..HEAD` = 1 commit)
- **Merge đích kiểm**: `main` **`ab24770c`** (`[ST-00] R38: sua audit final`)
- **Kết luận**: **PASS** — 0 P1, 0 P2; 2 P3 không chặn. Không sửa mã sản phẩm; chỉ thêm tệp
  nhật ký này.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Phạm vi diff `d1254b05..f133aea6` | **ĐÚNG 2 tệp** — 1 test aggregator (+142) + 1 journal (+85); **0 dòng** `vis-bytecode`/`vis-interp`/`vis-vm`/`vis-runtime` |
| `git status` trước audit | **sạch** (không tracked/untracked lạ) |
| 3 test `st320a_var_self_*` | **3/3 PASS**, mỗi ca chạy **6 đường**, assert stdout literal + `exit = 0` + `diagnostics` rỗng từng đường |
| `st317b_worker_orchestrate` | **35/35 PASS** (32 cũ + 3 mới) |
| Red proof `is_var_self` | **có** — bỏ `st.g.mutables.insert(p.sym)` làm ca 3 **ĐỎ** ở VM; khôi phục ⇒ xanh |
| `./vis-dev gate --fast` | **PASS**, **3937/3937** (4 skip), fmt/clippy/R1..R10/DAG OK |
| `VIS_DIFF_N=100000` | **`lech=0`**, `chay=94425`, seed `3523158054`, 140,0 s |
| clippy `--workspace --all-targets -D warnings` | **PASS** (exit 0) |
| Merge-tree với `ab24770c` | **SẠCH, 0 conflict** — cây gộp `7bcd0283…`, 35 test, giữ cả docs main + test nhánh |
| Lỗi thật | không có |

## 1. Phạm vi, commit

`git status --porcelain` rỗng tại `f133aea6`; `git diff --name-status d1254b05..HEAD`:

```
A  agent/JOURNAL/st320a-var-self-push.md
M  crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs
```

`git diff --stat=200 d1254b05..HEAD`: 2 tệp, **227 insertions(+), 0 deletions**. Kiểm máy:
`git diff d1254b05..HEAD -- crates/vis-bytecode crates/vis-interp crates/vis-vm crates/vis-runtime`
= **0 dòng** ⇒ đúng lane "chỉ test + journal", không chạm mã sản phẩm/VBC1/cache.

Test nằm trong aggregator đã có: `crates/vis-conformance/tests/lib_api_it.rs:99-100`
(`#[path = "async_api_it/st317b_worker_orchestrate.rs"] mod ...`), **không** tạo tệp `.rs` cấp 1
mới. R8: 39/40 tệp cấp 1 (`crates/*/tests` 38 + `vis-host-linux/tests` 1), `vis-cli/tests` 8/15 —
giữ nguyên trước/sau (`git ls-tree` nền và HEAD đều 39).

## 2. Ba test mới (`st317b_worker_orchestrate.rs:1051-1191`)

Helper `sau_duong_st320a` chạy **6 đường** cho mỗi ca:

1. `vis_interp::run` (interp mặc định, non-frame);
2. `vis_vm::run` (VM mặc định);
3. `run_workers_local` (interp W=1 non-frame);
4. `run_workers_local_vm` (VM W=1 non-frame);
5. `frame_1` = `vis_interp::run_async_with_hosts_caps` (frame W=1 — đường `vis run` async);
6. `frame_vm_1` = `vis_vm::run_async_with_full_hosts_caps` (frame VM W=1).

Mỗi đường assert **stdout literal**, `exit == 0`, `diagnostics.is_empty()`; thêm `so_khop` với
interp mặc định (stdout/exit/codes/giá trị cuối `main`). Không `#[ignore]`.

| Ca | Kỳ vọng mọi đường |
|---|---|
| `st320a_var_self_push_truong_parity` | `[1, 2]\n[1]\n[1]\n[1, 2]\n[1]\n` — `self.xs.push` không rebind field |
| `st320a_var_self_gan_truong_cuc_bo` | `[1, 2]\n[1, 2]\n[1]\n` — gán tường minh `self.xs = …` chỉ đổi cục bộ |
| `st320a_var_self_la_list_push_ghi_o` | `[1, 2]\n[1, 2]\n2\n[1]\n` — nhánh `(Var, "push")` + `mutables.is_var_self` |

Đọc code xác nhận khớp SPEC §4.7/§4.8 + VD-51: receiver `Field` (`self.xs`) là biểu thức tạm,
không ghi lại; receiver `Var` trỏ `var`/`var self` mới ghi ô (CoW). Interp là oracle, VM đã khớp
từ trước `ST-319d`; test chỉ khoá parity, không đổi ngữ nghĩa.

## 3. Red proof

Bằng chứng `/home/ps/vislogs/st320a-redproof.log` (sha256
`b11943a4f50ecbf6a857152522dea20b5fb7fa5adbd7bb7d7b0e91f455bcbf62` khớp journal builder):

```
test st317b_worker_orchestrate::st320a_var_self_la_list_push_ghi_o ... FAILED
assertion `left == right` failed: var self list push/VM: stdout
  left: "[1, 2]\n[1, 2]\n2\n[1]\n"   (interp, đúng)
 right: "[1]\n[1, 2]\n1\n[1]\n"      (VM khi bỏ is_var_self)
test result: FAILED. 0 passed; 1 failed
```

Đúng dấu hiệu bỏ `st.g.mutables.insert(p.sym)` tại `crates/vis-bytecode/src/codegen.rs:324`:
VM không ghi ô `self` ⇒ `self` vẫn `[1]`/`len = 1`. Sau khôi phục, `st320a-test5.log` **3/3 PASS**.
Trạng thái hiện tại: `codegen.rs` khớp blob HEAD `734aac36…`, `sha256 = ed77d472…`, trùng
`/home/ps/vislogs/st320a-codegen.rs.bak` (bản gốc) ⇒ **không còn dấu vết sửa**.

## 4. Cổng Gatekeeper chạy lại (worktree này, toolchain 1.100.0)

Lưu ý môi trường: `PATH` mặc định trỏ `/usr/bin/cargo` (1.93.1) ⇒ Cargo từ chối
`rust-version = 1.100.0`; phải `export PATH=$HOME/.cargo/bin:$PATH` (rustup 1.100.0) như
`bin/vis-dev` tự chọn. `CARGO_BUILD_JOBS=2`, `TMPDIR=/home/ps/vislogs/tmp-review-st320a`
(đĩa), **không** `cargo clean`; đĩa `/home` 16 GB (> 6 GB) trước và sau.

| Lệnh | Kết quả | Log (sha256) |
|---|---|---|
| `cargo test -p vis-conformance --test lib_api_it st317b_worker_orchestrate` | **35/35 PASS**, 0 failed/0 ignored (399 filtered) | `review-st320a-st317b.log` — `ed7f9525…` |
| `./vis-dev gate --fast` | **PASS** — **3937/3937**, 4 skipped, 79,6 s; fmt/clippy/R1..R10/self-test/DAG OK; `GATE_EXIT=0` | `review-st320a-gate.log` — `5f3acfa1…` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — tổng 100000, `loi_tinh=5575`, `chay=94425`, 140,0 s, seed `3523158054` (trùng seed builder) | `review-st320a-diff100k.log` — `a7034477…` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** (exit 0, 0,23 s — cache ấm; gate cũng đã chạy) | `review-st320a-clippy.log` — `f2066ca2…` |

## 5. Merge-tree với `main ab24770c`

`git merge-tree --write-tree --messages ab24770c HEAD` ⇒ exit 0, **không conflict**, cây gộp
`7bcd028338f1d064916652901600afca32c45679`.

- `git diff --stat ab24770c..<cây gộp>` = **đúng 2 tệp của nhánh** (test +142, journal +85);
- `git diff --stat HEAD..<cây gộp>` = **đúng 9 tệp docs của main** (ACTIVE, DECISIONS/INDEX,
  JOURNAL A0, QUEUE, ROADMAP, STATE, R38 md/json);
- tệp test sau gộp: **35 `#[test]`** (32 của `ab24770c` + 3 `st320a_var_self_*`), 0 marker
  conflict.

Không có conflict `STATE.md`/test như dự phòng: nhánh **không chạm** tệp docs nào, main **không
chạm** test `st317b`, nên phép gộp thuần cộng — giữ nguyên cả hai phía, không cần resolve tay.

## 6. P3 (không chặn)

1. `st320a-redproof.log` chỉ giữ output test đỏ, **không** kèm `git diff` của dòng tạm bỏ; chứng
   minh dựa vào journal + dấu hiệu output. Đề nghị các red proof sau `tee` thêm diff vào log.
2. Helper `sau_duong_st320a` trả `RunOutput` nhưng cả 3 caller bỏ qua; có thể đổi `-> ()` cho gọn
   (không ảnh hưởng ngữ nghĩa/cổng).

## 7. Kết luận

**PASS** — lane đúng "chỉ test + journal", 0 mã sản phẩm; 3 ca khoá đủ 6 đường và nhánh
`is_var_self` có red proof thật; 35/35 + gate 3937/3937 + diff 100k lệch 0 + clippy sạch; merge
với `main ab24770c` sạch. Đủ điều kiện merge; giữ cả docs main lẫn test nhánh.
