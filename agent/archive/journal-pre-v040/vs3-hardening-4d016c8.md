<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 hardening / soak — `4d016c8` (sau tag `v0.3.0`)

> **Người chạy**: W3 (QA/hardening), 2026-11-21 17:08–17:32 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main` | **`4d016c8`** — `[VS3] release: v0.3.0 tagged; tiep tuc hardening toi 09:00 T2` |
| Tag `v0.3.0` | `0d26527` (code `6f0ea95`), đã đánh trước hardening |
| Branch QA | `vs3/w3-registry`, merge `main` → merge commit **`ef4e6c5`**; cây = `4d016c8` |
| Gate thực tế | **3501 passed / 3 skipped** (W1g async-net production thêm 5 ca so với 3496) |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **5,8G** → dọn cache **~13G** → thấp nhất **5,7G** (first soak) → cuối **28G** |

## 1. Soak

### 1.1 Lần 1 — FAIL do hạ tầng dùng chung (T-INFRA)

`bin/vis-soak --log /tmp/opencode/qa-w3-soak-4d016c8-soak.log` với target chung
`/home/ps/ViScript/target`, `CARGO_INCREMENTAL=0`:
- gate-1 OK 130 s, gate-2 OK 134 s, diff-1 OK 74 s, diff-2 OK 142 s;
- **gate-3 ĐỎ sau 84 s**, dừng theo script (không retry):
  `extern location for vis_fmt does not exist …/libvis_fmt-….rlib`,
  `failed to write …/fingerprint/test-integration-test-…`, load average **26,19**.
- Nguyên nhân: agent khác build/test cùng target chung, xoá/thay artifact giữa lúc gate.
  Không phải test sản phẩm đỏ. Log: `/tmp/opencode/qa-w3-soak-4d016c8-soak.log`.

### 1.2 Lần 2 — 10/10 OK với target riêng

`CARGO_TARGET_DIR=/home/ps/ViScript/target/qa-w3-hardening`,
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`:

| Vòng | Kết quả | Thời gian |
|---|---|---|
| gate-1 | OK | 175 s |
| gate-2 | OK | 97 s |
| diff-1 | OK | 54 s |
| diff-2 | OK | 57 s |
| gate-3 | OK | 64 s |
| diff-3 | OK | 53 s |
| host-1 | OK | 3 s |
| host-2 | OK | 1 s |
| cache-1 | OK | 10 s |
| cache-2 | OK | 11 s |
| **TỔNG** | **10/10 OK** | **525 s** |

- Mỗi gate: **3501 passed / 3 skipped**; diff dùng seed cố định `20261115`, lệch 0.
- Log: `/tmp/opencode/qa-w3-soak-4d016c8-soak-b.log`; wrapper:
  `/tmp/opencode/qa-w3-soak-4d016c8-b.log`.
- Load đầu 18,25 → cuối 2,86; target riêng tránh đúng lỗi T-INFRA lần 1.

## 2. QA lại `4d016c8`

| # | Lệnh | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | gate (trong soak ×3) | **PASS — 3501/3** mỗi lần | 175/97/64 s | soak log |
| 2 | `./vis-dev nightly --skip-gate --fuzz-minutes 2` | **PASS** — fuzz **18.552.223 ca / 0 vi phạm**; fmt 475 tệp | 120,70 s | `/tmp/opencode/qa-w3-hardening-nightly.log` |
| 3 | `VIS_DIFF_N=100000 VIS_DIFF_SEED=202611211727 ./bin/vis-diff` | **PASS — 100.000 ca, lệch 0** | 52,74 s | `/tmp/opencode/qa-w3-hardening-diff.log` |
| 4 | `./bin/vis-host-suite` (trong soak ×2) | **PASS — 54/54** + POSIX | 3 s / 1 s | soak log |
| 5 | `./bin/vis-targets` | **PASS** — core **18/18**; sqlite **16 PASS + 2 EXPECTED_RED (#156)** | 12,78 s | `/tmp/opencode/qa-w3-hardening-targets.log` |
| 6 | fresh-clone `4d016c8` | **PASS — gate 3501/3 + host 54/54** | 77,40 s + 2,78 s | `/tmp/opencode/qa-w3-hardening-freshclone.log` |

- Diff seed mới `202611211727`: 93.766 chạy, 6.234 lỗi tĩnh, **lệch 0**.
- Fresh-clone: `/home/ps/vis-qa-hardening-4d016c8`, target tên `target` trong clone,
  debug=0/incremental=0; checkout đúng `4d016c897a17797fcb5b7216421b3a4979264e22`; đã xoá clone.
- Targets: 2 ô `EXPECTED_RED` msvc/wasm đúng #156 (C toolchain), không có FAIL.

## 3. Đĩa / dọn cache

- Đầu phiên **5,8G** (sát ngưỡng 5G).
- Đã xoá `target/debug/incremental` (**7,2G**) và target cũ `target/vs3-w5` (**1,5G**) — báo cáo rõ
  theo yêu cầu; không xoá source hay target của agent đang chạy. Sau dọn **~13G**.
- Trong lúc soak chung, agent khác build làm đĩa xuống **5,7G**; sau đó họ dọn và target riêng của QA
  tăng lên; cuối phiên **28G**.
- Không có thời điểm < 5G trong các lệnh QA chính.

## 4. Đánh giá

- **Hardening local: PASS** — soak 10/10 (target riêng), gate 3501/3, nightly PASS, diff 100k lệch 0,
  host 54/54, targets 18/18, fresh-clone PASS.
- Lần soak đầu **không** tính là lỗi sản phẩm: artifact dùng chung bị thay bởi build song song
  (T-INFRA), đã tái lập sạch với target riêng.
- `v0.3.0` đã tag `0d26527` (code `6f0ea95`); `4d016c8` là commit hardening sau tag (W1g async-net).
  Nếu cần tag hardening tiếp, A0 quyết; W3 không tag.

## 5. Blocker / còn lại

1. **Windows psvn** tại `4d016c8` chưa đo trong phiên này; trước đó W4 verify `a26ec4b` (3450/0).
2. **Build song song trên target chung** gây T-INFRA; QA sau nên dùng `CARGO_TARGET_DIR` riêng
   (đã làm ở lần 2) hoặc chờ máy rảnh.
3. **2 ô EXPECTED_RED** msvc/wasm do #156 — không đổi nhãn PASS.
4. Không GitHub Actions; mọi bằng chứng local Linux.
5. Tàn dư sản phẩm đã biết (async net production mới merge, worker evaluator, `select` VS3-2c) —
   ngoài phạm vi soak; cần W4/Windows nếu muốn tuyên bố thêm.

## 6. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
# Dùng target RIÊNG để tránh T-INFRA khi có agent khác build
export CARGO_TARGET_DIR=/home/ps/ViScript/target/qa-w3-hardening
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0

./bin/vis-soak --log /tmp/opencode/qa-w3-soak-hardening.log
./vis-dev nightly --skip-gate --fuzz-minutes 2
VIS_DIFF_N=100000 VIS_DIFF_SEED=<seed-moi> ./bin/vis-diff
./bin/vis-host-suite
./bin/vis-targets
```

**Kết luận**: `4d016c8` **xanh local** sau soak 10/10; lần fail duy nhất là T-INFRA do target chung,
không phải sản phẩm. Không có blocker kỹ thuật mới cho `v0.3.0`; còn Windows `4d016c8` nếu muốn
tuyên bố hardening trên Windows.
