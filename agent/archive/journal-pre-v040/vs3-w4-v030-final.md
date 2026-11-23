<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — verify **final v0.3.0** `0d26527` trên Windows (local/psvn)

> **Người đo**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Ứng viên cuối**: main `0d26527` (`v0.3.0 candidate 6f0ea95, gate 3496`),
> code `6f0ea95` (HTTP/2 default-on + h2 queue/idle, W1f async cap, W5 worker-copy).
> **Commit test trên psvn**: `3a9aa00` — merge `0d26527` vào `vs3/w4-windows`.
> **Code tương đương**: `git diff --stat 0d26527..3a9aa00 -- crates/ Cargo.toml Cargo.lock` **rỗng**.
> **Kết luận**: **xanh 0 failed — đề xuất tag `v0.3.0` tại `0d26527`**. Không tag/push; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả | Exit |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **349/349 block; 3452 passed / 0 failed / 3 ignored** | **0** |
| `bin/vis-dev check` | **PASS** (fmt, clippy `-D warnings`, R1..R6, toolchain, release, build, doc/readme examples) | 0 |
| `bin/vis-host-suite` | **54/54 PASS** (`host_posix_real` bỏ qua — Windows `test-child`) | 0 |
| `bin/vis-targets` | core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED**; 0 FAIL | **0** |
| `psvn` cuối | `3a9aa00`, cây sạch | — |

## 1. Mốc / merge

- Main `0d26527` (code `6f0ea95`); branch `vs3/w4-windows` merge main → `3a9aa00`, **không conflict**.
- `psvn`: fetch bundle `dd9e428..3a9aa00`, checkout `3a9aa00`, `git status --porcelain` = 0.
- `0d26527..3a9aa00` chỉ khác docs/journal W4; **0 tệp** trong `crates/`, `Cargo.toml`, `Cargo.lock`
  ⇒ cây code đã test **chính là** code `0d26527`.
- **Không GitHub Actions**: repo/branch không có `.github`; chỉ verify local/psvn.

## 2. Chi tiết kết quả

### 2.1. HTTP/2 default-on + queue/idle

- `crates/vis-cli/Cargo.toml`: `default = ["sqlite", "tls", "pkg-sig", "http2"]` — H2 **bật mặc định**.
- `h2 0.4.20` + `tokio` (IO traits) + `http`/`bytes` build trên Windows; **không** BLOCKED-ENV
  `lib.exe`/C toolchain (ring/rustls và rusqlite bundled vẫn build bình thường).
- `vs3_http2`: **8/8 PASS** (gồm queue reuse + idle timeout 15 s; 15,22 s); `vs3_tls`: **12/12**;
  `vs3_keepalive`: **8/8**.

### 2.2. Test trọng tâm khác

| Bộ | Kết quả |
|---|---|
| `vs3_worker` (W5 worker-copy payload lớn) | **9/9 PASS** |
| `vs3_chan_cap` (W1f async cap) | **6/6 PASS** |
| `vs3_chan_timeout` | **17/17 PASS** |
| `vs3_default` (async default) | **8/8 PASS** |
| Toàn workspace | **3452 passed / 0 failed / 3 ignored**; 349 block |

Không có ca đỏ Windows; không phải sửa test/sản phẩm.

### 2.3. Target matrix

| Cấu hình | PASS | EXPECTED_RED | FAIL |
|---|---:|---:|---:|
| `core (vis-cli --no-default-features)` | **18** | 0 | **0** |
| `sqlite (vis-cli default)` | **17** | 1 | **0** |

- `EXPECTED_RED` duy nhất: `vis-cli × wasm32` — `libsqlite3-sys`/`ring` cần C toolchain cho wasm
  (nợ #156/VS3-4a, đã biết). `vis-host-wasm` wasm PASS; `vis-host-linux` Linux PASS.
- `vis-cli × msvc` (sqlite) PASS; script nhắc rà soát nợ #156.

## 3. Đề xuất

- **Tag `v0.3.0` tại `0d26527`**: code đã verify **0 failed** trên Windows (H2 default-on);
  Linux gate 3496/3 (A0).
- Nếu A0 muốn tag đúng commit đã chạy test: `3a9aa00` — chỉ khác docs/journal W4, code y hệt.
- Không tag/push/merge trong lần này; không dùng GitHub Actions.

## 4. Log

| Lệnh | Log | sha256 | Exit |
|---|---|---|---|
| `cargo test` | `/tmp/opencode/psvn-0d26-cargo.log` | `8cd91575…1934` | **0** |
| `vis-dev check` + host | `/tmp/opencode/psvn-0d26-gate.log` | `16bbe8ef…8bf1` | 0 |
| `vis-targets` | `/tmp/opencode/psvn-0d26-targets.log` | `fbcd8b18…f005` | **0** |
| trạng thái cuối | `/tmp/opencode/psvn-0d26-final-state.log` | — | 0 |

Log máy đích: `vis-remote-20261121T075458Z.log` (cargo), `…T080245Z.log` (gate + host),
`…T080347Z.log` (targets), `…T080407Z.log` (trạng thái cuối).
