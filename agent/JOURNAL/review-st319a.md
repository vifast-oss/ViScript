<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Review độc lập `ST-319a` — closure chụp biến cấp tệp theo **by-value** (`#206`, phần lõi)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Gatekeeper (audit độc lập, không sửa mã)
- **Nhánh**: `plan/st319a` @ `ae0089c0` (`[ST-319a] WIP: closure capture by-value (checkpoint)`)
- **Nền**: `main` @ `d97773de`; `main` hiện tại `c125bc0e` (mã `5408290a` + docs `c125bc0e`)
- **Worktree**: `/home/ps/viswt/st319a` (tách khỏi repo chính)
- **Kết luận**: **PASS có điều kiện** — lõi đúng, cổng xanh; còn **1 việc bắt buộc trước merge/đóng `#206`**
  (nhãn `PHIEN_BAN` cho cache bytecode) và `#206` **chưa đóng hẳn** khi `#209` (frame/async) chưa xong.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Phạm vi | 8 tệp, +452/−8: codegen + test + journal + generator `vis-diff`; **0 chạm** `vis-vm`, `vis-interp`, `frame/worker.rs` |
| VBC1 | **không đổi**: `git diff … -- agent/API-VBC1.txt` = 0 byte; `VERSION=2`; lambda **không** chạm global giữ **bytecode y hệt** |
| Repro `#206` (đồng bộ) | đọc: trước VM `0,1` → sau `0,0` (interp vốn `0,0`); ghi: trước VM `5,5,5` → sau `5,0,5` (khớp interp) |
| Test mới | `st319a_closure_cap_tep` **3/3 PASS thật**, không `#[ignore]` |
| `gate --fast` | **PASS 3915/3915** (4 skip), fmt/clippy/R1..R10 OK |
| `vis-diff` 100k | **`lech=0`** (`chay=94425`, 111,3 s) |
| `vis test test/sem` / `--vm` | **525/525** cả hai engine, exit 0 |
| Hợp nhất `main` | `merge-tree --write-tree` **sạch**, tree `a76e1b01`, **0 xung đột**, 13 tệp từ `main` |
| Điều kiện | (1) thêm nhãn `PHIEN_BAN` (**bắt buộc**); (2) `#206` chỉ đóng cùng/ sau `ST-319c2` (`#209`) |

## 1. Phạm vi, trạng thái, VBC1

- `git status --porcelain` **sạch** tại HEAD `ae0089c0`; đúng **1** commit trên nền `d97773de`
  (`git merge-base main HEAD` = `d97773de`).
- Tệp thay đổi (`git diff --stat d97773de..HEAD`): `codegen.rs` (+1), `codegen/bien_tu_do.rs` (mới, 206),
  `codegen/cau.rs` (12), `codegen/expr.rs` (27), `lib_api_it.rs` (+3 `#[path]`),
  `misc_api_it/st319a_closure_cap_tep.rs` (mới, 123), `vis-vm/tests/diff_100k.rs` (40),
  `agent/JOURNAL/st319a-closure-cap-tep.md` (48). **Không** có `crates/vis-interp/**`,
  `crates/vis-vm/src/**`, `frame/worker.rs`.
- **R8**: `crates/vis-cli/tests/*.rs` = **8/15**; tổng tệp `.rs` cấp 1 = **39/40** (ca mới nằm trong
  thư mục con + `#[path]` — đúng luật).
- **VBC1**: `git diff d97773de..HEAD -- agent/API-VBC1.txt` rỗng; `crates/vis-bytecode/src/format.rs`
  không đổi; dump cho `VBC1 version=2`, không opcode mới. `LoadCaptured`/`Assign` là opcode **cũ**
  (đúng cơ chế capture `var` ST-60b), chỉ đổi **tập** biến được chụp.
