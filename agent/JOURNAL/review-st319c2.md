<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-319c2 — closure ghi global by-value trên frame/async (`#206`)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Gatekeeper (Agent 1)
- **Nhánh**: `plan/st319c2` · **Worktree**: `/home/ps/viswt/st319c2`
- **HEAD**: `2629fef0` · **Nền**: `ae0089c0` (ST-319a WIP) · **319a F2**: `5d1d12e8` (nhánh đã
  được dọn sau merge; kiểm bằng hash)
- **Kết luận**: **PASS có điều kiện** — 0 P1; **P2-1 là hồi quy mới phải vá trước merge**
  (`nap_vao_env` bỏ sót khung hàm tên nằm trên closure by-value); **P2-2 là nợ sẵn có** của
  `#206` (hàm tên qua giá trị tạo *trong* closure); 2 P3 về nhãn commit/assert test.
  Không sửa mã sản phẩm trong lượt review; chỉ thêm tệp nhật ký này.
  **Addendum F1 (`0caa9184`, §8): P2-1 đã vá và đóng — PASS; P2-2 vẫn là nợ riêng không chặn.**

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Phạm vi diff | **ĐÚNG** — chỉ `vis-interp/src/frame/**` + test async + journal; không chạm crate cấm |
| 5 ca w19–w23 | **5/5 PASS thật**, literal khớp cả frame-interp và frame-VM (W=1) |
| `st317b` | **25/25 PASS** (0 ignore), gồm các ca `#203` cũ |
| `cargo test -p vis-interp` | **55 + 2 doc-test PASS** |
| `./vis-dev gate --fast` | **PASS** (`/home/ps/vislogs/review-st319c2-gate.log`) |
| `vis-diff 100k` | **`lech=0`**, `chay=94425` (`review-st319c2-diff.log`) |
| clippy workspace all-targets | **PASS** `-D warnings` (`review-st319c2-clippy.log`) |
| Merge-tree | 319a tip, main `2ad4ac93`, main `2e243561`: **đều sạch** (`ec=0`) |
| Lỗi thật | **P2-1** hồi quy mới: `stale.vis` frame `0\n3` — nền `3\n3`, VM `3\n3` |

## 1. Phạm vi, commit

`git status` sạch trước review; `git log --oneline ae0089c0..HEAD` đúng **2 commit**:

```
2629fef0 [ST-319a] F1: test closure await/task/ham-ten (frame)
ef0f213d [ST-319a] F1 WIP: frame closure ghi global (checkpoint)
```

`git diff --stat ae0089c0..HEAD`:

```
 agent/JOURNAL/st319c2-frame-closure-ghi-global.md          |  71 +++++++
 .../async_api_it/st317b_worker_orchestrate.rs              | 160 +++++++++++++++
 crates/vis-interp/src/frame/expr.rs                        |   5 +
 crates/vis-interp/src/frame/state.rs                       |   7 +
 crates/vis-interp/src/frame/worker.rs                      |  24 +++-
```

Không có thay đổi ở `vis-bytecode` / `vis-vm` / `vis-core/value` / `vis-hir` / `vis-diag` /
`vis-types`. **P3-1**: cả hai commit mang tiền tố `[ST-319a]` dù nội dung/nhánh là ST-319c2 —
traceability lệch, nên sửa nhãn khi rebase/merge (không chặn kỹ thuật).

## 2. Kiểm code

| Điểm | Xác nhận |
|---|---|
| `CallFrame.by_value` mặc định | `false` ở `IFrame::new` và `expr_frame`; 4/4 điểm dựng `CallFrame` đều set |
| `call_value` lambda | `by_value = c.name.is_empty()` — đúng cho `MakeLambda` (tên rỗng) |
| `dispatch` hàm tên | `by_value: false`; `Value::Fn` hàm tên ở `eval_work` giữ `f.name` ⇒ false |
| `DongBo::ghi` | `if frame.cur().by_value { return; }` **trước** `ghi_bien_cap_tep` ⇒ không ghi globals, không dirty, không đổi `frame_seq`; khung hàm tên vẫn xuyên (`#203`) |
| `nap_vao_env` | `position` closure by-value đầu tiên, chỉ sync `calls[..i]`; không đẩy dirty vào snapshot closure treo |
| `#203` task/main ghi global | giữ: `w11/w14/w15/w16/w18` + `w23` PASS; probe `task_write` = `1` |
| `#206` mặc định non-frame | không đổi (không sửa crate ngoài frame); probe sync closure = `5\n0` |

