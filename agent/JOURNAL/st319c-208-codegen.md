<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-319c — chẩn đoán `#208`: `xs.push` trong task, root cause **codegen thiếu tập mutable** (ngoài lane)

- **Nhánh**: `plan/st319c`, nền `d97773de` (`main`); worktree `/home/ps/viswt/st319c`.
- **Phạm vi**: **chẩn đoán, không sửa mã, không chạy `gate --fast`/clippy**; chỉ ghi journal.
- **Kết luận ngắn**: `#208` **không** phải lỗi frame/worker (`frame/worker.rs`, `vm/worker.rs`);
  root cause nằm ở **`crates/vis-bytecode/src/codegen/**`** — codegen chưa có tập `mutable`
  nên `push` ở **vị trí biểu thức** không phát writeback (`Assign`), còn `push` ở câu lệnh
  lại ghi lại **vô điều kiện** (kể cả `let`). Ngoài lane `ST-319c` ⇒ dừng, báo A0.

## 1. Tái lập (binary debug `vis 0.6.0`, HEAD `d97773de`)

| Ca | Nội dung | interp | VM (`--vm`, `--workers 1`) |
|---|---|---|---|
| `a` | đúng repro `#208`: `var xs=[1]`; `async fn them(): xs.push(2)`; `spawn`+`await`+`print(xs)` | `[1, 2]` exit 0 | **`[1]`** exit 0 |
| `b` | như `a` nhưng thân task xuống dòng, push vẫn là **biểu thức cuối** | `[1, 2]` | **`[1]`** |
| `c` | **hàm sync** `fn them(): xs.push(2)` (không task/async/frame) | `[1, 2]` | **`[1]`** |
| `d` | `xs.push(2)` là câu lệnh giữa `main` | `[1, 2]` | `[1, 2]` |
| `e` | `let y = xs.push(2)` trên `var xs` | xs `[1,2]`, y `[1,2]` | xs **`[1]`**, y `[1,2]` |
| `f` | trong task: push **rồi** `print(xs)` (push không phải biểu thức cuối) | `[1,2]` ×2 | `[1,2]` ×2 ✅ |
| `g` | bản nháp dùng `sleep` — **không dùng** (VS0301, cả 2 engine) | — | — |
| `h` | `let xs=[1]` + push **câu lệnh** | `[1]` | **`[1,2]`** (lệch ngược) |
| `i` | `let xs=[1]` + `let y = xs.push(2)` | xs `[1]`, y `[1,2]` | xs `[1]`, y `[1,2]` |
| `j` | `var m={...}`; task `m.insert("b",2)` | `VS0506`/`VS0501` exit 101 | `VS0506`/`VS0501` exit 101 (parity; `map.insert` chưa hỗ trợ) |
| `a` W=2 | `--workers 2` | `VS0509` exit 101 | `[1]` exit 0 (hệ quả cùng lỗi codegen) |

- `c.vis` lệch dù **không có task/async/frame** ⇒ loại trừ worker/frame.
- `f.vis` khớp 2 engine ⇒ đường frame task + `vm.globals` **đã đúng** khi VBC có `Assign`.
- Lệnh/log đầy đủ: `/home/ps/vislogs/st319c-repro.log` (225 dòng), sha256
  `74ff100e0fe710eebc89d0c1dfed3954c82115fdb1184e7d5c612361da340a8a`.

## 2. Root cause

### 2.1 VBC: thiếu `Assign` khi push là biểu thức

`a.vis`, hàm `them` (dump `vis run --vm --dump-bytecode`):

```
LoadLocal(32768)   ; global xs
Const(2)
CallMethod(push,1)
Return             ; KHÔNG có Assign(32768) ⇒ global xs vẫn [1]
```

`f.vis`, cùng hàm nhưng push là câu lệnh **không phải biểu thức cuối**:

```
LoadLocal(32768)
Const(2)
CallMethod(push,1)
Assign(32768)      ; có writeback ⇒ parity [1,2]
LoadLocal(32768)
CallBuiltin(print,1)
Return
```

