<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 soak 10/10 — tag `v0.3.3` / `bbf3484`

> **Người chạy**: W3 (QA/hardening), 2026-11-21 21:58–22:12 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| Tag tested | **`v0.3.3`** (annotated `28c2ebb`) → commit **`bbf3484`** |
| `main` | **`fefe9bd`** — `[VS3] release: v0.3.3 tagged (Linux 3535/3, Windows 3469/0)` |
| Branch QA | `vs3/w3-registry`, merge `main` → merge commit **`506b297`** (docs only) |
| Target | `.../target/qa-w3-bbf3484soak` (riêng) |
| Env | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` |
| Disk | đầu **19G**; thấp nhất **17G** (> 5G) |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |

## 1. Soak 10/10

`./bin/vis-soak --log /tmp/opencode/qa-w3-soak-bbf3484-soak.log`

| Vòng | Kết quả | Thời gian |
|---|---|---|
| gate-1 | OK | 171 s |
| gate-2 | OK | 134 s |
| diff-1 | OK | 80 s |
| diff-2 | OK | 75 s |
| gate-3 | OK | 93 s |
| diff-3 | OK | 69 s |
| host-1 | OK | 4 s |
| host-2 | OK | 1 s |
| cache-1 | OK | 9 s |
| cache-2 | OK | 10 s |
| **TỔNG** | **10/10 OK** | **646 s** |

- Mỗi gate: **3535 passed / 3 skipped**.
- Diff trong soak seed cố định `20261115`: lệch 0.
- Host: **54/54** + `host_posix_real` PASS (Linux x86_64).
- Không T-INFRA; target riêng.
- Wrapper: `/tmp/opencode/qa-w3-soak-bbf3484.log`.

## 2. Bổ sung ngoài soak

| Lệnh | Kết quả | Thời gian | Log |
|---|---|---|---|
| `./vis-dev nightly --skip-gate --fuzz-minutes 2` | **PASS** — fuzz **13.704.667 ca / 0 vi phạm**; fmt 475 tệp | 120,84 s | `/tmp/opencode/qa-w3-soak-bbf3484-nightly.log` |
| `VIS_DIFF_N=100000 VIS_DIFF_SEED=202611212209 ./bin/vis-diff` | **PASS — 100.000 ca, lệch 0** | 69,66 s | `/tmp/opencode/qa-w3-soak-bbf3484-diff.log` |

- Diff seed mới: 93.766 chạy, 6.234 lỗi tĩnh, **lệch 0**.

## 3. Đánh giá

- **Soak 10/10 PASS** trên tag `v0.3.3` (`bbf3484`): gate 3535/3, nightly PASS, diff 100k lệch 0,
  host 54/54.
- Khớp tag v0.3.3: Linux **3535/3**, Windows **3469/0** (theo commit message release).
- Không cần tag mới.

## 4. Blocker / còn lại

1. Không có blocker local.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows do W4 thủ công.
4. Tàn dư đã biết: worker evaluator, `select` VS3-2c, async HTTP Unix — release notes.

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR=$PWD/target/qa-w3-bbf3484soak
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./bin/vis-soak --log /tmp/opencode/qa-w3-soak-bbf3484-soak.log
./vis-dev nightly --skip-gate --fuzz-minutes 2
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611212209 ./bin/vis-diff
```

**Kết luận**: tag `v0.3.3` (`bbf3484`) **soak 10/10 PASS** (646 s), gate 3535/3, nightly PASS, diff
100k lệch 0, host 54/54. Không có blocker kỹ thuật local.