**P2-1 (hồi quy mới, bắt buộc vá trước merge)** — `nap_vao_env` cắt *mọi* khung từ closure
by-value đầu tiên, kể cả **khung hàm tên** nằm trên closure. Khung hàm tên do `dispatch` dựng
`env_for_params` từ `globals`, không sống trên snapshot closure, nên bị bỏ sync là sai. Kịch bản
(thân hàm tên gọi một closure có `await`, rồi đọc global):

```vis
var g = 0
async fn dat() -> int:
    g = 3
    3
fn doc() -> int:
    let h = fn() -> int:
        await dat()
        g
    h()
    print(g)
    g
fn main():
    let f = fn() -> int:
        doc()
    f()
    print(g)
```

Đo thật (binary dựng từ HEAD): frame `0\n3`, **nền main (không có c2) `3\n3`**, VM `3\n3`.
Debug tạm (đã khôi phục nguyên văn) cho stack `flags=[false, true, false, true]`,
`position=1` ⇒ chỉ `main` được sync, `doc` bị bỏ. Đề nghị: sync theo `!cf.by_value` (hoặc thêm
cờ nguồn env) thay vì tiền tố, và thêm ca `w24` khoá kịch bản này. **P2** (sai âm thầm, hiếm,
nhưng là regression so với nền).

**P2-2 (nợ sẵn có, không do bản vá gây ra)** — hàm tên **qua giá trị tạo trong closure**:
`let h = tang` lấy `env = env closure`; `call_value` đặt `by_value=false` (tên khác rỗng), rồi
`merge` chép ngược vào snapshot closure. Kết quả frame `1\n1\n1` còn VM `0\n0\n1`; nền main
cũng `1\n1\n1` ⇒ không phải hồi quy của c2, nhưng `w23` chỉ khoá trường hợp `let h = tang` ở
`main`. Cần lane riêng cho quy tắc env của hàm tên qua giá trị trong closure.

## 3. Test — 5 ca w19–w23

Chạy thật qua API production (`vis run`, frame path) và VM, W=1; cũng chạy lại bộ
`cargo test -p vis-conformance --test lib_api_it st317b_worker_orchestrate` = **25/25 PASS**,
0 ignore. Stdout literal:

| Ca | Nội dung | frame-interp | frame-VM | exit/diag |
|---|---|---|---|---|
| w19 | closure ghi global, không xuyên | `5\n0\n` | `5\n0\n` | 0 / rỗng |
| w20 | closure đọc sau ghi + task ghi global | `5\n5\n10\n10\n3\n` | giống hệt | 0 / rỗng |
| w21 | closure `await` rồi ghi global sau resume | `5\n3\n` | `5\n3\n` | 0 / rỗng |
| w22 | closure trong thân task ghi global | `5\n0\n1\n` | `5\n0\n1\n` | 0 / rỗng |
| w23 | hàm tên qua giá trị ở `main` vẫn xuyên | `1\n` | `1\n` | 0 / rỗng |

W=4: w19/w21/w22/w23 giữ nguyên parity; w20 báo `VS0509` ở **cả hai** engine vì task `dat` ghi
global — đúng luật `w2`, không phải lỗi của vá.

**P3-2**: `w19`, `w20`, `w23` không assert `diagnostics.is_empty()` (chỉ exit 0), và `w20`
assert VM bằng stdout interp thay vì literal riêng của VM. Nên bổ sung để đúng tiêu chí
"literal từng engine + diagnostics rỗng".

## 4. Cổng độc lập

