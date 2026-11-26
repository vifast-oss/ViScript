<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-319d — `#208` codegen thiếu tập **mutable**: `push` trên `var` ghi ô ở mọi vị trí

- **Nhánh**: `plan/st319d`, nền `plan/st319a` = `7f209dfb` (đã có closure capture by-value +
  `slot_cua`/`Var`/`Lambda` của ST-319a); worktree `/home/ps/viswt/st319d`.
- **Lane**: `crates/vis-bytecode/src/codegen/**` + test async. **Không** chạm `vis-vm`, `vis-interp`,
  `vis-core/value`, `vis-hir`, `vis-diag`, `vis-types`, `vis-loader`; không sửa SPEC/QUEUE/DECISIONS/ACTIVE.
- **Chẩn đoán**: `agent/JOURNAL/st319c-208-codegen.md`; **thiết kế**: `/home/ps/vislogs/design-208-codegen.md`.
- **Quy trình**: VD-279 — commit `[ST-319d]` sản phẩm; nghiệm thu (gate/diff) ở mục §3 dưới đây.

## 1. Patch (đúng thiết kế §3–§4, không thêm opcode/tag, `VERSION` VBC1 giữ 2)

| Tệp | Thay đổi |
|---|---|
| `crates/vis-bytecode/src/codegen.rs` | `Gen.mutables: HashSet<u32>` (+ doc); init; **nạp 3 điểm**: biến cấp tệp `var` (vòng cấp ô global), `var self` của hàm, `var self` của closure |
| `crates/vis-bytecode/src/codegen/cau.rs` | `HStmt::Var`: `mutables.insert(sym)` (trừ `sym == 0`); **xoá** đặc cách `push` trong `HStmt::Expr` (−31 dòng) ⇒ mọi biểu thức đi `expr()` + `Pop` |
| `crates/vis-bytecode/src/codegen/expr.rs` | nhánh `HExpr::Call`/`HExpr::Field`: base `Var` + `name == "push"` + `mutables.contains` ⇒ `LoadLocal(slot)` thay `expr(base)`; sau `CallMethod` phát **`Assign(slot)` NGAY** + `LoadLocal(slot)` (giữ `tach_dich` O(1)) |

Bỏ qua đúng các ca không được ghi: base không `Var`, `?.push` (nhánh `optional` riêng ở trên),
`insert`/phương thức khác, `let`/param thường (không có trong `mutables`). Không đụng `slot_cua`.

Hình dạng VBC (`/home/ps/vislogs/st319d-dump.log`):
- `var xs` + `let y = xs.push(2)`: `CallMethod; Assign(0); LoadLocal(0); StoreLocal(1)` — `Assign`
  **ngay sau** `CallMethod`;
- `let xs` + `xs.push(2)` câu lệnh: `CallMethod; Pop` — **không** writeback.

## 2. Test — 6 ca mới cuối `crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs`

`w19_push_var_bieu_thuc_cuoi` (tail push var, W=1 interp↔VM **và** frame W=1 interp↔VM ⇒ `[1, 2]`),
`w20_push_var_ham_sync` (`[1, 2]`), `w21_push_let_khong_ghi_lai` (`[1]`), `w22_push_var_trong_let`
(`[1, 2]` ×2), `w23_push_var_w2_ghi_cheo_vs0509` (W=1 `[1, 2]`; W=2/4 `VS0509` cả 2 engine),
`w24_map_insert_parity` (`VS0506` + `` `map.insert()` chưa hỗ trợ ``, exit 1 cả 2). Không `ignore`.

Kết quả: 6/6 PASS (`/home/ps/vislogs/st319d-tests6.log`), cả module `st317b_worker_orchestrate`
**26/26 PASS** (`/home/ps/vislogs/st319d-st317b.log`).

## 3. Cổng đã chạy

| Cổng | Kết quả | Log |
|---|---|---|
| `cargo fmt --all` | sạch (diff chỉ 4 tệp sản phẩm/test) | — |
| `./vis-dev gate --fast` | **PASS**, 3921/3921 test (4 skip), clippy + R1..R10 + self-test OK | `/home/ps/vislogs/st319d-gate-fast.log` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** (94425 chạy, 99,1 s, seed `3523158054`) | `/home/ps/vislogs/st319d-diff100k.log` |
| `cargo clippy -p vis-bytecode --all-targets -- -D warnings` | **OK** | `/home/ps/vislogs/st319d-clippy.log` |
| `api_dong_bang` (3 ca) | **OK**; `agent/API-VBC1.txt` **không đổi** | `/home/ps/vislogs/st319d-api-vbc1.log` |
| R8 | `crates/vis-cli/tests` = **8** (≤15); tổng `.rs` cấp 1 = **39** (≤40) — **không tệp mới** | — |

