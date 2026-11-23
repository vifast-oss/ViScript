<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 hardening dài — 3× soak 10/10 trên `d1f8f70`

> **Người chạy**: W3 (QA/hardening), 2026-11-21 19:43–20:14 JST (~31 phút).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main` | **`d1f8f70`** (v0.3.2 ổn định; merge fast-forward) |
| Code | `5c13741` / v0.3.2; không đổi trong 3 lần soak |
| Target | mỗi lần riêng: `.../target/qa-w3-loop1`, `loop2`, `loop3` |
| Env | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` |
| Disk | đầu **19G**; các lần kết thúc **18G**; target xoá sau mỗi lần; không < 5G |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |

## 1. Kết quả 3 lần

| Lần | Start (JST) | Tổng | Gate | Host | Kết quả | Log |
|---|---|---|---|---|---|---|
| 1 | 19:43:30 | **776 s** | 3528/3 ×3 | 54/54 ×2 | **10/10 OK** | `/tmp/opencode/qa-w3-loop1-soak.log` |
| 2 | 19:56:31 | **535 s** | 3528/3 ×3 | 54/54 ×2 | **10/10 OK** | `/tmp/opencode/qa-w3-loop2-soak.log` |
| 3 | 20:05:31 | **526 s** | 3528/3 ×3 | 54/54 ×2 | **10/10 OK** | `/tmp/opencode/qa-w3-loop3-soak.log` |
| **Tổng** | — | **1837 s (~30,6 phút)** | — | — | **30/30 vòng OK** | — |

- Mỗi lần đủ 10 vòng: gate ×3, diff ×3 (seed cố định `20261115`), host ×2, cache ×2.
- Không lần nào có T-INFRA; target riêng tránh xung đột artifact.
- Wrapper log: `/tmp/opencode/qa-w3-loop{1,2,3}.log`.
- Load trước/sau:
  - loop1: 4,04 → 5,19 (sau)
  - loop2: 4,85 → 3,10
  - loop3: 2,93 → 3,36

## 2. Chi tiết từng lần

### Loop 1 — 776 s

- Gate-1/2/3: **3528 passed / 3 skipped**; thời gian nextest lần lượt 72,37 / 65,63 / (lần 3 trong log).
- Diff-1/2/3: OK (123/117/99 s); host 3/1 s; cache 10/9 s.
- Load đầu 4,04; máy có agent khác build (không dùng target này).

### Loop 2 — 535 s

- Gate: **3528/3** ×3 (55,30 / 47,56 / …).
- Diff: 72/69/72 s; host 3/1 s; cache 10/9 s.
- Load đầu 4,85 → cuối 3,10.

### Loop 3 — 526 s

- Gate: **3528/3** ×3 (51,85 / 44,46 / …).
- Diff: 71/69/69 s; host 3/1 s; cache 10/9 s.
- Load đầu 2,93 → cuối 3,36.

## 3. Đánh giá

- **30/30 vòng soak PASS**, không lỗi sản phẩm, không flake, không T-INFRA.
- Ổn định qua 3 lần chạy độc lập (~31 phút), khớp tag v0.3.2: Linux **3528/3**.
- Không cần tag mới; đây là hardening sau `v0.3.2`.

## 4. Blocker / còn lại

1. Không có blocker local.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows v0.3.2 đã verify 3462/0 theo tag, không đo lại trong phiên này.
4. Tàn dư đã biết: worker evaluator, `select` VS3-2c, async HTTP/1.1 chỉ Unix — release notes.

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
for n in 1 2 3; do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-loop$n
  export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
  ./bin/vis-soak --log /tmp/opencode/qa-w3-loop$n-soak.log
  rm -rf "$CARGO_TARGET_DIR"
done
```

**Kết luận**: 3× soak 10/10 **PASS** trên `d1f8f70` — tổng **30/30 vòng**, **1837 s**, gate
**3528/3**, host 54/54, không T-INFRA, không lỗi sản phẩm.
