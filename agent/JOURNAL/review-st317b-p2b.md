<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW — ST-317b phần 2b (Gatekeeper độc lập)

- **Ngày**: 2026-11-27 · **Vai**: Gatekeeper (Agent 1) · **Worktree**: `/home/ps/viswt/st317b`
- **Nhánh**: `plan/st317b` · **HEAD**: `d22fa461` · **Base thật**: `426b83a3` (merge-base)
- **`main` được yêu cầu đối chiếu**: `fa74f294` — là commit `[ST-00]` **docs-only** *sau* `426b83a3`,
  **chưa có trong nhánh** (xem P2-C).
- **Kết luận: PASS** — không có P1 trong phạm vi; 3 P2 (2 test-quality, 1 đồng bộ nhánh) + 4 P3.
  Máy gate, clippy all-targets, DAG, rules và `vis-diff 100k` đều xanh; không sửa mã.

## 1. Bảng kiểm

| # | Hạng mục | Kết quả | Bằng chứng |
|---|---|---|---|
| 1 | `git status` sạch; không tệp lạ ngoài phạm vi | **PASS** | status rỗng; diff thật `426b83a3..HEAD` 25 tệp, không chạm `vis-lsp/**`, `vis-hir/src/stdlib/**`, `vis-diag/**`, `vis-pkg/**`, `tyck/pat/**`, `rt/sched/**` |
| 2 | Move `value` + `sqlite`; API/`size_of`/private-field test | **PASS** | đối chiếu từng dòng significant: `value` 0 thiếu, `sqlite` 1 thiếu chỉ do `loi_prepare` xuống dòng; `vis-core` 35 test xanh (có `size_of_value_bang_32`); sqlite 87 unit + 106 tích hợp xanh |
| 3 | Worker một nguồn; `codes::CROSS`; không giữ borrow 50 ms | **PASS** | `orchestrate.rs` 135 dòng; grep 0 `struct Proxy`/`code_cross`/`loi_task` cục bộ; 2 engine gọi `ap_dung_outcome`/`flush_proxy`/`pump`/`ap_va_flush` |
| 4 | 10 test W1..W10 chạy thật | **PASS có P2** | `10 passed; 0 failed; 0 ignored; 388 filtered`; W6 sai kỳ vọng đặc tả (P2-A), W7 thiếu assert `Pending` (P3-A) |
| 5 | Dedup `opt_depth_ty`/`loi_arity`/`thu_duong` | **PASS** | `opt_depth_ty` chỉ còn 1 định nghĩa `interp/goi.rs:373`; `loi_arity`/`loi_khong_phai_ham` free fn `pub(crate)` dùng 2 đường; `cargo check --all-targets` exit 0 |
| 6 | Cổng độc lập | **PASS** | gate **3906 passed / 0 failed / 5 ignored** (tương đương 3901/3901 + 4 skip); diff 100k **lech=0** 56,5 s; clippy 0 warning; DAG 23/5/0; R8 **39/40**, 0 tệp >1.500, target <1.000 |
| 7 | Hợp nhất với `fa74f294` | **PASS có P2** | `merge-tree --write-tree fa74f294 HEAD` = `69b894e…`, **0 conflict**; cây hợp nhất giữ VD-294 + thêm 25 tệp P2b; nhánh chưa chứa `fa74f294` (P2-C) |
| 8 | Rủi ro/lỗi thật | **PASS có finding** | không thấy regression worker/value/sqlite/dedup; 2 lỗi parity có sẵn ngoài lane (P2-A/P2-B) |

## 2. Số thật (log ngoài repo, `/home/ps/vislogs/`)

| Cổng | Kết quả | Log · sha256 |
|---|---|---|
| `./vis-dev gate --fast` | **exit 0** — 3906 passed / 0 failed / 5 ignored | `review-st317b-p2b-gate.log` · `3ecfa219…04dcb` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **exit 0 — `lech=0`**, 100.000 chương trình, 56,5 s, 1.770/s | `review-st317b-p2b-diff.log` · `de0caf62…fdf8f` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0**, 0 warning | `review-st317b-p2b-clippy.log` · `2e82ca30…dfaa` |
| `cargo check --workspace --all-targets` | **exit 0**, 9,96 s | `review-st317b-p2b-check.log` · `0b707723…1d52` |
| `./vis-dev rules` | **OK** — R1..R10; **R8 39/40** (vis-cli 8/15) | `review-st317b-p2b-rules.log` · `d6d12f11…7623` |
| `./bin/vis-dag-check` | **OK** — 23 crate / 5 tầng / 0 cạnh cấm | `review-st317b-p2b-dag.log` · `5ee0d7f4…af4d` |
| W1..W10 | **10/10**, 0 ignored, 388 filtered | `review-st317b-p2b-w10.log` · `d5bd4fef…cfed` |
| `cargo test -p vis-core` | **35** (30 unit + 5 doc), có `size_of_value_bang_32` | `review-st317b-p2b-core.log` · `882e4370…bab0` |
| `cargo test -p vis-host-native --features sqlite --lib db::sqlite` | **87/87** (test đọc field private) | `review-st317b-p2b-sqlite-unit.log` · `44019d57…431d` |
| `cargo test -p vis-host-native --features sqlite --test sqlite_it` | **106/106** | `review-st317b-p2b-sqlite.log` · `4db49056…ce6` |

