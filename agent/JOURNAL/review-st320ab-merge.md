# REVIEW — Sóng 9: cây gộp ST-320a + ST-320b/F2 (addendum bắt buộc)

- **Ngày**: 2026-11-27T01:20Z · **Vai**: Builder/A0 integrator (ghi nhật ký audit) · **Repo**: `/home/ps/ViScript`, worktree `/home/ps/viswt/st320b`.
- **Addendum cho**: merge plan `/home/ps/vislogs/merge-plan-320-F1-20261126T2217Z.md` + runbook `/home/ps/vislogs/post-peak-runbook.md`.
- **Kết luận**: **PASS** — cây gộp `2ef50717f4f29e4677522e6e63b1395dd89496ee` (trước addendum): **38 test / 1342 dòng / blob `61a82fd7b0ca342828e334f32bdeafd17da76dde`**, gate **3940/3940 (4 skip)**, diff 100k `lech=0`, clippy 0.

> **HIỆU CHỈNH so với plan/runbook (bắt buộc ghi rõ)**: plan lập lúc 22:17Z chốt 37 test / 1307 dòng / blob `d04197b2…` / gate 3939 khi tip 320b là `8b164d48` (F1). Sau đó nhánh tiến thêm `a02f3443` (test `w27` đỏ) + `985b0883` (F2 vá HOF-write, `w27` xanh). Vì luật resolve là **giữ nguyên văn cả hai block**, block 320b hiện gồm `w25`+`w26`+`w27` ⇒union đúng là **38 test / 1342 dòng / blob `61a82fd7…` / gate 3940** (= 3934 + 3 + 3). Không được bỏ `w27` để khớp hash cũ (mất regression của F2). Các hash `d04197b2`/`87d7f93a` là **lỗi thời**.

## 1. Tip audit + worktree sạch

| Mục | Hash |
|---|---|
| `main` trước merge 320a (nền chain) | `ff585af8aad8156e30b288b93f2de321305f83ea` (= `origin/main`, dirty 0) |
| `main` sau merge `--no-ff plan/st320a` | `bbbb99bee7226dda96d82c3311e7fad7e881c9f9` |
| `plan/st320a` | `2f60d02371c54b8b841e17350cea55a6d03a3664` (PASS, test-only, 35 test, gate 3937) |
| 320b trước tích hợp | `985b0883ee4849f86f9981c217d01b79d9099b50` (`[ST-320b] F2: HOF callback ghi global qua DongBo`) |
| Tip tích hợp (citation của addendum này) | `2ef50717f4f29e4677522e6e63b1395dd89496ee` |
| Merge-base chung | `d1254b05bb2a1ecc04116160af07be323422c3f2` |

`git status --porcelain` rỗng ở cả 3 worktree trước và sau các bước; `cargo fmt --all` không sinh đổi đổi (`exit=0`, status vẫn rỗng). Mã `main` đóng băng: `git diff --stat d1254b05..main -- crates/ bin/ test/` rỗng.

## 2. Merge-tree — đúng 1 conflict

| Phép | Kết quả |
|---|---|
| `merge-tree main plan/st320a` | **sạch** (`a2769105…`, EXIT=0) |
| `merge-tree main 985b0883` | **EXIT=1**, `CONFLICT (content)` đúng 1 file: `crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs` |
| `merge-tree 985b0883 plan/st320a` | **EXIT=1**, cùng 1 file (append/append ở EOF sau dòng 1049) |
| `merge-tree main <tip tích hợp 2ef50717>` | **sạch** (main là tổ tiên trực tiếp) |

Thực tế `git merge --no-ff main` trong `st320b` báo đúng 1 conflict, `git diff --name-only --diff-filter=U` chỉ trả tệp test đó. Không conflict `STATE.md`/`QUEUE.md`/`ACTIVE.md`/journal/mã sản phẩm.

## 3. Blob cây gộp — 38 test, không trùng

Tái tạo tất định `base(1..1049) + block 320a(1050..1191) + block 320b(1192..1342)`:

- `git hash-object` = **`61a82fd7b0ca342828e334f32bdeafd17da76dde`** (khớp dự tính read-only trước merge).
- `grep -c '^#\[test\]'` = **38**; `wc -l` = **1342**; tên test trùng = **0**; `git diff --check` sạch.
- Hash đoạn: base `f06fa05a…` (khớp), 320a `8c26c0d6…` (khớp), 320b `fa8d12e1…` (khác `6c886862…` vì thêm `w27`, **đúng**).
- 6 test mới, thứ tự cuối tệp: `st320a_var_self_push_truong_parity`, `st320a_var_self_gan_truong_cuc_bo`, `st320a_var_self_la_list_push_ghi_o`, `w25_frame_w1_ham_ten_qua_gia_tri_trong_closure`, `w26_frame_w1_const_ham_ten_qua_hof`, `w27_frame_w1_hof_callback_ghi_global`. Không đổi tên, không `#[ignore]`, không tạo binary test mới (R8 giữ nguyên).

## 4. Phạm vi task diff

