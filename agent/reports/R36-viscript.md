<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R36 — ViScript vòng 36 (`v0.5.0`, CHỐT)

> **Trạng thái: CHỐT.** Tag **`v0.5.0` trên commit R36 này** (annotated, sau gate).
> Head code/QA: **`f381c8f`** (`7930e90` + Tier1 docs), version `0.5.0`; tag trước
> **`v0.4.5` @ `3daff62`**. macOS arm64 **Tier 1 đã công bố** (3× full + soak 4h + sem).
> Linux final QA `e9c62cb`: gate **3754/4 ×3**, release **3759/0/5**, soak **10/10 650 s**;
> Windows full `f9e7503` **3701/0/4**; **`v0.5.0` đã tag**.

## 1. Phạm vi

`v0.5.0` (GĐ 3: Pha 2 ngôn ngữ + ViShell + LSP/VS Code + plugin) đã vào `main` trước head này:

| Task | Nội dung | Trạng thái thật @`e9c62cb` |
|---|---|---|
| `ST-300m2` | Đo độc quyền `mps` arm64, đóng `#147`/`#152` | **DONE**, code `101a4ad`, merge `5741d23` |
| `ST-302` | `async fn` trong `impl` + `async cmd` (`#13`), `derives Json` (`#8`) | **DONE**, `d3ea467`, merge `b79ae3a` |
| `ST-302b` | `spawn obj.m()` cho method async (không có hàng QUEUE riêng) | **DONE**, `204af84`, merge `6bc4ac3` |
| `ST-304` | ViShell phần A — `crates/vish-edit` | **DONE**, `6e0675a`, merge `8e73e81` |
| `ST-304b` | ViShell phần B — gợi ý tab + POSIX job control | **DONE**, `426d636`, merge `1eb2768` |
| `ST-305` | LSP hover/completion + `editors/vscode/` + release `v0.5.0` | **DONE** — code `fa1a70b`/merge `374b4e8`, prep `60d64e2`/merge `75e5a53`; tag `v0.5.0` trên R36 |
| `ST-306` | Plugin tĩnh `VIS_PLUGIN_ABI_V1` + macros | **DONE**, `90b2964`, merge `6093aef` |
| `ST-307` | Plugin động WASI/WASM + sandbox `cap` | **DONE**, `3d247c4`, merge `ab6beb7`/`b5aa40b` |
| `ST-308` | `Terminal` trait + `vish-term` | **DONE**, `79a6d52`, merge `a9eef01` |
| `ST-309` | SPEC + kiểm toán bảo mật + QA dài | **BLOCKED 2/3 DONE** — SPEC `51e937d`/`f9e7503` + security `5902887`; QA dài `623f3ba` mới một phần (`fuzz_mutate` chưa xong, `#195`); 1/3 cuối chờ `ST-310f`/`v0.5.5` |
| `ST-300m3` | Chứng nhận macOS arm64 Tier 1 | **DONE** — 3× full PASS (220/216/210 s), soak **1878 vòng/14405 s 0 fail**, RSS 6960–7008 kB, fd 6/6, dirty 0; sem **388/390**; docs `f381c8f` |
| `ST-310a..f` | Refactor kiến trúc 4 tầng & `v0.5.5` | **BLOCKED** — chờ tag `v0.5.0` |

`ST-300m2` đo trên `mps` độc quyền: soak 10 vòng `mps` **379,7 s** / `ps` **517,1 s = 0,73×**;
24 cặp A/B `opt-bench.vis`: interp **1,0385**, VM **1,0626** (median < 1,10) — đóng `#147`/`#152`
(`JOURNAL/st300m2-mac-bench.md`).

## 2. Bằng chứng QA trên head `e9c62cb`

### 2.1 Linux — gate `--fast` (3 lần, `3754/4`)

| Lần | nextest | Thời lượng | Kết quả |
|---|---|---:|---|
| gate-1 | **3754 passed / 4 skipped** | 100,969 s | PASS |
| gate-2 | **3754 passed / 4 skipped** | 70,450 s | PASS |
| gate-3 | **3754 passed / 4 skipped** | 68,886 s | PASS |

`vis-dev gate --fast: PASS` cả 3 lần (fmt, clippy, R1..R7, toolchain, release, fixtures,
readme-examples xanh); doc-test toàn bộ `ok`. Đây là gate **tại `e9c62cb`**, gồm security hotfix
(`5902887`), `SPEC.md` (`51e937d`) và `f9e7503`; +6 ca so với `3748/4` tại `c4337ce`.

