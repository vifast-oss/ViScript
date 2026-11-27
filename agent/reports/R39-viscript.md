<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R39 — ViScript, vòng 39 (Sóng 9 nhỏ + 4 fix + chain cao điểm 1) — **CHỐT (A0)**

> **Trạng thái: CHỐT (A0, 2026-11-27).** Báo cáo vòng 39 trên nền `main` = **`096da1b5`**
> (`[ST-00] docs: VD-306 chot Song 10`), `origin` sync, sạch. Phần chain điền lúc **2026-11-27
> 10:13 JST / 01:13 UTC**; phần merge/fix/docs điền theo snapshot `0b32c5dc` (tổ tiên). Các ô
> **CÒN TODO** là việc thật chưa làm, không phải số 0.
>
> **Snapshot cuối**: `main` HEAD **`096da1b5e233d3942120f896a1a9b4ec88a474db`**
> (`[ST-00] docs: VD-306 chot Song 10`), **đã push**, `origin/main` cùng giá trị, `git status --porcelain` rỗng.
> Sóng 9 merge **`56d349a6`** (gate **3940/3940**, **38 test**, blob **`61a82fd7`**); sau 4 fix
> `ST-321a/b/c/d` (merge `5d57d8e4` / `22d7108b` / `39effc26` / `0b32c5dc`) gate **3942/3942**,
> **40 test** (38 worker + 2 `main_sig`), diff 100k `lech=0`, clippy exit 0 — log §5. Worktree chỉ còn
> `main`, nhánh chỉ còn `main` + `plan/st309-qa` (đúng VD-288); đĩa `/home/ps` **20 G**.
> `report-check --all` **PASS: 37 báo cáo / 386 hàng QUEUE** (sau khi A0 nộp R39). Sổ sau A0
> `VD-306`: **215 = 82 mở / 114 đóng / 16 / 2 / 1**.
>
> **Chain `peak1 20261127T0100Z`**: **đã chạy xong, exit=1** — 16/18 bước PASS, **2 FAIL remote**
> (`psvn-gate-full` exit=101, `mps-gate-full` exit=1), **bench 3-OS PASS**. Lỗi đã có fix (`ST-321d` /
> `ST-321c`) nhưng **chưa re-run remote gate** — kế hoạch
> `/home/ps/vislogs/peak2-rerun-plan-20261127T0142Z.md` (sha
> `209c9a62495cb97cb458f2eeae08ee68b7b7ca6b6f740c0b86ca98d40a2e7bc9`, **chưa chạy**) ⇒ `ST-316c`
> giữ **BLOCKED**, **không tuyên bố 3-OS**.
>
> **Quy ước**: chỉ ghi số có log ngoài repo + sha256; “dự kiến” là chưa đo; không tuyên bố 3-OS,
> không tuyên bố phát hành khi chưa có log `psvn`/`mps` chạy lại sau fix. `R39-viscript.json`,
> `.license` và `report-check --all` đã do **A0** nộp (commit `[ST-00] R39`).

## 1. Bối cảnh

### 1.1 R38 đã phủ Sóng 5–8

R38 chốt trên `main` docs **`4caa600e`**, mã cuối **`ee8f252d`**; cổng cuối
`/home/ps/vislogs/final-main-ee8f252d.log` sha256 **`904d45a685367c4a15261a82536c93b17454109763c9821b91732ffcb735569f`**:
**3934/3934** (4 skip), diff 100k `lech=0` 79,7 s, clippy/rules 0, R8 **39/40**, 0 `unsafe`.

| Sóng | Nội dung | Trạng thái R38 | Mốc chính |
|---|---|---|---|
| 5 | `Value`/CoW, lowering O(N), DAG/test hygiene | `ST-316a/b/d` DONE; `ST-316c` BLOCKED | `882b7ffa`, gate **3880/3880** |
| 6 | `Value` 32 B, tách crate, `StdId` 167 | 3/3 DONE | `4e03a19c`, gate **3901/3901** |
| 7 | 9 lát R1 mềm + parity `#203/#204` | 9/9 DONE | `2cda3a68`, gate **3912/3912** |
| 8 | `#206`–`#210`, frame closure/codegen | 5/5 DONE (`319c` chẩn đoán) | `ee8f252d`, gate **3934/3934** |

WASI final2 của R38: **398×2 (371+27)** @`8c883a16` (code `ee8f252d`, `dirty=0`), sem **390/390**,
suite **28/28**, core 47/0, sqlite 45+2; log sha **`60064a5c…996bbb`**.

### 1.2 `main` cuối: Sóng 9 + 4 fix đã merge/push

- Docs R38 `4caa600e` → `8c883a16` (R38) → `d1254b05` (citation) → `17359848` (`VD-304`) →
  `977a61b0`/`ab24770c`/`dac4cfcb`/`5ae018a3` (vá R38) → `417ef426` (`320a/b` REVIEW + chain armed) →
  `d37cab2a` (tip/audit) → `ff585af8` (mốc lập draft/chain) → `3bb8fe5c` (`[ST-00]` đóng P3 + đóng số
  `#195`) → `298714cf` (`[ST-00]` VD-305 chốt Sóng 9 + numbering `#211..#215`).
- Sóng 9: `bbbb99be` (merge ST-320a) → `2ef50717`/`05dd131e` (ST-320b F2, 38 test) → **`56d349a6`**
  (`[REVIEW] merge ST-320b`, PASS `05dd131e`, gate **3940**).
- 4 merge fix (Sóng 10): **`5d57d8e4`** (ST-321a, `#200/#201`), **`22d7108b`** (ST-321b, runner
  568/568), **`39effc26`** (ST-321d, `cfg(unix)` `Command`), **`0b32c5dc`** (ST-321c, errno 61/111).
- **HEAD cuối `0b32c5dc0ff867e538cff1f66311dbcf731c4a79` = `origin/main`**, sạch, đã push. Gate trên
  cây gộp sau 4 fix: **3942/3942** (4 skip), **40 test**, diff 100k `lech=0`, clippy exit 0 — log §5.
  Chain `01:00Z` chạy trên `ff585af8`, tức **trước** các merge/fix này. Đĩa `/home/ps` 20 G;
  worktree/nhánh chỉ còn `main` (+ `plan/st309-qa`); `report-check --all` **37 / 386** (sau R39).

