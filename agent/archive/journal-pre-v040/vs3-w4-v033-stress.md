<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — stress 3 vòng Windows `psvn` @ `bbf3484` (v0.3.3)

> **Người đo**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Mốc**: `bbf3484` = tag **v0.3.3** (code `0f18592`; main `fefe9bd` docs-only so với tag).
> **Phương pháp**: 3 vòng **tuần tự** `cargo test --workspace --no-fail-fast` +
> `bin/vis-host-suite` + `bin/vis-targets`; sau đó xác nhận `vs3_http2` 19/19 debug + release.
> **Kết luận**: **3/3 vòng xanh, không flake**; H2 19/19 cả debug và release.

## 0. TL;DR

| Vòng | Thời gian (3 lệnh) | Passed | Failed | Ignored | Block | Host | Targets |
|---|---:|---:|---:|---:|---:|---|---|
| 1 | **416 s** (12:58:55→13:05:51Z) | 3469 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |
| 2 | **421 s** (13:05:51→13:12:52Z) | 3469 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |
| 3 | **422 s** (13:12:52→13:19:54Z) | 3469 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |

- Tổng ~21 phút; chênh 416–422 s (~1,5%).
- Không `test result: FAILED`, không ca flake; host/targets log giống nhau từng byte giữa 3 vòng.
- H2: `vs3_http2` **19/19** debug (15,33 s) và **19/19** release (15,26 s), gồm 8 MiB, header cap,
  trailer, GOAWAY, timeout, stress 100 request.
- `psvn` @ `bbf3484`, cây sạch.

## 1. Mốc / môi trường

- Tag v0.3.3 = `bbf3484` (code `0f18592`: h2 flow-control body >64 KiB + release limits);
  main `fefe9bd` chỉ docs. `psvn` checkout đúng `bbf3484`, cây sạch.
- Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc `1.100.0`; target wasm32 + Linux đã cài.
- Không GitHub Actions; không build/test song song.

## 2. Kết quả từng vòng

### Vòng 1 — 416 s
- Full: **349/349 block; 3469 passed / 0 failed / 3 ignored**.
- Host **54/54**; targets core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED**.

### Vòng 2 — 421 s
- Cùng **3469 / 0 / 3**; 349 block; host **54/54**; targets **18/18 + 17/1**.

### Vòng 3 — 422 s
- Cùng **3469 / 0 / 3**; 349 block; host **54/54**; targets **18/18 + 17/1**.

### H2 xác nhận
- Debug: `cargo test -p vis-cli --test vs3_http2` → **19/19 PASS**, 15,33 s.
- Release: `cargo test --release -p vis-cli --test vs3_http2` → **19/19 PASS**, 15,26 s.
- `EXPECTED_RED` duy nhất ở matrix: `vis-cli × wasm32` (C toolchain sqlite/ring — nợ #156 đã biết).

## 3. Log / bằng chứng

| Vòng | Cargo log / sha256 | Host log | Targets log |
|---|---|---|---|
| 1 | `/tmp/opencode/psvn-v033s-r1-cargo.log` — `6c3c136e…4272` | `…r1-host.log` (`a9101624…cf67`) | `…r1-targets.log` (`8f824cc9…9df3`) |
| 2 | `/tmp/opencode/psvn-v033s-r2-cargo.log` — `f161e778…285c` | `…r2-host.log` (cùng sha) | `…r2-targets.log` (cùng sha) |
| 3 | `/tmp/opencode/psvn-v033s-r3-cargo.log` — `00866888…b9f9` | `…r3-host.log` (cùng sha) | `…r3-targets.log` (cùng sha) |
| H2 debug | `/tmp/opencode/psvn-v033s-h2-debug.log` — `2f40a742…9337` | — | — |
| H2 release | `/tmp/opencode/psvn-v033s-h2-rel.log` — `87c62da4…e2b4` | — | — |
| tổng hợp | `/tmp/opencode/psvn-v033s-all.log` — `948e5a42…3c15a` | — | — |

Log máy đích: `/c/Users/ps/vislogs/w4v033s-r{1,2,3}-{cargo,host,targets}.log`,
`w4v033s-h2-{debug,rel}.log`; vis-remote: `vis-remote-20261121T125852Z.log` (3 vòng),
`…T132102Z.log` (trạng thái cuối).

## 4. Blocker

- **Không có blocker**. Local `/` còn **19G** (>5G); `psvn` còn ~184G, cây sạch.
- Không sửa sản phẩm/test; không tag/push/merge; không GitHub Actions.
