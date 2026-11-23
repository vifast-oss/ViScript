<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 soak 10/10 — `c85b523` (target riêng)

> **Người chạy**: W3 (QA), 2026-11-21 17:48–18:00 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main` | **`c85b523`** (v0.3.1 candidate; W4 đang verify) |
| Code | **`e689c3d`** / `a96841a` (docs candidate, không đổi code so với QA v0.3.1) |
| Branch QA | `vs3/w3-registry`, merge `main` → merge commit **`9518f32`** |
| Target | `/home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry/target/qa-w3-soak2` (riêng, **không** dùng target chung) |
| Env | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` |
| Disk | đầu **29G**; thấp nhất **18G** (> 5G) |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |

## 1. Soak 10/10

`./bin/vis-soak --log /tmp/opencode/qa-w3-soak2-c85b523-soak.log`

| Vòng | Kết quả | Thời gian |
|---|---|---|
| gate-1 | OK | 73 s |
| gate-2 | OK | 77 s |
| diff-1 | OK | 59 s |
| diff-2 | OK | 61 s |
| gate-3 | OK | 103 s |
| diff-3 | OK | 95 s |
| host-1 | OK | 5 s |
| host-2 | OK | 2 s |
| cache-1 | OK | 15 s |
| cache-2 | OK | 12 s |
| **TỔNG** | **10/10 OK** | **502 s** |

- Mỗi gate: **3512 passed / 3 skipped**.
- Diff trong soak dùng seed cố định `20261115`: lệch 0.
- Host: **54/54** + `host_posix_real` PASS (Linux x86_64).
- Target riêng nên **không** tái diễn T-INFRA từng gặp khi dùng target chung.
- Wrapper: `/tmp/opencode/qa-w3-soak2-c85b523.log`.

## 2. Bổ sung ngoài soak

| Lệnh | Kết quả | Thời gian | Log |
|---|---|---|---|
| `./vis-dev nightly --skip-gate --fuzz-minutes 2` | **PASS** — fuzz **15.692.665 ca / 0 vi phạm**; fmt 475 tệp | 121,79 s | `/tmp/opencode/qa-w3-soak2-c85b523-nightly.log` |
| `VIS_DIFF_N=100000 VIS_DIFF_SEED=202611211757 ./bin/vis-diff` | **PASS — 100.000 ca, lệch 0** | 99,32 s | `/tmp/opencode/qa-w3-soak2-c85b523-diff.log` |

- Diff seed mới: 93.765 chạy, 6.235 lỗi tĩnh, lệch **0**.

## 3. Đánh giá

- **Soak 10/10 PASS** trên `c85b523`: không lỗi sản phẩm, không T-INFRA.
- Gate/nightly/diff/host ổn định; khớp candidate v0.3.1 (`3512/3`).
- `c85b523` chỉ là docs cập nhật trạng thái QA, code vẫn `a96841a`/`e689c3d`.
- Đề xuất tag `v0.3.1` **sau khi W4 xác nhận Windows psvn**; W3 không tag.

## 4. Blocker / còn lại

1. **Windows psvn** tại `a96841a`/`e689c3d` chưa có kết quả trong phiên này — blocker tag.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows do W4 thủ công.
4. Tàn dư đã biết: worker evaluator, `select` VS3-2c, async net/TLS chỉ Unix — release notes.

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR=$PWD/target/qa-w3-soak2
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0

./bin/vis-soak --log /tmp/opencode/qa-w3-soak2-c85b523-soak.log
./vis-dev nightly --skip-gate --fuzz-minutes 2
VIS_DIFF_N=100000 VIS_DIFF_SEED=<seed-moi> ./bin/vis-diff
```

**Kết luận**: `c85b523` **soak 10/10 PASS** (502 s), gate 3512/3, nightly PASS, diff 100k lệch 0,
host 54/54. Không có blocker kỹ thuật local; còn Windows W4 trước tag v0.3.1.