| Cổng | Kết quả | Log / hash |
|---|---|---|
| `cargo test -p vis-interp` | **55 passed + 2 doc-test** | — |
| `cargo test -p vis-conformance --test lib_api_it st317b_worker_orchestrate` | **25/25** | — |
| `./vis-dev gate --fast` | **PASS**, L0 OK, 89 nhóm test `ok` | `/home/ps/vislogs/review-st319c2-gate.log` · `b9a1d9c2…b8a6` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`**, `chay=94425`, 126,1 s, seed `3523158054` | `review-st319c2-diff.log` · `b0927fca…6237` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** | `review-st319c2-clippy.log` · `f2066ca2…49ec` |

Toolchain dùng đúng `1.100.0` (PATH `~/.rustup/toolchains/1.100.0-…/bin`); `cargo`/`rustc` mặc
định trên PATH là 1.93.1 **không** dựng được workspace — ghi để tránh kết luận FAIL oan.

## 5. Hợp nhất

| Phép merge | Kết quả | Ghi chú |
|---|---|---|
| `merge-tree 5d1d12e8 HEAD` (319a F2) | **sạch**, tree `00c80ed5` | chỉ thêm journal REVIEW 319a, sửa journal/test 319a, `vis-loader/cache.rs`, `vis-vm/tests/diff_100k.rs`; **không** đụng tệp c2 |
| `merge-tree HEAD main` (`2ad4ac93`) | **sạch**, tree `2c50f7d4` | `STATE.md`, `QUEUE.md`, docs, `crates/vis-interp/src/frame.rs` (rustdoc 319b), test/cache 319a — không xung đột |
| `merge-tree HEAD 2e243561` (main cũ) | **sạch**, tree `4a0d08e6` | mốc yêu cầu; main đã tiến thêm trong lúc review |

Nhánh `plan/st319a` đã bị dọn (worktree `st319a` không còn) sau khi main merge 319a; dùng
hash `5d1d12e8` thay ref. Dự kiến xung đột `STATE.md`/journal **không** xảy ra ở cả ba phép thử.

## 6. Rủi ro đã kiểm

| Rủi ro | Kết quả |
|---|---|
| Closure trong `main` ghi global | không xuyên (w19); W=1/4 ổn |
| Closure trong task ghi global | chỉ snapshot task (w22); W=1/4 ổn |
| `await` resume làm mất dirty/sai snapshot | w21 đúng `5\n3`; `nap` chỉ sync khung ngoài |
| Hàm tên gọi trực tiếp trong closure ghi global | frame `0,0,1` = VM `0,0,1` (probe `named_write_async`) |
| Hàm tên **qua giá trị** ở `main` | xuyên đúng (w23) — khoá heuristic |
| Hàm tên **qua giá trị trong closure** | **P2-2**, lệch VM (nền cũng lệch) |
| Khung hàm tên trên closure bị bỏ sync | **P2-1**, regression `0\n3` vs nền `3\n3` |
| `#203` task/main ghi global | giữ (w11/w14/w15/w16/w18, probe `task_write=1`) |
| `#206` mặc định non-frame | không đổi; sync closure `5\n0` |

## 7. Đề nghị

1. **P2-1 phải vá trước merge**: đổi `nap_vao_env` để không bỏ sót khung `by_value == false`
   nằm trên closure (hoặc tách cờ "env dẫn xuất globals"); thêm test hồi quy.
2. **P2-2** mở nợ riêng cho quy tắc env của hàm tên qua giá trị trong closure (không chặn lát
   này vì là hành vi sẵn có trên `main`).
3. P3: sửa tiền tố commit `[ST-319c2]`; bổ sung assert `diagnostics.is_empty()` cho w19/w20/w23
   và literal VM riêng cho w20.
4. Sau khi vá P2-1: chạy lại `gate --fast` + `vis-diff 100k` + `st317b` trước khi A0 chốt.

## 8. Addendum F1 (`0caa9184`) — vá P2-1

> **Kết luận addendum: PASS** — **P2-1 đã đóng**; repro gốc `3\n3` cả frame-interp và frame-VM;
> `st317b` **26/26** (thêm `w24`); gate/diff/clippy xanh; merge-tree với main sạch.
> P2-2 (hàm tên qua giá trị *trong* closure) **vẫn còn nhưng là nợ sẵn có trên nền**, không do
> F1 gây ra và không chặn phạm vi vá P2-1.

### 8.1 Phạm vi

`672ccf5b..HEAD` đúng 1 commit `0caa9184` (`[ST-319c2] F1: nap_vao_env sync frame ham ten + test`),
3 tệp:

```
M agent/JOURNAL/st319c2-frame-closure-ghi-global.md
M crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs
M crates/vis-interp/src/frame/worker.rs
```

Không chạm `vis-vm`/`vis-bytecode`/`vis-core/value`/`vis-hir`/`vis-diag`/`vis-types`; working tree
sạch. (P3-1 cũ vẫn còn: 2 commit nền mang nhãn `[ST-319a]`.)

### 8.2 Logic

`nap_vao_env` bỏ **tiền tố** `calls[..position(by_value)]`, đổi thành lọc từng khung:

```rust
frame.calls.iter().filter(|cf| !cf.by_value).map(|cf| cf.env.clone())
```

⇒ chỉ khung closure snapshot (`by_value`) bị bỏ; **mọi** khung hàm tên/thường — kể cả khung hàm
tên nằm *trên* closure — nhận sync F2/F3. Đúng chẩn đoán P2-1: khung hàm tên do `dispatch` dựng
env từ `globals`, không sống trên snapshot closure. Closure by-value vẫn được bảo vệ (w19/w20/w23
giữ nguyên).

### 8.3 Repro P2-1 + test

- `stale.vis` (bản gốc của audit: closure `f` → hàm tên `doc` → closure `h` có `await dat()` →
  `doc` đọc global): trước F1 frame `0\n3`; **sau F1 frame `3\n3`**, VM `3\n3` (nền cũng `3\n3`).
- `w24_frame_w1_ham_ten_tren_closure_nap_van_sync`: `doc()` gọi từ closure, tự `await dat()`, đọc
  `g`; assert literal **`3\n3\n`** cho **cả** interp + VM frame W=1, `diagnostics` rỗng, exit 0,
  `so_khop`; không `#[ignore]`.
- w19/w20/w23 đã thêm `diagnostics.is_empty()`; w20 assert stdout VM bằng literal riêng ⇒ đóng P3-2.
- `cargo test ... st317b_worker_orchestrate`: **26/26 PASS** (392 filtered out), 0 ignore.
- Output giữ nguyên: w19 `5,0`; w20 `5,5,10,10,3`; w21 `5,3`; w22 `5,0,1`; w23 `1` (frame = VM).

### 8.4 Cổng độc lập

| Cổng | Kết quả | Log / hash |
|---|---|---|
| `./vis-dev gate --fast` | **PASS**, L0 OK, fmt/clippy/R1..R10 OK | `/home/ps/vislogs/review-st319c2-f1-gate.log` · `a76e02c0…` |
| `cargo nextest run --workspace` | **3921/3921 PASS, 4 skipped**, 79,8 s | `review-st319c2-f1-nextest.log` · `24bc025d…` |
| gate fallback `cargo test --workspace` | **3926 passed, 5 ignored** (gồm 5 doc-test; nextest không có trên PATH mặc định) | `review-st319c2-f1-gate.log` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`**, `chay=94425`, 95,5 s, seed `3523158054` | `review-st319c2-f1-diff.log` · `4d6c3c86…` |
| clippy `--workspace --all-targets -D warnings` | **PASS** | `review-st319c2-f1-clippy.log` · `2bc683fb…` |

Số **3921/3921** xác nhận bằng nextest trực tiếp (`cargo-nextest 0.9.152`); gate chạy fallback
`cargo test` khi nextest không có trên PATH nên tổng 3926 gồm thêm 5 doc-test — không phải sai lệch.

### 8.5 Merge-tree

`merge-tree HEAD main` với main **`e4c9f6f9`** (và cũng là main hiện tại): **sạch**, tree
`521f8906` (`ec=0`). Tệp khác biệt dự kiến từ main: `STATE.md`, `QUEUE.md`, `ROADMAP.md`,
`DECISIONS*.md`, journal 319a/319b/319c/VD-300/VD-302, `bin/vis-targets`,
`crates/vis-interp/src/frame.rs` (rustdoc 319b), test/cache 319a — **không** xung đột với 3 tệp F1.

### 8.6 Tồn đọng

- **P2-2 (không chặn F1)**: hàm tên qua giá trị tạo *trong* closure vẫn lệch VM do `merge` chép
  ngược vào snapshot. Write-only: frame `1,1,1` vs VM `0,0,1`; read-only + await: frame
  `903,203,303,403` vs VM `903,200,300,403`. F1 cho kết quả **trùng nền main** ở cả hai ca (nền
  cũng lệch VM), nên không phải hồi quy của F1; cần lane riêng cho quy tắc env hàm tên qua giá
  trị (kèm test `w25`).
- P3-1 (nhãn commit nền `[ST-319a]`) vẫn mở; không ảnh hưởng kỹ thuật.

### 8.7 Đề nghị

1. **Chấp nhận PASS cho F1/P2-1** và merge `plan/st319c2`.
2. Mở nợ riêng P2-2 (hàm tên qua giá trị trong closure) trước khi tuyên bố đóng trọn `#206`.
