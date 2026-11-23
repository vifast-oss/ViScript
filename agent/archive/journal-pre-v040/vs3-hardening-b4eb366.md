<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 hardening — `b4eb366` (fresh-clone + 3× soak + diff)

> **Người chạy**: W3 (QA/hardening), 2026-11-22 01:46–02:38 JST (~52 phút, time-box 2 h).
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/HEAD | **`b4eb366`** (Windows async net `f9a4c48` + evaluator P1 `54021de`) |
| Gate kỳ vọng | **3547/3** |
| Branch QA | `vs3/w3-registry`, merge `main` → **`b4eb366`** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **25G**; thấp nhất **15G** (> 5G); cuối **15G** |

## 1. Fresh-clone `b4eb366`

- Clone `/home/ps/vis-qa-mainb4e`, checkout `CHECKOUT=b4eb366f971069aa3d88d5ba516c4782b159b917`.
- Target nội bộ tên **`target`**, `CARGO_INCREMENTAL=0`, `debug=0` (tiết kiệm đĩa).
- Log: `/tmp/opencode/qa-w3-b4e-clone.log`; clone 1,5G đã xoá.

| Lệnh | Kết quả | Thời gian |
|---|---|---|
| `./vis-dev gate --fast` | **PASS — 3547 passed / 3 skipped** | 132,02 s |
| `cargo test -p vis-cli --test vs3_http2` | **19 passed** | 19,35 s |
| `cargo test -p vis-cli --features async-h2 --test vs3_http2` | **34 passed / 1 ignored** | 19,77 s |
| `cargo test -p vis-cli --test vs3_default` | **22 passed** | 2,49 s |
| `cargo test -p vis-cli --test vs3_tls` | **21 passed** | 6,64 s |
| `./bin/vis-host-suite` | **54/54** + `host_posix_real` | 1,41 s |

- Tất cả số ca khớp yêu cầu; không test đỏ, không T-INFRA.

## 2. 3× soak 10/10

Mỗi lần target riêng `target/qa-w3-b4e-soakN`, debug=0/incremental=0, log mới; xoá target sau
từng lần.

| Lần | Tổng | Gate ×3 | Host | Kết quả | Log |
|---|---|---|---|---|---|
| 1 | **730 s** | 3547/3 | 54/54 | 10/10 OK | `/tmp/opencode/qa-w3-b4e-soak1-soak.log` |
| 2 | **1026 s** | 3547/3 | 54/54 | 10/10 OK | `.../qa-w3-b4e-soak2-soak.log` |
| 3 | **938 s** | 3547/3 | 54/54 | 10/10 OK | `.../qa-w3-b4e-soak3-soak.log` |
| **Tổng** | **2694 s (~44,9 phút)** | — | — | **30/30 vòng OK** | — |

- Mỗi lần: gate ×3, diff ×3 (seed cố định `20261115`), host ×2, cache ×2.
- Không T-INFRA, không lỗi sản phẩm; load có lúc 25,97 nhưng target riêng.
- Wrapper: `/tmp/opencode/qa-w3-b4e-soakN.log`.

## 3. Diff 100k

`VIS_DIFF_N=100000 VIS_DIFF_SEED=202611220235 ./bin/vis-diff`

| Chỉ số | Giá trị |
|---|---|
| Tổng | **100.000** |
| Chạy | 93.766 |
| Lỗi tĩnh | 6.234 |
| **Lệch** | **0** |
| Thời gian | 119,06 s (real 127,37 s) |
| Log | `/tmp/opencode/qa-w3-b4e-diff.log` |

## 4. Đánh giá

- **Fresh-clone + feature tests + host + 3× soak + diff đều PASS** trong ~52 phút.
- Gate **3547/3**; h2 default **19**, async-h2 **34+1 ignored**, `vs3_default` **22**, `vs3_tls` **21**,
  host **54/54**, soak **30/30 vòng**, diff **lệch 0**.
- Không cần tag mới (v0.3.3 đã tag); đây là hardening sau tag.

## 5. Blocker / còn lại

1. Không có blocker local.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows `b4eb366` chưa đo trong phiên này (W4 đã verify v0.3.3 3469/0).
4. async-h2 vẫn default-off; evaluator P1 mới ở mức prototype/theo dõi — release notes.
5. Tàn dư đã biết: worker evaluator, `select` VS3-2c, async HTTP — release notes.

## 6. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone . /home/ps/vis-qa-mainb4e && cd /home/ps/vis-qa-mainb4e
git checkout b4eb366
export CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./vis-dev gate --fast
cargo test -p vis-cli --test vs3_http2
cargo test -p vis-cli --features async-h2 --test vs3_http2
cargo test -p vis-cli --test vs3_default
cargo test -p vis-cli --test vs3_tls
./bin/vis-host-suite

for n in 1 2 3; do
  export CARGO_TARGET_DIR=$PWD/target/qa-w3-b4e-soak$n
  ./bin/vis-soak --log /tmp/opencode/qa-w3-b4e-soak$n-soak.log
  rm -rf "$CARGO_TARGET_DIR"
done

VIS_DIFF_N=100000 VIS_DIFF_SEED=202611220235 ./bin/vis-diff
```

**Kết luận**: `b4eb366` **xanh toàn bộ** — fresh-clone gate 3547/3, h2 19 + async-h2 34+1 ignored,
`vs3_default` 22, `vs3_tls` 21, host 54/54, 3× soak **30/30 vòng OK** (2694 s), diff 100k **lệch 0**.
Không có blocker kỹ thuật local.
