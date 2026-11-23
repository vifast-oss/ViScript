<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — stress 3 vòng Windows `psvn` @ main `77e2899` (v0.3.2)

> **Người đo**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Mốc**: main `77e2899` (`v0.3.2 tagged` = `864cc96`; code trùng `93dc71f` — diff chỉ docs).
> **Phương pháp**: 3 vòng **tuần tự**, mỗi vòng `cargo test --workspace --no-fail-fast` +
> `bin/vis-host-suite` + `bin/vis-targets`; không chạy build/test song song.
> **Kết luận**: **3/3 vòng xanh, không flake**; 3462 passed / 0 failed / 3 ignored mỗi vòng.

## 0. TL;DR

| Vòng | Thời gian (3 lệnh) | Passed | Failed | Ignored | Block | Host | Targets |
|---|---:|---:|---:|---:|---:|---|---|
| 1 | **423 s** (09:59:00→10:06:03Z) | 3462 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |
| 2 | **430 s** (10:06:03→10:13:13Z) | 3462 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |
| 3 | **422 s** (10:13:13→10:20:15Z) | 3462 | **0** | 3 | 349 | 54/54 | 18/18 + 17/1 |

- Tổng ~21 phút 15 giây; chênh 422–430 s (~2%).
- Không `test result: FAILED`, không ca flake; không cần chạy lại.
- `psvn` @ `77e2899`, `git status --porcelain` = 0.

## 1. Mốc / môi trường

- Main `77e2899` (`[VS3] release: v0.3.2 tagged`); code `93dc71f` (tag `864cc96` cùng cây code).
- `psvn` checkout đúng `77e2899`, cây sạch; Windows `MINGW64_NT-10.0-26300`, 6 CPU;
  cargo/rustc `1.100.0`; target đã cài wasm32 + x86_64-unknown-linux-gnu.
- Không GitHub Actions; không build/test song song.

## 2. Kết quả từng vòng

### Vòng 1 — 423 s
- `cargo test --workspace --no-fail-fast`: **349/349 block; 3462 passed / 0 failed / 3 ignored**.
- Host **54/54**; targets core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED**.

### Vòng 2 — 430 s
- Cùng **3462 / 0 / 3**; 349 block.
- Host **54/54**; targets **18/18 + 17/1**.

### Vòng 3 — 422 s
- Cùng **3462 / 0 / 3**; 349 block.
- Host **54/54**; targets **18/18 + 17/1**.

### Flake
- Không có ca đỏ/flake; số block/pass/fail/ignored trùng khít cả 3 vòng.
- Host log và targets log **giống nhau từng byte** giữa 3 vòng (cùng sha256).
- Chênh thời gian 422–430 s nằm trong dao động bình thường của máy 6 CPU; không có dấu hiệu
  deadlock/timeout hay phụ thuộc thứ tự.

## 3. Log / bằng chứng

| Vòng | Cargo log / sha256 | Host log | Targets log |
|---|---|---|---|
| 1 | `/tmp/opencode/psvn-v032s-r1-cargo.log` — `7757b352…b431` | `…r1-host.log` (`a9101624…cf67`) | `…r1-targets.log` (`8f824cc9…9df3`) |
| 2 | `/tmp/opencode/psvn-v032s-r2-cargo.log` — `7f950e32…57eb` | `…r2-host.log` (cùng sha) | `…r2-targets.log` (cùng sha) |
| 3 | `/tmp/opencode/psvn-v032s-r3-cargo.log` — `aebee695…900e` | `…r3-host.log` (cùng sha) | `…r3-targets.log` (cùng sha) |
| tổng hợp | `/tmp/opencode/psvn-v032s-all.log` — `efc1b4f8…7682` | — | — |

Log máy đích: `/c/Users/ps/vislogs/w4v032s-r{1,2,3}-{cargo,host,targets}.log`;
vis-remote: `vis-remote-20261121T095856Z.log` (3 vòng), `…T102240Z.log` (trạng thái cuối).

## 4. Blocker

- **Không có blocker**. `ps` còn **11G** (>5G); `psvn` sạch.
- Không sửa sản phẩm/test; không tag/push/merge; không GitHub Actions.