### 1.3 Chain cao điểm 1 — **đã chạy xong, exit=1 (có FAIL)**

| Hạng mục | Giá trị |
|---|---|
| Target | epoch **`1795741200`** = **`2026-11-27T01:00:00Z`** (10:00 JST / 08:00 ICT) |
| Script | `/home/ps/vislogs/peak1-2026-11-27-chain.sh`, 25.762 B, 674 dòng, sha256 **`e239c317b29f6a0b4e75167d423889e659860bceaec15ca448c15a4aa593fa1c`** |
| Launcher | log `/home/ps/vislogs/peak1-launch-2026-11-27.log`, **4.982 B**, sha256 **`7c6ba5df28d967948119fde3d217a6db0b634ab3d7f59433559c2aa39d738399`** (đã append `TONG KET`; sha tạm `0adce422…c96712` **không còn đúng**) |
| Thư mục log | `/home/ps/vislogs/peak1-20261127T0100Z/` (27 tệp log/json) |
| Bắt đầu / kết thúc | `start 2026-11-27T01:00:01Z`, `HEAD=ff585af8…`, `origin/main=ff585af8…`, `dirty=0`; `psvn` KET `01:04:25Z`, `mps` KET `01:05:43Z` |
| Kết quả thật | **18 bước: 16 PASS / 2 FAIL**; `psvn-gate-full` **exit=101**, `mps-gate-full` **exit=1**; `psvn`/`mps` đều **probe PASS + sync PASS** (không phải SKIP) |
| Exit tổng | **1** (có FAIL thật) — **không phải 3-OS**; dự kiến cũ `3` (SKIP) **không đúng** |

### 1.4 Sóng 9 → Sóng 10 — đã merge, đã push

| Lát | Merge / PASS | Nội dung | Test | Gate | Kết luận |
|---|---|---|---:|---:|---|
| `ST-320a` | `bbbb99be` / PASS `2f60d023` (code `f133aea6`) | `var self` + `push` trường — **test-only**, 0 dòng sản phẩm | 35 nhánh / **38 cây gộp** | **3940** | DONE `VD-305`; `#211` đóng |
| `ST-320b` | `56d349a6` / PASS `05dd131e` (F1 `7c807ba8`, F2 `985b0883`) | frame `env_module` + F1 `const`/HOF + F2 w27 | **38** (32+3+3), blob `61a82fd7` | **3940** | DONE `VD-305`; `#212` đóng |
| `ST-321a` | `5d57d8e4` / code `aacd749e` | tyck chữ ký `main` `VS0412`/`VS0401` | corpus **573/573** | **3942** | **`#200/#201` đóng** |
| `ST-321b` | `22d7108b` / code `501964cc` + journal `d28fccc5` | vistest `.trap/.out_prefix/.exit`; runner **568/568** | conformance 568 | **3942** | xong |
| `ST-321c` | `0b32c5dc` / code `34e71725` | normalize errno `61`/`111` (`mps-gate-full`) | target 1/1 | **3942** | fix có; **cần re-run `mps-gate-full`** |
| `ST-321d` | `39effc26` / code `025e9750` | `cfg(unix)` import `Command` (`psvn-gate-full`) | `test_vis_cli` 101→0 | **3942** | fix có; **cần re-run `psvn-gate-full`** |

Gate **3942/3942** (4 skip), **40 test** (38 `st317b_worker_orchestrate` + 2 `lang_it/main_sig.rs`) là số
đo trên cây gộp sau 4 fix (§5). Chain `01:00Z` vẫn ghi kết quả trên `ff585af8`; không được sửa thành
PASS dù fix đã vào `main`. Re-run `psvn-gate-full`/`mps-gate-full` theo
`/home/ps/vislogs/peak2-rerun-plan-20261127T0142Z.md` — **chưa chạy**.

## 2. Sóng 9 — chi tiết và bằng chứng

### 2.1 `ST-320a` — `var self` + `push` trường (test-only)

- Diff `d1254b05..f133aea6`: **2 tệp, +227** — 1 test aggregator (+142) + 1 journal (+85);
  `git diff … -- crates/vis-bytecode crates/vis-interp crates/vis-vm crates/vis-runtime` = **0 dòng**.
- 3 ca mới `st320a_var_self_*` (mỗi ca chạy **6 đường**: interp/VM mặc định + W=1 non-frame +
  frame W=1), assert stdout literal + `exit = 0` + `diagnostics` rỗng từng đường.
- **Red proof** `is_var_self`: bỏ `st.g.mutables.insert(p.sym)` ⇒ ca 3 **đỏ** ở VM; khôi phục ⇒ xanh.
- Cổng trên nhánh: **3937/3937** (4 skip), `st317b` **35/35**, diff 100k **`lech=0`** (140,0 s, seed
  `3523158054`), clippy **exit 0**, R8 **39/40**. Trên cây gộp: **38 test**, gate **3940**.
- Chốt ngữ nghĩa: `push` trên **trường** (`self.xs.push`) là biểu thức tạm, **không rebind** —
  interp (oracle) và VM đã thống nhất; nếu A0 đổi ngữ nghĩa thì 2 test sẽ đỏ **có chủ đích**.

### 2.2 `ST-320b` — frame env module + F1 `const`/HOF + F2 w27

- Phạm vi `d1254b05..8b164d48`: 7 tệp, +633/−7 — `interp.rs` (+18), `frame/expr.rs` (+23/−7),
  `frame/params.rs` (+3), `frame/worker.rs` (+9), test (+116), 2 journal (184 + 280); **0** chạm
  VM/bytecode/loader/cache/SPEC.
- Logic: `HExpr::Var` hàm tên dùng `it.env_module()`; `call_value` tách
  `la_closure = c.name.is_empty()` — lambda giữ `goc = c.env`/`by_value = true`, hàm tên dùng
  `goc = it.env_module()`/`merge = None`.
