<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — stress 3 vòng Windows `psvn` @ main `71d52b4`

> **Người đo**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Mốc**: main `71d52b4` (`CLI --version/doctor features merged, gate 3530`).
> **Kết quả**: 3 vòng đầu có **1 flake test-only** (`net.rs::tcp_explain_tong`, Windows 10054);
> đã sửa test server (`0bd3a64`), chạy lại **3/3 vòng xanh, không flake**.
> Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Lần chạy | Vòng 1 | Vòng 2 | Vòng 3 | Ghi chú |
|---|---|---|---|---|
| `71d52b4` (exact) | 3464/0 (426 s) | 3464/0 (426 s) | **3463/1 (425 s)** | flake `tcp_explain_tong` (10054) |
| **`0bd3a64`** (+fix test-only) | **3464/0 (421 s)** | **3464/0 (427 s)** | **3464/0 (421 s)** | **0 flake** |

- Mỗi vòng: `cargo test --workspace --no-fail-fast` + `bin/vis-host-suite` + `bin/vis-targets`.
- Cả 3 vòng sau fix: 349 block, 3464 passed / 0 failed / 3 ignored; host **54/54**;
  targets core **18/18**, sqlite **17 + 1 EXPECTED_RED**.
- `psvn` cuối @ `0bd3a64`, cây sạch.

## 1. Flake exact `71d52b4` — test-only

- Vòng 3: `net.rs::tcp_explain_tong` FAILED, exit 101:
  ```text
  assertion `left == right` failed: hai đường phải cùng byte (kể cả stderr)
  interp: (0, "nhan hi\n", …)
  vm:     (0, "loi lỗi đọc từ `C2`: An existing connection was forcibly closed
                by the remote host. (os error 10054)\n", …)
  ```
- Nguyên nhân: `server_n`/`server_bytes` trong `tests/net.rs` ghi dữ liệu rồi **drop socket ngay**;
  trên Windows client (VM) có thể nhận RST (10054) thay vì đọc dữ liệu, trong khi interp kịp đọc.
  Đây là **race của test server**, không phải lỗi sản phẩm.
- Sửa **test-only** `0bd3a64`: sau `write_all`, `shutdown(Shutdown::Write)` + đọc nốt tới EOF/timeout
  500 ms trước khi drop (`server_n`, `server_bytes`). Local `tests/net` 9/9, clippy PASS.

## 2. 3 vòng sau fix `0bd3a64`

| Vòng | Thời gian | Passed | Failed | Ignored | Block | Host | Targets |
|---|---:|---:|---:|---:|---:|---|---|
| 1 | **421 s** | 3464 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |
| 2 | **427 s** | 3464 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |
| 3 | **421 s** | 3464 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |

- Không `test result: FAILED`, không flake; host/targets log giống nhau từng byte giữa 3 vòng.
- Chênh 421–427 s (~1,5%) — bình thường.

## 3. CLI xác nhận trên Windows

Sau 3 vòng, dùng `target/debug/vis.exe`:
- `--version` → `vis 0.1.0` (rc 0); `-V` → `vis 0.1.0` (rc 0).
- `doctor` → `features sqlite,tls,pkg-sig,http2` (rc 0).
- `run --version` → rc **2**, `vis run: không hỗ trợ cờ --version` (không bị nuốt).

## 4. Log / bằng chứng

| Vòng | Cargo log / sha256 | Host log | Targets log |
|---|---|---|---|
| 1 | `/tmp/opencode/psvn-71d2-r1-cargo.log` — `00bca85b…2e59` | `…r1-host.log` (`a9101624…cf67`) | `…r1-targets.log` (`8f824cc9…9df3`) |
| 2 | `/tmp/opencode/psvn-71d2-r2-cargo.log` — `8665bd3e…d51f` | `…r2-host.log` (cùng sha) | `…r2-targets.log` (cùng sha) |
| 3 | `/tmp/opencode/psvn-71d2-r3-cargo.log` — `a86b423b…3162` | `…r3-host.log` (cùng sha) | `…r3-targets.log` (cùng sha) |
| flake (exact) | `/tmp/opencode/psvn-71d-r3-flake-cargo.log` — `446ef863…116a` | — | — |
| tổng hợp | `/tmp/opencode/psvn-71d2-all.log` — `0ebd4fa1…e857` | — | — |

Log máy đích: `/c/Users/ps/vislogs/w471d2-r{1,2,3}-{cargo,host,targets}.log`;
vis-remote: `vis-remote-20261121T115555Z.log` (3 vòng sau fix), `…T121759Z.log` (trạng thái cuối).

## 5. Đề xuất / blocker

- Đưa `0bd3a64` (test-only, 1 tệp `tests/net.rs`) vào main nếu muốn Windows stress sạch; code sản
  phẩm không đổi.
- **Không có blocker**. Local `/` còn **19G** (>5G); `psvn` sạch.
- Không tag/push/merge; không GitHub Actions.
