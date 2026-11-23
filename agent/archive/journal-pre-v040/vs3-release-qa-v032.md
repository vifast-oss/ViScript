<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 QA — v0.3.2 candidate `0ff2cac` / code `5c13741`

> **Người chạy**: W3 (QA/release), 2026-11-21 18:34–18:42 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main` | **`0ff2cac`** (docs v0.3.2) |
| Code candidate | **`5c13741`** — async HTTP/1.1 cooperative (Unix), h2 stress, worker 50k, TaskId proto |
| Branch QA | `vs3/w3-registry`, fast-forward `0ff2cac`; cây sản phẩm = `5c13741` |
| Tag | `v0.3.0` = `0d26527`; `v0.3.1` = `e689c3d` |
| Gate kỳ vọng | **3528 passed / 3 skipped** |
| Disk | đầu **12G**; thấp nhất **8,5G** (> 5G); cuối **9,9G** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |

## 1. Kết quả local

| # | Lệnh | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | `./vis-dev gate --fast` | **PASS — 3528 passed / 3 skipped** | 92,80 s (nextest 47,87 s) | `/tmp/opencode/qa-w3-v032-gate.log` |
| 2 | `./vis-dev nightly --skip-gate --fuzz-minutes 2` | **PASS** — fuzz **18.655.570 ca / 0 vi phạm**; fmt 475 tệp | 120,72 s | `/tmp/opencode/qa-w3-v032-nightly.log` |
| 3 | `VIS_DIFF_N=100000 VIS_DIFF_SEED=202611211836 ./bin/vis-diff` | **PASS — 100.000 ca, lệch 0** | 53,95 s | `/tmp/opencode/qa-w3-v032-diff.log` |
| 4 | `./bin/vis-host-suite` | **PASS — 54/54** + `host_posix_real` | 2,78 s | `/tmp/opencode/qa-w3-v032-host.log` |
| 5 | `./bin/vis-targets` | **PASS** — core **18/18**; sqlite **16 + 2 EXPECTED_RED (#156)** | 14,44 s | `/tmp/opencode/qa-w3-v032-targets.log` |
| 6 | Fresh-clone `0ff2cac` | **PASS — gate 3528/3 + host 54/54** | 95,77 s + 2,71 s | `/tmp/opencode/qa-w3-freshclone-v032.log` |

- Diff prelude 2.000 ca seed cố định `20261115` lệch 0; 100k seed mới `202611211836` lệch 0
  (93.767 chạy, 6.233 lỗi tĩnh).
- Targets: 2 ô `EXPECTED_RED` msvc/wasm đúng #156 (C toolchain), không FAIL.
- Target riêng `target/qa-w3-v032`, debug=0/incremental=0.

## 2. Checklist v0.3.2

| # | Hạng mục | Bằng chứng local | Trạng thái |
|---|---|---|---|
| 3.2.1 | Async HTTP/1.1 cooperative (Unix) | gate 3528/3; test async HTTP pass | **PASS local** |
| 3.2.2 | h2 stress | `vs3_http2`/h2 stress pass | **PASS local** |
| 3.2.3 | Worker 50k + TaskId proto | `vs3_worker` pass | **PASS local** |
| 3.2.4 | Không hồi quy v0.3.1 | gate/nightly/diff/host/targets xanh | **PASS local** |
| 3.2.5 | Fresh-clone | `0ff2cac` gate 3528/3 + host 54/54 | **PASS** |
| 3.2.6 | Windows psvn | `5c13741` chưa có kết quả trong phiên này | **CHỜ W4** |

### Điều kiện tag v0.3.2

- [x] Gate local **3528/3**, nightly PASS, diff 100k lệch 0, host 54/54, core targets 18/18.
- [x] Fresh-clone `0ff2cac` PASS.
- [ ] **Windows psvn** tại `5c13741` — blocker tag duy nhất.
- [ ] A0 tạo/push tag; W3 không tự tag.

## 3. Đề xuất

**Đề xuất A0 tag `v0.3.2` tại code `5c13741` (docs `0ff2cac`) sau khi W4 xác nhận psvn xanh.**
Local Linux + fresh-clone đã xanh; không còn thay đổi code sau candidate.

## 4. Blocker / rủi ro

1. Windows psvn `5c13741` chưa verify — blocker tag.
2. Async HTTP/1.1 chỉ Unix; Windows dùng gating, không tuyên bố backend đầy đủ.
3. 2 ô `EXPECTED_RED` msvc/wasm do #156 — không đổi nhãn PASS.
4. Không GitHub Actions; Windows do W4 thủ công.
5. Tàn dư đã biết: worker evaluator, `select` VS3-2c — release notes.

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR=$PWD/target/qa-w3-v032
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0

./vis-dev gate --fast
./vis-dev nightly --skip-gate --fuzz-minutes 2
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611211836 ./bin/vis-diff
./bin/vis-host-suite
./bin/vis-targets
```

**Kết luận**: candidate `5c13741` / docs `0ff2cac` **xanh local Linux** (gate **3528/3**, nightly
PASS, diff 100k lệch 0, host 54/54, targets 18/18, fresh-clone PASS). Đề xuất tag `v0.3.2` sau khi
W4 xác nhận Windows.