- `w25` red-first thật (`1,1,1` vs `0,0,1`) → xanh; literal cho **cả** frame-interp và frame-VM.
- **F1 (`7c807ba8`)**: sync `it.env = globals.clone(); it.dat_env_module(globals.clone())` ngay sau
  `let globals` trong `frame/worker.rs`, **trước** vòng `const`; vá P2-1 (const giữ hàm tên +
  free HOF trong frame trap `VS0501`/exit 101 → `[1]`/exit 0); thêm `w26` (frame interp W=1 +
  frame VM W=1), red-first có log `st320b-f1-w26-red.log`. Cổng F1: **3936/3936**, `st317b` **34/34**.
- **F2 (`985b0883`)**: HOF callback ghi global qua `DongBo` — thêm `w27` (W>1 read-only/HOF ghi global),
  hợp nhất với `ST-320a` thành **38 test** (32+3+3; w25/w26/w27). Merge `56d349a6` (PASS `05dd131e`),
  gate **3940**, blob test **`61a82fd7`**, diff 100k `lech=0`.

### 2.3 Merge Sóng 9 — **đã xong** (`56d349a6`), gate 3940

| Hạng mục | Kết quả thật |
|---|---|
| Thứ tự + conflict | `plan/st320a` → `bbbb99be`, rồi `plan/st320b` → `56d349a6` (`--no-ff`); resolve **1 conflict** `st317b_worker_orchestrate.rs` (**3 hunk**, giữ cả 2 khối) |
| Test `st317b` cây gộp | **38/38** (32 + 3 + 3; w25/w26/w27) |
| Tệp test gộp | blob **`61a82fd7b0ca342828e334f32bdeafd17da76dde`** (43.477 B) — kiểm bằng `git cat-file`/`git ls-tree HEAD` |
| Gate cây gộp | **3940/3940** (4 skip), 38 test; log `final-main-song9-main-gate-20261127T0115Z.log` sha `3ec597f5…2631d93` |
| Diff 100k / clippy | `lech=0` / exit 0 (log §5) |
| Merge commit | `bbbb99be` (320a), **`56d349a6`** (320b) — `head` R39 = `0b32c5dc`, không còn TODO |
| Sau 4 fix `ST-321a/b/c/d` | gate **3942/3942** (4 skip), **40 test**; diff 100k `lech=0`; clippy exit 0; log `final-main-st321{abcd}-*` (§5) |

Nguồn kế hoạch: `/home/ps/vislogs/merge-plan-320-F1-20261126T2217Z.md` (462 dòng, sha256
`4e003ba0…063841`) — dự kiến cũ **3939 / 37 test / blob `d04197b2`** đã bị ghi đè bởi số thật
**3940 / 38 / `61a82fd7`** (nhờ ca `w27` F2). Sau 4 fix gate lên **3942** (§5).

### 2.4 Nợ còn lại sau Sóng 9 / Sóng 10

| Mã | Mức | Nội dung | Trạng thái cuối |
|---|---|---|---|
| `#200` | P2 | `fn main(a: int, b: int)`: interp `VS0501`/exit 101 vs VM exit 0 với `b = ()` | **Đóng ở `ST-321a`** (merge `5d57d8e4`, `VS0412`/`VS0401`) |
| `#201` | P3 | `fn main(a: int)` gán `argv` vào tham số `int` không kiểm kiểu | **Đóng ở `ST-321a`** (merge `5d57d8e4`) |
| P2-2 | P2 | Free HOF ghi global trong frame không đánh `Dirty` cho frame khác | có ca `w27` trong F2 (`985b0883`); phần còn lại proposal-only |
| P3-2 | P3 | W>1 read-only ca mới | `w27` đã dùng; proposal-only, chờ A0 |
| P3-1 | P3 | Bất biến ngầm `name.is_empty()` trong `call_value` | proposal-only |
| P2-1 (319e) | P2 | Rò tham số khi trap sớm trong `call_hfn_than` | Mở, ngoài phạm vi 320b |

### 2.5 Sổ `SO-NO.md` — đã chốt tại VD-305, cập nhật VD-306

- **§8 cuối (sau A0 `VD-306`): `215 = 82 mở / 114 đóng / 16 VS3 / 2 không làm / 1 điều hướng`**
  (82+114+16+2+1 = 215). Công thức: `VD-305` chốt `84/112`, rồi `#200`/`#201` `mở → đóng` ở
  `ST-321a` ⇒ `84−2=82`, `112+2=114`; tổng dòng 215 giữ nguyên. `84/112`, `84/113`, `85/111` là số
  lịch sử — **không lưu hành**.
- Nguồn: `VD-305` commit **`298714cf`** (`[ST-00] docs`) + patch `#195` commit **`3bb8fe5c`**
  (`[ST-00] docs`: `#195` đóng, nền `210 = 82/109` → `81/110`); numbering `#211..#215` theo ballot R1
  `b2fc7e12…`; cập nhật `VD-306` trong `SO-NO.md` dòng "Cập nhật" + §8 tại repo HEAD.
- `#211` (ST-320a) và `#212` (ST-320b/F2) **mở → đóng**; `#213`/`#214`/`#215` **mở** theo ballot R1.
- `#200`/`#201` **đóng ở `ST-321a`** (merge `5d57d8e4`, `VS0412`/`VS0401`). `#195` **đóng** ở `ST-311a/H4`.
- P2-2 / P3-2 / P3-1: proposal-only, không tự cấp số (đã có ca `w27` cho phần F2, phần còn lại chờ Sóng 10).

## 3. Chain cao điểm 1 — kết quả thật

### 3.1 Ý nghĩa exit của chain

| Exit | Nghĩa | Chain này |
|---:|---|---|
| **0** | Mọi bước bắt buộc PASS (đủ 3-OS) | — |
| **1** | Có **FAIL** thật của sản phẩm/bước bắt buộc | **← exit thật `1`** |
| **2** | Preflight/abort: worktree bẩn, `HEAD` không phải tổ tiên `origin/main`, fetch lỗi, đĩa `< 10 GiB` | — (preflight PASS, `dirty=0`) |
| **3** | Có **SKIP** (chưa đủ 3-OS) — **không** được tuyên bố 3-OS | — (0 SKIP) |