- **Bytecode không đổi khi không chạm global**: chương trình closure chỉ bắt biến cục bộ
  (`let x = 1; let f = fn() -> int: x`) cho dump `--vm --no-cache --dump-bytecode` **giống hệt từng
  byte** trước/sau. Chương trình closure đọc global: `MakeClosure(2,0)` → `MakeClosure(2,1)` + prologue
  `LoadCaptured(0);Assign(0)` — đúng thiết kế, VBC1 vẫn 2.

## 2. Đối chứng hành vi trước/sau (binary debug `vis 0.6.0`, `--stdin`)

“Trước” = binary `main` (codegen y hệt `d97773de`; đã kiểm `git diff d97773de..main -- crates/vis-bytecode`
= 0 byte). “Sau” = binary nhánh này.

| Ca | interp trước | VM trước | interp sau | VM sau |
|---|---|---|---|---|
| đọc global trong closure, gán sau (`0,0` kỳ vọng) | `0 0` | **`0 1`** | `0 0` | **`0 0`** |
| ghi global trong closure (`5,0,5`) | `5 0 5` | **`5 5 5`** | `5 0 5` | **`5 0 5`** |
| tạo closure sau khi gán `g=2`, rồi gán `g=3` | `2 3` | **`3 3`** | `2 3` | **`2 3`** |
| vòng lặp tạo 3 closure, `g` đổi mỗi vòng | `1 2 3 100` | **`100 100 100 100`** | `1 2 3 100` | **`1 2 3 100`** |
| lambda lồng (ngoài chụp, trong chụp lại snapshot) | `0 7` | `0 7`* | `0 7` | `0 7` |
| lambda lồng **ghi** global | `9 0` | **`9 9`** | `9 0` | **`9 0`** |
| 2 closure độc lập cùng ghi `g` | `1 2 1 0` | **`1 2 3 3`** | `1 2 1 0` | **`1 2 1 0`** |
| closure bắt `x` cục bộ + `g`,`h` global | `11 4` | **`15 4`** | `11 4` | **`11 4`** |
| closure trả về từ hàm, `g` đổi sau đó | `0 9` | **`9 9`** | `0 9` | **`0 9`** |
| `g += 1` trong closure, 2 lần | `1 2 0` | **`1 2 2`** | `1 2 0` | **`1 2 0`** |
| tệp `let g = 0` + closure đọc (chỉ đọc) | `0 0` | `0 0`* | `0 0` | `0 0` |

(\* các ca vốn đã khớp trước khi vá.) Như vậy **snapshot tại điểm tạo closure** và **ghi không xuyên
global** đúng như SPEC §4.7 (“Closure bắt biến theo giá trị”) ở **mọi ca đồng bộ**, cả W=1 lẫn W=2
cho chương trình không async.

Test thật (không ignore): `cargo nextest run -p vis-conformance --test lib_api_it -E 'test(st319a)'`
→ **3 passed, 409 skipped**; 3 ca phủ đọc/ghi/lồng, mỗi ca 4 đường (interp/VM × mặc định/W=1).

## 3. Cổng độc lập

| Cổng | Kết quả | Log (sha256) |
|---|---|---|
| `./vis-dev gate --fast` (TMPDIR đĩa) | **PASS** — nextest **3915/3915** (4 skip), fmt OK, clippy OK, R1..R10 OK, doc-examples OK | `/home/ps/vislogs/review-st319a-gate.log` — `d74ea67bdab3851c2b0f3a9123bd5cb49044593649f44380307bd724049ed254` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`tong=100000 chay=94425 lech=0`** (111,3 s, seed `3523158054`) | `/home/ps/vislogs/review-st319a-diff100k.log` — `e9cf32d38d3c92e6a2c106bbf269ba121d22ff4b50230ab2edcf1b3ca54fd1dc` |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 (41,5 s) | `/home/ps/vislogs/review-st319a-clippy.log` — `601728342a2f472a1eb2dd957d269d863eea223f2a310963350ee65ebaf6df1e` |
| `vis test test/sem` | **525/525**, exit 0 | `/home/ps/vislogs/review-st319a-sem-interp.log` — `8b1cdc44e1d8a149a14cb1f2f54a75e65cf92eefbe2ed23193fdd6458078de16` |
| `vis test --vm test/sem` | **525/525**, exit 0 | `/home/ps/vislogs/review-st319a-sem-vm.log` — `8b1cdc44e1d8a149a14cb1f2f54a75e65cf92eefbe2ed23193fdd6458078de16` |

