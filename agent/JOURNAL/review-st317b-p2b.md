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

---

# Addendum F1 — audit `a801ccc6` + `d56b96f1` (2026-11-27)

- **HEAD audit**: `d56b96f1` · **Merge**: `a801ccc6` (parents `ef3e6928` + `fa74f294`) · **`main`**: `fa74f294`.
- **Kết luận: FAIL (P2 tài liệu)** — mã/test/cổng đều PASS; **P2-B ghi nợ sai** và **bỏ sót nợ thật**
  (W2 frame W=1), nên chưa thể chuyển A0 đăng ký nợ. Không sửa mã.

## F1.1 Bảng kiểm

| # | Hạng mục | Kết quả | Bằng chứng |
|---|---|---|---|
| 1 | `git status` sạch; merge main; docs VD-294 | **PASS** | status rỗng; `main` là ancestor; `merge-tree main HEAD` = **`f5957db`** = tree HEAD, **0 conflict**; `git diff main..HEAD -- ACTIVE/QUEUE/DECISIONS/SO-NO/ROADMAP/DECISIONS-INDEX` **0 tệp**; VD-294 còn (ACTIVE 7 dòng) |
| 2 | W6 frame API + assert | **PASS** | `frame_1` (`run_async_with_hosts_caps`) W=1, `frame_w` (`run_async_workers_with_hosts_caps`) W=2/4; assert `exit == 1` + `code.short == "VS0506"` + `message.contains("hong")`; 0 `#[ignore]` |
| 3 | P2-B chỉ ghi nợ, không sửa code/test giả | **PASS cơ học / FAIL nội dung** | `git diff ef3e6928..HEAD` không có `frame/worker.rs`/`frame/*`; không thêm test P2-B; nhưng nội dung nợ §2b.7:351–354 **sai** (F1.3) |
| 4 | P3 W3/W7/LOC; W2 | **PASS một phần** | W3 assert `VS0506`; W7 không thêm assert `Pending` giả; LOC **312/111** khớp; W2 nguồn dùng `counter` nhưng diagnostic vẫn `s14` (F1.4) |
| 5 | Cổng lại | **PASS** | gate **3906 passed / 0 failed / 5 ignored** (3901/3901 + 4 skip); diff 100k **lech=0** 56,5 s; clippy 0 warning; W **10/10** |
| 6 | `merge-tree main HEAD` | **PASS** | `f5957db…`, **0 conflict**; nhánh đã chứa `fa74f294` |

## F1.2 Số thật (log `/home/ps/vislogs/`)

| Cổng | Kết quả | Log · sha256 |
|---|---|---|
| `./vis-dev gate --fast` | **exit 0** — 3906 passed / 0 failed / 5 ignored | `review-st317b-p2b-f1-gate.log` · `b2f123bd…b5ca7` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **exit 0 — `lech=0`**, 100.000, 56,5 s, 1.769/s | `review-st317b-p2b-f1-diff.log` · `94b02241…1c493` |
| clippy `--workspace --all-targets -D warnings` | **exit 0**, 0 warning | `review-st317b-p2b-f1-clippy.log` · `63622df7…830cc` |
| W1..W10 | **10/10**, 0 ignored, 388 filtered | `review-st317b-p2b-f1-w10.log` · `5731f1f8…ee2ab` |

W6 output thật (frame, `--workers`):

```text
--workers 1/2/4: error VS0506: tác vụ `hong` lỗi: VS0502: lấy dư cho 0 (`%0`); exit=101
```

## F1.3 FAIL — P2-B ghi sai, bỏ sót nợ thật

Vị trí: `agent/JOURNAL/st317b-tach-tep.md` §2b.7 **`### P2-B` dòng 351–354**:

> “production frame W=1 lệch interp (exit 0) vs VM (exit 1) cho cùng kịch bản lỗi task không await;
> … interp phải surface `VS0506` như VM”

Đo lại **cùng HEAD**, kịch bản lỗi task không await (`w6.vis`):

```text
vis run w6.vis              -> error VS0506 ... `hong` ...; exit=101
vis run --vm w6.vis         -> error VS0506 ... `hong` ...; exit=101
vis run --workers 1/2/4     -> error VS0506 ...; exit=101
```

⇒ **không có** lệch frame W=1 interp 0 vs VM 1 cho task-error; nợ ghi trong journal là **không tồn tại**.

Nợ **thật** của P2-B vẫn còn nguyên (W2 toàn cục, frame W=1):

```text
vis run w2.vis              -> stdout "0"; exit=0
vis run --vm w2.vis         -> stdout "1"; exit=0
```