### 3.2 Kết quả chain — **18 bước, exit=1 (2 FAIL, 0 SKIP)**

Snapshot chain: `HEAD=ff585af8aad8156e30b288b93f2de321305f83ea`, `origin/main` cùng giá trị,
`dirty=0`; đĩa `ps` **13 GiB**, `psvn` **12 GiB**, `mps` **30 GiB** (đều không chặn);
`cargo_jobs=2`. Log tổng hợp `/home/ps/vislogs/peak1-launch-2026-11-27.log` sha256
**`7c6ba5df28d967948119fde3d217a6db0b634ab3d7f59433559c2aa39d738399`**. Cột NOTE chép nguyên
`TONG KET`; `timeout …s` là **hạn mức cấp cho bước**, không phải bị quá hạn.

| # | STEP | EXIT | NOTE (nguyên TONG KET) | Log / sha256 (12 ký tự đầu) |
|---:|---|---|---|---|
| 1 | `ps-disk` | **PASS** | 13 GiB | `preflight-ps-disk.log` `cdb536795d21c…` |
| 2 | `ps-gate-fast` | **PASS** | timeout 3600s | `ps-gate-fast.log` `041002e3f7053…` |
| 3 | `ps-diff-100k` | **PASS** | timeout 1800s | `ps-diff-100k.log` `14760dc9dfb87…` |
| 4 | `ps-targets-wasm` | **PASS** | timeout 1800s | `ps-targets-wasm.log` `9efad9c0da17d…` |
| 5 | `ps-clippy` | **PASS** | timeout 3600s | `ps-clippy.log` `a6b5669e1680b…` |
| 6 | `probe-psvn` | **PASS** | timeout 120s | `probe-psvn.log` `b8c51ff11a061…` |
| 7 | `psvn-disk` | **PASS** | 12 GiB | `psvn-disk.log` `abd760e989a36…` |
| 8 | `psvn-sync` | **PASS** | timeout 300s | `psvn-sync.log` `a0e279dd1f34…` |
| 9 | `psvn-gate-full` | **FAIL** | **exit=101** (timeout 9000s) | `psvn-gate-full.log` `739eed05e51b0…` |
| 10 | `probe-mps` | **PASS** | timeout 120s | `probe-mps.log` `588b01bd01d7c…` |
| 11 | `mps-sync` | **PASS** | timeout 300s | `mps-sync.log` `2aff8d1098027…` |
| 12 | `mps-clippy` | **PASS** | timeout 3600s | `mps-clippy.log` `8c7fb144a5522…` |
| 13 | `mps-gate-full` | **FAIL** | **exit=1** (timeout 9000s) | `mps-gate-full.log` `fcb43073f5cc4…` |
| 14 | `bench-ps` | **PASS** | runs=5 | `bench-ps.log` `fbe9802b76916…` / `bench-ps.json` `117ed5a8216ba…` |
| 15 | `psvn-bench-build` | **PASS** | timeout 2400s | `psvn-bench-build.log` `acc6723dbf784…` |
| 16 | `bench-psvn` | **PASS** | runs=5 | `bench-psvn.log` `4c0f38a01ef34…` / `bench-psvn.json` `90f2e47171825…` |
| 17 | `mps-bench-build` | **PASS** | timeout 2400s | `mps-bench-build.log` `39841cae38d33…` |
| 18 | `bench-mps` | **PASS** | runs=5 | `bench-mps.log` `c5a8c8674e0d5…` / `bench-mps.json` `1548208bdde4f…` |

**Exit tổng = 1** (2 FAIL thật, **0 SKIP**). `psvn`/`mps` đều probe+sync PASS nên đây **không**
phải exit 3.

#### Chi tiết 2 FAIL (bằng chứng từ log; fix đã merge nhưng CHƯA re-run)

- **`psvn-gate-full` exit=101** (Windows, KET `2026-11-27T01:04:25Z`): biên dịch `vis` lỗi do
  `-D warnings` — ``error: unused import: `std::process::Command` `` tại
  `crates\vis-cli\src\vistest\tests.rs:5:5`, dẫn ``could not compile `vis-cli` (bin "vis" test) ``.
  Log nguồn sha256 `739eed05e51b0944dea4d08760ea13609fee630dbf27f843369d8ad7ac4225a6`; log máy đích
  `/c/Users/ps/vislogs/vis-remote-20261127T010404Z-1262702.log` sha256
  `9d6dd0dbc88a4fceb9aac9f95bcd2ae2c9caa3137dc349f6506ef5c3b3830cd5`.
  **Fix**: `ST-321d` (merge `39effc26`, code `025e9750`, `cfg(unix)` gate import `Command`) —
  `fmt` rc=0, `test_vis_cli` **101 → 0**, gate/clippy rc=0. **Cần re-run `psvn-gate-full` trên `psvn`.**
- **`mps-gate-full` exit=1** (macOS, KET `2026-11-27T01:05:43Z`): test
  `host_parity::net_http::net_loopback_parity` panic tại
  `crates/vis-conformance/tests/misc_api_it/host_parity/net_http.rs:148:5` — assert `left == right`
  thất bại: macOS `os error 61` (Connection refused) vs Linux hardcode `os error 111`. Tóm tắt
  `1974/3905 tests run: 1973 passed, 1 failed, 3 skipped`; `1931/3905 tests were not run due to test
  failure` (fail-fast). Log nguồn sha256 `fcb43073f5ccac2d04da909a3d77dd2022837ce7349b00b4b6d5d5c52d2a774a`;
  log máy đích `/Users/mps/vislogs/vis-remote-20261127T010443Z-1264036.log` sha256
  `2b80f5ca09ab2032258212b75bdf456bbc1a9b1ac5ebcd85f8b668b8319381fa`.
  **Fix**: `ST-321c` (merge `0b32c5dc`, code `34e71725`, normalize errno `ECONNREFUSED` theo
  `target_os`) — target test **1/1 PASS**, clippy exit 0, gate PASS. **Cần re-run `mps-gate-full` trên `mps`.**

