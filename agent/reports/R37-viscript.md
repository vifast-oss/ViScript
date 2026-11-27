<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R37 — ViScript, vòng 37 (Phase C — `v0.6.0` / `v1.0.0-rc1`)

> **Trạng thái: KHUNG — CHƯA CHỐT.** Đây là **khung báo cáo** cho phần cuối `ST-309`
> (Phase C, `VD-285`), soạn trên nhánh `plan/r37-phasec` từ **`3d85bd2`**.
> **Chưa có `R37-viscript.json`**; **chưa đủ điều kiện** Phase C; **tag `v0.6.0` và
> `v1.0.0-rc1` CHƯA chốt**. Không dùng tệp này để tuyên bố phát hành.
>
> **Nền**: tag annotated **`v0.5.5` @ `3d85bd2`** (`[ST-310f] docs: v0.5.5 QA 3-OS + tag`).
> Bằng chứng Phase C đã có đo tại **`b4db374`** (Linux QA + macOS) và **`e6ca52e`** (Windows);
> cả hai là tổ tiên của `3d85bd2`. Khoảng `b4db374..3d85bd2` chỉ đổi **docs + `bin/vis-targets`**
> (6 tệp: 3 journal mới, `ACTIVE.md`, `ROADMAP.md`, script CRLF), **không đổi mã sản phẩm**.
>
> Ma trận lệnh/tiêu chí còn thiếu và nhật ký phiên: [`agent/JOURNAL/r37-phasec.md`](../JOURNAL/r37-phasec.md).
>
> **Quy ước**: ô “CHƯA ĐO” là **chưa chạy**, không phải số 0; chỉ điền số thật khi có log ngoài
> repo + sha256. Không suy số từ R35/R36.

## 1. Phạm vi và trạng thái tổng

Phase C nghiệm thu cuối trên nền kiến trúc sạch **`v0.5.5`** (`ST-310a..ST-310f`), gồm:
fresh-clone 3-OS Tier 1 (`ps`/`psvn`/`mps`) + WASI (`vis-wasm-suite`/`vis-wasm-sem`) +
differential 100k × 5 seed + **soak 24 h** + **fuzz parser 8 h** + **fuzz verifier/decoder
100 M ca** + đóng **`#194`/`#195`** → `R37-viscript.{md,json,json.license}` → tag **`v0.6.0`**
và **`v1.0.0-rc1`** trên cùng một commit.

| # | Điều kiện Phase C | Trạng thái @`3d85bd2` | Bằng chứng |
|---|---|---|---|
| 1 | Linux fresh-clone: `check` + `gate --fast` + host + release + version | **ĐẠT (pre-H)** | gate **3773/0/4**, release **3778/0/5**, host **54/54**, `vis 0.5.5` @`b4db374` |
| 2 | Linux diff 100k × 5 seed | **ĐẠT (pre-H)** | **500.000 chương trình, `lech=0`** @`b4db374` |
| 3 | WASI suite + sem | **ĐẠT (pre-H, known gap `#193`)** | suite **28/28**, sem **388/390** (VM 390/390) @`b4db374` |
| 4 | Windows `psvn` fresh: full gate (+ host/targets) | **MỘT PHẦN** | full **3720/0/4**, host **54/54**, targets core **38 PASS** + sqlite **37/1** (`#156`), phủ **18/18** crate @`e6ca52e`; **1 lần**, ma trận cần **3×** |
| 5 | macOS `mps` fresh: full gate + sem | **MỘT PHẦN** | full **3764/0/3**, host-wasm 29/29, sem **388/390** @`b4db374`; **1 lần**, ma trận cần **3×** |
| 6 | Soak **24 h** Linux (+ 4 h Windows/macOS) | **CHƯA ĐO** | — |
| 7 | Fuzz parser **8 h** | **CHƯA ĐỦ** | **2 h đang chạy** (seed `202611240550`, pid 2321124, bắt đầu `05:05:25+09:00`); chưa có verdict |
| 8 | Fuzz verifier/decoder **100 M ca** | **CHƯA ĐỦ** | mới **60.000.000 ca / 0 vi phạm** (`st309_fuzz_verify`) |
| 9 | Fuzz có VM/watchdog 2 h (xác nhận `#195`) | **ĐẠT (H4)** | watchdog **2 h** trên `H4` = `db67eab`: **74.811.821 ca, `vi_pham=0`, `max_case_ms=23355`** (23,355 s < 300 s), 7.200,34 s; log `/home/ps/vislogs/v060/watchdog2h-h4.log` sha256 `df24613e…5daf` khớp `sha256-h4.txt` (lịch sử @`3d85bd2`: **CHƯA ĐO** — harness watchdog chưa có) |
| 10 | Đóng `#194`, `#195` | **ĐẠT (H4)** | `#194` đóng ở **ST-311b**; `#195` đóng sổ ở **ST-311a/`H4`** (watchdog 2 h PASS, log `watchdog2h-h4.log` sha256 `df24613e…5daf`) — lịch sử @`3d85bd2`: cả hai còn mở/đề xuất, **điều kiện cứng**, xem §4 |
| 11 | Chốt `H` = commit `0.6.0`, chạy lại ma trận trên `H` | **CHƯA** | mọi số ở trên là **`v0.5.5`**, chưa phải `H` |
| 12 | `R37-viscript.json` + `.license`, `report-check --all` | **CHƯA** (đúng chủ đích) | chỉ sinh JSON sau khi đủ 1–11 |

