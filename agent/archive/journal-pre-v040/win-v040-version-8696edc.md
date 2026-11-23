<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Windows — xác minh bản phát hành `0.4.0` @ `8696edc`

> **Người đo**: subagent VS3-W4, 2026-11-23 · **trước tag**, time-box 1 h.
> **Mốc**: `main` **`8696edc2f04a0c916832af06a65b44c0a4d9b168`**
> (`merge ST-296: release v0.4.0 (version/cache/docs)`; Linux gate 3656/4).
> `psvn` worktree **riêng** `/c/Users/ps/viscript-v040-version-8696edc` @ `8696edc`, dirty **0**.
> `TMPDIR=/c/Users/ps/tmp`. Không tag/push/merge; không GitHub Actions.
> **Kết quả: PASS** — `vis 0.4.0`, cache key mới đúng, worker-frame + budget CLI đúng.
> **Xác nhận: sẵn sàng tag `v0.4.0` @ `8696edc`.**

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Release build | `vis.exe` **11 530 240 B**, sha256 `3813a1b3…821cb065` |
| `vis --version` / JSON | **`0.4.0`** (doctor `vis 0.4.0`, os windows, sqlite/tls/pkg-sig/http2) |
| `check` / `run` | TAP `1..1 ok 1`; `run` → `42`; rc 0 |
| `st289_version` | **2/2 pass** (ghim `BAN = "0.4.0"`) |
| Cache key | `cache::tests` **3/3 pass**; ghim **`22572d0b3267c93ddecc223a7522cc69767b3a5c1f6b46f8bff04d95d3803a8b`**; `assert_ne` khoá cũ `c7e09d0f…` xanh |
| `vs3_worker_frame` / `_vm` (release) | **12/12** / **6/6** |
| Budget CLI | loop `--workers 4 --max-budget 5000` → **rc 101 + VS0510**; `--workers 0` → **rc 2**; `--workers 4` → **42** |
| Blocker | **Không** |

## 1. Release build / CLI

- `cargo build --release -p vis-cli --bin vis` rc 0;
  `target/release/vis.exe` **11 530 240 B**,
  sha256 **`3813a1b3e7aa2f8608aaf0a168e2dffbc0d904554bf0a096fee5927a821cb065`**.
- `vis --version` → **`vis 0.4.0`**; `version --json` → `{"name":"vis","version":"0.4.0"}`;
  `doctor` → `vis 0.4.0`, `os windows`, `arch x86_64`, `sqlite on`,
  `features sqlite,tls,pkg-sig,http2`.
- Smoke: `check` TAP `1..1 ok 1` (rc 0); `run` → `42` (rc 0).

## 2. Version / cache

- `cargo test -p vis-cli --test st289_version` → **2 passed / 0 failed**.
  Test ghim `const BAN = "0.4.0"` và quét `[workspace.package] version` + mọi crate thành viên
  `version.workspace = true`.
- `cargo test -p vis-cli --bin vis cache::tests` → **3 passed / 0 failed** (gồm `khoa_cache_ghim`).
  - Khoá mới ghim: **`22572d0b3267c93ddecc223a7522cc69767b3a5c1f6b46f8bff04d95d3803a8b`**.
  - `assert_ne` khoá cũ `c7e09d0f0b535a3f8edb9d794c9d852f4e15c5fe07586b5d37fcdbe55f3fd393` xanh
    (cache bản 0.3.6 không tái dùng).
  - Các assert `VFEE`/`fee::VERSION = 4`/`WEIGHT_TABLE_VERSION = 1`/`INSTRUMENTED_VERSION = 3`
    trong cùng test đều xanh.

## 3. Worker frame / budget (release)

| Lệnh | Kết quả |
|---|---|
| `cargo test --release --test vs3_worker_frame` | **12/12** |
| `cargo test --release --test vs3_worker_frame_vm` | **6/6** |
| `run --workers 4 --max-budget 5000` chương trình ngắn | **rc 0**, `42` (trong ngân sách) |
| `run --workers 4 --max-budget 5000` loop 1e6 | **rc 101**, `VS0510: vượt ngân sách ngữ nghĩa (E_EXPR)` |
| `run --workers 0` | **rc 2**, `--workers 0` không hợp lệ (phải >= 1) |
| `run --workers 4` | **rc 0**, `42` |

- Budget `W=4` khớp `W=1` và Linux; `--workers 4` không đổi chương trình ngắn.

## 4. Bằng chứng

| Log `psvn` (`/c/Users/ps/vislogs/`) | sha256 |
|---|---|
| `v040ver-build.log` | `b753be1f…3886fdc` |
| `v040ver-st289.log` | `127d3ef5…7ceedf634` |
| `v040ver-cacheall.log` | `3edc0d0c…933dc4ce` |
| `v040ver-wf.log` / `v040ver-wfvm.log` | `15a10887…5e58c70` / `a72a3dea…4d0202bd` |

- Worktree HEAD `8696edc`, dirty **0**; `psvn` `C:` còn **92 G**; local `/` còn **16 G** (>10 G).
- Không tag/push/merge; không GitHub Actions.

## 5. Kết luận

- Bản phát hành **0.4.0** tại `8696edc` đạt trên Windows: release binary/CLI **0.4.0**, version test
  và cache key mới xanh, worker-frame **12/12 + 6/6**, budget CLI parity.
- **Xác nhận sẵn sàng tag `v0.4.0` @ `8696edc`.** Không blocker.