| Host | Kết quả thật (trên `ff585af8`) | Kết luận |
|---|---|---|
| `ps` (Linux) | 5/5 bước PASS + `bench-ps` PASS | **PASS** (trước merge 320a/b) |
| `psvn` (Windows) | probe/disk/sync PASS; **gate-full FAIL exit=101**; bench build + bench PASS | **FAIL** — fix có (`39effc26`), **chưa re-run** |
| `mps` (macOS) | probe/sync/clippy PASS; **gate-full FAIL exit=1**; bench build + bench PASS | **FAIL** — fix có (`0b32c5dc`), **chưa re-run** |
| Exit tổng | **1** (2 FAIL thật, 0 SKIP), bench **3-OS PASS** | **KHÔNG tuyên bố 3-OS / phát hành** |

### 3.3 WASI final2 (đã có, trước merge)

| Hạng mục | Kết quả |
|---|---|
| Cây đo | `8c883a16` (docs R38), code `ee8f252d`, `dirty=0` |
| Conformance | **398×2** (371 `.out` + 27 `.trap`), 0 hỏng |
| Sem / suite | **390/390** / **28/28** |
| Core / sqlite | 47/0 / 45 + 2 expected-red |
| Log | `st316c-wasm-final2-20261126T2130Z.log`, 7.659 B, sha256 **`60064a5cc1fba281b2aece59315bc4d7445a5dd10a0a42c5107543e9a5996bbb`**, exit 0 |

**CÒN TODO**: sau merge 320b/4 fix (chạm `vis-interp`/`vis-cli`) **chưa đo lại WASI** trên `0b32c5dc`;
nếu R39 muốn tuyên bố final trên cây cuối phải chạy lại và ghi log mới.

### 3.4 Bench A/B — không hồi quy (theo R38, trước merge)

- Binary release đo: `target/release/vis` sha256 **`a47b583e1252909b878514bcd9839421167b9c23e670b9e7905031b37c5179be`**
  (`vis 0.6.0`), code `ee8f252d`.
- A/B ghim 1 E-core, 5 vòng × 5 mẫu/engine/binary: **không ô nào tăng median**; p90 tệ nhất
  **+7,4 %** (< 15 %); trung vị paired **−7,4 %**; stdout 5/5 khớp; không hồi quy 319d/319e.
- Nguồn: `final-bench-report.md` sha **`e6a03804…8737f7`**; `final-bench-runs5-20261126T213803Z.json`
  sha **`3ca403ff…5ce953`**.
- **Bench trong chain** (trên `ff585af8`): `bench-ps`, `bench-psvn`, `bench-mps` đều **PASS, runs=5**;
  build `psvn-bench-build`/`mps-bench-build` **PASS** → **bench 3-OS PASS**. Đây là bằng chứng chain
  chạy được trên 3 host, **không** phải bằng chứng A/B hồi quy cây cuối `0b32c5dc`.
- **CÒN TODO**: **chưa đo lại bench sau merge** — chỉ tuyên bố “không hồi quy” cho cây `ee8f252d`.

## 4. Việc tiếp sau chain (thứ tự đề xuất)

| # | Việc | Nguồn / phạm vi | Cổng / bằng chứng | Trạng thái |
|---|---|---|---|---|
| 1 | Merge `plan/st320a` + `plan/st320b` (`--no-ff`), resolve conflict 3 hunk | merge plan F1 §3 | merge **`bbbb99be`** + **`56d349a6`**; **38 test**, blob **`61a82fd7`**, gate **3940**, diff `lech=0`, clippy 0 | **XONG** |
| 2 | `VD-305` + patch `#195` + `VD-306` (docs `[ST-00]`, tách khỏi merge commit) | QUEUE/ACTIVE/STATE/ROADMAP/DECISIONS/SO-NO | `3bb8fe5c` + `298714cf` + VD-306; `ST-320a/b/c/d` DONE; §8 **215 = 82/114/16/2/1**; `report-check --all` hiện **PASS (36/386)** — sẽ đổi khi nộp R39 | **XONG** |
| 3 | 4 merge fix chain (`ST-321a/b/c/d`) | `5d57d8e4`/`22d7108b`/`39effc26`/`0b32c5dc` | `#200/#201` đóng; runner 568/568; errno 61/111; `cfg(unix) Command`; gate **3942** | **XONG** (remote gate chưa re-run) |
| 4 | Dọn worktree + nhánh `st320a/b/321a/b/c/d` | `git worktree remove`, `git branch -d`, `push origin --delete`, `fetch --prune` | chỉ còn worktree `main`, nhánh `main` + `plan/st309-qa` | **XONG** |
| 5 | **Re-run `psvn-gate-full` + `mps-gate-full`** sau fix | `psvn`/`mps`, khung 0-LLM tiếp | log PASS mới + sha → mới được mở `ST-316c` / nói 3-OS | **CÒN TODO** |
| 6 | `R39-viscript.json` + `.license` nộp vào repo | `agent/reports/R39-viscript.json` (dàn ý §6) | 11 khoá, `head=0b32c5dc`, `tasks[]` phủ 386 hàng QUEUE; `report-check --all` chính thức | **CÒN TODO — do A0 làm** (bản này chỉ đối soát ngoài repo) |
| 7 | WASI/bench đo lại trên cây `0b32c5dc` | §3.3 / §3.4 | log mới + sha, nếu muốn tuyên bố final | **CÒN TODO** |
| 8 | R1 `run_cmd.rs` = 1.000 dòng | `crates/vis-cli/src/cli/run_cmd.rs` | tách < 700 (ít nhất < 1.000), 0 đổi hành vi | **CÒN TODO** |

Nguyên tắc: **1 task/1 nhánh/1 merge**; correctness trước refactor; `run_cmd.rs` tách lát W riêng.

## 5. Bảng bằng chứng (log ngoài repo + sha256)

> Log tại `/home/ps/vislogs/`; sha256 của log Sóng 9 tính lúc snapshot 22:39 UTC, sha256 của log
> chain tính lúc **2026-11-27 01:13 UTC** sau khi chain đóng; sha của log merge/fix tính lúc đối soát
> cuối. Log đang mở/append sẽ đổi hash.