**Kết luận thẳng**: Phase C đã có **bằng chứng nền rất mạnh cho `v0.5.5`** (3-OS + WASI + diff),
nhưng **chưa đủ điều kiện để tag `v0.6.0`/`v1.0.0-rc1`**: thiếu soak 24 h, fuzz parser 8 h,
fuzz 100 M, fuzz watchdog, đóng `#194`/`#195`, và **toàn bộ ma trận phải chạy lại trên `H`**
sau khi bump `0.6.0` (vì `H` khác `v0.5.5` về version/cache/test kỳ vọng).

## 2. Bằng chứng 3-OS và WASI đã có

### 2.1 Linux — fresh clone `/home/ps/vis-qa-v055` @`b4db374` (tag `v0.5.5`)

| Hạng mục | Kết quả | Log / sha256 |
|---|---|---|
| `./vis-dev gate --fast` | **3773 passed / 0 failed / 4 skipped**, 103 binary, **81,569 s**; fmt/clippy/R1..R9/toolchain/release/self-test/doc-examples/readme-examples/L0 xanh | `v055-gate-fast.log` — `891aa03d…0dff32` |
| `cargo test --release --workspace --no-fail-fast` | **3778 passed / 0 failed / 5 ignored**, 121 binary, exit 0 | `v055-release-test.log` — `1b971cf2…796dc` |
| Diff 100k × 5 seed (`20261221..25`) | **`lech=0` cả 5** (`tong=100000`, `loi_tinh=6233`, `chay=93767`); 86,1/104,3/97,1/83,3/99,2 s; **500.000 chương trình** | `v055-diff-5seed.log` + 5 log seed — sha256 trong `JOURNAL/linux-qa-v055.md` §7 |
| `bin/vis-wasm-suite --require-runtime` | **28/28**, runtime `node` (Node 26.5.0), exit 0 | `v055-wasm-suite.log` — `a664405e…61d8` |
| `bin/vis-wasm-sem --require-target` | **388/390 = 99,5 %** (interp 388, **VM 390/390**), đúng nợ `#193` | `v055-wasm-sem.log` — `5d018ca8…a9878` |
| `bin/vis-host-suite` | **54/54** + `host_posix_real` PASS | `v055-host-suite.log` — `3ced5d53…1db0` |
| `vis --version` | **`vis 0.5.5`** (debug + release) | — |

Chi tiết: [`JOURNAL/linux-qa-v055.md`](../JOURNAL/linux-qa-v055.md).

### 2.2 Windows `psvn` — worktree riêng @`e6ca52e`

| Hạng mục | Kết quả |
|---|---|
| `bin/vis-gate-windows --full` | **exit 0**, **3720 passed / 0 failed / 4 ignored**, 122 block, wall **552 s** (header 550 s) |
| Workspace | **3719/0/4**, 121 test-result block; host chạy riêng **+1** ⇒ 3720 |
| `host_conformance` | **54/54** (120 lần chạy interp + VM, VM 54/54) |
| `bin/vis-targets` | core **38 PASS / 0 FAIL**; sqlite **37 PASS + 1 EXPECTED_RED** (nợ `#156`), 0 FAIL |
| Dirty cuối | **0** mục; HEAD không đổi |

