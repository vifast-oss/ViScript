<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-318h — tách `vis-interp/src/frame.rs` 1030 → 159 (move nguyên văn)

> **Người kiểm**: Gatekeeper (Agent 1 độc lập), 2026-11-27 (giờ thấp điểm).
> **Đối tượng**: nhánh `plan/st318h`, HEAD **`d65f0c3d`**, nền **`640180e0`** (`plan/st318a` F1);
> 2 commit `8c22a682` (tách tệp) + `d65f0c3d` (journal + `STATE`).
> **`plan/st318a` đối chiếu**: tip **đã tiến** từ `991b89b6` → **`3e6f3468`** (F2 dirty-set, đụng
> `frame/worker.rs`) trong phiên; kiểm merge-tree với tip mới.
> **`main` đối chiếu**: **`7900a308`** (`[ST-00]` đồng bộ số Sóng 7).
> **Worktree**: `/home/ps/viswt/st318h` — **không sửa mã**, không push, không `cargo clean`.
> **Cấu hình đo**: cargo **1.100.0** (`$HOME/.cargo/bin/cargo`; `vis-dev` tự chọn),
> `CARGO_BUILD_JOBS=2`; đĩa `/home` **15 GB** trống (> 6 GB). Log ngoài repo tại `/home/ps/vislogs/` (R7).
>
> **Kết luận: PASS** — **0 P1**, **2 P2** (thứ tự hợp nhất với `plan/st318a`; xung đột `STATE.md`
> với `main`) và **1 P3** (link rustdoc hỏng **có sẵn từ nền**, không do lát này). Không tệp mã nào
> ngoài `frame*` bị chạm; `frame/worker.rs` **byte-identical**; move đúng nghĩa token-identical.

## 0. TL;DR

| # | Hạng mục | Kết quả | Bằng chứng chính |
|---|---|---|---|
| 1 | Phạm vi git | **PASS** | `git status` rỗng; 2 commit; **7 tệp, +998/−886** = mã `frame*` + journal + `STATE`; `frame/worker.rs` hash **không đổi** |
| 2 | Move thật | **PASS** | 26 item đối chiếu theo token: **0 mất/0 đổi logic**; chỉ thêm `pub(super)` + 2 dấu phẩy do `rustfmt`; test cũ 897 token identical |
| 3 | API | **PASS** | `pub use state::IFrame` / `pub use step::run_frame_for_test`; `pub(crate)` `frame_charge`/`run_frame_driver`/`FrameConfig`/`FrameOutput` nguyên; 43 `pub(super)` = đúng phạm vi private cũ trong `frame` |
| 4 | Cổng độc lập | **PASS** | fmt 0; `cargo test -p vis-interp` **62** (5+55+2); gate **3906/3906** (4 skip); diff 100k **`lech=0`**; clippy exit 0; R8 **39/40** |
| 5 | Hợp nhất | **PASS (dự kiến)** | `merge-tree` với `plan/st318a@3e6f3468` **sạch** (tree `209e1861`, đã `cargo check` cây gộp OK); với `main@7900a308` chỉ xung đột `agent/STATE.md` |
| 6 | Rủi ro/lỗi thật | **PASS có finding** | 0 P1; 2 P2 (thứ tự merge + `STATE`) + 1 P3 rustdoc có sẵn (§6) |

## 1. Phạm vi git

```
d65f0c3d [ST-318h] journal + STATE: so lieu cong (gate 3906/3906, diff 100k lech=0, clippy 0)
8c22a682 [ST-318h] tach frame.rs 1030 -> 159 + 4 tep (work/state/helper/step), move nguyen van
```

`git diff --numstat 640180e0..HEAD` — **7 tệp, +998/−886**:

| Tệp | + | − | Loại |
|---|---:|---:|---|
| `crates/vis-interp/src/frame.rs` | 15 | 886 | module root + 2 test cũ |
| `crates/vis-interp/src/frame/work.rs` | 344 | 0 | mới |
| `crates/vis-interp/src/frame/state.rs` | 112 | 0 | mới |
| `crates/vis-interp/src/frame/helper.rs` | 366 | 0 | mới |
| `crates/vis-interp/src/frame/step.rs` | 99 | 0 | mới |
| `agent/JOURNAL/st318h-tach-frame.md` | 61 | 0 | journal Builder |
| `agent/STATE.md` | 1 | 0 | trạng thái |

