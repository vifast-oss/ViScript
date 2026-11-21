<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — Windows release check @ main `5103da4`

> **Người đo**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Mốc**: main `5103da4` (`docs: Windows net flake fix, psvn 3/3`); code trùng `0bd3a64`
> (`git diff --stat 0bd3a64..5103da4 -- crates/ Cargo.toml Cargo.lock` rỗng).
> **Kết luận**: release build/smoke/host/targets **PASS**; artifact không đổi (cache).
> Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `cargo build --release -p vis-cli` | **PASS**, **0 s** (cache; source không đổi) |
| `target/release/vis.exe` | 10.550.784 B, sha256 `e955706f…14bc` (không đổi) |
| `vis --version` / `-V` | `vis 0.1.0`, rc 0 |
| `vis doctor` / `--json` | PASS; `features sqlite,tls,pkg-sig,http2` |
| Smoke `vis run`/`vis check` | PASS — `release-smoke-ok`, TAP `ok 1` |
| `vis run --version` | rc 2 (không bị nuốt) |
| `bin/vis-host-suite` | **54/54** |
| `bin/vis-targets` | core **18/18**; sqlite **17 + 1 EXPECTED_RED**; exit 0 |
| Thời gian toàn bộ check | **15 s** (12:22:47→12:23:02Z) |
| `psvn` cuối | `5103da4`, cây sạch |

## 1. Mốc / build

- Main `5103da4` (docs-only so với `0bd3a64`; gồm test-only net flake fix `ec9fe72`).
- Branch `vs3/w4-windows` merge main → `7803a53`; `psvn` checkout đúng `5103da4`, cây sạch.
- `cargo build --release -p vis-cli`: **PASS**, 0 s vì release binary đã build từ code CLIs
  `c5d2be6` (không đổi từ đó); `target/release/vis.exe` 10.550.784 B, sha256 không đổi.

## 2. Smoke release

- `vis --version` và `-V` → `vis 0.1.0`, rc 0.
- `vis doctor` → `features sqlite,tls,pkg-sig,http2`; `--json` schema cũ, mảng đủ 4 feature.
- Smoke `vis run` → `release-smoke-ok`; `vis check` → TAP `1..1`, `ok 1`, rc 0.
- `vis run --version` → rc **2** + `vis run: không hỗ trợ cờ --version` (không đổi hành vi lệnh con).

## 3. Host / targets

- `bin/vis-host-suite`: **54/54 PASS** (`host_posix_real` bỏ qua — Windows `test-child`).
- `bin/vis-targets`: core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** (vis-cli × wasm do C
  toolchain sqlite/ring — nợ #156 đã biết); 0 FAIL; exit 0.

## 4. Log

| Lệnh | Log | sha256 | Exit |
|---|---|---|---|
| release build + smoke + host + targets @ psvn | `/tmp/opencode/psvn-5103-check.log` | `20ea9de1…69a5` | 0 |
| trạng thái cuối | `/tmp/opencode/psvn-5103-final-state.log` | — | 0 |

Log máy đích: `/c/Users/ps/vislogs/w45103-build.log`;
vis-remote: `vis-remote-20261121T122245Z.log` (check), `…T122342Z.log` (trạng thái cuối).

## 5. Blocker

- **Không có blocker**. Local `/` còn **19G** (>5G); `psvn` còn ~184G, cây sạch.
- Không sửa sản phẩm/test trong đợt này; không tag/push/merge; không GitHub Actions.
