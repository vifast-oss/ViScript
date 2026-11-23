<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — 5 vòng Windows `psvn` @ tag v0.3.3 `bbf3484`

> **Người đo**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Mốc**: tag **v0.3.3** = `bbf3484` (code `0f18592`; main `a0ff400` docs-only so với tag).
> **Phương pháp**: 5 vòng **tuần tự**, mỗi vòng `cargo test --workspace --no-fail-fast` +
> `bin/vis-host-suite` + `bin/vis-targets`; không build/test song song.
> **Kết luận**: **5/5 vòng xanh, không flake**; 3469 passed / 0 failed / 3 ignored mỗi vòng.

## 0. TL;DR

| Vòng | Thời gian (3 lệnh) | Passed | Failed | Ignored | Block | Host | Targets |
|---|---:|---:|---:|---:|---:|---|---|
| 1 | **416 s** (13:23:06→13:30:02Z) | 3469 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |
| 2 | **417 s** (13:30:02→13:36:59Z) | 3469 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |
| 3 | **418 s** (13:36:59→13:43:57Z) | 3469 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |
| 4 | **422 s** (13:43:58→13:51:00Z) | 3469 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |
| 5 | **425 s** (13:51:00→13:58:05Z) | 3469 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |

- Tổng **2098 s (~34 phút 58 giây)**; chênh 416–425 s (~2%).
- Không `test result: FAILED`, không ca flake; không cần chạy lại.
- Host log và targets log **giống nhau từng byte** cả 5 vòng (cùng sha256).
- `psvn` @ `bbf3484`, cây sạch.

## 1. Mốc / môi trường

- Tag v0.3.3 `bbf3484`; main `a0ff400` chỉ docs (không đổi code).
- `psvn` checkout đúng `bbf3484`; Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc `1.100.0`;
  target wasm32 + Linux đã cài. Không GitHub Actions.

## 2. Kết quả từng vòng

Mỗi vòng đều:
- `cargo test --workspace --no-fail-fast`: **349/349 block; 3469 passed / 0 failed / 3 ignored**;
- `bin/vis-host-suite`: **54/54 PASS** (`host_posix_real` bỏ qua — Windows `test-child`);
- `bin/vis-targets`: core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED**; 0 FAIL; exit 0.

`EXPECTED_RED` duy nhất: `vis-cli × wasm32` (C toolchain `libsqlite3-sys`/`ring` cho wasm — nợ #156
đã biết). Không có ca đỏ/flake nào khác.

## 3. Log / bằng chứng

| Vòng | Cargo log / sha256 | Host | Targets |
|---|---|---|---|
| 1 | `/tmp/opencode/psvn-v033-5v-r1-cargo.log` — `5207cc87…b4e2` | `…r1-host.log` (`a9101624…cf67`) | `…r1-targets.log` (`8f824cc9…9df3`) |
| 2 | `…r2-cargo.log` — `f4997bf2…84e2` | `…r2-host.log` (cùng sha) | `…r2-targets.log` (cùng sha) |
| 3 | `…r3-cargo.log` — `e94a666d…837c` | `…r3-host.log` (cùng sha) | `…r3-targets.log` (cùng sha) |
| 4 | `…r4-cargo.log` — `9d61b4cb…8332` | `…r4-host.log` (cùng sha) | `…r4-targets.log` (cùng sha) |
| 5 | `…r5-cargo.log` — `bc31f9fd…8b69` | `…r5-host.log` (cùng sha) | `…r5-targets.log` (cùng sha) |
| tổng hợp | `/tmp/opencode/psvn-v033-5v-all.log` — `ac322f7c…61b8` | — | — |

Log máy đích: `/c/Users/ps/vislogs/w4v033-5v-r{1..5}-{cargo,host,targets}.log`;
vis-remote: `vis-remote-20261121T132303Z.log` (5 vòng), `…T135933Z.log` (trạng thái cuối).

## 4. Blocker

- **Không có blocker**. Local `/` còn **18G** (>5G); `psvn` còn ~184G, cây sạch.
- Không sửa sản phẩm/test; không tag/push/merge; không GitHub Actions.