Log local `/home/ps/vislogs/vis-gate-windows-full-v055b.log` (254.035 B) —
`2349d428a280bc4836b9049167293e056e5203c65a50ead4c966dfe7251d447e`;
bản máy đích `vis-remote-20261123T202505Z.log` — `8686d080…34ed8e`.
Chi tiết: [`JOURNAL/win-full-v055b.md`](../JOURNAL/win-full-v055b.md).

### 2.3 macOS `mps` — clone `/Users/mps/dvchd/ViScript-v050` @`b4db374`

| Hạng mục | Kết quả |
|---|---|
| `bin/vis-gate-macos --full` | **exit 0**, wall **237 s**; `vis-dev check` + `gate --fast` PASS |
| nextest | **3764 passed / 0 failed / 3 skipped**, 103 binary, **75,6 s** (chạy 2 lần cùng số) |
| doc-test | **5 passed / 0 failed / 1 ignored** (18 binary) |
| host `vis-host-wasm` | **29/29** (17 lib + 12 `wasi_fs`) |
| WASI | suite **28/28**; sem **388/390 = 99,5 %** (**61 s**), 2 fail đúng `#193`, VM 390/390 |
| Dirty cuối | `mps` **0**; HEAD không đổi |

Log gate `27e0fd91…c81d897` (bản máy đích `45e8489b…b180d91f5b`); log sem `91b58b6b…110f16d`.
Chi tiết: [`JOURNAL/mac-full-v055.md`](../JOURNAL/mac-full-v055.md).

### 2.4 Bằng chứng fuzz/diff bổ sung @`b4db374` (nhánh `plan/fuzz-v055`)

| Hạng mục | Kết quả | Log / sha256 |
|---|---|---|
| Verifier/decoder `st309_fuzz_verify` **60.000.000 ca** | **`vi_pham=0`**, decode OK 6.888.776, verify OK 6.007.365, **40,66 s** | `verify-60M.log` — `b6a1a299…adf7ab` |
| Differential 100k seed mới `202611240550` | **`lech=0`**, 93.768 chương trình chạy, 41,7 s | `diff-100k.log` — `27e019b7…54914a` |
| Fuzz parser **2 h** seed `202611240550` | **ĐANG CHẠY** — pid **2321124**, bắt đầu `2026-11-24T05:05:25+09:00`, log 81 B (chưa có dòng kết quả) | `parser-2h.log` (chưa chốt sha256) |

**Không** dùng 60 M thay cho mốc 100 M; **không** dùng 2 h đang chạy thay cho mốc 8 h.

## 3. Fuzz, differential và soak — phần còn thiếu

| Hạng mục | Yêu cầu Phase C | Đã có | Kết luận |
|---|---|---|---|
| Fuzz parser | **8 h**, `vi phạm 0` | 2 h đang chạy (≈35 phút tại lúc soạn) | **CHƯA ĐỦ** |
| Fuzz verifier/decoder | **100 M ca**, `vi_pham=0` | **100 M ca, 0 vi phạm** (35,69 s) | **ĐẠT** |
| Fuzz có VM + watchdog | **2 h**, 0 ca > 300 s | `H4` = `db67eab`: **74.811.821 ca, 0 vi phạm, max 23,355 s**, 7.200,34 s; log `watchdog2h-h4.log` sha256 `df24613e…5daf` (lịch sử @`3d85bd2`: chưa chạy, harness chưa có) | **ĐẠT (H4)** |
| Differential | 100k × 5 seed, `lech=0` | 5 seed @`b4db374` + 1 seed mới | **ĐẠT (pre-H)** |
| Soak Linux | **24 h** liên tục, 0 fail sản phẩm, RSS/fd phẳng, dirty 0 | **10/10 628 s** @`b4db374` — **không phải 24 h** | **CHƯA ĐO** |
| Soak Windows/macOS | 4 h mỗi nền (mức Tier 1) | chưa chạy trên `v0.5.5` | **CHƯA ĐO** |
| Windows full | 3× `--full` | 1× @`e6ca52e` | **CHƯA ĐỦ** |
| macOS full | 3× `--full` | 1× @`b4db374` | **CHƯA ĐỦ** |

Tiêu chí PASS và lệnh chính xác: [`JOURNAL/r37-phasec.md`](../JOURNAL/r37-phasec.md) §2.
Mọi số phải truy được về log ngoài repo + sha256; **không** suy từ lần chạy cũ.

## 4. Nợ chặn tag: `#194`, `#195` (điều kiện cứng) và nợ đã biết

### 4.1 `#194` — race metadata-only FS (**đóng ở ST-311b**)

