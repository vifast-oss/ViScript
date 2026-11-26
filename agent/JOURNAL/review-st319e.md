<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-319e — Gatekeeper độc lập: hàm tên trong closure dùng **env module** (`#210`)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Gatekeeper (Agent 1), **không sửa mã**
- **Worktree**: `/home/ps/viswt/st319e` · **Nhánh**: `plan/st319e`
- **HEAD audit**: `189ccc76` (journal-only trên `327247ea`) · **Nền**: `main` @ `07a5e2b3`
- **`main` lúc audit**: `d2b2b69a` (`[ST-00] docs: dong #209 + 319c2 DONE`), tức đã mới hơn
  `e4c9f6f9` như brief dự kiến.
- **Kết luận**: **PASS** — 0 P1; 3 ghi nhận P2 không chặn (§7).

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Phạm vi mã | 5 tệp, **+175/−4** (interp + test conformance + generator `diff_100k`) |
| Commit | `327247ea` (mã/test) + `189ccc76` (journal-only; `git diff 327247ea..HEAD -- crates/` = **0 dòng**) |
| Không chạm | `vis-interp/src/frame/**`, `vis-vm/src/**`, `vis-bytecode/src/**`, SPEC, VBC1 |
| Test mới A/B/C2/E/P5 | **5/5 xanh**, không `#[ignore]`, 4 đường khớp; literal `1`, `9`, `1`, `0\n1`, `1` |
| `cargo test -p vis-interp -p vis-conformance` | **PASS** (lib_api_it 416 + 3 ignored; interp lib 55; 0 fail) |
| `./vis-dev gate --fast` | **PASS** — nextest **3922/3922** (4 skip) + doc-test |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** (`tong=100000`, `chay=94425`, 144,8 s) |
| clippy `--workspace --all-targets -D warnings` | **exit 0** (gate cũng báo `clippy: OK`) |
| Hợp nhất `main` `d2b2b69a` | `merge-tree --write-tree` **sạch**, tree `e76155ca`, **0 xung đột** |
| Journal Builder | 8 log + SHA-256 trong journal **khớp thật** từng byte (§6) |

## 1. Phạm vi, trạng thái, commit

- `git status --porcelain` **sạch** tại `189ccc76` trước khi ghi journal này.
- `git log --oneline 07a5e2b3..HEAD` = **2 commit**:
  1. `327247ea [ST-319e] #210: hàm tên gọi từ closure dùng env module (interp non-frame)`
  2. `189ccc76 [ST-319e] journal: bang chung #210 (env module cho ham ten trong closure)`
- `git diff --stat 07a5e2b3..189ccc76`: **6 tệp, +328/−4**; trong đó **journal +153** và
  **mã/test +175/−4** đúng brief:
  `interp.rs` (+12/−1), `interp/eval.rs` (+5/−2), `interp/goi_ham.rs` (+34),
  `misc_api_it/st319a_closure_cap_tep.rs` (+81), `vis-vm/tests/diff_100k.rs` (+47/−1).
- `git diff --check` exit **0**; không có tệp ngoài phạm vi; không sửa `agent/QUEUE.md`,
  `STATE.md`, `DECISIONS.md`, SPEC.
- R8: ca mới nằm trong tệp con `tests/misc_api_it/st319a_closure_cap_tep.rs` đã có từ ST-319a;
  không thêm tệp `.rs` cấp 1, không đổi aggregator.
- `327247ea..189ccc76` chỉ thêm `agent/JOURNAL/st319e-closure-ham-ten.md`; **mã y hệt** commit
  được brief nêu. Mọi kết luận dưới đây áp cho HEAD cuối `189ccc76`.

## 2. Kiểm logic

