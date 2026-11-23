<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R36 — ViScript vòng 36 (`v0.5.0`, QA prep)

> **Trạng thái: ĐANG DỰNG — CHƯA CHỐT, CHƯA TAG.** Báo cáo mở tại head **`c3b7290`**
> (merge `ST-300m1`, hậu tag **`v0.4.5` @ `3daff62`**) để chuẩn bị mốc **`v0.5.0`**
> (GĐ 3: Pha 2 ngôn ngữ + CLI Developer + ViShell + LSP/VS Code). Nhánh soạn
> **`plan/r36-qa-prep`**; **không merge/push/tag**, **không ssh `mps`**.
> Số QA 3-OS/soak **chưa chạy** — không dùng báo cáo này để tuyên bố phát hành.

## 1. Phạm vi

`v0.5.0` gồm các lát đã có bằng chứng cục bộ:

| Task | Nội dung | Trạng thái thật |
|---|---|---|
| `ST-301` | `T??` + lambda param/return annotation + đóng `#192`/`#155` | **DONE**, merge `2bc6be7` |
| `ST-302` | `async` method/`cmd` (`#13`) + `derives Json` (`#8`) | **REVIEW** — `d3ea467` local, chưa merge |
| `ST-303` | `vis new` / `vis doc` / `vis repl` | **DONE**, merge `a114e38` |
| `ST-304` | ViShell MVP phần A — `crates/vish-edit` | **REVIEW** — `6e0675a` local, chưa merge |
| `ST-304b` | ViShell phần B — gợi ý tab + job control + PTY giả | **ĐANG CHẠY** — chưa có commit |
| `ST-305` | LSP hoàn thiện + extension VS Code + release `v0.5.0` | **REVIEW** — `fa1a70b` local; release còn chờ |
| `ST-306` | Plugin tĩnh Native Rust (`VIS_PLUGIN_ABI_V1`) | **ĐANG CHẠY** — chưa có commit |

Nguồn quyết định: `DECISIONS.md` `VD-278`, `VD-283`, `VD-284`; hàng việc: `QUEUE.md`
`ST-300m1..m3`, `ST-301`..`ST-306`.

## 2. Baseline và bằng chứng hiện có

- `main` **`c3b7290`** (merge `ST-300m1`); tag gần nhất **`v0.4.5` @ `3daff62`**
  (Tier 1 Windows, soak 4 h/985 vòng/0 fail — `R35-viscript.md`).
- **`ST-300m1` — DONE tại `c3b7290`**: Darwin errno `LOOP`/`XDEV`/`INPROGRESS`, quyền symlink
  trả `VS0604`, `bin/vis-gate-macos`, release `aarch64-apple-darwin`, đóng `#137`; gate Linux
  **3672/4**; `vis-gate-macos --quick` **PASS** trên `mps`; APFS `0/100` rò.
- **`ST-302` — `d3ea467`**: `async fn` trong `impl` + `async cmd`, `derives Json` sinh
  `to_json`/`from_json` thật, từ chối `cap`/`Chan`/`Task`/`fn` bằng `VS0412`; 15 + 3 ca;
  gate Linux **3676/4**; diff 100 k seed `20261115` **lech=0** (84,6 s). Chưa merge;
  Windows/macOS quick chưa chạy.
- **`ST-304` — `6e0675a`**: `crates/vish-edit` (raw mode `rustix` Unix/macOS + `crossterm`
  Windows, đa dòng tự thụt lề, lịch sử append-only + tìm mờ, tô màu CST, Ctrl-C); 8/8 test;
  gate Linux **3679/4**; khởi động ấm median **50 µs**. Chưa merge; Windows/macOS chưa chạy.
- **`ST-305` — `fa1a70b`**: hover generic (hàm/phương thức in `[T]`), completion sau `std.`,
  `editors/vscode/` (JSON-RPC client + TextMate grammar); `lsp_it` **155/155**, `vis-lsp`
  **29/29**; gate Linux **3674/4**. Chưa merge; release `v0.5.0` chưa chạy.
- **`ST-304b`, `ST-306`**: đang chạy, **chưa có commit/kết quả** trong repo tại thời điểm
  dựng báo cáo — không ghi nhận DONE.

## 3. `ST-300m2` / `ST-300m3` — CHƯA XONG