Số 3915 = nền 3912 + 3 ca mới, khớp journal Builder. Số liệu journal Builder (`3915`, `lech=0`,
clippy, `525/525`) **tái lập được**; chỉ thời gian `vis-diff` khác (80,2 s vs 111,3 s — máy có tải).

## 4. Phát hiện

### F1 — **(bắt buộc, P2/tích hợp)** thiếu nhãn `PHIEN_BAN`: bản vá không có hiệu lực với cache cũ

`ST-319a` đổi **ngữ nghĩa codegen** nhưng giữ `VERSION` VBC1 = 2 và **không** đổi
`crates/vis-loader/src/cache.rs::PHIEN_BAN` (dòng 42). Khoá cache chỉ phủ source + `PHIEN_BAN` +
`VERSION` + bảng thẻ (dòng 97–118), nên artifact do compiler **trước** bản vá sinh ra vẫn được nạp.
Đã tái lập bằng 2 binary trên cùng một tệp mới:

```
$ (binary main cũ)   vis run --vm cache_repro.vis   # ghi cache → 0 / 1
$ (binary bản vá)    vis run --vm cache_repro.vis   # đọc cache cũ → 0 / 1   ← vá vô hiệu
$ (binary bản vá)    vis run --vm --no-cache …      # → 0 / 0            ← chỉ đúng khi bỏ cache
```

Đây đúng lớp `#187` đã đóng bằng `ST-249`; repo có **quy ước thành văn**: mỗi lần đổi ngữ nghĩa
compiler mà VBC1 không đổi thì thêm nhãn logic vào `PHIEN_BAN` + assert trong test ghim
(`cache.rs` dòng 12–40 và 345+). **Điều kiện trước khi merge/đóng `#206`**: thêm
`+st319a-closure-cap-tep-2026-11-27` (hoặc tên tương đương) vào `PHIEN_BAN` + 1 assert trong test
ghim. (Sửa 1 dòng, không đổi VBC1/opcode.) `ST-319c2` sửa interp nên **không** cần nhãn này.

### F2 — **(điều kiện đóng `#206`)** residual frame/async đúng như journal: `#209`

Tái lập bằng `async fn` + `spawn/await` (đường frame; chương trình có `async` là đủ, không cần
`--workers`):

```
var g = 0
async fn viec() -> int:
    let f = fn() -> int:
        g = 5
        g
    f()
fn main() -> ():
    let t = spawn viec()
    print(await t)
    print(g)
```

| | interp | VM |
|---|---|---|
| **trước** bản vá | `5 5` | `5 5` |
| **sau** bản vá | **`5 5`** | **`5 0`** |

- Nguyên nhân đúng như journal: frame interp gọi `DongBo::ghi` (`crates/vis-interp/src/frame/worker.rs`)
  cho work-item `Assign(sym global)`, kể cả khi `sym` đã được closure chụp thành ô cục bộ ⇒ ghi xuyên
  global. **Ngoài lane 319a** (nhánh không chạm `vis-interp`), đang sửa ở `plan/st319c2` (`#209`,
  commit `ef0f213d`, lane `frame/**`).
- Lưu ý nghiệm thu: trước bản vá **hai engine cùng sai** (`5 5`); sau bản vá VM đúng SPEC còn interp
  frame sai ⇒ **parity async tạm lệch** cho tới khi `ST-319c2` vào. Vì vậy **không đóng `#206`** chỉ
  với 319a; đóng cùng `#209`, hoặc đóng phần lõi và giữ `#209` mở tường minh.

### F3 — **(ngoài lane, nợ mới đề nghị)** closure gọi **hàm cấp tệp** mà hàm đó đọc/ghi global