### 2.2 Linux — release (fresh clone `/home/ps/vis-qa-v050-check`)

`cargo test --release --workspace --no-fail-fast` tại **`c4337ce`** → **3753 passed / 0 failed /
5 ignored** (126 test binary, exit 0). Suite đích: `vis-plugin-abi` 6/6 + 6/6 + 3/3,
`vis-cli vs3_plugin` 4/4, `vish-edit` 17/17, `vish-term` 19/19, `vis-lsp` 29/29, `vis-loader` 29/29.
`bin/vis-host-suite` **54/54** + `host_posix_real` PASS. `vis --version` = **`vis 0.5.0`**
(debug + release). Chi tiết: `JOURNAL/final-linux-release-c4337ce.md`.

QA `623f3ba` xác nhận lại trên worktree `c4337ce`: lần 1 **3753/0/5**, lần 2 sau khi thêm 4 test
chẩn đoán **3754/0/8** (3 test dài `#[ignore]`, `dump_only` tự bỏ qua khi thiếu `SLOWCASE`).
**Đây là số pre-security/pre-SPEC** (đo trước `5902887` và `51e937d`/`f9e7503`); số hậu-security
tại `e9c62cb` là gate **3754/4** ở §2.1.

### 2.3 Linux — soak final `e9c62cb` (10/10 theo log; chưa có `.md`)

`./bin/vis-soak --log /home/ps/vislogs/final-linux-e9c62cb.log`:

- Bắt đầu `2026-11-23T13:24:46Z`, HEAD `e9c62cb1`; kết thúc `13:35:36Z` = **650 s**,
  **`SOAK_EXIT=0`**, **10/10 vòng OK** (không flake/retry).
- **gate-1/2/3: OK**, mỗi lần **3754 passed / 4 skipped** (196 s / 92 s / 92 s).
- **diff-1/2/3: `lech=0`** (`tong=100000`, `loi_tinh=6230`, `chay=93770`, seed `20261115`;
  77,8 s / 79,7 s / 76,6 s); bước 2.000 cũng `lech=0`.
- **host-1/2: 54/54** + `host_posix_real` PASS; **cache-1/2: 5/5**.
- **Chưa có `/home/ps/vislogs/final-linux-e9c62cb.md`** (bản tổng kết R7): báo cáo ghi nhận
  “đang chạy/chưa chốt” theo quy ước, nhưng số thật từ log đã đủ **10/10**; A0 cần đọc log
  và tạo `.md` trước khi tuyên bố điều kiện 3.

### 2.4 Windows — full `f9e7503` (`psvn`, worktree riêng `viscript-win-full-f9e7503`)

`bin/vis-gate-windows --full` → **exit 0**, wall **550 s**, worktree dirty **0**:

- workspace `cargo test --workspace --no-fail-fast`: **3700 passed / 0 failed / 4 ignored**
  (127 block); host chạy riêng **+1 passed** ⇒ gate **3701/0/4** (128 block).
- `host_conformance` **54/54** (120 lần chạy interp + VM, VM 54/54).
- `bin/vis-targets`: core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED** (`vis-cli` wasm32,
  đúng nợ `#156`).
- 4 ignored đúng danh sách có chủ đích; 10 warning, 0 error. Chi tiết:
  `JOURNAL/win-full-f9e7503.md`.

### 2.5 Bảo mật — F1/F2 tại `5902887`

- **F1 (cao)**: gốc `cap` escape qua symlink ở **thư mục trung gian** (`quyen_mo::mo_goc`) —
  vá `openat2(RESOLVE_NO_SYMLINKS)`/fallback `O_NOFOLLOW` từng thành phần; ca đỏ trước, xanh sau.
- **F2 (vừa)**: plugin tĩnh ghi xuyên symlink ở **tên cuối** (`PluginCtx::write_text`) — thêm
  `cap_fs.rs` (`RESOLVE_BENEATH|NO_SYMLINKS`, `O_NOFOLLOW`); ca đỏ trước, xanh sau.
- Kèm: Landlock ca âm, WASI preopen symlink `VS0604`, fuzz decoder+verifier **1,5 M ca / 0 panic**;
  gate sau vá **3754/4**. Mở nợ **`#194`** (metadata TOCTOU). Chi tiết:
  `JOURNAL/st309-security.md`.

### 2.6 SPEC — `51e937d` / `f9e7503`

