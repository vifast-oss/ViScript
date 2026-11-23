<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-310f — dọn `agent/JOURNAL`, `CHANGELOG`, script & nhóm thư mục (phần chuẩn bị)

> Nhánh `plan/st310f-cleanup` từ `e9c62cb` (main). **Chưa** bump/tag `v0.5.5`, **chưa** merge/push,
> **không** ssh `mps`/`psvn`. Đây là phần **docs/repo** của ST-310f; `ST-310b`/`ST-310e` vẫn là
> điều kiện của phần tag theo `VD-285`.

## 1. Script khỏi `agent/JOURNAL/`

- `agent/JOURNAL/gen-r35-json.py` + `agent/JOURNAL/gen-r36-json.py` → **hợp nhất** thành
  [`test/report/gen_report_json.py`](../../test/report/gen_report_json.py).
  - `--round 35|36`, `--head <sha>` (mặc định head đã chốt trong `ROUNDS`).
  - Sinh lại **đúng byte** JSON hiện có của R35/R36: `diff -q` = 0 cho cả hai (đã kiểm).
  - Cấu hình từng vòng giữ nguyên `commits`/`notes`/`blocked_q`/`perf`; chỗ chỉnh tay của bản
    chốt (ST-298 R35) được ghi thành `overrides`/`note_truoc_queue`.
- `agent/JOURNAL/st300-sem-native-baseline.sh` → [`test/wasm/st300-sem-native-baseline.sh`](../../test/wasm/st300-sem-native-baseline.sh);
  giữ nguyên đường dẫn mặc định ST-300, thêm biến môi trường `B`/`ROOT`/`OUT`/`EXC`/`T` để chạy lại.
- Cập nhật tham chiếu: `agent/reports/R35-viscript.{md,json.license}`, `R36-viscript.{md,json.license}`,
  `agent/JOURNAL/r35-report.md`, `agent/JOURNAL/st300-native-baseline.md`, `test/README.md`.
- `agent/JOURNAL/` còn **0** tệp `.py`/`.sh`.

## 2. Journal/log tiền-`v0.4.0` → `agent/archive/journal-pre-v040/`

- Dùng danh sách `git ls-tree -r v0.4.0 agent/JOURNAL` rồi `git mv` nguyên trạng:
  **1.137 tệp**, **26 MB** (734 `.log`, 313 `.md`, 65 `.license`, 10 `.vis`, 8 `.txt`,
  3 `.rs`, 2 `.tsv`, 2 `.tap`); giữ tên tệp + thư mục con `st234-repro/`, `vs3-w5-spike/`.
- 60 tệp tạo **sau** `v0.4.0` vẫn ở `agent/JOURNAL/` (còn 420 KB); `.gitkeep` giữ nguyên.
- Thêm [`agent/archive/journal-pre-v040/README.md`](../archive/journal-pre-v040/README.md);
  cập nhật [`agent/archive/README.md`](../archive/README.md) và [`INDEX.md`](../archive/INDEX.md).
- 72 tham chiếu `agent/JOURNAL/<tệp-đã-chuyển>` trong 30 tệp đang dùng được đổi sang
  `agent/archive/journal-pre-v040/<tệp>` (tài liệu trong `archive/` giữ nguyên, đọc theo README).
- `test/gate/r7-legacy-logs.txt`: đổi **174** mục sang đường dẫn archive tương ứng; R7 chỉ quét
  `agent/JOURNAL/` nên không còn tệp > 50 KB nào trong vùng quét.

## 3. `CHANGELOG.md` tiền-`v0.1.0`

- Tách mục **“Mốc đáng chú ý”** sang [`agent/archive/CHANGELOG-pre-v0.1.0.md`](../archive/CHANGELOG-pre-v0.1.0.md)
  (183.579 B, giữ nguyên văn). `CHANGELOG.md`: **204.932 B → 21.673 B** (< 40 KB), còn liên kết
  tới bản lưu.

## 4. `bin/vis-targets` — 100% crate workspace

- `CRATES` đủ **16/16** crate thành viên (thêm `vis-host-native`, `vis-pkg`, `vis-loader`,
  `vis-lsp`, `vish-edit`, `vis-plugin-abi`, `vis-plugin-macros`, `vish-term`).