- `git status --short` **rỗng** trước và sau phiên; `git diff --check` exit 0.
- 15 dòng thêm trong `frame.rs` chỉ là `mod helper/state/step/work;`, các `use` gom từ item đã
  chuyển và 2 `pub use` giữ API (`IFrame`, `run_frame_for_test`).
- **`frame/worker.rs` không đổi**: blob `640180e0` = `HEAD` = `f49a460671b198abe9c48847b96661f97c7e2bae`
  (cả 2 commit `[ST-318h]` đều không có đường dẫn này trong `--name-status`).
- 0 tệp test mới/không mất tệp test nào; 0 `unsafe` trong 5 tệp (`grep -c unsafe` = 0).

## 2. Bằng chứng move nguyên văn (kiểm độc lập)

**Cách kiểm**: `git show 640180e0:crates/vis-interp/src/frame.rs` → tách item ở mức cây cú pháp
(brace-depth), tokenize bỏ khoảng trắng + comment (giữ nguyên chuỗi/ký tự/lifetime), so **chuỗi
token theo thứ tự** với từng item trong 4 tệp mới; đối chiếu cả **tập tên item**.

| Tệp | Item | Kết quả token |
|---|---|---|
| `work.rs` | `Work`, `MethodKind`, `Callee`, `LoopKindInit` (4) | **IDENTICAL** — chỉ thêm `pub(super)` |
| `state.rs` | `LoopState`, `LoopKindState`, `CallFrame`, `IFrame`, `impl IFrame` (5) | chỉ thêm `pub(super)` + **1 dấu phẩy cuối** ở chữ ký `fn new` (rustfmt nối dòng) |
| `helper.rs` | `block_works`…`underflow` (14) | **IDENTICAL** — chỉ thêm `pub(super)` |
| `step.rs` | `run_frame_for_test`, `eval_expr_in`, `step_frame_owned` (3) | chỉ thêm `pub(super)` + **1 dấu phẩy cuối** ở `step_frame_owned` (rustfmt) |

- **Tổng 26 item**: 0 item mất, 0 item đổi thân; 43 `pub(super)` mới (0 ở bản cũ), 2 dấu phẩy fmt.
- Tập tên item cũ **⊆** tập tên mới; tên "mới" duy nhất là `mod helper/state/step/work` + `use super::*`.
- `mod tests` (2 test cũ: `closure_moi_lan_goi_env_rieng`, `ten_khong_ton_tai_khong_nhan_argv`)
  ở lại `frame.rs`: **897 token identical** với bản `640180e0`.
- Thuộc tính giữ nguyên: `#[doc(hidden)]` trên `run_frame_for_test`, `#[cfg(test)]` trên
  `use step::step_frame_owned`/`mod tests`.
- LOC thật: `frame.rs` **159**; `work` 344; `state` 112; `helper` 366; `step` 99 (tổng 1080 = 1030
  + ~50 dòng header/`use super::*` của 4 tệp mới). Tệp lớn nhất **366** (< 1.000).

## 3. API công khai — không mất, không nới

| Item | Trước (`640180e0`) | Sau (`HEAD`) |
|---|---|---|
| `vis_interp::frame::IFrame` | `pub struct` trong `frame.rs` | `pub use state::IFrame` (cùng đường dẫn) |
| `vis_interp::frame::run_frame_for_test` | `pub fn` + `#[doc(hidden)]` | `pub use step::run_frame_for_test` (giữ `#[doc(hidden)]`) |
| `frame_charge`, `run_frame_driver` | `pub(crate)` | `pub(crate)` (nguyên) |
| `FrameConfig`, `FrameOutput` | `pub(crate) use vis_core::frame` | nguyên |
| 26 item nội bộ còn lại | private trong module `frame` (thấy được ở module con) | `pub(super)` trong `frame::*` = `pub(in crate::frame)` — **đúng phạm vi cũ**, không lộ ra ngoài `frame` |