| Log | Bytes | sha256 | Nội dung |
|---|---:|---|---|
| `review-st320a-gate.log` | 413.243 | `5f3acfa1b906a72eb0758a18c0e92b458d4486bdbb5599cf7019dd9e2fd17e36` | 320a nhánh: **3937/3937**, 4 skip |
| `review-st320a-st317b.log` | 2.889 | `ed7f95258905cef33f85bdb402986ae12507b28df2c41f7bb97321b9865c147a` | `st317b` **35/35** |
| `review-st320a-diff100k.log` | 266 | `a7034477a74ce2f675b93c6783fb49c75b3b15d36cfee953e0e78c8c460c5d82` | diff 100k `lech=0`, 140,0 s |
| `review-st320a-clippy.log` | 72 | `f2066ca264b9a8598ecd436c70919e1efe1acea4098ad2a13b5d82d67bc049ec` | clippy exit 0 |
| `review-st320b-f1-gate.log` | 413.115 | `5f63ad402faf6cbd0ea8d4103880e2bb91d5cb2dff674da0a8f1a1de328321f8` | 320b F1 nhánh: **3936/3936** |
| `review-st320b-f1-st317b.log` | 2.828 | `b7b2cb804662b1c3ac59e36116ec0f844403987faa3e277df0fa017297813999` | `st317b` **34/34** |
| `st320b-f1-diff100k.log` | 265 | `a5a46ebefda633d7465039e5ddaea2a37964a07135d993a5c4e5a1716f5c8527` | diff 100k `lech=0`, 89,6 s |
| `review-st320b-f1-clippy.log` | 72 | `c92508a29848820f033cd2ea015c85dde0953c88b07cc967ce34b7c08f610549` | clippy exit 0 |
| `final-main-song9-main-gate-20261127T0115Z.log` | 414.969 | `3ec597f5b1df103384a7b56f0a71f7b8839be7d124164057c5948db2b2631d93` | merge Sóng 9 `56d349a6`: **3940/3940**, 4 skip |
| `final-main-st321a-gate.log` | 415.558 | `c0c7206fca1eb0694f53d9cf790d01cc7d8a0efe019b42eaf0f204c9b5ce40c6` | merge `5d57d8e4`: **3942/3942**, 4 skip |
| `final-main-st321b-20261127T014107Z-gate.log` | 414.320 | `81420918ab339710b8c628e6f4344b5df65d4dfd05f2a2963860e2519121f2da` | merge `22d7108b`: **3942/3942**, 4 skip |
| `final-main-st321d-gate-20261127T0138Z.log` | 414.479 | `bfe42f77da63a619bcf382bc74ec1f5efd5f92a9920d8ccf4c66d62edb77d96c` | merge `39effc26`: **3942/3942**, 4 skip |
| `final-main-st321c-20261127-103843.log` | 413.937 | `9f0b770b361143f5395d83bb10fe84fc255d7572531ba1ec83a43adf3f70a4f0` | merge `0b32c5dc`: **3942/3942**, 4 skip |
| `final-main-song9-main-diff-20261127T0115Z.log` | 348 | `f1bbbdd582165ad130a6e30a583a06b7b496af577b50a4d7ac496026ebda629e` | diff 100k `lech=0`, 143,5 s @`56d349a6` |
| `final-main-st321b-diff-100k.log` | 483 | `737de7c9edf1b70e7217a2d036eb734632361ded8ffbe56fb4ad66ef44e7cf88` | diff 100k `lech=0`, 63,1 s @`0b32c5dc` |
| `final-main-song9-main-clippy-20261127T0115Z.log` | 163 | `78a50bca264848465aad61cf91c5f5ca2ae1b1e9adb725940c9811fc01580574` | clippy exit 0 @`56d349a6` |
| `final-main-st321a-clippy.log` | 3.707 | `cf71d722430852c9e858a6b85e87d31dadb71c50995a904d76c8f9baeb647d9a` | clippy exit 0 @`5d57d8e4` |
| `final-main-st321b-clippy.log` | 184 | `a13916aae11715c6780b4c20876a8d6b89652d6c5cfb70a4658746aebe17c29c` | clippy exit 0 @`22d7108b` |
| `final-main-st321d-clippy-20261127T0138Z.log` | 3.673 | `151b4a4e112cc265f971113f373ae94739dc1d01f2f2ac2b377f7c5cdc93afd8` | clippy exit 0 @`39effc26` |
| `peak2-rerun-plan-20261127T0142Z.md` | 4.740 | `209c9a62495cb97cb458f2eeae08ee68b7b7ca6b6f740c0b86ca98d40a2e7bc9` | kế hoạch re-run `psvn`/`mps` full gate @`0b32c5dc` — **chưa chạy** |
| `st316c-wasm-final2-20261126T2130Z.log` | 7.659 | `60064a5cc1fba281b2aece59315bc4d7445a5dd10a0a42c5107543e9a5996bbb` | WASI **398×2**, sem 390/390, suite 28/28 |
| `final-main-ee8f252d.log` | 414.933 | `904d45a685367c4a15261a82536c93b17454109763c9821b91732ffcb735569f` | cổng cuối R38: **3934/3934** |
| `peak1-2026-11-27-chain.sh` | 25.762 | `e239c317b29f6a0b4e75167d423889e659860bceaec15ca448c15a4aa593fa1c` | chain 18 bước — **đã chạy, exit=1** |
| `peak1-launch-2026-11-27.log` | 4.982 | `7c6ba5df28d967948119fde3d217a6db0b634ab3d7f59433559c2aa39d738399` | launcher + `TONG KET` (sha tạm cũ `0adce422…c96712`) |
| `merge-plan-320-F1-20261126T2217Z.md` | 27.124 | `4e003ba0a3d1d3fccd5fb83c1c6e0e5cf59bfcc35dcf4c95219cfebfa9063841` | kế hoạch cũ 37 test/`d04197b2`/3939 — thực tế 38/`61a82fd7`/3940 |
| `final-bench-report.md` | 6.944 | `e6a0380405a89697aea74b2eb309c344d28a2a1a5032b0b593bd5f50d78737f7` | A/B không hồi quy (cây `ee8f252d`) |
| `final-bench-runs5-20261126T213803Z.json` | 7.532 | `3ca403ff0ecb950aa0755ec493c6914de06dbaded9ed66c56ee48fd7565ce953` | 5 mẫu/engine/binary, binary `a47b583e…` |

