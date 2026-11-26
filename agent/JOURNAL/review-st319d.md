<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# REVIEW ST-319d — codegen tập `mutable` + `push` ghi ô mọi vị trí (`#208`)

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Gatekeeper (Agent 1)
- **Nhánh**: `plan/st319d` · **Worktree**: `/home/ps/viswt/st319d`
- **HEAD audit**: **`c9521ea7`** — `[ST-319d] merge main + resolve test c2/d (32 test)`,
  parents `73264909` (sản phẩm) + `d2b2b69a` (main, sau merge ST-319c2 `#209`)
- **Commit sản phẩm trước merge**: `73264909` (F1: bump `PHIEN_BAN` + repro); nền `fd11558b`
  (merge `main e4c9f6f9`), gốc `7f209dfb` (ST-319a)
- **Kết luận**: **PASS** — 0 P1, 0 P2; 4 P3 (không chặn). Không sửa mã sản phẩm; chỉ thêm tệp
  nhật ký này.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Phạm vi diff (vs main `d2b2b69a`) | **ĐÚNG 6 tệp** — 3 codegen + 1 test + `cache.rs` + journal; không chạm crate cấm |
| Resolve conflict merge | **Sạch, thuần cộng** — chỉ test `st317b`, 0 xoá mỗi phía, **32 test** không trùng tên |
| `./vis-dev gate --fast` (cây gộp) | **PASS**, `3929/3929` (4 skip) |
| `VIS_DIFF_N=100000` | **`lech=0`**, `chay=94425`, 82,0 s, seed `3523158054` |
| clippy workspace all-targets `-D warnings` | **PASS** |
| `cargo test -p vis-loader -p vis-bytecode` | **136 PASS** (107 + 29) |
| `st317b_worker_orchestrate` | **32/32 PASS**, gồm 6 ca `w19..w24` của d **và** 6 ca `*_frame_w1_*` của c2 |
| Cache hash | pin **`27a1e2c2…`**, `assert_ne` **`de308705…`**, test `khoa_cache_ghim` **ok** |
| VBC1 | `VERSION = 2` **không đổi**, `agent/API-VBC1.txt` **rỗng diff** |
| Lỗi thật | không có |

## 1. Phạm vi, commit

`git status` sạch ở cả `73264909` (trước merge) và `c9521ea7` (sau merge, trước khi thêm journal
này). `git diff --stat d2b2b69a..HEAD`:

```
 agent/JOURNAL/st319d-208-codegen.md                | 145 +++++++++++++++++++++
 crates/vis-bytecode/src/codegen.rs                 |  20 ++-
 crates/vis-bytecode/src/codegen/cau.rs             |  42 ++-----
 crates/vis-bytecode/src/codegen/expr.rs            |  23 +++-
 .../async_api_it/st317b_worker_orchestrate.rs      | 134 +++++++++++++++++++++
 crates/vis-loader/src/cache.rs                     |  14 +-
 6 files changed, 340 insertions(+), 38 deletions(-)
```

`e4c9f6f9` và `1ec1105a` (merge REVIEW c2) đều là tổ tiên của HEAD. Không có thay đổi ở
`vis-vm`, `vis-interp`, `vis-core/value`, `vis-hir`, `vis-diag`, `vis-types`, SPEC/QUEUE/DECISIONS.

**Resolve conflict (F1 merge `d2b2b69a`)**: đúng 1 conflict add/add cuối
`crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs` giữa block
`ST-319c2 (#206)` (6 test `w19..w24_frame_*`) và block `ST-319d (#208)` (6 test push). Kiểm máy:

- `git diff 73264909..HEAD -- <test>`: **0 dòng xoá**, chỉ +6 hàm `w19..w24_frame_w1_*` (bản c2);
- `git diff d2b2b69a..HEAD -- <test>`: chỉ +6 hàm `w19..w24_push_*` (bản d);
- 32 tên hàm `w*`, `uniq -d` **rỗng**, 0 marker conflict; `crates/vis-interp` giữ nguyên bản main.