LOC thật (`wc -l`):

| Tệp | Dòng | Ghi chú |
|---|---:|---|
| `vis-core/src/value/mod.rs` | **748** | từ 1.415 |
| `vis-core/src/value/lit.rs` / `ten.rs` | **403** / **291** | thuần move |
| `vis-host-native/src/db/sqlite/mod.rs` | **282** | từ 1.132 |
| `sqlite/{host,truy_van,loi,chuyen_doi,token,guard}.rs` | **261 / 312 / 111 / 90 / 78 / 67** | journal ghi 309/106 — lệch 3/5 dòng (P3-C) |
| `vis-runtime/src/worker/orchestrate.rs` | **135** | một nguồn |
| `vis-interp/src/rt/exec.rs` / `vis-vm/src/vm/worker.rs` | **368** / **485** | từ 461 / 586 |
| test W1..W10 | **405** | 0 binary R8 mới |

## 3. W1..W10 (chạy thật, không `#[ignore]`)

| # | Nội dung đã kiểm | Kết quả |
|---|---|---|
| W1 | output 3 task merge theo spawn order; parity interp/VM W=2/4 | PASS |
| W2 | ghi top-level W>1 ⇒ `VS0509` cả 2 engine; W=1 dùng runner non-frame | PASS, nhưng P2-B |
| W3 | `cancel_after` ⇒ exit 1 + message "huỷ" (chỉ assert message, không assert mã `VS0506`) | PASS (P3-D) |
| W4 | `VS0508` direct + **program-level** đối số hàm (spawn) + kết quả hàm (await) | PASS |
| W5 | await 2 lần cùng task: stdout `7\n7\n`, chạy/flush một lần | PASS |
| W6 | task lỗi không await: W=1 surface cả 2 engine; **W=2/4 test assert `exit == 0`** | **PASS máy, sai đặc tả — P2-A** |
| W7 | frame mode W=2/4, stdout byte-identical W=1, `steps > 0` | PASS, thiếu assert `Pending` — P3-A |
| W8 | `run` vs `run_workers_local` cùng stdout/exit | PASS |
| W9 | parity interp/VM W=1/2/4 (stdout/exit/codes/value) | PASS |
| W10 | cancel 2 lần cùng handle: sau khi xong (giữ kết quả) + trước join (1 outcome `cancelled`) | PASS |

## 4. Finding

### P2-A — W6 khoá hành vi `exit 0` trái đặc tả `st317b-tach-tep.md:180–193`

- Test `w6_loi_task_khong_await_duoc_surface` (dòng 255–273) assert `exit == 0` cho **cả hai**
  engine ở W=2/4, trong khi bảng W6 yêu cầu “trap `VS0506` surfaced từ `final_join` (không exit 0)”.
- Đường **production frame** (CLI mặc định cho chương trình async) **đã đúng**:
  `./target/debug/vis run --workers 2 /tmp/opencode/st317b-audit/w6.vis` → in `sau`, rồi
  `error VS0506: tác vụ \`hong\` lỗi: VS0502…`, **exit 101** (W=1 cũng 101).
- Nguyên nhân: test dùng `run_workers_parallel*` **non-frame**; đường này nuốt lỗi task không await.
  Đây là hành vi **có trước** (đã đối chiếu `fa74f294:crates/vis-interp/src/rt/exec.rs` +
  `vis-vm/src/vm/worker.rs`: `final_join` chỉ trả trap cho `dirty`/`cancelled`, không trả `Err` đã cache).
- Đề nghị: viết lại W6 bằng `run_async_workers_with_hosts_caps` (frame) và assert `VS0506`/exit 101;
  giữ ca non-frame exit 0 thành test nợ riêng có tên rõ, hoặc đăng ký nợ trước `DONE`.

### P2-B — W2 W=1 chỉ kiểm runner non-frame; production frame lệch interp↔VM

- Test W2 W=1 dùng `run_workers_local*` (non-frame) và thấy `1\n`; production `vis run` (interp frame)
  cho **`0`**, còn `vis run --vm` cho **`1`** trên cùng `w2.vis`:
  `./target/debug/vis run w2.vis` → `0`; `./target/debug/vis run --vm w2.vis` → `1`.
- `crates/vis-interp/src/frame/worker.rs` **không bị sửa** trong nhánh (`git diff 426b83a3..HEAD`
  rỗng cho tệp này) ⇒ lỗi parity có sẵn, ngoài lane P2b; nhưng W2/W9 hiện không phủ đường frame nên
  không phát hiện. Đề nghị đăng ký nợ VS3/correctness + thêm ca frame W=1 ghi global.