sha256 log: tests6 `da7d09e760348a69087590bffaed6e0b46db0e7a1f7c41770edf13b49064b190`;
st317b `872787cb09382c734ae8f6d87a73e978511b2789bd89e974b83956c19f91c084`;
gate `e80013700cf0ff3fc48e26c1989e0abb9ad9ad452e69ccf47a8554e2fc51917d`;
diff `1cd4b07a428e173314409753b4d76ebb3dd188ffb8bd7a3d7d58ca69976caef2`;
clippy `2e97b353706fa951c28f788fcadf2d9aa963ce7a160a33b3eb5a1013b28d38ae`;
api `126b487efdc4bf541465823890e52421df49d073d4a069f2b7e743aca7bb45b3`;
repro `b99c452a6150468117e29c2deddd056781ecf2041a4a7273c914c48d8b1e87ab`;
dump `d06e87eaa3544838b61de287663b901743f8dd0add6d428f695d2797c712be23`.

## 4. Tái lập CLI (`/home/ps/vislogs/st319d-repro.log`) — `vis run` / `vis run --vm`

| Ca | interp | VM |
|---|---|---|
| a `var`+push tail task | `[1, 2]` | `[1, 2]` |
| c hàmsync `xs.push(2)` | `[1, 2]` | `[1, 2]` |
| d push câu lệnh trên `var` | `[1, 2]` | `[1, 2]` |
| e `let y = xs.push` | xs `[1, 2]`, y `[1, 2]` | y hệt |
| f push rồi `print` trong task | `[1, 2]`×2 | `[1, 2]`×2 |
| h `let`+push câu lệnh | `[1]` | `[1]` |
| j task `m.insert` | `VS0506`/`VS0501` exit 101 | y hệt |
| a W=2/4 | `VS0509` exit 101 | `VS0509` exit 101 (message khác tên biến `s14`/`g0`, đúng thiết kế) |

**Cảnh báo cache (Q5 của thiết kế)**: lần chạy đầu `c.vis` cho `[1]` vì **VBC cache cũ** cùng
`source_hash` (khoá `cache.rs` = source + `PHIEN_BAN`, không phủ danh tính codegen). Chạy lại với
nội dung khác 1 byte ⇒ đúng `[1, 2]`. Đây là rủi ro hệ thống có thật khi phát hành binary mới mà
không bump `PHIEN_BAN` (ngoài lane `vis-loader` — A0 quyết, xem §5).

## 5. Việc còn lại / đề nghị

1. **Agent 1 nghiệm thu** theo VD-279: `gate --fast` + diff 100k + 6 ca + API-VBC1 (log ở §3).
2. **Q5 thiết kế** (cache `PHIEN_BAN`, `vis-loader` — ngoài lane): cân nhắc bump nhãn ở mốc merge
   release để tránh VBC cache cũ (bằng chứng §4).
3. **Q4 thiết kế** (nhóm mẫu `vis-diff` `PushBien`): tuỳ chọn, **commit riêng** sau merge (vùng
   `DS_NHOM` chồng ST-319a) — không làm trong lát này.
4. `var self` + push (Q6): patch đã nạp `is_var_self`; **chưa có ca test** (khó dựng `impl`+list) —
   tuỳ chọn.

## 6. F1 — merge `main` + bump `PHIEN_BAN` (cache) + repro **có cache**

- **Merge `main`**: `plan/st319d` merge `main` (lúc mở `07a5e2b3`; `main` tiến thêm 2 commit
  **docs-only** `2ad4ac93` VD-302 + `e4c9f6f9` đóng `#206`/319a trong lúc làm ⇒ merge lại
  `main e4c9f6f9`). **Sạch, 0 conflict** — merge commit cuối **`fd11558b`**; F1 nằm **trên** merge
  commit đó. Nhánh này chỉ sửa `codegen/**` + test + journal + `cache.rs` nên
  `cache.rs`/`STATE.md`/`diff_100k.rs` của `main` được giữ nguyên (không hunk chồng).
  `git diff main..HEAD` cuối cùng đúng **6 tệp**: 3 codegen + 1 test + `cache.rs` + journal ST-319d.
- **Nhãn cache**: thêm `+st319d-push-mutable-2026-11-27` vào `PHIEN_BAN`
  (`crates/vis-loader/src/cache.rs`), kèm doc-comment ca thật (`[1]` thay vì `[1, 2]`).
  `khoa_cache_ghim`: ghim hash mới
  **`27a1e2c287e16db790b06b55b196ae05859188e119eda1fc36aaaab2eae8728e`**; thêm
  `assert_ne` hash cũ **`de308705a335d3da380dc8ff57b57b50f2ffccbf281af4f1a3bc45eb2e117bba`**
  (nền F2 ST-319a); giữ các `assert_ne` cũ. `VERSION` VBC1/ABI/định dạng **không đổi**.
