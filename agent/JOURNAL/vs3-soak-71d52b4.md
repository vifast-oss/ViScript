<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 soak 10/10 — `71d52b4` (CLI --version/doctor)

> **Người chạy**: W3 (QA/hardening), 2026-11-21 20:31–20:48 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`71d52b4`** (CLI `--version`/`doctor`; gate kỳ vọng 3530/3) |
| Code | v0.3.2 + CLI hardening; không đổi trong phiên |
| Target | `.../target/qa-w3-soak-71d52b4` (riêng) |
| Env | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` |
| Disk | đầu **19G**; thấp nhất **17G** (> 5G) |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |

## 1. Soak 10/10

`./bin/vis-soak --log /tmp/opencode/qa-w3-soak-71d52b4-soak.log`

| Vòng | Kết quả | Thời gian |
|---|---|---|
| gate-1 | OK | 171 s |
| gate-2 | OK | 134 s |
| diff-1 | OK | 103 s |
| diff-2 | OK | 106 s |
| gate-3 | OK | 126 s |
| diff-3 | OK | 102 s |
| host-1 | OK | 4 s |
| host-2 | OK | 2 s |
| cache-1 | OK | 11 s |
| cache-2 | OK | 10 s |
| **TỔNG** | **10/10 OK** | **769 s** |

- Mỗi gate: **3530 passed / 3 skipped**.
- Diff trong soak seed cố định `20261115`: lệch 0.
- Host: **54/54** + `host_posix_real` PASS (Linux x86_64).
- Không T-INFRA; target riêng.
- Wrapper: `/tmp/opencode/qa-w3-soak-71d52b4.log`.

## 2. Bổ sung ngoài soak

| Lệnh | Kết quả | Thời gian | Log |
|---|---|---|---|
| `./vis-dev nightly --skip-gate --fuzz-minutes 2` | **PASS** — fuzz **12.886.921 ca / 0 vi phạm**; fmt 475 tệp | 120,74 s | `/tmp/opencode/qa-w3-soak-71d52b4-nightly.log` |
| `VIS_DIFF_N=100000 VIS_DIFF_SEED=202611212045 ./bin/vis-diff` | **PASS — 100.000 ca, lệch 0** | 71,08 s | `/tmp/opencode/qa-w3-soak-71d52b4-diff.log` |

- Diff seed mới: 93.767 chạy, 6.233 lỗi tĩnh, **lệch 0**.

## 3. Đánh giá

- **Soak 10/10 PASS** trên `71d52b4`; gate **3530/3**, nightly PASS, diff 100k lệch 0, host 54/54.
- CLI `--version`/`doctor` không gây hồi quy trong soak.
- Không cần tag mới; đây là hardening sau `v0.3.2`.

## 4. Blocker / còn lại

1. Không có blocker local.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows do W4 thủ công (v0.3.2 đã verify debug 3462/0 theo tag).
4. Tàn dư đã biết: worker evaluator, `select` VS3-2c, async HTTP Unix — release notes.

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR=$PWD/target/qa-w3-soak-71d52b4
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./bin/vis-soak --log /tmp/opencode/qa-w3-soak-71d52b4-soak.log
./vis-dev nightly --skip-gate --fuzz-minutes 2
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611212045 ./bin/vis-diff
```

**Kết luận**: `71d52b4` **soak 10/10 PASS** (769 s), gate 3530/3, nightly PASS, diff 100k lệch 0,
host 54/54. Không có blocker kỹ thuật local.
