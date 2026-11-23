<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — stress 3 vòng Windows `psvn` @ main `d1f8f70`

> **Người đo**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Mốc**: main `d1f8f70` (`merge VS3 W1j: fuzz 30m 260M ca 0 vi phạm`),
> code có hardening test `vs3_http2`/`vs3_worker` (không chỉ docs).
> **Phương pháp**: 3 vòng **tuần tự**, mỗi vòng `cargo test --workspace --no-fail-fast` +
> `bin/vis-host-suite` + `bin/vis-targets`; không chạy build/test song song.
> **Kết luận**: **3/3 vòng xanh, không flake**; 3462 passed / 0 failed / 3 ignored mỗi vòng.

## 0. TL;DR

| Vòng | Thời gian (3 lệnh) | Passed | Failed | Ignored | Block | Host | Targets |
|---|---:|---:|---:|---:|---:|---|---|
| 1 | **419 s** (10:42:24→10:49:23Z) | 3462 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |
| 2 | **424 s** (10:49:23→10:56:27Z) | 3462 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |
| 3 | **427 s** (10:56:27→11:03:34Z) | 3462 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |

- Tổng ~21 phút 10 giây; chênh 419–427 s (~2%).
- Không `test result: FAILED`, không ca flake; không cần chạy lại.
- `psvn` @ `d1f8f70`, `git status --porcelain` = 0.

## 1. Mốc / môi trường

- Main `d1f8f70` (W1j hardening; test `vs3_http2`/`vs3_worker` đổi so với v0.3.2, không docs-only).
- `psvn` checkout đúng `d1f8f70`, cây sạch; Windows `MINGW64_NT-10.0-26300`, 6 CPU;
  cargo/rustc `1.100.0`; target wasm32 + x86_64-unknown-linux-gnu đã cài.
- Không GitHub Actions; không build/test song song.

## 2. Kết quả từng vòng

### Vòng 1 — 419 s
- `cargo test --workspace --no-fail-fast`: **349/349 block; 3462 passed / 0 failed / 3 ignored**.
- Host **54/54**; targets core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED**.

### Vòng 2 — 424 s
- Cùng **3462 / 0 / 3**; 349 block.
- Host **54/54**; targets **18/18 + 17/1**.

### Vòng 3 — 427 s
- Cùng **3462 / 0 / 3**; 349 block.
- Host **54/54**; targets **18/18 + 17/1**.

### Flake
- Không có ca đỏ/flake; số block/pass/fail/ignored trùng khít cả 3 vòng.
- Host log và targets log **giống nhau từng byte** giữa 3 vòng (cùng sha256).
- Chênh thời gian 419–427 s nằm trong dao động bình thường của máy 6 CPU; không có dấu hiệu
  deadlock/timeout hay phụ thuộc thứ tự.

## 3. Log / bằng chứng

| Vòng | Cargo log / sha256 | Host log | Targets log |
|---|---|---|---|
| 1 | `/tmp/opencode/psvn-d1f8-r1-cargo.log` — `604960b6…62ac` | `…r1-host.log` (`a9101624…cf67`) | `…r1-targets.log` (`8f824cc9…9df3`) |
| 2 | `/tmp/opencode/psvn-d1f8-r2-cargo.log` — `14f1c0a1…1a5e` | `…r2-host.log` (cùng sha) | `…r2-targets.log` (cùng sha) |
| 3 | `/tmp/opencode/psvn-d1f8-r3-cargo.log` — `46c5ab72…477d` | `…r3-host.log` (cùng sha) | `…r3-targets.log` (cùng sha) |
| tổng hợp | `/tmp/opencode/psvn-d1f8-all.log` — `7fe72dc2…1bbb` | — | — |

Log máy đích: `/c/Users/ps/vislogs/w4d1f8-r{1,2,3}-{cargo,host,targets}.log`;
vis-remote: `vis-remote-20261121T104221Z.log` (3 vòng), `…T110347Z.log` (trạng thái cuối).

## 4. Blocker

- **Không có blocker**. `ps` còn **19G** (>5G); `psvn` sạch.
- Không sửa sản phẩm/test; không tag/push/merge; không GitHub Actions.