| Điểm | Xác nhận |
|---|---|
| Khởi tạo `env_module` | `Interp::new` tạo **một** `Env`, gán `env` và `env_module` **cùng `Rc`**, `closure_dong = 0` (`interp.rs:148-159`). |
| `closure_dong` | Tăng ngay trước khi đổi `self.env = c.env`; giảm sau khi khôi phục `self.env = saved_env` (`goi_ham.rs:179,204`). Kiểm arity nằm **trước** tăng nên không âm; mọi nhánh thân closure (Ok/Ret/trap) đều đi qua cặp tăng/giảm. |
| Đồng bộ env module | Chỉ closure **ngoài cùng** (`closure_dong == 0`) đặt `env_module = saved_env`; closure lồng không ghi đè — đúng: lúc đó `saved_env` là env closure ngoài, không phải module. |
| `call_hfn_trong` (depth > 0) | `std::mem::replace(&mut self.env, self.env_module.clone())`, gọi `call_hfn_than`, rồi khôi phục `self.env` **sau khi đã giữ `kq`** ⇒ phủ `Ok`, `Abort::Ret`, trap/abort. `?` trong thân không thoát khỏi wrapper. |
| `call_hfn_trong` (depth 0) | Chỉ `env_module = self.env` — non-frame `self.env` là env cấp tệp (interp phẳng, không tạo env con); frame tự gán `it.env = globals`. |
| `HExpr::Var` hàm tên | `Value::Fn` chụp `self.env_module` (`eval.rs:103`) ⇒ `let h = tang` trong closure gọi `h()` chạy trên module. `HExpr::Lambda` **vẫn** snapshot `self.env.borrow().clone()` (`eval.rs:423-431`) — không đổi ST-319a/SPEC §4.7. |
| Đường W>1 (`tao_interp`) | `Interp::new` tạo `env_module` **cùng Rc** với `env`; snapshot top-level được `insert` vào chính map đó ⇒ `env_module` thấy global. Kiểm thật: task đọc `let h = doc; h()` in `9` ở W=2 (interp + VM); task ghi global bị `VS0509` như thiết kế W>1. |
| Frame | Không sửa mã. `frame/expr.rs` có nhánh `HExpr::Var` riêng (chụp `frame.cur().env`), không đi qua `eval.rs`; `HExpr::Lambda` frame dùng `Work::MakeLambda`. Rủi ro tiềm ẩn ở §7 P2-2. |
| #206/#209 | 5 ca ST-319a cũ vẫn xanh trong cùng binary; toàn bộ nextest (gồm frame/ST-318/319c2 trên nền cũ) PASS; merge-tree với `main` mới **sạch**. |

**Đối chứng thêm ngoài 5 ca** (interp vs `--vm`, đều khớp): closure lồng gọi hàm tên (`1`), hàm
tên `return` giữa thân rồi closure đọc snapshot (`0/7/1`), hàm tên gán ở top-level rồi gọi qua
closure (`1`), callback `xs.map(fn(x): tang(x))` (`6`), W=2 đọc global qua giá trị hàm tên (`9`).

## 3. Test A/B/C2/E/P5 (red-first → xanh)

- **Red-first thật** (`/home/ps/vislogs/st319e-red.log`): đúng 5 ca mới đỏ, 5 ca ST-319a cũ xanh;
  failure là `I↔V: stdout` với `left` = interp, `right` = VM:
  A `0` vs `1`; B `7` vs `9`; C2 `0` vs `1`; E `1\n0` vs `0\n1`; P5 `5` vs `1`.
- **Xanh** (`st319e-green.log`): `10 passed; 0 failed; 0 ignored`.
- **4 đường** (`st319e-green4.log`, `VIS_319E_DEBUG=1`): A `1`; B `9`; C2 `1`; E `0\n1`; P5 `1`
  cho cả `I / V / I-W1 / V-W1`.
- **Audit chạy lại** (`review-st319e-test.log`): `lib_api_it` **416 passed / 3 ignored / 0 failed**;
  10 test `st319a_closure_cap_tep` đều xanh, gồm đúng 5 tên mới; không ca nào `ignored`.
  `vis-interp` lib 55/55; doc-test 2/2.
- **Literal độc lập** bằng binary `target/debug/vis run` (interp mặc định, W=1):
  A=`1`, B=`9`, C2=`1`, E=`0|1`, P5=`1` — khớp bảng journal Builder.
- Generator `ClosureCapTep`: `duoi(4)` → `duoi(8)`, thêm mẫu 4/5/6 + nhánh `_` cho A/B/C2/E;
  mẫu cũ 0..3 giữ nguyên. `diff_100k` 100k lượt `lech=0` chứng minh mẫu mới chạy thật.

## 4. Cổng máy (độc lập)

| Cổng | Kết quả | Log |
|---|---|---|
| `cargo test -p vis-interp -p vis-conformance` | **PASS**, 0 fail, 0 test mới bị ignore | `/home/ps/vislogs/review-st319e-test.log` |
| `./vis-dev gate --fast` | **PASS** — fmt/clippy/R1..R10/DAG/self-test OK; nextest **3922/3922** (4 skip) + doc-test | `/home/ps/vislogs/review-st319e-gate.log` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`**: `tong=100000 chay=94425 giay=144.8`; lượt 2000 đầu cũng `lech=0` | `/home/ps/vislogs/review-st319e-diff.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** (fingerprint ấm; gate cũng chạy `clippy: OK`) | `/home/ps/vislogs/review-st319e-clippy.log` |

Tái lập: `cargo` hệ thống là **1.93.1** nên bị `rust-version = 1.100.0` từ chối; phải dùng
`PATH=$HOME/.cargo/bin:$PATH` (rustup 1.100.0 theo `rust-toolchain.toml`), `CARGO_BUILD_JOBS=2`,
`TMPDIR=/home/ps/vislogs/tmp-st319e` trên đĩa, không `cargo clean`.
Đĩa lúc bắt đầu audit chỉ còn 4,8 GB (< ngưỡng 6 GB); một `target/` worktree anh em được dọn
trong lúc audit, đĩa về **16 GB** trước khi chạy cổng — không phải dọn target của nhánh này.