Không có item `Frame` trong module này; "Frame" trong đề bài tương ứng `FrameConfig`/`FrameOutput`
(đều còn). Nhiều integration test bên ngoài dùng `vis_interp::frame::run_frame_for_test` vẫn biên dịch
và chạy trong gate **3906/3906**.

## 4. Cổng độc lập (số thật, log ngoài repo)

| Cổng | Kết quả | Log · sha256 |
|---|---|---|
| `cargo fmt --all --check` | exit **0** | — |
| `cargo test -p vis-interp` | **62 pass** = lib **5** + `interp_it` **55** + doc **2**; 0 fail | `/home/ps/vislogs/review-st318h-test.log` · `a0834cfe…cacc4` |
| `./vis-dev gate --fast` | **PASS** — nextest **3906/3906 passed, 4 skipped** (77,3 s); `check` PASS (fmt/clippy/R1..R10/DAG/self-test/build/doc-examples 31 khối) | `/home/ps/vislogs/review-st318h-gate.log` · `6c95ee3f…97d9e` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** — 100.000 chương trình (93.768 chạy), **102,0 s**, 980/s | `/home/ps/vislogs/review-st318h-diff.log` · `0ab9c71f…0c95a` |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit **0** | `/home/ps/vislogs/review-st318h-clippy.log` · `47c652b6…0181` |
| R8 | **39/40**: `crates/*/tests` cấp 1 = **38** + `vis-host-linux/tests` = **1**; `vis-cli/tests` **8/15** | trong gate (`luật R1..R10: OK`) |
| Kích thước tệp | mọi tệp thuộc lát **< 1.000** (max 366); không tạo tệp > 1.000 mới | `wc -l` |

**Không đạt (không tính cổng, có sẵn từ nền)**: `RUSTDOCFLAGS="-D warnings" cargo doc -p vis-interp --no-deps`
exit 101 vì intra-doc link `[`Interp`]` ở header `frame.rs:5`. Đã kiểm **cùng lỗi tại nền**
(`/home/ps/viswt/st318a`, `frame.rs` 1030 dòng): `/home/ps/vislogs/review-st318h-doc-base.log`
`ba389401…d52d7`; log HEAD `57c790ee…2f1c5`. Không phải regression của ST-318h; `gate/check` không chạy `cargo doc`.

## 5. Hợp nhất

### 5.1 Với `plan/st318a` (tip **`3e6f3468`**)

- `git merge-base plan/st318a HEAD` = **`640180e0`** (nền của 318h) — 318a chỉ đi thêm
  `991b89b6` (review F1) và **`3e6f3468`** (F2 dirty-set: `frame/worker.rs` **+161/−39**, test
  `st317b_worker_orchestrate.rs` +126, 2 journal).
- `git merge-tree --write-tree plan/st318a HEAD` → **exit 0**, tree **`209e1861`**, **0 xung đột**
  (không tệp nào trùng: 318a chỉ đụng `worker.rs`/test/journal; 318h không đụng `worker.rs`).
- Kiểm thêm: dựng commit gộp tạm (`git commit-tree 209e1861 -p HEAD -p plan/st318a`) trong worktree
  tạm `/tmp/opencode/st318h-merge` → `cargo check -p vis-interp --all-targets` **PASS (3,96 s)**;
  đã `git worktree remove`, không để lại nhánh/tệp trong repo.
- **Bắt buộc**: merge (hoặc rebase) tip **`3e6f3468`** vào `plan/st318h` **trước khi hợp nhất** để
  bản gộp có F2 dirty-set + test `w2two/w2read`. Lưu ý worktree `/home/ps/viswt/st318a` đang **bẩn**
  (`M agent/STATE.md`, `M agent/JOURNAL/st318a-dong-no.md`) — không được cuốn các thay đổi chưa
  commit đó vào merge.

### 5.2 Với `main` (**`7900a308`**)

- Merge-base `main`/HEAD = **`2b5ccb51`** (`main` **không** chứa `640180e0`).
- `git merge-tree --write-tree main HEAD` → **exit 1**, tree `38a5f6ef`, **chỉ 1 xung đột
  `agent/STATE.md` (content)**; **0 xung đột mã nguồn**. `main` không đổi tệp nào dưới
  `crates/vis-interp/src/frame*` kể từ `2b5ccb51` (diff rỗng), nên bản tách của 318h gộp sạch.