`docs/SPEC.md` tự chứa **1.181 dòng** (từ vựng, EBNF, kiểu/cap/budget, 56 mã `VSxxxx`, CLI,
ABI plugin, phụ lục A/B/C); 31 khối doc-example PASS; `f9e7503` chỉnh wording R6 + cho fixture
tài liệu bỏ qua SPEC. Chi tiết: `JOURNAL/st309-spec.md`.

### 2.7 QA dài — `623f3ba` (nhánh `plan/st309-qa`, chưa merge)

| Hạng mục | Kết quả |
|---|---|
| Fuzz parser 1 h, seed `20261221` | **286.313.039 ca / 0 vi phạm** |
| Fuzz decode+verify 1 h, seed `20261226` | **2.517.069.629 ca / 0 panic** |
| Differential `VIS_DIFF_N=100000` × 5 seed | **lech=0** cả 5 (500.000 chương trình) |
| `cargo test --release --workspace` | **3753/0/5** (lần 1) → **3754/0/8** (lần 2, có test chẩn đoán) |
| WASM local Node | `vis-wasm-sem` **388/390 = 99,5%**, suite **28/28** |
| Fuzz `fuzz_mutate` 1 h | **KHÔNG HOÀN TẤT** — 1 ca verify-OK chạy >101 phút; xem `#195` |

**Mốc đo: `c4337ce` — pre-security/pre-SPEC** (trước `5902887`, `51e937d`, `f9e7503`). Các số này
không thay thế gate hậu-security tại `e9c62cb` (§2.1); chưa có lần chạy lại fuzz/diff trên
`e9c62cb`. Chi tiết + sha256 log: `JOURNAL/st309-qa.md`.

### 2.8 macOS arm64 và WASI

- **macOS**: `m1` DONE (`c3b7290`: errno Darwin, `#137`, `vis-gate-macos`, release arm64);
  `m2` DONE (`5741d23`); **`m3` ĐANG CHẠY** (3× `vis-gate-macos --full` + soak 4 h +
  `vis-wasm-sem` ≥ 388/390) — **Tier 1 chưa công bố**, không suy diễn từ `m1`/`m2`.
- **WASI**: local Node tại `c4337ce` đạt **388/390 eligible = 99,5%** (2 stack-overflow interp,
  nợ `#193`) + suite **28/28**; `m3` sẽ chạy lại trên `mps`.

## 3. Nợ đã biết và đề xuất mới

- **`#194` (mở, VD-285)** — race **metadata-only** FS (`size`/`list`/`exists`/`canonicalize`/
  `read_link`/`is_*`/`walk`): API `FsHost` còn kiểm-rồi-dùng theo tên, chưa có `statat/listat/
  readlinkat` theo fd. Không lộ **nội dung** tệp ngoài, chỉ có thể lộ metadata qua khe đua.
  Đã ghi `SO-NO.md`; xử lý dự kiến ở `ST-310c`.
- **`#195` (ĐỀ XUẤT, chưa cấp số chính thức)** — bytecode **verify-OK** có thể chạy **O(n²)**:
  back-edge nhảy về `IterInit` làm `loop_ends` phình, `dat_pc()` quét tuyến tính; fuel 100 M lệnh
  **không phải** cap wall-clock. Bằng chứng `plan/st309-qa` `623f3ba`: ca 196 byte chạy >101 phút;
  fuel 1 M = 3,03 s, 10 M = 482,72 s (10× lệnh ⇒ 160× thời gian), 100 M >31 phút (dừng tay).
  Đề xuất: dọn/khử trùng `loop_ends` hoặc chỉ mục theo `end_pc`, thêm deadline wall-clock cho
  `Vm::run`, harness fuzz có timeout từng ca. **Không sửa sản phẩm** trong QA này.
- Nợ cũ giữ nguyên: `#193` (2 stack-overflow interp WASI), `#156` (sqlite wasm32 EXPECTED_RED).

## 4. Điều kiện tag `v0.5.0` (5 điều kiện)

Checklist đầy đủ + tag message soạn sẵn tại [`tag-v050.md`](../JOURNAL/tag-v050.md). Trạng thái
tại head này:

| # | Điều kiện | Trạng thái |
|---|---|---|
| 1 | Linux `gate --fast` 0 failed, skipped đúng danh sách | **PASS** — 3754/4 ×3 @`e9c62cb` |
| 2 | Linux release `3753/0/5` + `vis 0.5.0` + host 54/54 (fresh clone) | **PASS** @`c4337ce` |
| 3 | Linux soak 10/10 + diff 100 k `lech=0` | **PASS** — 10/10, 650 s @`e9c62cb`; diff 5×100k `lech=0` @`623f3ba` |
| 4 | Windows full `3701/0/4` + host 54/54 + targets `18/18` & `17/1` | **PASS** @`f9e7503` |
| 5 | macOS arm64 Tier 1: 3× `--full` + soak 4 h + `vis-wasm-sem` ≥ 388/390 | **PASS** — `m3` 3/3, soak 1878 vòng 0 fail, sem 388/390 |

Đủ **5/5**; tag annotated `v0.5.0` được tạo trên commit R36 này.

## 5. Cổng và JSON

- `agent/reports/R36-viscript.json`: **339 task** (đủ **339 hàng QUEUE@`e9c62cb`**),
  `round=36`, `head=f381c8f97de0565d3bff6e7804d69447334fcd1c`, `build_id=gd3-v050-r36-e9c62cb-20261123`.
- **21 hàng** có trường `queue` ghi nguyên văn QUEUE@head: **19 hàng** chỉ khác phần chú thích
  `DONE (...)`/`DONE**...`; **2 hàng lệch trạng thái chuẩn hoá**: `ST-300m3`/`ST-305` đã chuyển DONE (QUEUE@head vẫn READY — override).
- `blocked[]` **7 mục** = các hàng QUEUE đang BLOCKED (`ST-309`, `ST-310a..f`).
- Mọi `commits[]` đã quét `git cat-file --batch-check` (script sinh tự chặn SHA không tồn tại).
- `python3 test/report/check_report.py --all`: **PASS** (35 báo cáo, 339 hàng QUEUE).
- Sidecar `agent/reports/R36-viscript.json.license`; sinh lại bằng
  `python3 test/report/gen_report_json.py --round 36` khi A0 đổi head.
- Báo cáo này **docs-only**: không đổi mã/test; không chạy lại `./vis-dev check` trên nhánh này.

## 6. Journal bằng chứng, rủi ro và việc tiếp theo

Journal trong repo (log đầy đủ ngoài repo + sha256):

| Tệp | Nội dung | Nguồn / sha256 |
|---|---|---|
| `JOURNAL/win-full-f9e7503.md` | Windows full `f9e7503` | log `/home/ps/vislogs/vis-gate-windows-full-f9e7503.log` — `cae0602f…e85c5` |
| `JOURNAL/st309-qa.md` | parser/decode/diff/release/WASM local | log `/tmp/opencode/st309-*.log` — sha256 trong §7 |
| `JOURNAL/final-linux-release-c4337ce.md` | release `c4337ce` | log `final-linux-release-c4337ce.log` — `6317c21d…6f02` |
| `JOURNAL/final-linux-gate-e9c62cb.md` | gate `e9c62cb` + soak 10/10 (650 s; thiếu `.md` vislogs) | log `final-linux-e9c62cb.log` + `.driver.log` — sha256 trong tệp |
| `JOURNAL/st309-security.md` | F1/F2, fuzz 1,5 M, `#194` | log `/home/ps/tmp/st309-gate-isolated2.log` — `e632c2d3…a820` |
| `JOURNAL/st309-spec.md` | `docs/SPEC.md` | — |

Rủi ro / blocker:

- **Chưa có release `v0.5.0`**: tag chỉ đặt sau khi **đủ 5/5** và A0 chốt head cuối.
- **macOS Tier 1 chưa công bố**: `m3` còn chạy; không suy diễn từ `m1`/`m2`.
- **Soak Linux final `e9c62cb` chưa có `.md` chốt**: driver log đã 10/10 OK (650 s), nhưng thiếu
  bản tổng kết R7; cần tạo/đọc `.md` trước khi tuyên bố điều kiện 3.
- **`#195` chưa được cấp số chính thức**: đề xuất, chờ A0 duyệt; không sửa sản phẩm trong QA.
- **Nhánh QA `plan/st309-qa` (`623f3ba`) chưa merge**: báo cáo chỉ dùng làm bằng chứng, không
  đưa code/test chẩn đoán vào nhánh docs-only này.

Bước kế: `m3` xanh → cập nhật `HOST-BAO-DAM.md`/`ROADMAP.md` + R36; soak `e9c62cb` đã xong
10/10 → A0 tạo/đọc `.md` tổng kết R7; A0 duyệt 5/5 → tạo tag `v0.5.0` (chưa push nếu chưa
được phép).
