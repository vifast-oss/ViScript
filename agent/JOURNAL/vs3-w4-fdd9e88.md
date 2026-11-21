<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — xác nhận P1 evaluator trên Windows (`fdd9e88`)

> **Người đo**: subagent VS3-W4, 2026-11-22.
> **Mốc**: `main` **`fdd9e88a`** (`merge VS3 W4q: verify a8100ab psvn + release`; có evaluator P1
> `54021de`; Linux gate **3547/3**). `psvn` worktree **detached đúng `fdd9e88`**, dirty 0.
> **Kết quả**: 2/2 workspace **3504/0/3**, 5/5 vòng targeted, **P1 8/8 debug + release**,
> host 54/54, targets 18/18 + 17/1, release/CLI OK; **0 flake 10053**.
> Không tag/push/merge vào `main`; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `cargo test --workspace --no-fail-fast` ×2 | **351 block; 3504 passed / 0 failed / 3 ignored** mỗi vòng |
| `vs3_worker_eval_local` debug | **8/8**, 0 failed (0,20 s) |
| `vs3_worker_eval_local` release | **8/8**, 0 failed (0,03 s; build 69 s) |
| `vs3_default` + `vs3_tls` + `net` ×5 | **22/22 + 22/22 + 9/9** mỗi vòng; 0 flake 10053 |
| host / targets | **54/54**; core **18/18**, sqlite **17 + 1 EXPECTED_RED** |
| Release | `vis.exe` **10 754 560 B**, sha256 `8476e9bc…d0d44` |
| CLI | `--version` `vis 0.1.0`; `doctor` đủ 4 feature; `check` TAP ok; `run` → `42`; `run --version` rc 2 |
| `cargo check --features async-h2` | **rc 101 — EXPECTED** (Unix-only), không sửa |
| Blocker | **Không có** |

## 1. Môi trường

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC native).
- Worktree đo `/c/Users/ps/viscript-w4d` detached **`fdd9e88a`**, dirty 0; repo chính dirty 0.

## 2. 2 vòng workspace

| Vòng | Kết quả | Thời gian |
|---|---|---:|
| WS1 | **351 block; 3504 passed / 0 failed / 3 ignored** | 521 s |
| WS2 | **351 block; 3504 passed / 0 failed / 3 ignored** | 459 s |

- Đúng kỳ vọng **+8 ca P1** (3496 → 3504). Số block 351 (không phải 350) do main có thêm
  test target `vs3_worker_eval_local` (và target cũ); nội dung pass/fail/ignored khớp.
- Không `FAILED`/`panicked`; không flake.

## 3. 5 vòng targeted

`cargo test -p vis-cli --test vs3_default --test vs3_tls --test net`

| Vòng | net | vs3_default | vs3_tls | Thời gian |
|---|---:|---:|---:|---:|
| R1 | 9/9 | 22/22 | 22/22 | 51 s (build) |
| R2 | 9/9 | 22/22 | 22/22 | 11 s |
| R3 | 9/9 | 22/22 | 22/22 | 11 s |
| R4 | 9/9 | 22/22 | 22/22 | 12 s |
| R5 | 9/9 | 22/22 | 22/22 | 12 s |

- **`10053` không tái hiện** (fix `54f813f`); fd-leak + invariant host_poll xanh mỗi vòng.

## 4. P1 evaluator (`vs3_worker_eval_local`)

| Chế độ | Kết quả | Thời gian |
|---|---|---:|
| debug | **8 passed / 0 failed** | 0,20 s |
| release | **8 passed / 0 failed** | build 69 s, chạy 0,03 s |

8 ca: `sync_byte_identical`, `print_va_thu_tu_byte_identical`, `budget_can_giong_default`,
`loi_than_boc_vs0506_giong_default`, `chan0_giu_vs0505_nhu_non_frame`,
`await_hai_lan_tra_cache_byte_identical`, `nested_spawn_await_byte_identical`,
`muoi_nghin_task_byte_identical` — **tất cả xanh trên Windows** (cả debug và release).

## 5. Host / targets / release / CLI

- `bin/vis-host-suite`: **54/54** (`host_posix_real` bỏ qua — Windows `test-child`).
- `bin/vis-targets`: core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** (#156); 0 FAIL.
- `cargo build --release -p vis-cli`: rc 0 (artifact đã dựng cùng release test).
- Artifact: `target/release/vis.exe` **10 754 560 B**,
  sha256 `8476e9bc693e7649528bcd4af3f32678ec21f7e4fbe6cad63c95f0e1efbd0d44`.
- `vis --version` → `vis 0.1.0`; `vis doctor` → `os windows`, `arch x86_64`, `sqlite on`,
  `features sqlite,tls,pkg-sig,http2`.
- Smoke: `vis check` → TAP `1..1 ok 1`; `vis run` → `42`; `vis run --version` → rc 2.

## 6. `--features async-h2` trên Windows — EXPECTED RED

```text
cargo check -p vis-cli --features async-h2 => rc 101
  h2_async.rs:15 #![cfg(all(feature = "async-h2", feature = "http2", feature = "tls", unix))]
```

- Đúng dự kiến: async-h2 **Unix-only/default-off**; Windows async net/TLS/HTTP-1.1 không phụ
  thuộc feature này. Không sửa, không tuyên bố hỗ trợ h2 async Windows.

## 7. Log / bằng chứng

| Log `psvn` | sha256 |
|---|---|
| `/c/Users/ps/vislogs/w4d-targeted.log` | `5a87f38f126ff4a0dc866388672e5b022c1be5e0b86eb27fcef8572e4a699c8a` |
| `/c/Users/ps/vislogs/w4d-workspace.log` | `7216e7bd45b945ec2bafaf0f2d46c22b1b483eb4cd2977a1cd0e88b1130f4b6f` |
| `/c/Users/ps/vislogs/w4d-release.log` | `f733369e877f0bfbd24ce5f5c05546d660a5e06401686df92781f75713f6f4d9` |

- Worktree `psvn` đã xoá sau đo; repo chính `psvn` @ `73af17bd`, dirty 0; `C:` còn **174G**.
- Local `/` còn **25G** (>5G).

## 8. Kết luận

- Commit **`fdd9e88`** đạt trên Windows: 2/2 workspace 3504/0/3, 5/5 targeted, P1 **8/8**
  debug+release, host/targets xanh, release/CLI smoke đạt; 0 flake.
- Windows async net/TLS/HTTP-1.1 + evaluator P1 **đã hỗ trợ theo phạm vi đo**;
  async-h2 vẫn **Unix-only/default-off** (`--features async-h2` đỏ là expected).
- **Không blocker.**