- Cách xử lý: giữ **cả hai** bullet `ST-318a`/`ST-318h` trong `STATE.md` (xung đột chỉ do hai bên
  cùng chèn 1 dòng).

## 6. Rủi ro / finding

### P1 — không có

Không có mất API/mất test/đổi hành vi: token-identical 26/26 item, test cũ nguyên 897 token,
gate 3906/3906, diff 100k `lech=0`, clippy 0.

### P2-1 — Thứ tự hợp nhất với `plan/st318a` + worktree 318a bẩn

318h dừng ở nền `640180e0`; 318a đã tiến **`3e6f3468`** (F2 sửa `frame/worker.rs`). Nếu hợp nhất
318h trước khi cập nhật 318a, bản gộp sẽ thiếu F2 (test `w2two/w2read` + dirty-set). Text-merge
sạch và cây gộp **đã biên dịch được**, nhưng thứ tự phải rõ: **318a tip → 318h → `main`**.
Ngoài ra **không** merge các thay đổi chưa commit trong worktree `/home/ps/viswt/st318a`.
*Tái lập*:
```bash
git merge-tree --write-tree plan/st318a HEAD   # exit 0, tree 209e1861
git -C /home/ps/viswt/st318a status --short    # M agent/STATE.md, M .../st318a-dong-no.md
```

### P2-2 — Xung đột `agent/STATE.md` với `main` (chỉ tài liệu)

`merge-tree main HEAD` báo đúng 1 xung đột `agent/STATE.md`; không có xung đột mã. Không chặn
PASS, nhưng khi gộp thật phải sửa tay (giữ cả 2 bullet), không `-X ours/theirs` toàn cục.
*Tái lập*: `git merge-tree --write-tree --name-only main HEAD` → `agent/STATE.md`, exit 1.

### P3-1 — Link rustdoc `[`Interp`]` hỏng **có sẵn từ nền**

`RUSTDOCFLAGS="-D warnings" cargo doc -p vis-interp --no-deps` đỏ ở `frame.rs:5`
("no item named `Interp` in scope"). Bản `640180e0` (1030 dòng) đỏ **y hệt** (đã chạy trong
`/home/ps/viswt/st318a`), nên **không phải regression**; gate hiện không chạy `cargo doc`.
Đề xuất (ngoài phạm vi lát này): đổi link thành ``[`crate::interp::Interp`]`` hoặc bỏ ngoặc khi
mở việc sửa doc.
*Tái lập*: xem 2 log `review-st318h-doc{,-base}.log`.

### Các rủi ro đề bài nêu — đã loại trừ

| Rủi ro | Kết quả |
|---|---|
| Mất helper public | Không: 26 item đủ mặt, `pub(crate)` nguyên, các test ngoài crate vẫn gọi `run_frame_for_test` |
| `state`/`step` đổi hành vi | Không: chỉ khác `pub(super)` + 1 dấu phẩy fmt mỗi tệp; không đổi token logic |
| Test cũ bị bỏ | Không: `mod tests` 897 token identical; tổng nextest vẫn **3906** như nền |
| `frame.rs` 159 chỉ còn re-export | Đúng thiết kế: 60 dòng đầu (doc + `mod`/`use`/`pub use`) + `mod tests` 99 dòng (2 test cũ); không còn logic |

## 7. Kết luận

**PASS** cho `ST-318h`: tách tệp đúng là **move nguyên văn** (bằng chứng token 26/26 item +
test 897 token), API/`worker.rs` không đổi, toàn bộ cổng độc lập xanh (62 test; gate 3906/3906;
diff 100k `lech=0`; clippy 0; R8 39/40; tệp < 1.000). Điều kiện hợp nhất: **merge `plan/st318a`
tip `3e6f3468` vào `plan/st318h` trước**, rồi gộp `main` và xử lý tay xung đột `agent/STATE.md`.
Commit Builder được nghiệm thu: `8c22a682` (mã) + `d65f0c3d` (journal/STATE); commit review này
ghi tiếp trên `plan/st318h`.