### P2-C — Nhánh chưa chứa `main` `fa74f294` (docs-only)

- `git merge-base fa74f294 HEAD` = **`426b83a3`**; `fa74f294` là commit `[ST-00]` **sau** đó
  (`VD-294` + `#202` + Sóng 6 P2b), không nằm trong `plan/st317b`.
- Vì vậy `git diff fa74f294..HEAD` *trông như* revert `ACTIVE/QUEUE/DECISIONS/SO-NO/ROADMAP` và xoá
  `JOURNAL/quyet-dinh-a0-2026-11-27-vd294.md`; **không phải** nhánh sửa các tệp đó. Diff thật
  `426b83a3..HEAD` chỉ có 25 tệp sản phẩm + `JOURNAL/st317b-tach-tep.md` + `STATE.md`.
- `git merge-tree --write-tree fa74f294 HEAD` = `69b894e…`, **0 conflict**; cây hợp nhất giữ nguyên
  VD-294 và thêm đúng 25 tệp P2b (+2393/−1855). Trước khi hợp nhất phải merge/rebase `fa74f294`.

### P3-A — W7 chưa chứng minh `Pending`/`frame_next` wake

- Test chỉ assert stdout và `steps > 0`; `steps` tăng mỗi turn của driver
  (`frame/worker.rs`), nên không phải bằng chứng `await` trả `Pending` rồi được đánh thức.
- Đề nghị: ca frame có task dài + đếm turn, assert lần `await` đầu trả `Pending` và `frame_next`
  trả frame sau khi outcome tới (hoặc hook đếm `Pending`).

### P3-B — W2 không kiểm tên biến/task; message dùng symbol nội bộ

- Test chỉ `contains("ghi chéo worker")`. Chạy thật:
  `error VS0509: ghi chéo worker (W>1): biến s14 (task \`tang\`)` — có tên task, nhưng “tên biến”
  là `s14`, không phải `counter`. Đặc tả W2 ghi “message nêu tên biến + tên task”.
- Đây là chất lượng message có sẵn (snapshot symbol), không phải regression P2b; nên ánh xạ
  symbol→tên nguồn hoặc ít nhất test assert tên task + sự hiện diện của tên biến.

### P3-C — Journal lệch LOC nhỏ

- `sqlite/loi.rs`: journal ghi **106**, thật **111**; `sqlite/truy_van.rs`: ghi **309**, thật **312**.
  Các số còn lại (748/403/291/282/135/368/485/405) khớp.

### P3-D — W3 chỉ assert message "huỷ", không assert mã `VS0506`

- Nên assert `out.codes()` chứa `VS0506` để khoá đúng hợp đồng, tránh bắt nhầm trap khác có chữ “huỷ”.

## 5. Ghi chú dedup

- `opt_depth_ty`: một định nghĩa `crate::interp::goi::opt_depth_ty`; `frame/params.rs` re-export
  `pub(super)`, `frame/expr.rs` dùng qua `super::*` — không còn bản sao.
- `loi_arity`/`loi_khong_phai_ham`: free fn `pub(crate)` trong `interp/goi_ham.rs`, re-export qua
  `interp/goi.rs`; frame + interp dùng chung, message/span giữ nguyên. `call_value` **không** gộp thân
  (frame dựng `Work`, interp trả `Value`) — đúng như journal §2b.4, không phải bỏ sót.
- `thu_duong`: vẫn một nguồn `interp::gan_long::thu_duong`; wrapper `frame/expr.rs` chỉ chuyển
  `HExpr` → `NutGan`, không bị ép gộp sai env closure.
- `pump`: `rut_outcome` lấy `ids` trước, chờ 50 ms trên `OutcomeBoard`, closure chỉ borrow
  `RefCell` của engine **sau** khi có outcome — không giữ borrow qua 50 ms (cả interp lẫn VM).

## 6. Tái lập nhanh

```bash
export PATH=/home/ps/.rustup/toolchains/1.100.0-x86_64-unknown-linux-gnu/bin:$PATH
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 ./vis-dev gate --fast
VIS_DIFF_N=100000 ./bin/vis-diff
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo clippy --workspace --all-targets -- -D warnings
./bin/vis-dag-check && ./vis-dev rules
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 \
  cargo test -p vis-conformance --test lib_api_it st317b_worker_orchestrate
./target/debug/vis run --workers 2 /tmp/opencode/st317b-audit/w6.vis   # P2-A: exit 101, VS0506
./target/debug/vis run w2.vis; ./target/debug/vis run --vm w2.vis       # P2-B: 0 vs 1
git merge-tree --write-tree fa74f294 HEAD                               # 69b894e…, 0 conflict
```

**Không sửa mã.** Chỉ thêm tệp journal này và commit `[REVIEW]`.
