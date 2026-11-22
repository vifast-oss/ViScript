<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Soak Windows 2h — main `0376dae` (GĐ1 combined)

> **Người đo**: subagent VS3-W4, 2026-11-22/23 · **test-only**, time-box 2,5 h.
> **Mốc**: `psvn` worktree **riêng** `/c/Users/ps/viscript-w4soak-gd1` @
> **`0376dae8d39c7f4026383e32a2697963b2b2a6d6`** (`merge ST-296: Windows release 621fad0 3588/0`);
> `git describe --tags --always` = `v0.3.6-36-g0376dae8`; dirty **0** trước/sau.
> `TMPDIR=/c/Users/ps/tmp` (ngoài `/tmp`). Không tag/push/merge; không GitHub Actions.
> **Kết quả: 580 vòng / 2h00m21s, 0 fail, 0 flake**; host 54/54; targets 18/18 + 17/1.
> Không blocker.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Thời gian | **7221 s (2h00m21s)** — vòng đo cộng lại **7142 s (1h59m)** |
| Số vòng | **580** |
| Fail / flake | **0 / 0** |
| Mỗi vòng | P2 **14/14**, P1 **8/8**, P3 **10/10**, `vs3_select` **25/25** — 580/580 vòng |
| Mỗi 5 vòng | default **22** + TLS **22** + net **9** + pkg **25** — **116/116 vòng** |
| Mỗi 10 vòng | `vs3_http2` **33/33** (58 lần); `vs3_http_cancel` default + serial **8/8** (58 vòng ×2) |
| Free RAM | **8162–8782 MB** (584 giá trị; không xu hướng rò rỉ) |
| Cuối soak | host **54/54**; targets core **18/18**, sqlite **17 + 1 EXPECTED_RED** |
| Đĩa | `psvn` `C:` **124 G**; local `/` **14 G** (>5 G) |

> Ghi chú kỳ vọng: yêu cầu ghi “cancel 5/5”, nhưng tại `0376dae` suite `vs3_http_cancel` có
> **8 ca** (ST-294/ST-296 mở rộng) — soak chạy **8/8** mỗi lần, đúng mã hiện tại.

## 1. Môi trường

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC).
- Worktree riêng `/c/Users/ps/viscript-w4soak-gd1` (không dùng repo chung
  `/c/Users/ps/viscript-moi`), detached `0376dae`, dirty 0; prebuild 40 s.
- `TMPDIR=TMP=/TEMP=/c/Users/ps/tmp`; free RAM đầu 8344 MB.
- Script: `/c/Users/ps/vislogs/soakgd1.sh`; log ngoài repo; kiểm dirty mỗi 30 vòng.

## 2. Ma trận chạy

Mỗi vòng (580 vòng):
```
cargo test -p vis-cli --test vs3_worker_eval_parallel   # P2 14
cargo test -p vis-cli --test vs3_worker_eval_local      # P1 8
cargo test -p vis-cli --test vs3_worker_eval_vm         # P3 10
cargo test -p vis-cli --test vs3_select                 # 25
```
- Mỗi 5 vòng: `vs3_default` 22, `vs3_tls` 22, `net` 9, `pkg` 25.
- Mỗi 10 vòng: `vs3_http2` 33; `vs3_http_cancel` 8 (default) + 8 (`--test-threads=1`).

## 3. Kết quả theo nhóm

| Nhóm | Số lần | Kết quả |
|---|---:|---|
| P2 / P1 / P3 / select | 580 mỗi suite | **tất cả 14/0, 8/0, 10/0, 25/0** |
| default / TLS / net / pkg | 116 mỗi suite | **22/0, 22/0, 9/0, 25/0** |
| h2 (`vs3_http2`) | 58 | **33/33** |
| cancel default | 58 | **8/8** |
| cancel serial | 58 | **8/8** |

- Không có dòng `FAIL`/`FLAKE`; không cần rerun.
- Free RAM min/max: **8162 / 8782 MB**; đầu 8344, cuối 8344 — phẳng.

## 4. Cuối soak

- host: **54/54** (`1..54`, 120 lần chạy interp+VM, 0 lệch; VM 54/54), log `soakgd1-host.log`.
- targets: rc 0 — core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED**, 0 FAIL.
- Trạng thái worktree: HEAD `0376dae` (không đổi), dirty **0**, `C:` còn 124 G.

## 5. Bằng chứng

| Log `psvn` (`/c/Users/ps/vislogs/`) | sha256 |
|---|---|
| `soakgd1-summary.log` | `b49c24067a342e9208d804db1e274a7ce31a72ec123aaa41a19a77e9e64c6de3` |
| `soakgd1-host.log` | `a9339d682a8368ca4439b73f1e2b7b7dafd929c2bc07371feef6c501e9743602` |
| `soakgd1-targets.log` | `8f824cc9e20cd9f217aa8a5c27ca77a0eb712a276814b6e0690001c0bd889df3` |
| `soakgd1-last-h2.log` | `77c5ec01893b05975c6d0ad39ebcea079c292c1c2d4acd809cfe3b9b45edc32f` |
| `soakgd1-last-cancel.log` | `f2b21497d8aeafc3b54492c17e664b17141305634efb806f9be96fb312f4fe34` |
| `soakgd1-last-cancels.log` | `7fec2c1795c16795ddceabce35a0da32fe135b8aa8b8badb62c2dc4bc518e5cd` |

- Bản tổng hợp trong repo: `agent/JOURNAL/gd1-winsoak-2h.log` (580 dòng R + header).
- Local `/` còn **14 G** (>5 G).

## 6. Kết luận / blocker

- `0376dae` (GĐ1 combined) **ổn định 2h trên Windows**: 580 vòng, **0 fail/0 flake**, cancel h1
  **116 lượt 8/8**, h2 **58 lượt 33/33**, host/targets xanh — không thấy rò rỉ RAM.
- **Không blocker.** Không tag/push/merge; không GitHub Actions.
