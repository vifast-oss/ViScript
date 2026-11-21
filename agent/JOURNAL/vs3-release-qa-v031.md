<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 QA — v0.3.1 candidate `e689c3d` / code `a96841a`

> **Người chạy**: W3 (QA/release), 2026-11-21 17:41–17:47 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main` | **`e689c3d`** (docs v0.3.1) |
| Code candidate | **`a96841a`** — async net production + async TLS (Unix), h2 hardening, worker stress, test Windows gating |
| Branch QA | `vs3/w3-registry`, merge `main` → merge commit **`d03bee9`**; cây = `e689c3d` |
| Tag | `v0.3.0` = `0d26527`; chưa có `v0.3.1` |
| Gate kỳ vọng | **3512 passed / 3 skipped** |
| Disk | đầu/cuối **29G**; thấp nhất **28G** (> 5G) |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |

## 1. Kết quả local

| # | Lệnh | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | `./vis-dev gate --fast` | **PASS — 3512 passed / 3 skipped** | 59,79 s (nextest 31,09 s) | `/tmp/opencode/qa-w3-v031-gate.log` |
| 2 | `./vis-dev nightly --skip-gate --fuzz-minutes 2` | **PASS** — fuzz **18.998.388 ca / 0 vi phạm**; fmt 475 tệp | 120,73 s | `/tmp/opencode/qa-w3-v031-nightly.log` |
| 3 | `VIS_DIFF_N=100000 VIS_DIFF_SEED=202611211742 ./bin/vis-diff` | **PASS — 100.000 ca, lệch 0** | 52,76 s | `/tmp/opencode/qa-w3-v031-diff.log` |
| 4 | `./bin/vis-host-suite` | **PASS — 54/54** + `host_posix_real` | 2,75 s | `/tmp/opencode/qa-w3-v031-host.log` |
| 5 | `./bin/vis-targets` | **PASS** — core **18/18**; sqlite **16 + 2 EXPECTED_RED (#156)** | 6,79 s | `/tmp/opencode/qa-w3-v031-targets.log` |
| 6 | Fresh-clone `e689c3d` | **PASS — gate 3512/3 + host 54/54** | 75,82 s + 2,83 s | `/tmp/opencode/qa-w3-freshclone-v031.log` |

- Diff prelude 2.000 ca seed cố định `20261115` lệch 0; 100k seed mới `202611211742` lệch 0
  (93.765 chạy, 6.235 lỗi tĩnh).
- Gate chạy các suite mới: async net/TLS, h2 hardening, worker stress; không test đỏ.
- Targets: 2 ô `EXPECTED_RED` msvc/wasm đúng #156 (C toolchain), không có FAIL.
- Fresh-clone: `/home/ps/vis-qa-v031`, target tên `target` trong clone, debug=0/incremental=0,
  checkout `e689c3d8dfbfe64a4f69d4f145f51d21c77ae242`; đã xoá clone.

## 2. Checklist v0.3.1

### 2.1 Nội dung candidate

| # | Hạng mục | Bằng chứng local | Trạng thái |
|---|---|---|---|
| 3.1.1 | Async net production (Unix) | gate 3512/3; `vs3_default` async-net pass | **PASS local** |
| 3.1.2 | Async TLS (Unix) | `vs3_tls` mở rộng pass | **PASS local** |
| 3.1.3 | HTTP/2 hardening | `vs3_http2` pass (default-on) | **PASS local** |
| 3.1.4 | Worker stress | `vs3_worker` stress pass | **PASS local** |
| 3.1.5 | Windows gating cho async net | `a96841a` thêm test gate Windows (chưa chạy psvn trong phiên này) | **Chờ W4** |
| 3.1.6 | Không hồi quy v0.3.0 | gate/nightly/diff/host/targets/fresh-clone xanh | **PASS local** |

### 2.2 Điều kiện tag v0.3.1

- [x] Gate local **3512/3**, nightly PASS, diff 100k lệch 0, host 54/54, core targets 18/18.
- [x] Fresh-clone `e689c3d` PASS.
- [ ] **Windows psvn** tại `a96841a`/`e689c3d` (W4) — cần xác nhận async net/TLS gating, 0 product failure.
- [ ] A0 tạo/push tag; W3 không tự tag.

## 3. Đề xuất

**Đề xuất A0 tag `v0.3.1` tại code `a96841a` (docs `e689c3d`) sau khi W4 xác nhận psvn xanh.**
Local Linux + fresh-clone đã xanh; không còn thay đổi code sau candidate. Nếu W4 chưa kịp, chưa tag và
ghi rõ Windows chưa verify.

## 4. Blocker / rủi ro

1. **Windows `a96841a` chưa verify trong phiên này** — blocker tag v0.3.1.
2. Async net/TLS hiện **Unix**; Windows dùng test gating (không tuyên bố backend đầy đủ).
3. 2 ô `EXPECTED_RED` msvc/wasm do #156 — không đổi nhãn PASS.
4. Không GitHub Actions; Windows do W4 psvn thủ công.
5. Tàn dư đã biết: worker evaluator chưa nối, `select` VS3-2c; ghi release notes, không chặn v0.3.1.

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR=/home/ps/ViScript/target/qa-w3-hardening
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0

./vis-dev gate --fast
./vis-dev nightly --skip-gate --fuzz-minutes 2
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611211742 ./bin/vis-diff
./bin/vis-host-suite
./bin/vis-targets
```

**Kết luận**: candidate `a96841a` / docs `e689c3d` **xanh local Linux** (gate **3512/3**, nightly
PASS, diff 100k lệch 0, host 54/54, targets 18/18, fresh-clone PASS). Đề xuất tag `v0.3.1` sau khi
W4 xác nhận Windows.