### 5.1 Log chain `peak1-20261127T0100Z` (đầy đủ sha256)

Tất cả tại `/home/ps/vislogs/peak1-20261127T0100Z/`; sha256 đối chiếu với `TONG KET` trong launcher log.

| Log | Bytes | sha256 | Bước / kết quả |
|---|---:|---|---|
| `preflight.log` | 401 | `0cd2901f6b7f02d8d8692ed0393c23062c1d4f0d27f79bbd4745def5172ab3dd` | preflight chain |
| `preflight-ps-disk.log` | 337 | `cdb536795d21cd8e165dd60a4f68cf2c8915b4f839a7855b0a0f8a1f2d58c131` | `ps-disk` PASS 13 GiB |
| `ps-gate-fast.log` | 413.098 | `041002e3f70535ce0ecc613e06f638beeda4bf190f84f608dc13fd5d08361409` | `ps-gate-fast` PASS |
| `ps-diff-100k.log` | 556 | `14760dc9dfb8763ca9c73d1591e5c68007e32a35732a778ab949c1f42bb6b044` | `ps-diff-100k` PASS |
| `ps-targets-wasm.log` | 7.324 | `9efad9c0da17dc0e1b6235a611c757446374bc7ed34a4c67cad55341b062b6c8` | `ps-targets-wasm` PASS |
| `ps-clippy.log` | 416 | `a6b5669e1680beb52aa8ddd8dcb077028111d35cb803927b5c9eea20c506a7cf` | `ps-clippy` PASS |
| `probe-psvn.log` | 1.616 | `b8c51ff11a0617d6efb308d8c5448a5746ff1dc012b39fa4e0640cb4b3c2b6c9` | `probe-psvn` PASS |
| `probe-psvn-true.log` | 2.406 | `b334af4d9f8be60deb0c89eff615a3638c03e0b831b8c97e76a6dadebef33d05` | probe phụ |
| `probe-psvn-false.log` | 2.412 | `be5f14bd97962e8ea0de5fd0db74d9d93fd3d85979fca3a5bbb35f4545e3e2a5` | probe phụ |
| `psvn-disk.log` | 3.180 | `abd760e989a3685f728edef1e33c29448e1024b5c3758981b170135abc898668` | `psvn-disk` PASS 12 GiB |
| `psvn-sync.log` | 4.581 | `a0e279dd1f34524be180cfd0976d40c440c7e67814ca65a8d7b213709dd67e04` | `psvn-sync` PASS |
| `psvn-gate-full.log` | 8.113 | `739eed05e51b0944dea4d08760ea13609fee630dbf27f843369d8ad7ac4225a6` | **`psvn-gate-full` FAIL exit=101** (fix `39effc26`, chưa re-run) |
| `probe-mps.log` | 1.249 | `588b01bd01d7ca9e90aa531dba143821e3e3f63c6fda503ab6d89f9a74ecde2c` | `probe-mps` PASS |
| `probe-mps-true.log` | 2.001 | `634811a037ff7fa31144570df78413b46a4348b0bd53960a92aa8cdb7043b8f0` | probe phụ |
| `probe-mps-false.log` | 2.007 | `54a20a9dd1deae668bfb38717c6fb1ea3a4f58124c7bf74e8ce19a0904b9ac17` | probe phụ |
| `mps-disk-note.log` | 2.517 | `2fc89a42359de4edcd7749eaf4b8587d13f26f8176a93068456fb29142100a0d` | `mps` disk 30 GiB (không chặn) |
| `mps-sync.log` | 3.718 | `2aff8d10980273e2ca691e6149a732b18e6335a1257d2c525eb6fd2f1c863dfa` | `mps-sync` PASS |
| `mps-clippy.log` | 8.106 | `8c7fb144a55221d265d59b0d60686726192784e574b64c6b1e5170e673141ab6` | `mps-clippy` PASS |
| `mps-gate-full.log` | 426.720 | `fcb43073f5ccac2d04da909a3d77dd2022837ce7349b00b4b6d5d5c52d2a774a` | **`mps-gate-full` FAIL exit=1** (fix `0b32c5dc`, chưa re-run) |
| `bench-ps.log` | 746 | `fbe9802b76916ff0aa943227692da40904a39b22e478dbdeafc6bc114f29aae9` | `bench-ps` PASS runs=5 |
| `bench-ps.json` | 7.498 | `117ed5a8216bae0c516fb8b6f0356660c4948f507aadc7c6a27beda6993e58b8` | bench `ps` |
| `psvn-bench-build.log` | 8.671 | `acc6723dbf7849b65896eb77547a5937a3f594632aff0a9b0de6c49e3f2b02ec` | `psvn-bench-build` PASS |
| `bench-psvn.log` | 11.959 | `4c0f38a01ef3486dd494a82625e40eff7e74faaf4b4ef79613cb16c6316305d2` | `bench-psvn` PASS runs=5 |
| `bench-psvn.json` | 7.531 | `90f2e471718258cd963275ab63e99b4fb3d63f3fcd386bbccc401383b4622b6b` | bench `psvn` |
| `mps-bench-build.log` | 9.928 | `39841cae38d33d453d44bb14e6645e930f464e21348f5fa9e41f69e9098293cf` | `mps-bench-build` PASS |
| `bench-mps.log` | 10.500 | `c5a8c8674e0d5d80003b493a732e98636768be81d6d98db08b1d6e5c4ff248e6` | `bench-mps` PASS runs=5 |
| `bench-mps.json` | 7.482 | `1548208bdde4f136cf2a7d3fcb018540e22976bd8cd1564bce40d4005767b00c` | bench `mps` |

## 6. `R39-viscript.json` (đã nộp — bản quy chiếu)

