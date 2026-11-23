<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 soak marathon — 10× soak 10/10 trên tag `v0.3.3` / `bbf3484`

> **Người chạy**: W3 (QA/hardening), 2026-11-21 22:23 → 2026-11-22 00:34 JST.
> **Lựa chọn phạm vi**: chạy **10 lần** (mức tối thiểu theo yêu cầu; 20 lần vượt khung phiên).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| Tag tested | **`v0.3.3`** (annotated `28c2ebb`) → code **`bbf3484`** |
| `main` | **`a0ff400`** (merge W4m psvn stress; docs, code v0.3.3) |
| Branch QA | `vs3/w3-registry`, merge `main` → **`a0ff400`**; cây sản phẩm = tag |
| Target | mỗi lần riêng `target/qa-w3-marathon-N`; xoá sau mỗi lần |
| Env | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` |
| Disk | đầu **18G**; thấp nhất **13G** (> 5G); cuối **13G** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |

## 1. Kết quả 10 lần

| Lần | Tổng | Gate (×3) | Host | Kết quả | Log soak |
|---|---|---|---|---|---|
| 1 | **923 s** | 3535/3 | 54/54 | 10/10 OK | `/tmp/opencode/qa-w3-marathon-1-soak.log` |
| 2 | **773 s** | 3535/3 | 54/54 | 10/10 OK | `.../marathon-2-soak.log` |
| 3 | **689 s** | 3535/3 | 54/54 | 10/10 OK | `.../marathon-3-soak.log` |
| 4 | **684 s** | 3535/3 | 54/54 | 10/10 OK | `.../marathon-4-soak.log` |
| 5 | **681 s** | 3535/3 | 54/54 | 10/10 OK | `.../marathon-5-soak.log` |
| 6 | **694 s** | 3535/3 | 54/54 | 10/10 OK | `.../marathon-6-soak.log` |
| 7 | **873 s** | 3535/3 | 54/54 | 10/10 OK | `.../marathon-7-soak.log` |
| 8 | **908 s** | 3535/3 | 54/54 | 10/10 OK | `.../marathon-8-soak.log` |
| 9 | **829 s** | 3535/3 | 54/54 | 10/10 OK | `.../marathon-9-soak.log` |
| 10 | **686 s** | 3535/3 | 54/54 | 10/10 OK | `.../marathon-10-soak.log` |
| **Tổng** | **7740 s (~2 h 09 m)** | — | — | **100/100 vòng OK** | — |

- Mỗi lần đủ 10 vòng: gate ×3, diff ×3 (seed cố định `20261115`), host ×2, cache ×2.
- Mọi gate: **3535 passed / 3 skipped**; host **54/54** + `host_posix_real`.
- Không T-INFRA, không lỗi sản phẩm, không cần dừng giữa chừng.
- Wrapper log từng lần: `/tmp/opencode/qa-w3-marathon-N.log`.
- Load dao động 1,96 → 25,20 (agent khác build), nhưng target riêng nên không hỏng artifact.

## 2. Đánh giá

- **100/100 vòng soak PASS** trên tag `v0.3.3`, trải 10 lần chạy độc lập.
- Gate/host/diff ổn định, không flake; khớp tag Linux **3535/3**.
- Không cần tag mới; đây là hardening sau `v0.3.3`.

## 3. Blocker / còn lại

1. Không có blocker local.
2. Đã chạy **10 lần** (tối thiểu theo yêu cầu); 20 lần cần thêm ~2 giờ, không thực hiện trong phiên.
3. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
4. Không GitHub Actions; Windows do W4 thủ công (v0.3.3 đã 3469/0 theo tag).
5. Tàn dư đã biết: worker evaluator, `select` VS3-2c, async HTTP Unix — release notes.

## 4. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
for n in $(seq 1 10); do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-marathon-$n
  ./bin/vis-soak --log /tmp/opencode/qa-w3-marathon-$n-soak.log
  rm -rf "$CARGO_TARGET_DIR"
done
```

**Kết luận**: 10× soak 10/10 **PASS** trên tag `v0.3.3` (`bbf3484`) — tổng **100/100 vòng**,
**7740 s**, gate **3535/3**, host 54/54, không T-INFRA, không lỗi sản phẩm; disk min 13G.