Không do 319a gây ra (trước/sau **giống nhau**), nhưng cùng họ “closure ↔ global” và **không** được
test mới/`vis-diff` phủ:

```
var g = 0
fn f() -> int: g
fn main():
    let d = fn() -> int: f()
    g = 9
    print(d())   # interp 0  | VM 9
    print(g)     # interp 9  | VM 9
```

Và bản ghi: `fn f(): g = 5; g` gọi trong closure ⇒ interp `5 / 0`, VM `5 / 5`. Nguyên nhân: interp
tra global của **hàm được gọi** theo chuỗi env của closure (snapshot), VM tra `Vm::globals` — theo
SPEC §4.7, hàm cấp tệp không phải closure nên VM mới đúng. Đề nghị A0 cấp nợ (lane `vis-interp`),
**không** xử lý ở 319a; khi sửa xong có thể thêm mẫu `vis-diff` (hiện thêm sẽ đỏ, nên chưa thêm).

### Quan sát nhỏ (không chặn)

- `Emit::captured` (`codegen.rs`) là map **rỗng vĩnh viễn** (không có `.insert` nào); nhánh
  `self.captured` trong `expr.rs`/`cau.rs` là code chết **có sẵn từ trước**. 319a kiểm nó trước
  `f.locals` nên vô hại, nhưng nên dọn ở dịp khác.
- Test mới ghi “W=1” nhưng chương trình **không async** ⇒ `run_workers_local` không đi scheduler
  frame; 4 đường thực chất là interp/VM × (mặc định, W=1 non-frame). Không sai tài liệu (journal đã
  nêu residual riêng), chỉ cần biết test **không** phủ `#209`.
- `agent/STATE.md` không đổi (chỉ journal) — hợp lý với checkpoint WIP, sẽ cập nhật khi A0 chốt.

## 5. Hợp nhất `main`

- `git merge-tree --write-tree main HEAD` (main `c125bc0e`, mới hơn `5408290a`) → **exit 0**,
  tree `a76e1b0193cc19a3d31405470edf2ea0367a8913`, **0 xung đột**.
- Không có xung đột `STATE.md` như dự kiến vì nhánh **không** sửa `STATE.md`; hợp nhất kéo **13 tệp**
  từ `main` (`ACTIVE/DECISIONS*/QUEUE/ROADMAP/SO-NO/STATE`, 3 journal, `bin/vis-targets`,
  `vis-interp/src/frame.rs` docs-only) — không tệp nào trùng tệp 319a sửa.

## 6. Việc A0 cần quyết

1. **Bắt buộc**: cho Builder thêm nhãn `PHIEN_BAN` + assert ghim (F1) rồi mới merge/đóng `#206`.
2. Chốt cách đóng `#206`: (a) đóng phần lõi ngay, giữ `#209` mở; hoặc (b) đóng `#206` cùng `#209`
   khi `ST-319c2` xanh — khuyến nghị (b) để không có khoảng parity async lệch trên `main`.
3. Cấp nợ cho F3 (interp tra global qua env closure khi gọi hàm cấp tệp) — lane `vis-interp`.
4. Xác nhận phạm vi hợp nhất: 319a chỉ codegen/test/journal; `#208`/`#209` vẫn ngoài.

## 7. Bằng chứng

- Log cổng: `/home/ps/vislogs/review-st319a-{gate,diff100k,clippy,sem-interp,sem-vm}.log`
  (sha256 ở §3; gate 4188 dòng).
- Repro tay: `/tmp/opencode/st319a/` (`doc.vis`, `ghi.vis`, `timing.vis`, `loop.vis`, `two.vis`,
  `nested_w.vis`, `mix.vis`, `ret_fn.vis`, `cau.vis`, `frame_ghi.vis`, `indirect_{r,w}.vis`,
  `cache_repro.vis`, dump `before/after.dump`, `cap_{before,after}.dump`).
- Không sửa mã sản phẩm; worktree sạch trước commit review này. Đĩa còn **15 GB** (> ngưỡng 6 GB).