- **Repro có cache** (không `--no-cache`, cùng `XDG_CACHE_HOME`):
  1. binary **trước 319d** (`/home/ps/ViScript/target/debug/vis`, `main 07a5e2b3`) chạy
     `run --vm c.vis` **2 lần** ⇒ `[1]` cả 2, sinh cache `dbcda781…vbc`
     (`/home/ps/vislogs/st319d-cache-old-poison.log`);
  2. binary **sau 319d** (nhãn mới) chạy **cùng cache dir** ⇒ **`[1, 2]`** ngay lần đầu, sinh
     thêm entry `a0b969c0…vbc` (`/home/ps/vislogs/st319d-cache-repro.log`) — cache cũ **không**
     được tái dùng.
- **Cổng lại** (trên cây cuối, mã trùng cây đã đo): `cargo test -p vis-loader` 24+5 PASS (log
  `st319d-cache-loader.log`); `st317b` **26/26**; `./vis-dev gate --fast` **PASS 3923/3923**
  (4 skip; clippy + R1..R10 + self-test OK); `VIS_DIFF_N=100000` **`lech=0`** (94425 chạy,
  139,0 s, seed `3523158054`); `cargo clippy -p vis-loader -p vis-bytecode --all-targets --
  -D warnings` **OK**.
- sha256 log F1 (tái sinh trên cây cuối): hash
  `7bc25d7fbd4b5df3fbf810be9c26ffddf616ba35ac162ae69314590367cca0e6`;
  poison `9b891f44dac07d141422a1a819fcfc679dc4d89b069f60296c72afec1b882c1e`;
  repro `ad1c0c5ec212fa2ddbaae00b7192b448d6ceddcc3b5040c49afb29c2700ae154`;
  loader `7007ef7a18726f0bd77893cb0a170ebe3e2aa2ab8bdb4a3c7661e7bccd7dda1b`;
  st317b `a6b90ab375a8d9d16dca833e76c5c6f6396c93aa9f03dda1688cfdb1300263ba`;
  gate `1977959dbbe6928a263463dbb4ca0849d3068160852815da771d8152f16018c8`;
  diff `6e383ae484589046ea15dbd7d49771582866432845131509aff63d38d8ba264c`;
  clippy `8abdd304d22d1693ed4a34bbccd3142973536a8958503283dfdd09d002cbe0ea`.
- ⇒ Q5 của thiết kế **đã đóng** trong lát này; Q4 (`PushBien` cho `vis-diff`) vẫn tuỳ chọn, commit
  riêng sau merge.

## 7. Merge `main` `d2b2b69a` (ST-319c2 `#209` DONE) — resolve 1 conflict thật

- `git merge main` (`d2b2b69a`, sau `1ec1105a` merge ST-319c2): **đúng 1 conflict**
  `crates/vis-conformance/tests/async_api_it/st317b_worker_orchestrate.rs` — add/add **cuối tệp**:
  `main`/c2 thêm block `ST-319c2 (#206)` (6 test closure frame `w19..w24_frame_*`), nhánh d thêm
  block `ST-319d (#208)` (6 test push `w19..w24_*`). Tên hàm **khác nhau**, không trùng.
- **Resolve**: giữ **cả hai block** theo thứ tự d → c2, mỗi block giữ header + separator riêng;
  **không đổi tên** `w19..w24` của d (log cũ còn hiệu lực); c2 giữ nguyên `w19..w24_frame_*`.
  Kết quả: `st317b_worker_orchestrate` **32 test** (`+6` so với 26 trước merge), 0 marker conflict,
  0 tên hàm trùng.
- **Docs**: `agent/STATE.md` (c2 `DONE`, `#209` đóng) và toàn bộ `agent/*` giữ **bản `main`** —
  `git diff main -- agent/STATE.md agent/SO-NO.md agent/QUEUE.md agent/ROADMAP.md agent/ACTIVE.md
  agent/DECISIONS.md` **rỗng**; chỉ journal ST-319d là tệp mới của nhánh. `crates/vis-interp`
  (frame `expr.rs`/`state.rs`) lấy nguyên bản `main`.
- **Cổng sau merge**: `st317b` **32/32** (`st319d-merge-st317b.log`, sha256
  `25cc406a6be44d3161804c0b4ec873cd82ee0029e914cd6015fc8cc11d3dc494`); `./vis-dev gate --fast`
  **PASS 3929/3929** (4 skip; clippy + R1..R10 + self-test OK; sha256
  `5427337a9b9b4474ee87ca2242ea0969f51a0c3e426df8502b01bc2a73518068`);
  `VIS_DIFF_N=100000` **`lech=0`** (94425 chạy, 85,4 s, seed `3523158054`; sha256
  `609172b10f6e1e50ab983cac862e27de1ab1e3c3b0050209162020965d6634a1`);
  `cargo clippy -p vis-loader -p vis-bytecode -p vis-interp --all-targets -- -D warnings` **OK**
  (sha256 `17ea336ccab8550d626b8fdc7e84f9f3640a28f0463505f9d0bab4e05f536480`).
  `agent/API-VBC1.txt` không đổi.