## 2. Kiểm code (`#208`)

| Điểm | Xác nhận |
|---|---|
| `Gen.mutables: HashSet<u32>` | khởi tạo rỗng; nạp ở **3+1 điểm**: `var` cấp tệp (vòng cấp ô global), `HStmt::Var` khi phát câu lệnh, `var self` của hàm, `var self` của closure |
| `HStmt::Var` single-symbol | đúng — `HStmt::Var` chỉ có `sym: u32` (không destructuring) ⇒ không sót biến `var` nào; guard `sym != 0` khớp cả 2 chỗ |
| Điều kiện writeback | `HExpr::Call` + callee `Field{base: Var, name=="push"}` + `mutables.contains` ⇒ `LoadLocal(slot)` thay `expr(base)`; **Assign ngay sau `CallMethod`** rồi `LoadLocal` giữ hợp đồng 1 giá trị; `tach_dich` (vm.rs) thấy Assign kề CallMethod ⇒ In-place O(1) |
| Không ghi bừa | base không `Var`, `?.push` (nhánh optional riêng phía trên), `insert`/method khác, `let`/param thường đều **không** trong `mutables` ⇒ không writeback (w21, w24 khoá) |
| Bỏ đặc cách `HStmt::Expr` | đúng — mọi biểu thức đi `expr() + Pop`; biểu thức cuối/đối số/`let` dùng chung một đường; không còn nhánh ghi cho `let` |
| Thứ tự nạp `mutables` | file-level `var` nạp **trước** khi phát thân hàm (pre-scan) ⇒ hàm/closures dùng trước vẫn thấy; local `var` nạp theo thứ tự phát câu lệnh — hợp lệ vì dùng-trước-khai là lỗi checker (xem P3-2) |
| Cache | `PHIEN_BAN` thêm `+st319d-push-mutable-2026-11-27`; `VERSION` VBC1 giữ 2; `API-VBC1.txt` rỗng diff; test ghim `27a1e2c2…` + `assert_ne` `de308705…` |
| Phí | receiver `var` đọc thẳng ô (không charge `E_EXPR`) — khớp `lay_o` của interp; case khác giữ nguyên |

## 3. Test — 6 ca `w19..w24` + 32/32

Chạy thật trong gate (không `#[ignore]`, 0 ignore cả tệp); literal dot-chấm từng engine:

| Ca | Nội dung | interp | VM | exit |
|---|---|---|---|---|
| w19 | `var` push là biểu thức cuối `async fn` (+frame W=1) | `[1, 2]` | `[1, 2]` | 0 |
| w20 | hàm sync `xs.push(2)` | `[1, 2]` | `[1, 2]` | 0 |
| w21 | `let` + push câu lệnh ⇒ **không** ghi | `[1]` | `[1]` | 0 |
| w22 | `let y = xs.push(2)` trên `var` | `[1, 2]`×2 | `[1, 2]`×2 | 0 |
| w23 | W=1 `[1, 2]`; W=2/4 `VS0509` **cả 2** engine | — | — | 1 |
| w24 | `m.insert` chưa hỗ trợ: `VS0506` + message, exit 1 cả 2 | — | — | 1 |

Gate ghi nhận đủ **12** dòng test `w19..w24` (6 của d + 6 `*_frame_w1_*` của c2); module
`st317b_worker_orchestrate` **32/32 PASS**.

## 4. Cổng độc lập

