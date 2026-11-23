<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 hardening — `f75b246` (fresh-clone + 5× soak + diff)

> **Người chạy**: W3 (QA/hardening), 2026-11-22 00:35–01:45 JST (~70 phút, time-box 2 h).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`f75b246`** (sau tag v0.3.3: `spawn_shared`, async-h2 default-off, test soak tách tệp) |
| Code | v0.3.3 + hardening; gate kỳ vọng **3538/3** |
| Branch QA | `vs3/w3-registry`, merge `main` → **`f75b246`** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **13G**; thấp nhất **12G** (> 5G); cuối **25G** (agent khác dọn) |

## 1. Fresh-clone `f75b246`

- Clone `/home/ps/vis-qa-mainf75`, checkout `CHECKOUT=f75b24687a23e365f5999a28f6ae8cb15b38eccf`.
- Target nội bộ **`target`**, `CARGO_INCREMENTAL=0`, `debug=0` (tiết kiệm đĩa).
- Log: `/tmp/opencode/qa-w3-f75-clone.log`.
- Clone 1,5G, đã xoá sau QA.

| Lệnh | Kết quả | Thời gian |
|---|---|---|
| `./vis-dev gate --fast` | **PASS — 3538 passed / 3 skipped** | 139,86 s |
| `cargo test -p vis-cli --test vs3_http2` | **19 passed / 0 failed / 0 ignored** | 19,14 s |
| `cargo test -p vis-cli --features async-h2 --test vs3_http2` | **34 passed / 0 failed / 1 ignored** | 19,96 s |
| `./bin/vis-host-suite` | **54/54** + `host_posix_real` PASS | 1,33 s |

- Các con số h2 khớp yêu cầu (19; 34+1 ignored).
- Không test đỏ, không T-INFRA.

## 2. 5× soak 10/10

Mỗi lần target riêng `target/qa-w3-f75-soakN`, debug=0/incremental=0, log mới; xoá target sau
từng lần.

| Lần | Tổng | Gate ×3 | Host | Kết quả | Log |
|---|---|---|---|---|---|
| 1 | **646 s** | 3538/3 | 54/54 | 10/10 OK | `/tmp/opencode/qa-w3-f75-soak1-soak.log` |
| 2 | **902 s** | 3538/3 | 54/54 | 10/10 OK | `.../qa-w3-f75-soak2-soak.log` |
| 3 | **744 s** | 3538/3 | 54/54 | 10/10 OK | `.../qa-w3-f75-soak3-soak.log` |
| 4 | **769 s** | 3538/3 | 54/54 | 10/10 OK | `.../qa-w3-f75-soak4-soak.log` |
| 5 | **785 s** | 3538/3 | 54/54 | 10/10 OK | `.../qa-w3-f75-soak5-soak.log` |
| **Tổng** | **3846 s (~64 phút)** | — | — | **50/50 vòng OK** | — |

- Mỗi lần: gate ×3, diff ×3 (seed cố định `20261115`), host ×2, cache ×2.
- Không T-INFRA, không lỗi sản phẩm; load có lúc 25,79 nhưng target riêng.
- Wrapper: `/tmp/opencode/qa-w3-f75-soakN.log`.

## 3. Diff 100k

`VIS_DIFF_N=100000 VIS_DIFF_SEED=202611220143 ./bin/vis-diff`

| Chỉ số | Giá trị |
|---|---|
| Tổng | **100.000** |
| Chạy | 93.765 |
| Lỗi tĩnh | 6.235 |
| **Lệch** | **0** |
| Thời gian | 122,24 s (real 134,55 s) |
| Log | `/tmp/opencode/qa-w3-f75-diff.log` |

## 4. Đánh giá

- **Fresh-clone + h2 feature + host + 5× soak + diff đều PASS** trong time-box ~70 phút.
- Gate **3538/3**, h2 default **19**, async-h2 **34+1 ignored**, host **54/54**, soak **50/50 vòng**,
  diff 100k **lệch 0**.
- Không cần tag mới (v0.3.3 đã tag); đây là hardening sau tag.

## 5. Blocker / còn lại

1. Không có blocker local.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows `f75b246` chưa đo trong phiên này (W4 đã verify v0.3.3 3469/0).
4. Tàn dư đã biết: worker evaluator, `select` VS3-2c, async HTTP/h2 default-off — release notes.

## 6. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
# Fresh-clone
git clone . /home/ps/vis-qa-mainf75 && cd /home/ps/vis-qa-mainf75
git checkout f75b246
export CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./vis-dev gate --fast
cargo test -p vis-cli --test vs3_http2
cargo test -p vis-cli --features async-h2 --test vs3_http2
./bin/vis-host-suite

# 5 soak
for n in 1 2 3 4 5; do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-f75-soak$n
  ./bin/vis-soak --log /tmp/opencode/qa-w3-f75-soak$n-soak.log
  rm -rf "$CARGO_TARGET_DIR"
done

VIS_DIFF_N=100000 VIS_DIFF_SEED=202611220143 ./bin/vis-diff
```

**Kết luận**: `f75b246` **xanh toàn bộ** — fresh-clone gate 3538/3, h2 19 + async-h2 34+1 ignored,
host 54/54, 5× soak **50/50 vòng OK** (3846 s), diff 100k **lệch 0**. Không có blocker kỹ thuật local.