Hai lát macOS còn lại **chưa chạy**, giữ `BLOCKED` đúng `QUEUE@c3b7290`; báo cáo **không**
điền số đo thay:

- **`ST-300m2`** (đo chuẩn độc quyền trên `mps` rảnh): soak 10 vòng + `gate --fast` tách
  biên dịch/chạy (đóng `#147`), 24 cặp A/B `opt-bench.vis` (đóng `#152`). Chưa có
  `JOURNAL/st300m2-mac-bench.md`, chưa cập nhật `SO-NO.md`.
- **`ST-300m3`** (chứng nhận macOS arm64 Tier 1): 3× `vis-gate-macos --full` + soak 4 h +
  `vis-wasm-sem` ≥ 388/390 trên `mps`, cập nhật `HOST-BAO-DAM.md`/`ROADMAP.md`.
  **Chưa chạy**; macOS **chưa** được tuyên bố Tier 1.

Điều kiện thứ tự: `m2` chạy trước (đo hiệu năng khi máy rảnh), `m3` sau; cả hai dùng
`bin/vis-lock` và yêu cầu cùng `HEAD` trên `mps` & `ps`.

## 4. QA 3-OS / WASI — ma trận cuối

Ma trận lệnh, tiêu chí PASS và đường log ngoài repo nằm tại
[`agent/JOURNAL/v050-qa-matrix.md`](v050-qa-matrix.md). Tóm tắt:

| Nền | Lệnh cuối | Trạng thái R36 |
|---|---|---|
| Linux | fresh-clone + `./vis-dev gate --fast` + `bin/vis-release` + `vis-wasm-sem` + diff 100 k | **chưa chạy** |
| Windows | `bin/vis-gate-windows --full` 3× + soak 4 h | **chưa chạy** |
| macOS arm64 | `bin/vis-gate-macos --full` 3× + soak 4 h + `vis-wasm-sem` | **chưa chạy** (`m2/m3` BLOCKED) |
| WASI | `bin/vis-wasm-sem` ≥ 388/390 eligible + suite 28/28 | kế thừa R35 99,5%; chưa chạy lại |

Mọi số đo cuối phải kèm log ngoài repo + sha256; repo chỉ giữ tóm tắt `.md` (R7).

## 5. Cổng và JSON

- `agent/reports/R36-viscript.json`: **333 task** (đủ **333 hàng QUEUE@`c3b7290`**),
  **10 blocked** (`ST-300m2`, `ST-300m3`, `ST-302`, `ST-304`, `ST-304b`, `ST-305`, `ST-306`,
  `ST-307`..`ST-309`), `round=36`, `head=c3b7290`. Trạng thái lệch sổ đều có trường `queue`
  ghi đúng giá trị QUEUE@head (`m1` DONE/queue READY; `302/304/304b/305/306` REVIEW).
- Sidecar `agent/reports/R36-viscript.json.license`; sinh lại bằng
  `python3 test/report/gen_report_json.py --round 36` khi A0 đổi head.
- `python3 test/report/check_report.py --all`: **PASS** (35 báo cáo, 333 hàng QUEUE).
- `./vis-dev check`: **PASS** (fmt, clippy, R1..R7, self-test, build, doc-examples).
- `gates.fast = PASS` theo gate `@c3b7290` **3672/4**; `full`/`nightly` = `—` (chưa chạy R36).

## 6. Rủi ro và việc tiếp theo

- **Chưa có release `v0.5.0`**; tag chỉ đặt sau khi ma trận 3-OS + WASI xanh và A0 chốt
  head JSON cuối (không phải head `c3b7290` này).
- **macOS Tier 1 chưa công bố**: `m2/m3` còn treo; không suy diễn từ `m1 --quick`.
- **Nhánh chưa merge**: `d3ea467` (302), `6e0675a` (304), `fa1a70b` (305) cần A0 review/merge
  trước khi chạy QA trên `main`; `304b`/`306` chờ kết quả.
- **Nợ đã biết**: 2 stack-overflow interp WASI (`#193`, VM pass); `spawn obj.m()` chưa mở
  (ST-302); warm-start `vish-edit` mới đo Linux.
- Bước kế: A0 review 3 nhánh; W2 chạy `m2` → `m3`; sau merge chạy ma trận QA R36, điền số
  thật vào JSON, rồi mới tag `v0.5.0`.