- **Nguồn**: `agent/SO-NO.md`; `agent/JOURNAL/st309-security.md` §4; mở tại `VD-285`.
- **Bản chất**: `size`/`list`/`exists`/`canonicalize`/`read_link`/`is_*`/`walk` còn
  **kiểm-rồi-dùng theo tên**; cửa sổ đua **metadata** (không lộ/ghi **nội dung** tệp ngoài `cap`).
- **Đóng bằng**: API theo **fd** (`statat`/`listat`/`readlinkat` hoặc `openat2`) trong
  `vis-host-native::quyen`; ca đua `rename`/`symlink` **100.000 vòng** trên Linux + macOS,
  **0 rò** metadata; cập nhật `agent/HOST-BAO-DAM.md`; ghi đóng trong `SO-NO.md`.
  Windows giữ caveat `FsGuard` (kiểm theo đường dẫn) — **không** tuyên bố đóng trên Windows
  nếu chưa có cơ chế tương đương.
- **Chặn tag**: **`#194` đã đóng ở `ST-311b`** (metadata theo `fd`: `statat`/`openat`/`readlinkat`/`fstat`; race 100.000 vòng Linux + macOS **0 rò**) ⇒ **không còn chặn**; caveat Windows (`FsGuard` theo đường dẫn) giữ nguyên.

### 4.2 `#195` — VM back-edge O(n²) / fuel không phải wall-clock (**đóng ở ST-311a/`H4`**)

- **Nguồn**: `plan/st309-qa` `623f3ba`; `agent/JOURNAL/st309-qa.md` §3/§8.
  **Chưa đăng ký số chính thức trong `SO-NO@3d85bd2`** — đã đăng ký ở `ST-311a` (`VD-286`);
  hiện có dòng `#195` trong `SO-NO.md` và đóng sổ trên `H4`.
- **Bản chất**: bytecode **verify-OK** bị đột biến cho back-edge nhảy về `IterInit` làm
  `loop_ends` phình mỗi vòng; `dat_pc()` quét tuyến tính ⇒ **O(n²)** thời gian + O(n) bộ nhớ.
  Bằng chứng gốc: module 196 byte chạy **>101 phút**; fuel 1 M = 3,03 s, 10 M = 482,72 s,
  100 M >31 phút. Fuel **không phải** cap wall-clock.
- **Đóng bằng**: (a) khử trùng/dọn `loop_ends` hoặc chỉ mục theo `(fi, end_pc)`;
  (b) **deadline wall-clock** cho `Vm::run`; (c) giữ 0 panic/UB; (d) harness watchdog bắt ca chậm.
- **Nghiệm thu**: slowcase < 300 s ở fuel 100 M; 1 M→10 M→100 M **không còn bậc hai**
  (mỗi bậc ≤ 15×, không phải 160×); watchdog 2 h **0 ca > 300 s**; `SO-NO.md` ghi đóng.
  **ĐẠT trên `H4`** — watchdog **74.811.821 ca / `vi_pham=0` / max 23,355 s** (7.200,34 s),
  log `watchdog2h-h4.log` sha256 `df24613e…5daf` khớp `sha256-h4.txt`; `#195` **đã đóng** trong `SO-NO.md`.
- **Chặn tag**: **`#195` đã đóng** (ST-311a/`H4`); tag `v0.6.0`/`v1.0.0-rc1` **vẫn chưa có**
  (chờ soak 24 h + JSON + A0 duyệt theo R37 §1).

### 4.3 Nợ đã biết, không chặn Phase C

- **`#193`** — 2/390 ca WASI interp tràn stack (`de-quy-1000`, `de-quy-vo-han`); **VM 390/390**.
  Đây là known gap **được phép** ở Phase C (ngưỡng ≥ 388/390). Muốn 390/390 phải xử lý `#193`
  trước tag và siết tiêu chí.
- **`#156`** — sqlite `wasm32` `EXPECTED_RED` trên `vis-cli`; đúng danh sách, không phải hồi quy.

> **Không waiver mặc định.** Nếu A0 quyết định khác cho `#194`/`#195`, phải có quyết định riêng
> trong `agent/DECISIONS.md`; báo cáo này không tự hạ điều kiện.

## 5. Cổng, JSON và điều kiện chốt

