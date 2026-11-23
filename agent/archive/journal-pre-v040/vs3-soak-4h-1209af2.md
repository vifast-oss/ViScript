<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 soak dài — 12× soak 10/10 trên `1209af2`

> **Người chạy**: W3 (QA/hardening), 2026-11-22 06:17–08:45 JST (**2 h 28**, mức tối thiểu
> 2,5 h/12 lần; không chạy đủ 4 h/20 lần).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`1209af2`** (final QA b37aa41 đã merge; gate 3571/3) |
| Branch QA | `vs3/w3-registry`, merge `main` → **`1209af2`** |
| Target | mỗi lần riêng `target/qa-w3-4h-N`; xoá sau mỗi lần |
| Env | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **23G**; thấp nhất **22G** (> 5G); cuối **22G** |

## 1. Kết quả 12 lần

| Run | Tổng | Gate ×3 | Host | Kết quả | Log |
|---|---|---|---|---|---|
| 1 | 722 s | 3571/3 | 54/54 | 10/10 | `/tmp/opencode/qa-w3-4h-soak1.log` |
| 2 | 845 s | 3571/3 | 54/54 | 10/10 | `.../soak2.log` |
| 3 | 678 s | 3571/3 | 54/54 | 10/10 | `.../soak3.log` |
| 4 | 674 s | 3571/3 | 54/54 | 10/10 | `.../soak4.log` |
| 5 | 764 s | 3571/3 | 54/54 | 10/10 | `.../soak5.log` |
| 6 | 720 s | 3571/3 | 54/54 | 10/10 | `.../soak6.log` |
| 7 | 753 s | 3571/3 | 54/54 | 10/10 | `.../soak7.log` |
| 8 | 718 s | 3571/3 | 54/54 | 10/10 | `.../soak8.log` |
| 9 | 743 s | 3571/3 | 54/54 | 10/10 | `.../soak9.log` |
| 10 | 743 s | 3571/3 | 54/54 | 10/10 | `.../soak10.log` |
| 11 | 764 s | 3571/3 | 54/54 | 10/10 | `.../soak11.log` |
| 12 | 737 s | 3571/3 | 54/54 | 10/10 | `.../soak12.log` |
| **Tổng** | **8861 s (~2 h 28)** | — | — | **120/120 vòng OK** | — |

- Mỗi lần: gate ×3, diff ×3 (seed cố định `20261115`), host ×2, cache ×2.
- **VM thread test** `vm_khong_tao_them_thread_sau_khi_join`: **36/36 gate PASS**, không FAIL, không
  `rò thread`.
- Không T-INFRA, không flake; target riêng từng lần.
- Wrapper: `/tmp/opencode/qa-w3-4h-runN.log`.

## 2. Đánh giá

- **12 lần soak 10/10 PASS**, tổng **120/120 vòng**, gate **3571/3** mỗi lần, host **54/54**.
- Không còn flake VM thread; phù hợp xác nhận fix test-only `eb99c19`.
- Chọn mức tối thiểu **12 lần/2,5 h** (20 lần/~4 h vượt khung phiên hiện tại); kết quả không có
  lỗi nên không cần dừng sớm.

## 3. Blocker / còn lại

1. Không có blocker local.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows `1209af2` chưa đo trong phiên này.
4. Tàn dư đã biết: evaluator P3 mới, async-h2 default-off, `select` VS3-2c — release notes.
5. Chưa chạy đủ 20 lần/4 h; nếu cần bằng chứng dài hơn, chạy tiếp 8 lần còn lại.

## 4. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
for n in $(seq 1 12); do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-4h-$n
  ./bin/vis-soak --log /tmp/opencode/qa-w3-4h-soak$n.log
  rm -rf "$CARGO_TARGET_DIR"
done
```

**Kết luận**: `1209af2` **120/120 vòng soak OK** qua 12 lần (8861 s ~2,5 h), gate 3571/3, host
54/54, VM thread 36/36 gate PASS, không flake/T-INFRA. Không có blocker kỹ thuật local.