Ngoài ra, nợ **non-frame W6 W≥2** (final join nuốt lỗi task không await) chỉ còn một dòng trỏ
“xem nợ P2-B bên dưới”, nhưng P2-B lại mô tả kịch bản frame task-error — nên nợ này **không được
đăng ký đúng**. Đề nghị:
1. Sửa §2b.7 P2-B thành đúng nợ W2 frame W=1: interp stdout `0` vs VM `1` (cùng exit 0), globals
   top-level không ghi xuyên frame; `frame/worker.rs` ngoài lane.
2. Đăng ký **riêng** nợ non-frame `run_workers_parallel*` W≥2 nuốt lỗi task không await (hoặc giữ
   một ca test nợ có tên rõ, không assert thành “đúng”).
3. Không cần sửa mã cho addendum này; sửa journal rồi audit lại.

## F1.4 P3 — W2 chưa đóng

- Test W2 **nguồn** dùng `counter` (đúng), nhưng diagnostic thực tế vẫn là symbol nội bộ:
  `error VS0509: ghi chéo worker (W>1): biến s14 (task \`tang\`)`.
- Journal §2b.7 dòng 358 ghi “W2: đã dùng `counter` (không phải `s14`) — không cần đổi” là **sai**
  với diagnostic. Đây là P3 (không chặn), nhưng cần sửa câu chữ hoặc ánh xạ symbol→tên nguồn.

## F1.5 Kết luận addendum

- **PASS kỹ thuật**: W6 đã đúng frame API + `VS0506` cho W=1/2/4; W3 code; W7 không assert giả;
  LOC khớp; merge main sạch; gate/diff/clippy/W xanh.
- **FAIL điều kiện P2-B**: nợ ghi trong §2b.7 sai kịch bản và bỏ sót nợ thật (W2 frame W=1 +
  non-frame W6 W≥2). Chỉ cần sửa journal builder, không sửa mã; sau đó audit lại.

---

# Addendum F2 — audit `7a1d7ea7` (sửa journal P2-B/P3-B)

- **HEAD**: `7a1d7ea7` · **Commit cha**: `63535b33` (Addendum F1) · `main`: `fa74f294`.
- **Kết luận: PASS** — chỉ journal, 3 mục đã sửa đúng bản chất; không đổi mã/test.
- `git show --stat 7a1d7ea7`: **1 tệp** `agent/JOURNAL/st317b-tach-tep.md`, **+12/−5**.
  `git diff --name-only 63535b33..7a1d7ea7` = chỉ tệp journal; **0 tệp mã/test**.
  (`git diff d56b96f1..7a1d7ea7` gồm thêm commit review F1 `63535b33` của Gatekeeper — không phải
  Builder sửa mã.)
- `git status` sạch; `git merge-base --is-ancestor main HEAD` **đúng**.

## F2.1 Ba mục đã kiểm

| Mục | Nội dung hiện tại | Kết quả |
|---|---|---|
| P2-B | W2 **frame W=1**: `vis run w2.vis` → stdout `0`, exit 0; `vis run --vm w2.vis` → stdout `1`, exit 0; nêu rõ **không phải** task-error W6 (W6 frame: cả hai `VS0506`, exit 101) | **ĐÚNG** — khớp đo lại ở F1 |
| Nợ non-frame W6 W≥2 | Mục **riêng** “Nợ non-frame W6 W≥2”: `run_workers_parallel*`/`run_workers_parallel_vm*` nuốt lỗi task không await, exit 0, cả 2 engine; W6 frame không phủ đường này; đề xuất nợ VS3/correctness | **ĐÚNG** — tách khỏi P2-B, không còn trỏ sai |
| P3-B | Diagnostic W2 vẫn dùng symbol `s14` (không phải `counter`), chưa đổi, không chặn | **ĐÚNG bản chất** — test nguồn dùng `counter` nhưng diagnostic vẫn `s14`; câu “test W2 in/đối chiếu s14” hơi rộng (test chỉ kiểm “ghi chéo worker”), không ảnh hưởng kết luận |

- LOC vẫn đúng: `sqlite/truy_van.rs` **312**, `sqlite/loi.rs` **111**.
- Không còn câu P2-B sai (task-error frame W=1) và không còn câu P3-B “đã dùng counter, không phải s14”.

## F2.2 Kết luận

**PASS** — điều kiện P2-B đã đóng ở mức journal; nợ thật (W2 frame W=1 interp↔VM) và nợ non-frame
W6 W≥2 đã được ghi đúng, tách riêng. Không cần chạy lại gate (không đổi mã/test); các cổng F1 vẫn
hiệu lực cho HEAD sản phẩm `d56b96f1` (gate 3906/0/5, diff 100k `lech=0`, clippy 0, W 10/10).
