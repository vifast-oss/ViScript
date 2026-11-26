<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-320b — Gatekeeper độc lập: env module cho hàm tên qua giá trị trong closure (`#212`)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Gatekeeper (Agent 1), **không sửa mã sản phẩm**
- **Worktree**: `/home/ps/viswt/st320b` · **Nhánh**: `plan/st320b`
- **HEAD audit**: `128cf40f` · **Nền**: `d1254b05`
- **`main` lúc audit**: local `dac4cfcb`; brief ghi `ab24770c` — merge-tree kiểm **cả hai**
- **Kết luận**: **PASS có điều kiện** cho phạm vi `#212` (red-first thật, 33/33, gate/diff/clippy
  xanh, merge-tree sạch). **Chưa merge nguyên trạng**: phát hiện **P2-1 là hồi quy mới, hẹp**
  (`const` giữ hàm tên + HOF free trong frame ⇒ trap `VS0501`), cần vá 1 dòng sync trước khi merge.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Phạm vi | **ĐÚNG** — 6 tệp: `interp.rs`, `frame/{expr,params,worker}.rs`, test `st317b_worker_orchestrate.rs`, journal; **không** chạm VM/bytecode/cache/SPEC |
| `git status` | **sạch** tại `128cf40f`; 1 commit `[ST-320b]` |
| Logic `env_module` | Sync đủ **2** điểm gán `it.env = globals` (`params.rs:20-23`, `worker.rs:104-106`); `HExpr::Var` hàm tên dùng `it.env_module()`; `call_value` tách `la_closure = c.name.is_empty()` đúng |
| `#203/#206/#209/#210` | **Không phá** — `w11/w19..w24` xanh; toàn bộ 33 test `st317b` xanh |
| `w25` | **Red-first thật** (`1,1,1` vs `0,0,1`, log khớp SHA) → **xanh**; literal cho **cả** frame-interp và frame-VM |
| `st317b` | **33/33 PASS**, 0 ignore (`399 filtered`) |
| `cargo test -p vis-interp` | **5 + 55 + 2 doc-test PASS** |
| `./vis-dev gate --fast` | **PASS** — nextest **3935/3935**, 4 skip; fmt/clippy/R1..R10 OK |
| `VIS_DIFF_N=100000` | **`lech=0`** — `chay=94425`, seed `3523158054`, 89,7 s |
| clippy `--workspace --all-targets -D warnings` | **exit 0** |
| Merge-tree `ab24770c` / `dac4cfcb` | **sạch**, tree `e4d58cc7` / `b47bbef0`, **0 xung đột** |
| Phát hiện mới | **P2-1** hồi quy hẹp `const` hàm tên + free HOF trong frame (repro §6.1); **P2-2** nợ sync HOF có sẵn (không do vá) |

## 1. Phạm vi, trạng thái, commit

- `git status --porcelain` **rỗng** tại `128cf40f` trước khi ghi journal này.
- `git log --oneline d1254b05..HEAD` = **1 commit**:
  `128cf40f [ST-320b] #212: ham ten qua gia tri trong closure dung env module (frame)`.
- `git diff --stat d1254b05..HEAD`: **6 tệp, +236/−7**:
  `interp.rs` (+18), `frame/expr.rs` (+23/−7), `frame/params.rs` (+3), `frame/worker.rs` (+2),
  `tests/async_api_it/st317b_worker_orchestrate.rs` (+83), `agent/JOURNAL/st320b-…md` (+107).
- `git diff --check` exit **0**; **0** `unsafe`; không tệp nào > 1.500 dòng (`expr.rs` 906,
  `worker.rs` 776). R8: test mới nằm trong tệp con **đã có** `tests/async_api_it/`
  (`#[path]` tại `tests/lib_api_it.rs:99-100`) ⇒ **38/40** tệp `.rs` cấp 1, `vis-cli/tests` **8/15**.
- Không chạm `vis-vm/**`, `vis-bytecode/**`, loader/cache, `PHIEN_BAN`/VBC1, `docs/SPEC*`,
  `agent/QUEUE.md`/`STATE.md`/`DECISIONS.md`.

## 2. Kiểm logic

