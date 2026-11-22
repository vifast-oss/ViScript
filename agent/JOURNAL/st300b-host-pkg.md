<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-300b — tách `vis-host-native` + `vis-pkg`, `main.rs` < 900, `lsp_it` < 800

> **Người làm**: VS3-W2, 2026-11-23. **Nhánh** `plan/w2-st300b-host-pkg` từ **`9af3976`**
> (main sau VD-283). **Không merge/push/tag** trước `v0.4.5`.
> `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/home/ps/tmp`; target riêng
> `target/st300b`; không có `sccache` trên máy (`which sccache` rỗng).

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `crates/vis-host-native` | **13.107 dòng**, 0 tham chiếu `vis_cli` ngược, `#![forbid(unsafe_code)]` |
| `crates/vis-pkg` | **3.270 dòng**, re-export `vis_cli::pkg`/`pkg_sig` |
| `crates/vis-cli/src` | **21.147 → 4.809 dòng** (`-77,3 %`) |
| `main.rs` | **1.403 → 225 dòng** |
| `tests/lsp_it/lsp.rs` | **1.478 → 9 dòng** + `lsp_part1` 759 + `lsp_part2` 728 |
| Feature matrix | 12 tổ hợp `cargo check` xanh (`vis-host-native`/`vis-pkg`/`vis-cli`) |
| Unit test crate mới | host-native **113/113** (`--all-features`), vis-pkg **27/27** |
| `bin/vis-targets` | core **18/18**, sqlite **16 PASS + 2 EXPECTED_RED** (#156) |
| Diff 100k | **lech=0** (93.768 chạy, 6.232 lỗi tính) |
| Gate | **3660 passed / 0 failed / 4 skipped**; fmt/clippy/R1..R7 PASS |
| Blocker | **không** |

## 1. Vì sao 4,8k chứ không đúng ~3,9k?

VD-283 tính `21,1k − 13,2k − 4,0k ≈ 3,9k`. Số đo thật: `pkg` chỉ **3.270** dòng
(không phải 4,0k), nên phần còn lại là **4.809**. Trong đó `vis test` (`vistest.rs` 1.249)
và helper test (`bin/test_child.rs` 229) **giữ nguyên trong `vis-cli`** vì là mặt lệnh công khai
(`vis test`) và hợp đồng `CARGO_BIN_EXE_test-child`; không nằm trong phạm vi tách của VD-283.
Nếu trừ hai tệp này, vỏ CLI còn **3.331** dòng. Không cắt bớt chức năng để đạt con số.

## 2. Cấu trúc sau tách

```
crates/vis-host-native/src/   quyen{,_chuan,_guard,_mo,_path,_st174,_st295}.rs, ngoai.rs,
                              fsmock.rs, net{,/**}, net_quyen.rs, async_net*.rs, db/**
crates/vis-pkg/src/           pkg.rs, pkg/**, pkg_sig.rs
crates/vis-cli/src/           main.rs (225), cli/{run_cmd,fmt,check,run,doctor,version}.rs,
                              vistest.rs, bin/test_child.rs
crates/vis-cli/tests/lsp_it/  lsp.rs (9) + lsp/lsp_part1.rs (759) + lsp/lsp_part2.rs (728)
```

- `vis-host-native` giữ nguyên body mã; chỉ đổi `crate::` nội bộ (đều trỏ trong crate mới) và
  fixture TLS dùng `CARGO_MANIFEST_DIR/../vis-cli/tests/fixtures/vs3_tls` (một bản, không nhân đôi).
- `vis-pkg` thay `vis_cli::pkg_sig` → `crate::pkg_sig`, `vis_cli::net::Net` →
  `vis_host_native::net::Net`.
- `vis-cli` re-export để API cũ không đổi: `vis_cli::{net, async_net, pkg_sig}`,
  `crate::{db,fsmock,ngoai,quyen}` trong `main.rs`.
- **Sai lệch nhỏ so với văn bản task**: `vis-pkg` cần thêm `vis-diag` (mã lỗi `VS0704/VS0708`) và
  `vis-host-native` (Net cho registry). Cả hai đều là crate nội bộ đã có trong cây, không thêm
  dependency bên thứ ba; `tls`/`pkg-sig` vẫn là feature tùy chọn.
- `#[cfg]` giữ nguyên: `sqlite` (`rusqlite`), `tls` (`rustls`/`webpki-roots`), `http2`, `async-h2`;
  `mio` chỉ `cfg(any(unix,windows))`, `rustix` chỉ `cfg(unix)`.

## 3. Kiểm chứng

- **Feature matrix** (log `st300b-features.log`): 4 tổ hợp host-native (default/tls/sqlite/all),
  3 tổ hợp vis-pkg (default/tls/pkg-sig), 5 tổ hợp vis-cli (no-default/tls/pkg-sig/sqlite/all) — xanh.
- **Crate mới**: `cargo test -p vis-host-native --all-features` → **113 pass**;
  `cargo test -p vis-pkg --all-features` → **27 pass**.
- **LSP**: 152 test xanh sau khi tách 3 tệp; `vis-dev` R1 không còn tệp > 1.500 dòng.
- **`bin/vis-targets`**: `core 18 PASS / 0 FAIL`; `sqlite 16 PASS + 2 EXPECTED_RED` (#156).
- **Diff 100k**: `DIFF tong=100000 ... lech=0`, 80,6 s, seed `3523158054`.
- **Gate**: `./vis-dev gate --fast` → PASS, **3660 passed / 0 failed / 4 skipped** (80,1 s);
  fmt, clippy, R1..R7, toolchain, release, self-test, explain, report, build, doc/readme examples OK.
- Một test nguồn `cache_fastpath::khong_con_fast_path_khoa_v1` được cập nhật để quét thêm
  `cli/run_cmd.rs` (nơi `run_run` đã chuyển tới) — vẫn khẳng định **không** đọc khoá v1 và
  **có** `khoa_multi`; không đổi hành vi runtime.

## 4. Log ngoài repo (R7) + sha256

| Tệp | Kích thước | sha256 |
|---|---|---|
| `/home/ps/vislogs/st300b-gate-6.log` | 361 KB | `08ce4b0586c4e18f1596fabfd8aec837bac6f4af700ceab177304f7a0397a9be` |
| `/home/ps/vislogs/st300b-targets.log` | 3,4 KB | `0f03e66f1389440d32d03f17c11ea029eb717970918d24d1405b9506cfff9bf7` |
| `/home/ps/vislogs/st300b-diff100k.log` | 265 B | `d76c46e3118fb9c3095efeee393dddd64f1b469fe73e76995274c0dad9027164` |
| `/home/ps/vislogs/st300b-features.log` | 527 B | `0492701a0d6cb98b8b74745fc2ef4f05c0faf236d1b6de9c389d3427dadd692e` |
| `/home/ps/vislogs/st300b-host-native-tests.log` | — | `e3d0daa6aa04ae24bdb550c0ea508d654d17cda4b999a4984c83edbd9afdd3ef` |
| `/home/ps/vislogs/st300b-pkg-tests.log` | — | `65a3ed9f98c6c30faca21943124e94bf6fec7a6f3cdb045a8cf0368948a7ecbf` |

## 5. Kết luận

- Tách xong hai crate, CLI chỉ còn điều phối + `vis test`; API cũ giữ qua re-export.
- **0 đổi hành vi** theo bằng chứng: gate 3660/4, targets 18/18, diff 100k lệch 0, LSP 152/152.
- Chưa merge/push/tag; chờ sau `v0.4.5`.
- **Blocker: không**.
