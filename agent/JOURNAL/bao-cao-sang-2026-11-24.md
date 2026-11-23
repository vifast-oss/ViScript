# Báo cáo sáng 2026-11-24 — ViScript (v0.5.0 → v0.5.5, Phase C một phần)

> **Trạng thái**: báo cáo chốt đợt chạy đêm 23→24/11; main `4d96a8a`. Tag đã push: **`v0.5.0` @ `c06d7c6`**,
> **`v0.5.5` @ `3d85bd2`**. `v0.5.5` được force-cập nhật từ `b4db374` sau khi vá `vis-targets`
> CRLF Windows và bổ sung full 3-OS; tag cũ không còn dùng.
> Không GitHub Actions; macOS chỉ arm64; 0 `unsafe`; R7 xanh.

## 1. Phase A — `v0.5.0` (GĐ 3: Pha 2 + ViShell + LSP/VS Code + Plugin)

- **ST-300m3 macOS arm64 Tier 1**: 3× `vis-gate-macos --full` PASS (220/216/210 s; nextest
  **3.745/3/0**, doc-test 5/0/1, host-wasm 29/29, WASI 28/28); soak **1.878 vòng/14.405 s, 0 fail**,
  RSS 6.960–7.008 kB, fd 6/6, dirty 0; `vis-wasm-sem` **388/390**; caveat không Landlock, chỉ arm64.
- Linux: gate **3754/4 ×3**, release **3759/0/5**, soak **10/10 650 s**, diff 0 ×3, host 54/54.
- Windows full: **3701/0/4**, host 54/54, targets 18/18 + 17/1.
- Security F1/F2 (symlink gốc cap + plugin write) vá; SPEC 1.181 dòng; QA fuzz parser 1h 286M/0,
  decode 2,517B/0, diff 5×100k=0.
- **Tag `v0.5.0` @ `c06d7c6`**; R36 md/json/license PASS `check_report`.

## 2. Phase B — `v0.5.5` (refactor 4 tầng)

- **ST-310a** `vis-core` (value/json/host/reactor/http_client), 4 crate **0 dep vis-interp**, #112 đóng;
  `interp.rs` 1.460→423, `rt.rs` 1.338→152.
- **ST-310b** `vish-term` là tầng Terminal/Render duy nhất; `vis repl` TTY/non-TTY + `--format viterm`.
- **ST-310c** `vis-plugin-abi` 350 dòng/0 dep + `vis-plugin-host`; xoá `cap_fs` trùng; #194 ghi nhận.
- **ST-310d** `vis-cli` 0 dep OS/TLS/SQLite trực tiếp; 12 tổ hợp feature xanh.
- **ST-310e** test `vis-cli` 27→8 binary, suite về crate chủ, R8/R9; 0 test mất.
- **ST-310f** archive journal/CHANGELOG, `vis-targets` 18/18; vá CRLF Windows.
- Gate Linux **3773/4**; targets 18/18; diff 100k **0**; Windows full **3720/0/4**; macOS full
  **3764/0/3** + sem 388/390; Linux fresh release **3778/0/5**, diff 5 seed 0.
- **Tag `v0.5.5` @ `3d85bd2`**.

## 3. Phase C — ST-309 tổng nghiệm thu (MỘT PHẦN)

| Hạng mục | Kết quả |
|---|---|
| Fresh Linux v0.5.5 | gate **3773/4**, release **3778/0/5**, diff 5×100k **0**, WASI 28/28 + sem 388/390, host 54/54 |
| Windows full | **3720/0/4**, host 54/54, targets 18/18 + 17/1 |
| macOS full | **3764/0/3**, host 29/29, WASI 28/28, sem 388/390 |
| Fuzz verifier/decoder | **100.000.000 ca / 0 vi phạm** (35,7 s) |
| Soak v0.5.5 | **10/10, 628 s** |
| Fuzz parser | **2 h xong** — 861.371.033 ca / 0 vi phạm; chưa đủ 8 h |

**Chưa đủ để tag `v0.6.0`/`v1.0.0-rc1`:** soak 24 h; fuzz parser 8 h; Windows/macOS 3× full + soak 4 h;
`#194` metadata TOCTOU còn mở; `#195` (VM back-edge O(n²)/fuel wall-clock) mới là đề xuất, chưa có
dòng `SO-NO`. Đã ghi R37 skeleton `be8affd` (nhánh `plan/r37-phasec`).

## 4. Sự cố & xử lý

- macOS `lat256_net::t3` flake sát timeout 8 s (cùng binary 2/25 pass); tăng timeout test lên 30 s,
  mps 20/20 PASS; `vis-gate-macos --full` dùng nextest + doc-test (libtest t3 là khác biệt harness).
- `vis-targets` Windows CRLF (Python 3.13) → `tr -d '\r'`; full Windows sau vá PASS.
- Windows quick thiếu `async-h2` → `--features http2,async-h2`, 91/91.
- Tag `v0.5.5` đã force cập nhật để gồm fix CRLF + full 3-OS; ghi rõ trong tag message.

## 5. Trạng thái máy & việc tiếp

- `ps` sạch, `mps` sạch, `psvn` sạch; không kill soak/task nền. Đĩa `ps` còn **13 G** (>10 G).
- Việc tiếp: hoàn tất fuzz parser 8 h, soak 24 h, 3× full + soak 4 h Windows/macOS; xử lý #194/#195;
  xuất R37 md+json; tag `v0.6.0`/`v1.0.0-rc1` sau khi A0 duyệt.
- Báo cáo chi tiết: `agent/JOURNAL/phase-c-qa-matrix.md`, `agent/reports/R37-viscript.md` (branch),
  `agent/JOURNAL/st300m3-macos-tier1.md`.
