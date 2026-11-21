<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 soak 10/10 — `a33f46c` (sau v0.3.1)

> **Người chạy**: W3 (QA/hardening), 2026-11-21 18:15–18:25 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main` | **`a33f46c`** (docs sau v0.3.1; W4 psvn stress 3/3 no flake) |
| Code | `e689c3d`/`a96841a` (v0.3.1) — nhánh QA fast-forward đúng `a33f46c` |
| Target | `/home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry/target/qa-w3-soak3` (riêng) |
| Env | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` |
| Disk | đầu **18G**; thấp nhất **16G** (> 5G) |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |

## 1. Soak 10/10

`./bin/vis-soak --log /tmp/opencode/qa-w3-soak3-a33f46c-soak.log`

| Vòng | Kết quả | Thời gian |
|---|---|---|
| gate-1 | OK | 93 s |
| gate-2 | OK | 94 s |
| diff-1 | OK | 77 s |
| diff-2 | OK | 91 s |
| gate-3 | OK | 127 s |
| diff-3 | OK | 84 s |
| host-1 | OK | 3 s |
| host-2 | OK | 1 s |
| cache-1 | OK | 11 s |
| cache-2 | OK | 10 s |
| **TỔNG** | **10/10 OK** | **591 s** |

- Mỗi gate: **3515 passed / 3 skipped** (tăng 3 ca so với `c85b523` do code/docs hardening).
- Diff seed cố định `20261115`: lệch 0.
- Host: **54/54** + `host_posix_real` PASS (Linux x86_64).
- Không T-INFRA; target riêng, không dùng target chung.
- Wrapper: `/tmp/opencode/qa-w3-soak3-a33f46c.log`.

## 2. Đánh giá

- **Soak 10/10 PASS** trên `a33f46c`, không lỗi sản phẩm, không flake trong 10 vòng.
- Khớp W4 psvn stress 3/3 no flake (theo docs main); local Linux ổn định.
- Đề xuất tag `v0.3.1` **sau khi W4 chốt Windows** (nếu chưa tag); W3 không tag.

## 3. Blocker / còn lại

1. Windows psvn tại code v0.3.1 — W4 đã stress 3/3 theo docs; cần xác nhận tag cuối.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows do W4 thủ công.
4. Tàn dư đã biết: worker evaluator, `select` VS3-2c, async net/TLS Unix — release notes.

## 4. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR=$PWD/target/qa-w3-soak3
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./bin/vis-soak --log /tmp/opencode/qa-w3-soak3-a33f46c-soak.log
```

**Kết luận**: `a33f46c` **soak 10/10 PASS** (591 s), gate 3515/3, host 54/54. Không có blocker kỹ
thuật local; Windows W4 là điều kiện tag còn lại.