## 5. Hợp nhất `main`

- `git merge-base main HEAD` = `07a5e2b3`; `main` hiện tại `d2b2b69a` (đã gồm #209/ST-319c2).
- `git merge-tree --write-tree --messages main HEAD` → tree **`e76155ca818e5c906866c78da73e42ab684391fe`**,
  **không thông điệp xung đột**, exit 0.
- `git diff --name-status main <tree>`: `A agent/JOURNAL/st319e-closure-ham-ten.md` + 5 tệp `M`
  đúng phạm vi ST-319e (`st319a_closure_cap_tep.rs`, `interp.rs`, `eval.rs`, `goi_ham.rs`,
  `diff_100k.rs`). **Không** đụng `STATE.md`/`QUEUE.md`; journal review này thêm một `A` nữa khi
  merge.

## 6. Đối chiếu journal Builder (log + SHA-256)

Đã `sha256sum` lại **8/8** log ngoài repo; khớp **byte-for-byte** bảng §7 journal:

| Log | Bytes | SHA-256 (khớp) |
|---|---|---|
| `st319e-red.log` | 4374 | `41e67e43…487415` |
| `st319e-green.log` | 1220 | `d51ad9bb…2f36f7` |
| `st319e-green4.log` | 1808 | `baf99537…18f6cf` |
| `st319e-gate-fast.log` | 417164 | `c9868a71…49b90d` |
| `st319e-diff100k.log` | 266 | `100d438e…a66f18` |
| `st319e-clippy.log` | 3619 | `843ea329…1e589` |
| `st319e-test-final2.log` | 32317 | `a815a8d3…bd1576` |
| `st319e-flake-thread.log` | 376 | `dc187490…1180e6` |

Nội dung journal khớp mã/HEAD: root cause, bảng trước/sau, 5 ca, 4 mẫu generator, cổng; ghi chú
flake `vs3_worker_eval_vm::vm_khong_tao_them_thread_sau_khi_join` là có sẵn từ nền (test đếm
thread trong process song song), chạy riêng + chạy lại toàn bộ đều xanh — không do vá.

## 7. Rủi ro

**P1: không có.** Không tìm thấy đường nào làm sai ngữ nghĩa đang được cam kết (closure snapshot;
hàm tên đọc/ghi global live; khôi phục env sau Ok/Ret/trap).

**P2 (không chặn merge, nên xử lý khi mở rộng):**

1. **Rò tham số vào `env_module` khi lỗi sớm**: `call_hfn_than` ghi tham số vào `self.env` rồi
   mới gặp `return Err` (thiếu đối số / `self.eval(default)?` trap); các tham số đã ghi **không
   được hoàn nguyên** trước khi trả. Khi depth > 0, đó là map `env_module` thật. Hiện trap là
   fatal (không có catch/khôi phục) nên không quan sát được; nếu sau này có trap phục hồi, cần
   hoàn nguyên trước mọi `return Err` trong vòng bind tham số.
2. **`env_module` đồng bộ lười**: chỉ cập nhật ở closure/hàm gọi đầu tiên. `frame/params.rs`
   gán `it.env = globals` mà không cập nhật `env_module`, nên `env_module` vẫn trỏ map rỗng của
   `Interp::new`. Hiện frame có nhánh `HExpr::Var` riêng và `call_closure` tự đồng bộ lại trước
   khi chạy, nên **không quan sát được lỗi**; nhưng đây là bẫy cho mọi entry point tương lai
   reassign `it.env`. Nên đồng bộ `env_module` ngay tại chỗ gán `it.env = globals` (hoặc gom một
   setter) khi có dịp sửa frame.
3. **Thiếu test W>1 cho giá trị hàm tên**: bộ 4 đường của ca mới chỉ phủ W=1. Audit đã kiểm tay
   W=2 (đọc `9`, ghi bị `VS0509` đúng thiết kế), nhưng nên thêm 1 ca `worker_api_it` để khoá
   hành vi này, tránh tái phát nếu `tao_interp` đổi cách nạp snapshot.

## 8. Việc còn lại (ngoài lane Gatekeeper)

1. A0 cập nhật `agent/SO-NO.md` `#210: mở → đóng` và commit `[ST-00]` docs-only.
2. Push `plan/st319e` (Builder chưa push) rồi merge theo `VD-288`; dọn worktree sau merge.
3. (Tùy chọn) xử lý P2-3 khi có sóng test worker; P2-1/P2-2 ghi nợ kỹ thuật, không chặn `#210`.

## 9. Kết luận

**PASS.** Mã đúng thiết kế (env module cho hàm tên, snapshot cho lambda), phạm vi sạch, red-first
thật, 4 đường khớp, toàn bộ cổng bắt buộc xanh, merge-tree với `main` mới không xung đột. Chỉ còn
3 P2 không chặn.