| Cổng | Kết quả | Log · sha256 |
|---|---|---|
| `./vis-dev gate --fast` @ `73264909` | **PASS**, `3923/3923` (4 skip) | `review-st319d-gate.log` · `09a3fc4f…e05a` |
| `VIS_DIFF_N=100000` @ `73264909` | **`lech=0`**, `chay=94425`, 125,9 s | `review-st319d-diff100k.log` · `299ca435…24d1` |
| `./vis-dev gate --fast` @ **`c9521ea7`** | **PASS**, **`3929/3929`** (4 skip), 64,5 s | `review-st319d-merge-gate.log` · `f5bc7456…5c14` |
| `VIS_DIFF_N=100000` @ `c9521ea7` | **`lech=0`**, `tong=100000`, `loi_tinh=5575`, `chay=94425`, 82,0 s, seed `3523158054` | `review-st319d-merge-diff100k.log` · `5e063583…42c7` |
| clippy `--workspace --all-targets -D warnings` | **PASS** (`Finished` 26,8 s) | `review-st319d-merge-clippy.log` · `9af46a1f…02b7` |
| `cargo test -p vis-loader -p vis-bytecode` | **136 PASS** — bytecode 3 + 103 + 1; loader 24 + 5 (`cache_khong_che`) | `review-st319d-merge-crate-tests.log` · `77d7cc65…b12e` |
| `./vis-dev gate --fast` @ cây có review này (mã = `c9521ea7`) | **PASS**, `3929/3929` (4 skip) — mã không đổi, chỉ thêm `.md` | `review-st319d-final-gate.log` · `4c881ab7…` |

**P3-4 (quy trình)**: `cargo`/`rustc` mặc định trên PATH là **1.93.1** (Debian) — workspace ghim
`rust-version = 1.100.0` nên `cargo test` báo `requires rustc 1.100.0` và **exit 101**. Phải dùng
`PATH=/home/ps/.cargo/bin:$PATH` (rustup → `1.100.0`), như `vis-dev` làm. Lượt đầu của audit dính
đúng bẫy này; kết quả trong bảng trên đều chạy lại bằng 1.100.0. Người sau đừng kết luận FAIL oan.

## 5. Finding

Không có **P1/P2**. Các P3:

1. **P3-1 — `var self` + push chưa có ca test.** Patch nạp `is_var_self` (hàm + closure) nhưng
   không test (journal §5.4 tự nhận). Rủi ro thấp: đường code giống hệt `var` thường; nên thêm ca
   khi có mẫu `impl` + list thuận tiện.
2. **P3-2 — `mutables` cho local phụ thuộc thứ tự phát.** Đúng hiện tại (dùng-trước-khai là lỗi
   checker), nhưng là bất biến ngầm; một pre-pass AST sẽ bền hơn nếu sau này có đường phát lệch
   thứ tự (vd. inline/hằng). Ghi nhận, không chặn.
3. **P3-3 — Q4 `vis-diff` nhóm `PushBien` vẫn tuỳ chọn** (journal §5.3): diff 100k hiện phủ ngẫu
   nhiên, chưa có nhóm sinh chuyên `push`; nên làm commit riêng sau merge như journal đề xuất.
4. **P3-4 — bẫy toolchain** ở §4 (đã nêu): tài liệu review nên ghi sẵn PATH 1.100.0.

Ghi nhận thêm: cache `PHIEN_BAN` chỉ đổi **nhãn** (VERSION VBC1 không đổi) — đúng thiết kế
(VD-129); ca thật `[1]` → `[1, 2]` có repro 2 chiều trong journal §6 (cache cũ `dbcda781…` không
được tái dùng, sinh entry mới `a0b969c0…`).

## 6. Đĩa / môi trường

- Bắt đầu audit: `/` chỉ còn **4,8 GB** (< ngưỡng dừng 6 GB của brief, brief tưởng 12 GB) — đã
  dọn **chỉ** `target/` cũ (Nov 23, không tiến trình giữ) trong `/home/ps/vislogs/st300m2-ps-*`,
  giữ nguyên log; sau đó **8,3 GB**, đủ chạy. Không `cargo clean`.
- Mọi cổng chạy `CARGO_BUILD_JOBS=2`, `TMPDIR=/home/ps/vislogs/tmp-st319d` (đĩa).

## 7. Đề nghị

1. **Chấp nhận PASS** và merge `plan/st319d` (cây gộp `c9521ea7`, 32 test).
2. Mở nợ nhỏ P3-1 (`var self` + push) và P3-3 (`PushBien`) — không chặn.
3. Khi merge, giữ nguyên nhãn cache `+st319d-push-mutable-2026-11-27`; không đổi VBC1.
