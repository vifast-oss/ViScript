<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — stress 3 vòng Windows `psvn` @ main `800be80`

> **Người đo**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Mốc**: main `800be80` (`v0.3.1 tagged`; code `e689c3d`, docs-only so với code đã verify).
> **Phương pháp**: 3 vòng **tuần tự** `cargo test --workspace --no-fail-fast` + `bin/vis-host-suite`,
> không chạy build/test song song. Log riêng từng vòng trên psvn.
> **Kết luận**: **3/3 vòng xanh, không flake**; 3463 passed / 0 failed / 3 ignored mỗi vòng.

## 0. TL;DR

| Vòng | Thời gian (cargo + host) | Passed | Failed | Ignored | Block | Host |
|---|---:|---:|---:|---:|---:|---|
| 1 | **388 s** (08:52:48→08:59:16Z) | 3463 | **0** | 3 | 349 | 54/54 |
| 2 | **390 s** (08:59:16→09:05:46Z) | 3463 | **0** | 3 | 349 | 54/54 |
| 3 | **396 s** (09:05:46→09:12:22Z) | 3463 | **0** | 3 | 349 | 54/54 |

- Tổng thời gian 3 vòng: **~19 phút 34 giây**; chênh 388–396 s (~2%).
- Không có `test result: FAILED`, không có ca flake, không cần chạy lại.
- `psvn` @ `800be80`, `git status --porcelain` = 0.

## 1. Mốc / môi trường

- Main `800be80` (`[VS3] release: v0.3.1 tagged`); `git diff --stat e689c3d..800be80` chỉ docs,
  **0 tệp** trong `crates/`, `Cargo.toml`, `Cargo.lock` — code trùng bản đã verify `v0.3.1`.
- `psvn` checkout đúng `800be80` (bundle), cây sạch; Windows `MINGW64_NT-10.0-26300`, 6 CPU,
  cargo/rustc `1.100.0`; target đã cài `wasm32-unknown-unknown` + `x86_64-unknown-linux-gnu`.
- Không GitHub Actions; verify local/psvn. Không build/test song song (một tiến trình mỗi vòng).

## 2. Kết quả từng vòng

### Vòng 1 — 388 s
- `cargo test --workspace --no-fail-fast`: **349/349 block; 3463 passed / 0 failed / 3 ignored**.
- `bin/vis-host-suite`: **54/54 PASS** (`host_posix_real` bỏ qua — Windows `test-child`).

### Vòng 2 — 390 s
- Cùng 349 block; **3463 / 0 / 3**.
- Host **54/54**.

### Vòng 3 — 396 s
- Cùng 349 block; **3463 / 0 / 3**.
- Host **54/54**.

### Nhận xét flake
- Không có ca đỏ/flake trong 3 vòng; số block/pass/fail/ignored **trùng khít**.
- Host suite 54/54 cả 3 vòng (log host giống nhau từng byte: cùng sha256).
- Chênh thời gian 388→396 s nằm trong dao động bình thường của máy 6 CPU; không có dấu hiệu
  deadlock/timeout hay test phụ thuộc thứ tự.

## 3. Log / bằng chứng

| Vòng | Log cargo | sha256 | Log host |
|---|---|---|---|
| 1 | `/tmp/opencode/psvn-stress-r1-cargo.log` | `57c0338d…b6fd` | `/tmp/opencode/psvn-stress-r1-host.log` (`a9101624…cf67`) |
| 2 | `/tmp/opencode/psvn-stress-r2-cargo.log` | `eb1779cb…0c41` | `…r2-host.log` (cùng sha) |
| 3 | `/tmp/opencode/psvn-stress-r3-cargo.log` | `c6cdc5d4…9db1` | `…r3-host.log` (cùng sha) |
| tổng hợp | `/tmp/opencode/psvn-stress-all.log` | `076a94ec…ea07` | — |

Log trên máy đích: `/c/Users/ps/vislogs/w4stress-r{1,2,3}-{cargo,host}.log`;
log vis-remote: `vis-remote-20261121T085245Z.log` (3 vòng), `…T091317Z.log` (trạng thái cuối).

## 4. Blocker

- **Không có blocker**. Đĩa `ps` còn **27G** (>5G), `psvn` còn nhiều dung lượng.
- Không sửa sản phẩm/test; không tag/push/merge; không GitHub Actions.