### 2.2 Hai vị trí codegen

- `crates/vis-bytecode/src/codegen/cau.rs:150-182` — nhánh `HStmt::Expr` đặc cách `push`:
  phát `LoadLocal(slot)` → args → `CallMethod` → **`Assign(slot)`**.
  Nhưng (1) **chỉ** ở câu lệnh, (2) **không kiểm `var`/`let`** ⇒ `h.vis` (`let`) bị ghi lại sai.
- `crates/vis-bytecode/src/codegen/expr.rs:250-263` — nhánh `HExpr::Field` gọi phương thức:
  chỉ phát `CallMethod`, **không writeback** ⇒ `a/b/c/e` mất cập nhật `xs`.
- `grep -rn mutable crates/vis-bytecode/src` = **0 hit** ⇒ codegen chưa có tập sym mutable.

### 2.3 Tham chiếu ngữ nghĩa đúng (interp)

- `crates/vis-interp/src/interp/goi_ham.rs:41-77`: `push` trên `var` (`self.mutable`) mới ghi lại
  ô; `let`/biểu thức tạm chỉ "trả list mới" (ST-15b, §7.1).
- `crates/vis-interp/src/frame/proc_expr.rs:459-500` + `frame/proc.rs:429-437` (`method_writeback`):
  frame path cũng kiểm `frame.mutable` trước khi ghi.
- Vì codegen không có thông tin mutable, VM không thể khớp interp ở cả hai chiều
  (`var` biểu thức thiếu ghi; `let` câu lệnh ghi thừa).

## 3. Đề xuất patch (4 bước, giao `ST-319a`/lát mới — **không** thực hiện ở `ST-319c`)

1. Codegen thu tập `mutable` từ `HStmt::Var` (top-level + thân hàm; **trừ** param, mirror interp).
2. `cau.rs` (`HStmt::Expr` push): chỉ phát `Assign(slot)` khi `mutable.contains(sym)`;
   `let`/không mutable ⇒ xử lý như biểu thức thường rồi `Pop` (bỏ `Assign`).
3. `expr.rs` (nhánh `HExpr::Field` gọi `push` với base `HExpr::Var` mutable): sau `CallMethod`
   thêm `Dup; Assign(slot)` để giữ giá trị biểu thức, hoặc `Assign(slot); LoadLocal(slot)` để
   giữ handoff `tach_dich` O(1); **bỏ qua captured** (VBC1 không có `StoreCaptured` — thuộc `#206`).
4. Test tại `crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs`:
   W=1 tail-expr `var` push ⇒ `[1,2]`; `let` push ⇒ `[1]`; W=2 `VS0509` parity interp↔VM.

## 4. Cổng / bằng chứng phụ

- `st317b_worker_orchestrate`: **20/20 PASS** trên nền này (chưa có ca `push`);
  log `/home/ps/vislogs/st319c-st317b.log`, sha256
  `30c613aebcfc2087f0b5d79d3a828ee13ec67625f86da9768b5b61189528e85d`.
- `VIS_DIFF_N=100000 ./bin/vis-diff`: **`lech=0`** (100k, 150,4 s, seed `3523158054`)
  ⇒ generator **không sinh** mẫu push-biểu-thức, nên `#208` lọt lưới diff.
  Log `/home/ps/vislogs/st319c-diff100k.log`, sha256
  `ae164196bde34eae9cd250f9b13402570cf007fe9831212fb9adf66387767f91`.
- **Không chạy `gate --fast`/clippy/vòng test khác** vì không có thay đổi mã.
- **Không commit mã**, không push/merge; worktree sạch tại `d97773de` (commit journal này là
  docs-only trên `plan/st319c`).

## 5. Việc còn lại cho A0

Mở lane `crates/vis-bytecode/src/codegen/**` (hoặc chuyển `#208` sang `ST-319a`) để áp patch §3,
kèm 2–3 ca parity trong `st317b_worker_orchestrate.rs`; sau đó `gate --fast` +
`VIS_DIFF_N=100000` và đóng `#208`.