`git diff --stat d1254b05..HEAD -- crates/ + 4 journal task` = **9 tệp, +1096/−7** (plan cũ +986/−7 cộng 110 dòng của `w27`+F2):

- test `st317b`: **+293** (142 của 320a + 151 của 320b gồm `w25`/`w26`/`w27`) → 1342 dòng / 38 test.
- 4 tệp `vis-interp`: `expr.rs` +30/−7, `params.rs` +3, `interp.rs` +39, `worker.rs` +46 (F2: sync `sym` cấp tệp từ globals vào `frame.cur().env` sau callback HOF + `DongBo::ghi`).
- 4 journal task: `review-st320a` +126, `st320a-var-self-push` +85, `review-st320b` +280, `st320b-frame-ham-ten-closure` +201.
- Docs ngoài `agent/JOURNAL/` **y hệt `main`** (`diff <(…main…) <(…HEAD…)` → `SAME`). Không sửa QUEUE/STATE/ACTIVE/ROADMAP/SO-NO trong merge này.
- `git diff --name-only d1254b05..HEAD -- crates/vis-vm crates/vis-bytecode crates/vis-loader crates/vis-cache docs/SPEC-lex.md SPEC.md` → **rỗng** ⇒ không đổi VM/VBC1/cache/SPEC.

## 5. Cổng trên cây gộp (log `/home/ps/vislogs/final-main-song9-*-20261127T0115Z.log`)

| Bước | Kết quả |
|---|---|
| `cargo fmt --all` | `exit=0`, không đổi tệp |
| `cargo test -p vis-conformance --test lib_api_it st317b_worker_orchestrate` | **38 passed, 0 failed, 0 ignored** |
| `cargo test … st320a` | **3/3** |
| `cargo test … w25` / `w26` / `w27` | **1/1 / 1/1 / 1/1** (`w27` xanh ⇒ F2 hợp lệ) |
| `./vis-dev gate --fast` | **Summary 3940 tests run: 3940 passed, 4 skipped**, `GATE_EXIT=0` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | `lech=0`, `chay=94425`, `loi_tinh=5575`, seed `3523158054`, `exit=0` |
| `cargo clippy --workspace --all-targets -- -D warnings` | `exit=0`, 0 warning |
| `git diff --check` | sạch |

Chẩn đoán số: 3940 = 3934 (nền) + 3 (320a) + 3 (320b: `w25`,`w26`,`w27`). Gate 3939/3938/3937/3936 trong plan/runbook là mốc **cũ trước `w27`**; nếu ra các số đó nghĩa là rơi `w27`/block 320b.

Môi trường: `PATH=$HOME/.cargo/bin`, `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/vislogs/tmp-final-song9` (đĩa), **không** `cargo clean`. Đĩa có lúc tụt 5.5 G sau `vis-diff` ⇒ đã chạy `bin/vis-gc-targets --apply` (xoá `st320a/target`, +3.9 G; `st321a` đang build nên được SKIP đúng), về 8.1 G trước khi tiếp tục.

## 6. R1 / R8 / VBC1 / unsafe

- **R1**: `interp.rs` 588, `expr.rs` 906, `params.rs` 86, `worker.rs` 820 — tất cả < 1000 dòng (tệp test 1342 dòng **không** tính sản phẩm, vẫn < trần 1500).
- **R8**: `find crates/*/tests vis-host-linux/tests -maxdepth 1 -name '*.rs'` = **39/40**; không tạo tệp `.rs` cấp 1 mới (`w27` nằm trong aggregator `lib_api_it.rs` có sẵn).
- **VBC1/cache**: `PHIEN_BAN`/`VERSION` không đổi; API-VBC1/STD/CODES không đổi. Không bump (interp-only, tiền lệ `ST-319e`).
- **R2**: `git diff d1254b05..HEAD -- crates/ | grep -c '^+.*unsafe'` = **0**.

## 7. Nợ mở (không chặn đóng Sóng 9)

- **P2-2** (có thật, không do F2): free HOF ghi global trong frame — frame interp `[3],3,0,0` vs VM `[3],3,0,3`; hướng đóng: đánh `Dirty` sym top-level sau callback builtin. **Chưa đăng ký sổ**, không chặn `#212`.
- **P3-1** `name.is_empty()`, **P3-2** thiếu test closure W>1 read-only — ghi proposal, chưa cấp số.
- Chain `peak1-20261127T0100Z`: 16/18 PASS; **2 FAIL ở remote gate** (`psvn-gate-full` exit=101, `mps-gate-full` exit=1) ⇒ **không tuyên bố 3-OS**, `ST-316c` giữ `BLOCKED`. 4 bước `ps` xanh: gate, diff `lech=0`, clippy, targets-wasm.
- Không áp 2 patch docs P3/#195 (việc của agent khác); không sửa sổ nợ.

## 8. Kết luận

**PASS** với hash cây gộp `2ef50717f4f29e4677522e6e63b1395dd89496ee` (trước commit addendum này). Blob test/code bất biến: `61a82fd7…`; merge-tree với `main` vẫn sạch. Đủ điều kiện để A0 `git merge --no-ff plan/st320b` vào `main`, gate/diff cuối, rồi push `origin main`.
