<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 QA — v0.3.3 candidate `bbf3484` / code `0f18592`

> **Người chạy**: W3 (QA/release), 2026-11-21 21:46–21:55 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main` | **`bbf3484`** (docs v0.3.3) |
| Code candidate | **`0f18592`** — fix h2 flow-control body >64 KiB; 8 MiB/header/trailer/GOAWAY |
| Branch QA | `vs3/w3-registry`, merge `main` → **`bbf3484`** |
| Tag | `v0.3.2` = `5c13741`; chưa có `v0.3.3` |
| Gate kỳ vọng | **3535 passed / 3 skipped** |
| Disk | đầu **19G**; thấp nhất **16G** (> 5G); cuối **17G** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |

## 1. Kết quả local

| # | Lệnh | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | `./vis-dev gate --fast` | **PASS — 3535 passed / 3 skipped** | 114,56 s (nextest 49,05 s) | `/tmp/opencode/qa-w3-v033-gate.log` |
| 2 | `./vis-dev nightly --skip-gate --fuzz-minutes 2` | **PASS** — fuzz **14.128.839 ca / 0 vi phạm**; fmt 475 tệp | 120,74 s | `/tmp/opencode/qa-w3-v033-nightly.log` |
| 3 | `VIS_DIFF_N=100000 VIS_DIFF_SEED=202611212148 ./bin/vis-diff` | **PASS — 100.000 ca, lệch 0** | 71,56 s | `/tmp/opencode/qa-w3-v033-diff.log` |
| 4 | `./bin/vis-host-suite` | **PASS — 54/54** + `host_posix_real` | 3,44 s | `/tmp/opencode/qa-w3-v033-host.log` |
| 5 | `./bin/vis-targets` | **PASS** — core **18/18**; sqlite **16 + 2 EXPECTED_RED (#156)** | 19,34 s | `/tmp/opencode/qa-w3-v033-targets.log` |
| 6 | Fresh-clone `bbf3484` | **PASS — gate 3535/3 + host 54/54** | 110,72 s + 3,46 s | `/tmp/opencode/qa-w3-freshclone-v033.log` |

- Diff prelude 2.000 ca seed cố định `20261115` lệch 0; 100k seed mới `202611212148` lệch 0
  (93.767 chạy, 6.233 lỗi tĩnh).
- Gate chạy `vs3_http2` default-on; h2 flow-control/release test pass, không hồi quy HTTP/1.1.
- Targets: 2 ô `EXPECTED_RED` msvc/wasm đúng #156 (C toolchain), không FAIL.
- Target riêng `target/qa-w3-v033`, debug=0/incremental=0.

## 2. Checklist v0.3.3

| # | Hạng mục | Bằng chứng local | Trạng thái |
|---|---|---|---|
| 3.3.1 | h2 body >64 KiB (flow-control) | gate `vs3_http2` pass; docs ghi h2 release 19/19 | **PASS local** |
| 3.3.2 | Giới hạn 8 MiB/header/trailer/GOAWAY | test h2 pass | **PASS local** |
| 3.3.3 | Không hồi quy v0.3.2 | gate/nightly/diff/host/targets xanh | **PASS local** |
| 3.3.4 | Fresh-clone | `bbf3484` gate 3535/3 + host 54/54 | **PASS** |
| 3.3.5 | Windows psvn `0f18592` | chưa có kết quả trong phiên này | **CHỜ W4** |

### Điều kiện tag v0.3.3

- [x] Gate local **3535/3**, nightly PASS, diff 100k lệch 0, host 54/54, core targets 18/18.
- [x] Fresh-clone `bbf3484` PASS.
- [ ] **Windows psvn** tại `0f18592` — blocker tag duy nhất.
- [ ] A0 tạo/push tag; W3 không tự tag.

## 3. Đề xuất

**Đề xuất A0 tag `v0.3.3` tại code `0f18592` / docs `bbf3484` sau khi W4 xác nhận Windows psvn
`0f18592` xanh.** Local Linux + fresh-clone đã xanh; không còn thay đổi code sau candidate.

## 4. Blocker / rủi ro

1. Windows psvn `0f18592` chưa verify — blocker tag.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 — không đổi nhãn PASS.
3. Không GitHub Actions; Windows do W4 thủ công.
4. Tàn dư đã biết: worker evaluator, `select` VS3-2c, async HTTP Unix — release notes.

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR=$PWD/target/qa-w3-v033
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0

./vis-dev gate --fast
./vis-dev nightly --skip-gate --fuzz-minutes 2
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611212148 ./bin/vis-diff
./bin/vis-host-suite
./bin/vis-targets
```

**Kết luận**: candidate `0f18592` / docs `bbf3484` **xanh local Linux** (gate **3535/3**, nightly
PASS, diff 100k lệch 0, host 54/54, targets 18/18, fresh-clone PASS). Đề xuất tag `v0.3.3` sau khi
W4 xác nhận Windows.
