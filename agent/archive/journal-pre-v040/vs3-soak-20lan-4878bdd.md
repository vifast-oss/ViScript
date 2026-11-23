<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 soak đủ 20 lần/200 vòng — `4878bdd`

> **Người chạy**: W3 (QA/hardening), 2026-11-22 06:17–10:27 JST (**~4 h 10**).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`4878bdd`** (12× soak đã merge; gate 3571/3) |
| Code | v0.3.3 + hardening; không đổi trong 20 lần |
| Branch QA | `vs3/w3-registry`, merge `main` → **`4878bdd`** |
| Target | mỗi lần riêng `target/qa-w3-4h-N`; xoá sau mỗi lần |
| Env | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **23G**; thấp nhất **22G** (> 5G); cuối **22G** |

## 1. 12 lần trước (đã ghi trong `vs3-soak-4h-1209af2.md`)

| Run | Tổng | Gate ×3 | Host | Kết quả |
|---|---|---|---|---|
| 1 | 722 s | 3571/3 | 54/54 | 10/10 |
| 2 | 845 s | 3571/3 | 54/54 | 10/10 |
| 3 | 678 s | 3571/3 | 54/54 | 10/10 |
| 4 | 674 s | 3571/3 | 54/54 | 10/10 |
| 5 | 764 s | 3571/3 | 54/54 | 10/10 |
| 6 | 720 s | 3571/3 | 54/54 | 10/10 |
| 7 | 753 s | 3571/3 | 54/54 | 10/10 |
| 8 | 718 s | 3571/3 | 54/54 | 10/10 |
| 9 | 743 s | 3571/3 | 54/54 | 10/10 |
| 10 | 743 s | 3571/3 | 54/54 | 10/10 |
| 11 | 764 s | 3571/3 | 54/54 | 10/10 |
| 12 | 737 s | 3571/3 | 54/54 | 10/10 |
| **Tổng** | **8861 s** | — | — | **120/120 vòng** |

## 2. 8 lần còn lại trên `4878bdd`

| Run | Tổng | Gate ×3 | Host | Kết quả | Log |
|---|---|---|---|---|---|
| 13 | 757 s | 3571/3 | 54/54 | 10/10 | `/tmp/opencode/qa-w3-4h-soak13.log` |
| 14 | 741 s | 3571/3 | 54/54 | 10/10 | `.../soak14.log` |
| 15 | 774 s | 3571/3 | 54/54 | 10/10 | `.../soak15.log` |
| 16 | 756 s | 3571/3 | 54/54 | 10/10 | `.../soak16.log` |
| 17 | 746 s | 3571/3 | 54/54 | 10/10 | `.../soak17.log` |
| 18 | 751 s | 3571/3 | 54/54 | 10/10 | `.../soak18.log` |
| 19 | 756 s | 3571/3 | 54/54 | 10/10 | `.../soak19.log` |
| 20 | 770 s | 3571/3 | 54/54 | 10/10 | `.../soak20.log` |
| **Tổng** | **6051 s** | — | — | **80/80 vòng** | — |

- Mỗi lần: gate ×3, diff ×3 (seed cố định `20261115`), host ×2, cache ×2.
- **VM thread test** `vm_khong_tao_them_thread_sau_khi_join`: 24/24 gate ở 8 lần mới PASS; tổng
  **60/60 gate** của 20 lần PASS, không `rò thread`.
- Không T-INFRA, không flake; target riêng.

## 3. Tổng 20 lần

| Chỉ số | Giá trị |
|---|---|
| Số lần | **20** |
| Số vòng | **200/200 OK** |
| Tổng thời gian soak | **14.912 s (~4 h 08)** |
| Thời gian wall | **06:17–10:27 JST (~4 h 10)** |
| Gate | 3571/3 mỗi gate (60 gate) |
| Host | 54/54 mỗi lần |
| Flake/T-INFRA | **0** |
| Disk | 23G → 22G; không <5G |

> 12 lần đầu chạy trên `1209af2`; 8 lần sau trên `4878bdd` (docs merge 12× soak, code không đổi).

## 4. Đánh giá

- **20 lần soak 10/10 PASS**, **200/200 vòng**, 60/60 gate **3571/3**, host 54/54.
- VM thread test xanh **60/60 gate**; flake đã đóng.
- Không cần tag mới; đây là hardening sau v0.3.3.

## 5. Blocker / còn lại

1. Không có blocker local.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows `4878bdd` chưa đo trong phiên này.
4. Tàn dư đã biết: evaluator P3 mới, async-h2 default-off, `select` VS3-2c — release notes.

## 6. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
for n in $(seq 1 20); do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-4h-$n
  ./bin/vis-soak --log /tmp/opencode/qa-w3-4h-soak$n.log
  rm -rf "$CARGO_TARGET_DIR"
done
```

**Kết luận**: đủ **20 lần soak 10/10**, tổng **200/200 vòng**, **14.912 s (~4,1 h)**, gate 3571/3,
host 54/54, VM 60/60 gate PASS, không flake/T-INFRA. Không có blocker kỹ thuật local.