Báo cáo máy đọc chính thức là `agent/reports/R39-viscript.json` (cùng dòng SPDX tại
`agent/reports/R39-viscript.json.license`), sinh từ **QUEUE@`096da1b5`** gồm **386 hàng**:
`head = 096da1b5e233d3942120f896a1a9b4ec88a474db`, `build_id = r39-song9-st321-096da1b5`,
`round = 39`. `tasks[]` phủ **100% 386 hàng** với status khớp QUEUE (6 lát Sóng 9/10
`ST-320a/b`, `ST-321a..d` **DONE**; `ST-316c`/`ST-309` **BLOCKED**). `blocked[]` gồm
**ST-316c + ST-309**. `gates`: `fast=PASS` (3942/3942, 40 test @`0b32c5dc`), `full=FAIL`
(chain `peak1 01:00Z` exit=1: 16/18 PASS, 2 FAIL remote, bench 3-OS PASS), `nightly=—`.

**Lưu ý schema**:
- Đủ **11 khoá**: `project`, `round`, `head`, `build_id`, `tasks`, `gates`, `sim`, `coverage`,
  `mutants`, `perf`, `blocked`.
- `head` = **`096da1b5…`**, `build_id` = `r39-song9-st321-096da1b5`; `gates.fast`/`perf.linux_gate`
  đã có log **3942/3942**, **40 test** thật — không còn ghi “dự kiến 3939”.
- `blocked[]` giữ **ST-316c** (**vẫn BLOCKED** — fix đã merge nhưng remote gate chưa re-run theo kế
  hoạch `peak2-rerun-plan-20261127T0142Z.md`) và **ST-309** (VD-289) — chỉ bỏ khi có quyết định mới.
- `perf.so_no` dùng số cuối **82/114** (sau VD-306), không dùng `84/112` lịch sử.
- `tasks[]` phủ **100% 386 hàng QUEUE @`096da1b5`**; `report-check --all` **PASS (37 báo cáo,
  386 hàng)**.
- `R39-viscript.json` + `.license` (mẫu R38) đã **do A0 nộp** tại commit `[ST-00] R39`.

## 7. Checklist chốt R39

- [x] **ĐÃ ĐIỀN** Chain `01:00Z` xong; **exit=1** (16/18 PASS, 2 FAIL, 0 SKIP), bench **3-OS PASS**;
      `TONG KET` + sha 27 log tại §3.2/§5.1. Launcher sha `7c6ba5df…738399`.
- [x] **ĐÃ ĐIỀN** `psvn-gate-full` FAIL 101 do unused import → fix `ST-321d` (`39effc26`);
      `mps-gate-full` FAIL 1 do errno 61/111 → fix `ST-321c` (`0b32c5dc`). **Chưa re-run remote gate.**
- [x] **ĐÃ ĐIỀN** Merge Sóng 9 `bbbb99be` + `56d349a6`, **38 test**, blob `61a82fd7`, gate **3940**,
      diff `lech=0`; 4 merge fix `5d57d8e4`/`22d7108b`/`39effc26`/`0b32c5dc`, gate **3942/3942**,
      **40 test**, diff `lech=0`, clippy 0.
- [x] **ĐÃ ĐIỀN** `VD-305` `298714cf` + patch `#195` `3bb8fe5c` + VD-306; §8 cuối
      **215 = 82/114/16/2/1**; `#200/#201` đóng ở ST-321a; `#211/#212` đóng, `#213..#215` mở theo R1;
      `#195` đóng.
- [x] **ĐÃ CHỐT** `head`/`build_id`/`gates`/tasks bằng số thật; JSON đủ **11 khoá**;
      `blocked[]` = ST-316c + ST-309; `report-check --all` **PASS (37 / 386)**.
- [x] **ĐÃ ĐIỀN** Dọn worktree/nhánh: chỉ còn `main` + `plan/st309-qa`; HEAD `0b32c5dc` sạch, đã push.
- [ ] **CÒN TODO** Re-run `psvn-gate-full` + `mps-gate-full` sau fix theo
      `/home/ps/vislogs/peak2-rerun-plan-20261127T0142Z.md` (chưa chạy) → mới mở `ST-316c` / nói 3-OS.
- [x] **ĐÃ NỘP (A0)** `R39-viscript.json` + `.license` vào repo; `report-check --all` PASS (37/386).
- [ ] **CÒN TODO** WASI/bench đo lại trên `0b32c5dc` nếu muốn tuyên bố final.
- [ ] **CÒN TODO** R1 `run_cmd.rs` **1000** dòng — tách < 700 (ít nhất < 1000), 0 đổi hành vi.

---

**Tóm tắt trạng thái cuối**: HEAD **`0b32c5dc`** (đã push, sạch, `origin/main` cùng); worktree chỉ còn
`main`, đĩa 20 G, `report-check --all` PASS 37/386. Sóng 9 merge `56d349a6` — **38 test**, blob
`61a82fd7`, gate **3940** (dự kiến cũ 37/`d04197b2`/3939 đã ghi đè). 4 merge fix
`5d57d8e4`/`22d7108b`/`39effc26`/`0b32c5dc` — gate **3942/3942**, **40 test**, diff `lech=0`, clippy 0;
`#200/#201` đóng, runner **568/568**, errno **61/111**, `cfg(unix) Command`. Sổ sau A0 VD-306
**215 = 82 mở / 114 đóng / 16 / 2 / 1** (`298714cf`, patch `#195` `3bb8fe5c`, `#195` đóng, `#211/#212`
đóng, `#213..#215` mở). Chain `peak1 20261127T0100Z` **exit=1**: 16/18 PASS, 2 FAIL remote, bench 3-OS
PASS — fix đã có nhưng **chưa re-run `psvn-gate-full`/`mps-gate-full`** (kế hoạch
`peak2-rerun-plan-20261127T0142Z.md`, chưa chạy) ⇒ `ST-316c` giữ **BLOCKED**, **không tuyên bố
3-OS/phát hành**. Dàn ý JSON đã điền số thật (11 khoá, `blocked[]` ST-316c + ST-309). **Còn TODO**:
re-run remote gate; **A0** nộp `R39-viscript.json` + `.license` và chạy `report-check --all` chính thức;
đo lại WASI/bench; tách `run_cmd.rs` (1000 dòng).