| Điểm | Xác nhận |
|---|---|
| Hai điểm gán `it.env = globals` | `frame/params.rs:20` (đường có `main`, trước khởi tạo top-level) và `frame/worker.rs:104` (không `main`) — **cả hai** gọi `it.dat_env_module(globals.clone())` ngay sau. `grep 'it\.env *='` toàn crate chỉ có **2** dòng mã này. |
| `dat_env_module`/`env_module()` | Setter/getter `pub(crate)` (`interp.rs:226,234`); giữ `Env` là `Rc` chia sẻ, không clone map. |
| `HExpr::Var` hàm tên | `frame/expr.rs:52` chụp `it.env_module()` thay `frame.cur().env`; khớp non-frame `interp/eval.rs:103` (ST-319e). Lambda vẫn do `MakeLambda` chụp `frame.cur().env` (`proc_expr.rs:609`) — **không đổi**. |
| `call_value` tách lambda/hàm tên | `la_closure = c.name.is_empty()`; lambda: `goc = c.env`, `merge = Some(c.env)`, `by_value = true`; hàm tên: `goc = it.env_module()`, `merge = None`, `by_value = false` (`expr.rs:715-751`). Đúng đặc tả: lambda bắt theo giá trị (#206), hàm tên không bắt biến. |
| `name` rỗng ⇔ lambda | Chỉ 2 nơi tạo `Closure` trong frame: `HExpr::Var` hàm tên (`name: f.name`, không rỗng) và `MakeLambda` (`String::new()`). Không có producer thứ ba. |
| `merge = None` cho hàm tên | Ghi global vẫn xuyên nhờ `DongBo::ghi → ghi_bien_cap_tep → globals` + `nap_vao_env` (`by_value = false`); env hàm tên là **bản clone của env module tại thời điểm gọi** nên chỉ đọc/ghi đúng. `restore` tham số vẫn chạy; local `let` không rò vì env khung bị bỏ. |
| `#203` | `w11/w14/w15/w16/w18/w23` xanh; `DongBo`/`nap_vao_env` **không sửa**. |
| `#206/#209` | `w19..w22/w24` xanh; nhánh lambda giữ nguyên `merge`/`by_value`. |
| `#210` | Non-frame không đổi (`interp/eval.rs`, `goi_ham.rs` không nằm trong diff); 55 test `vis-interp` xanh. |
| `const` (xem P2-1) | Vòng `const` chạy **trước** hai điểm sync ⇒ giá trị hàm tên trong `const` chụp `env_module` cũ của `Interp::new`. Đây là lỗ hổng của cách đặt sync, không phải của `call_value`. |

## 3. Test `w25` + đối chứng

- **Red-first thật**: `/home/ps/vislogs/st320b-w25-red.log` (sha256 `85cfe180…` khớp journal
  Builder): chỉ test mới, mã cũ bị stash; panic đúng `w25` dòng `1084`:
  `left: "1\n1\n1\n"` (frame) vs `right: "0\n0\n1\n"` (VM). README log ghi 1 failed / 0 ignored.
- **Xanh**: `st320b-w25-green.log` (sha `e476d2a2…`) 1 passed; audit chạy lại:
  `review-st320b-st317b.log` — **33 passed, 0 failed, 0 ignored, 399 filtered**.
- `w25` chạy **frame interp W=1** (`run_async_with_hosts_caps`) và **frame VM W=1**
  (`run_async_with_full_hosts_caps`), assert **literal riêng từng engine**:
  `0\n0\n1\n` (write-only) và `903\n203\n303\n403\n` (read-only + `await`), thêm
  `diagnostics.is_empty()`, `exit == 0`, `so_khop` — đúng yêu cầu, không chỉ so chéo hai engine.
- Test nằm trong `st317b_worker_orchestrate.rs`, aggregator có sẵn ⇒ không thêm binary test.

## 4. Cổng máy (audit chạy độc lập)

| Cổng | Kết quả | Log · SHA-256 (audit) |
|---|---|---|
| `cargo test -p vis-interp` | **5 + 55 + 2 doc-test PASS**, 0 fail/ignore | `/home/ps/vislogs/review-st320b-interp.log` · `680d9191…ca241` |
| `./vis-dev gate --fast` | **PASS** — nextest **3935/3935 (4 skip)**, fmt/clippy/R1..R10 OK | `/home/ps/vislogs/review-st320b-gate.log` · `61e28cc2…44fb` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`**: `tong=100000 chay=94425 giay=89.7 seed=3523158054` | `/home/ps/vislogs/review-st320b-diff.log` · `14760eeb…f6377` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** | `/home/ps/vislogs/review-st320b-clippy.log` · `6f7452fb…3bfe` |
| `st317b_worker_orchestrate` (audit) | **33/33 PASS**, 0 ignore | `/home/ps/vislogs/review-st320b-st317b.log` · `35269ba2…267b` |

Tái lập: `PATH=$HOME/.cargo/bin:$PATH` (rustup **1.100.0**; `cargo` hệ thống 1.93.1 không dựng
được workspace), `CARGO_BUILD_JOBS=2`, `TMPDIR=/home/ps/vislogs/tmp-st320b` trên đĩa, không
`cargo clean`. Đĩa lúc bắt đầu/kết thúc: **16 GB** trống; `target/` worktree **4,0 GB**.
Log Builder (7 tệp) đã `sha256sum` lại, khớp **byte-for-byte** bảng §5 journal Builder.

## 5. Hợp nhất `main`

- `git merge-base main HEAD` = `d1254b05`.
- `git merge-tree --write-tree ab24770c HEAD` → tree **`e4d58cc7…`**, exit 0, **không xung đột**.
- `git merge-tree --write-tree dac4cfcb HEAD` → tree **`b47bbef0…`**, exit 0, **không xung đột**.
- Khác brief dự kiến: **không** có xung đột `STATE.md`/test. Lý do: nhánh **không sửa**
  `agent/STATE.md`; `main` từ `d1254b05` chỉ sửa docs (`ACTIVE/DECISIONS/QUEUE/ROADMAP/STATE/
  reports`) và **không** sửa `st317b_worker_orchestrate.rs` (main 32 test, nhánh 33). Vậy giữ
  **cả hai** một cách tự nhiên; không cần resolve tay.
- `git diff --stat ab24770c <tree>` = đúng 6 tệp của nhánh; journal review này thêm 1 `A` nữa.

## 6. Phát hiện

### 6.1 P2-1 (MỚI — hồi quy hẹp, phải vá trước merge): `const` hàm tên + free HOF trong frame

`frame/expr.rs:52` chụp `it.env_module()`, nhưng `run_frame_driver` đánh giá **`const` trước**
khi `dat_env_module` được gọi (`worker.rs:31-47` so với `params.rs:23`/`worker.rs:106`). Hệ quả:
`Value::Fn` của hàm tên nằm trong `const` giữ **map rỗng của `Interp::new`**. Đường gọi trực tiếp
qua frame (`H()`) không lộ vì `call_value` đọc `it.env_module()` **lúc gọi**; nhưng free HOF
(`where`/`each`/`sort_by`/…) gọi callback qua `host.call → Interp::call_value → call_closure`
dùng `c.env` ⇒ trap.

Repro tối thiểu (hợp lệ cú pháp `const_decl := 'const' IDENT ':' type '=' expr`, tyck sạch):

```viscript
var g = 1

async fn dummy() -> int:
    1

fn lay(x: int) -> bool:
    g > 0

const H: fn(int) -> bool = lay

fn main():
    let t = spawn dummy()
    await t
    print(where([1], H))
```

| Engine | `d1254b05` (main) | `128cf40f` (nhánh) |
|---|---|---|
| frame interp | `[1]`, exit 0 | **`VS0501: tên 'g' chưa có giá trị lúc chạy`, exit 101** |
| frame VM | `[1]`, exit 0 | `[1]`, exit 0 |

- Ca có ghi thêm cũng lệch: `lay` in `g` rồi `g = x` — nhánh frame trap trước khi in; nền in
  `0` rồi `[1]`.
- **Không** do `call_value`: nếu bỏ hunk `HExpr::Var` thì `w25` vẫn xanh (vì `call_value` đã đọc
  `env_module` lúc gọi) — hunk này chỉ cần cho đường HOF. Vì vậy cách vá rẻ nhất là **sync sớm**.
- **Đề xuất vá (1 dòng)**: ngay sau `let globals = …` trong `frame/worker.rs` (trước vòng `const`)
  gọi `it.dat_env_module(globals.clone());` (có thể kèm `it.env = globals.clone();` cho nhất quán;
  hai sync hiện có giữ nguyên, idempotent).
- **Test khoá**: thêm ca đọc-only như trên (const hàm tên + `where`, có `spawn/await` để ép frame)
  vào `st317b_worker_orchestrate.rs`; kỳ vọng `[1]` cả frame-interp và frame-VM.

### 6.2 P2-2 (nợ sẵn có, không do vá): free HOF ghi global không đánh `Dirty` cho frame khác

`where([1,3], tang)` trong frame: callback chạy **non-frame** qua `host.call`, ghi vào
`globals` nhưng `DongBo` không biết ⇒ env cache của frame khác (kể cả `main`) không được
`nap_vao_env`. Đo với `spawn/await` để ép frame:

| Bản | frame interp | frame VM |
|---|---|---|
| `d1254b05` | `[3] / doc()=0 / print(g)=3` (global 0, cache main 3) | `[3] / 3 / 3` |
| `128cf40f` | `[3] / doc()=3 / print(g)=0` (global 3, cache main 0) | `[3] / 3 / 3` |

Nền cũng lệch VM, chỉ **khác góc nhìn**; nhánh làm phần “global + hàm tên đọc lại” đúng hơn nhưng
chưa đóng gap. Không chặn `#212` (brief chỉ yêu cầu gọi trực tiếp), nên ghi nợ riêng — cùng họ
với `#203` nhưng cho callback qua builtin. Muốn đóng: sau work-item gọi builtin có thể chạy
callback, đánh dirty các sym top-level đã đổi (hoặc cho đọc top-level `var` fallback `globals`).

### 6.3 P3 (không chặn)

1. **Bất biến ngầm `name.is_empty()`**: `call_value` phân biệt lambda/hàm tên bằng tên rỗng.
   Hiện đúng với 2 producer, nhưng nên ghi chú/`debug_assert` nếu sau này có closure đặt tên.
2. **W>1 read-only vẫn chưa có test** (nợ P2-3 review 319e). Audit kiểm tay: write-only `0,0,1`
   khớp VM ở W=2/4; read-only + task ghi global bị `VS0509` **cả hai** engine, nhưng thông điệp
   khác tên (`s15` vs `g0`) — **có sẵn trên main**, không do vá.
3. **P2-1 của review 319e** (rò tham số khi trap sớm trong `call_hfn_than`) vẫn mở, ngoài phạm vi.

## 7. Kiểm tay ngoài `w25` (frame interp vs frame VM)

| Ca | frame | VM | Kết luận |
|---|---|---|---|
| `let h = tang` trong lambda, gọi trực tiếp (w25 GHI) | `0,0,1` | `0,0,1` | khớp |
| read-only + `await` (w25 DOC) | `903,203,303,403` | `903,203,303,403` | khớp |
| W=2/W=4 write-only | `0,0,1` | `0,0,1` | khớp |
| W=2/W=4 read-only | `VS0509` | `VS0509` | cùng trap, khác nhãn sym (có sẵn) |
| closure lồng 2 lớp gọi hàm tên | `0,1,1,1,1` | `0,1,1,1,1` | khớp |
| hàm tên qua giá trị có tham số + `let` cục bộ | `5,1,5,1` | `5,1,5,1` | khớp; tham số không rò |
| trap giữa hàm tên qua giá trị | VS0502, exit 101, không treo | VS0502, exit 101 | khớp |
| default `--h: fn() -> () = tang` | `1,1` | `1,1` | khớp |
| top-level `var h = tang` + gọi trực tiếp | `2` | `2` | khớp (nền trap — vá này sửa luôn) |
| `const` hàm tên + `where` | **trap VS0501** | `[1]` | **P2-1** |
| `var h = tang` + `where` (frame) | `[3], g=0` | `[3], g=3` | **P2-2** |

## 8. Kết luận & đề xuất

- **Phạm vi `#212`: PASS.** Nguyên nhân đúng, sửa tối thiểu, red-first thật, literal hai engine,
  33/33, gate 3935, diff 0, clippy 0, merge-tree sạch.
- **Merge nguyên trạng: KHÔNG** — vá **P2-1** (1 dòng sync `env_module` trước vòng `const` +
  1 ca test const/HOF) rồi chạy lại `gate --fast` + `st317b`; sau đó merge theo `VD-288`.
- P2-2/P3 ghi nợ, không chặn `#212`.

## 9. Việc còn lại

1. Builder vá P2-1 + thêm ca test (cùng tệp `st317b_worker_orchestrate.rs`), chạy lại cổng.
2. A0 cập nhật `#212`/`SO-NO` sau khi merge; dọn worktree/nhánh theo `VD-288`.
3. Mở nợ riêng cho P2-2 (free HOF ghi global trong frame) và P3-2 (test W>1 read-only).