- Thêm **canh phủ**: `cargo metadata --no-deps` phải khớp `CRATES`; lệch ⇒ FAIL rõ (ST-310f).
- Kết quả trên `ps` (rustc 1.100.0, `TMPDIR=/home/ps/tmp`):
  - `core (vis-cli --no-default-features)`: **PASS=34 / EXPECTED_RED=0 / FAIL=0**.
  - `sqlite (vis-cli default)`: **PASS=32 / EXPECTED_RED=2 / FAIL=0** — đúng hai ô
    `vis-cli` × msvc/wasm32 của nợ **#156**, có lý do.
  - Khối runtime `wasm32-wasip1` và `aarch64-apple-darwin` giữ nguyên.
- Đây vẫn **chỉ là CHECK biên dịch chéo**; không thay bằng chứng chạy 3-OS/WASI.

## 5. Nhóm thư mục `vis-lsp/src/` & `vis-host-native/src/`

- `vis-lsp/src/`: 24 tệp → `protocol/` (5: jsonrpc, khung, vi_tri, sua, phuong_loi),
  `analysis/` (5: chan_doan, module, phan_tich, thanh_vien, workspace),
  `features/` (14 tệp còn lại). `lib.rs` dùng `#[path = "..."]` giữ **nguyên tên module phẳng**
  (`super::jsonrpc`, `crate::phan_tich`, …) ⇒ **0 đổi hành vi/API**.
- `vis-host-native/src/`: `async_net/` (5 tệp), `quyen/` (7 tệp: `mod.rs` + chuan/guard/mo/path/st174/st295),
  `net/quyen.rs` (từ `net_quyen.rs`). `lib.rs` + `quyen/mod.rs` + `async_net/mod.rs` chỉ đổi `#[path]`;
  tên module công khai (`quyen_chuan`, `quyen_mo`, `net_quyen`, …) giữ nguyên.
- Kiểm: `cargo check -p vis-lsp -p vis-host-native` + `cargo check --tests` = PASS; `cargo fmt --check` = PASS.

## 6. Cổng & bằng chứng

| Lệnh | Kết quả |
|---|---|
| `./vis-dev rules` | **PASS** R1..R7 |
| `git diff --check` | sạch (rc 0) |
| `python3 test/report/check_report.py --all` | **PASS** (35 báo cáo, 339 hàng QUEUE) |
| `./bin/vis-targets` | **PASS** — core 34/34; sqlite 32/34 + 2 EXPECTED_RED (#156) |
| `./vis-dev gate --fast` | **PASS** — check PASS + L0 **3754 passed / 4 skipped** |
| `cargo fmt --all -- --check` | **PASS** |

Log đầy đủ (ngoài repo): `/home/ps/tmp/st310f-logs/`:

| Tệp | sha256 |
|---|---|
| `gate-fast.log` | `d1bd229830ca40c070d00732e3cb8d490178024c2214236f4b04a7b37db5d3fb` |
| `vis-targets.log` | `086dcb00af725e4ddafdc3e4b51aa5a4d04ada2edc0e3bdda77acef959c97d64` |
| `rules.log` | `36846bda2389faf4d8bf7efd4dba6aecbdfa7ea9e2c1744668c90a19b8dffc96` |
| `report-check.log` | `423032c597617a51912558abfe4bd430e950a262654570e162a4606ec8a466c0` |
| `diff-check.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (rỗng) |

## 7. Còn lại / blocker

- `ST-310f` **chưa xong toàn bộ**: `ST-310b`/`ST-310e` chưa có trên `main`, nên **chưa** bump
  `0.5.5`, chưa tag, chưa push, chưa chạy 3-OS/diff 100k. Giữ nguyên hàng QUEUE `BLOCKED`.
- `vis-targets` mới phủ **crate workspace**; `vis-host-wasm`/`vis-host-linux` vẫn là `EXTRA` như cũ.
- Không có `unsafe` mới, không đổi ngữ nghĩa ngôn ngữ/SPEC/golden/ngưỡng.