- **Hiện tại**: `python3 test/report/check_report.py --all` → **PASS (35 báo cáo, 339 hàng QUEUE
  phủ đủ)**. `R37-viscript.json` **chưa tồn tại**, nên R37 **không nằm trong tập kiểm** — cùng
  trạng thái với R34 trước đây. Không tạo JSON rỗng/placeholder.
- **Khi đủ bằng chứng**: sinh `R37-viscript.json` + `.license` từ `QUEUE@H`; đúng **11 khoá**,
  `project="viscript"`, `round=37`, `head=H` (tổ tiên commit R37), `tasks` phủ **đủ** hàng QUEUE@H,
  `blocked[]` phủ mọi `BLOCKED`/`REVIEW`; mọi `commits[]` phải tồn tại thật.
- **Điều kiện chốt**: `report-check --all` PASS; `./vis-dev check` + `gate --fast` PASS tại `H`;
  R37 md điền số thật; `#194`/`#195` đóng; `SO-NO` R5 PASS; `QUEUE` `ST-309` DONE.
- Báo cáo này **docs-only**: không đổi mã/test; không chạy lại `./vis-dev check` trên nhánh soạn.

## 6. Checklist tag `v0.6.0` / `v1.0.0-rc1` và việc tiếp theo

- [ ] `v0.5.5` đã tag/push (`3d85bd2`); `H` giống nhau trên `ps`/`psvn`/`mps`; cây sạch.
- [ ] Bump `0.6.0` (`Cargo.toml`, 2 `Cargo.lock`, `PHIEN_BAN`, khoá cache, test version) và **chốt `H`**.
- [ ] Đóng `#194` (API theo fd + ca đua 100k × Linux/macOS + `HOST-BAO-DAM.md`) và `#195`
      (dọn `loop_ends`/chỉ mục + deadline wall-clock + watchdog; đăng ký dòng `SO-NO`).
- [ ] Chạy lại **toàn bộ** ma trận trên `H`: Linux fresh (check/gate/host/release/diff 5 seed/WASI),
      Windows **3× full** + soak 4 h, macOS **3× full** + soak 4 h, soak Linux **24 h**,
      fuzz parser **8 h**, verifier/decoder **100 M**, watchdog **2 h**.
- [ ] R37 md điền đủ số + sha256; sinh R37 JSON + `.license`; `report-check --all` PASS.
- [ ] `git status --porcelain` rỗng; `D` = commit R37; `v0.6.0^{commit}` = `v1.0.0-rc1^{commit}` = `D`.
- [ ] A0 duyệt tag bằng quyết định ghi trong `DECISIONS.md`/`QUEUE.md`.
- [ ] Tag annotated `v0.6.0` + `v1.0.0-rc1` trên **cùng `D`**; push; xác nhận `git ls-remote` khớp.
- [ ] Không `tag -f`, không xoá/di chuyển tag, không `push --force`; nếu sai ⇒ dừng, báo A0.

**Việc tiếp theo ngay**: (1) fuzz parser 2 h đã xong **861.371.033 ca/0**; cần chạy thêm 6 h để đủ 8 h;
(2) chạy verifier/decoder **100 M** (`VBC_FUZZ_CA=100000000`, ~70 s theo tốc độ 60 M/40,66 s);
(3) A0 chốt hướng đóng `#194`/`#195` **trước** khi bỏ 24 h soak; (4) chốt `H` bump `0.6.0`
rồi chạy lại ma trận. **Không merge/push/tag từ nhánh này.**


## 7. Bổ sung 2026-11-24 (sau skeleton)

- **Verifier/decoder 100M ca**: `VBC_FUZZ_CA=100000000 cargo test --release -p vis-bytecode --test st309_fuzz_verify` → **100.000.000 ca, decode_ok 11.483.948, verify_ok 10.015.121, vi_pham 0**, 35,69 s (`/tmp/opencode/verifier-100m.log`).
- **Soak 10/10 v0.5.5** (fresh clone `b4db374`): **PASS**, 628 s, gate 3773/4, diff 100k `lech=0`, host 54/54, cache 5/5; log `/home/ps/vislogs/v055-soak-10.log`.
- **Fuzz parser 2 h xong**: 861.371.033 ca / 19.550.360.573 token / 0 vi phạm; verifier 100 M 0; diff seed mới 0.
- Vẫn **chưa đủ** Phase C: soak **24 h**, fuzz parser **8 h** (mới 2 h), Windows/macOS 3× full + soak 4 h; tag `v0.6.0`/`v1.0.0-rc1` chưa chốt.
