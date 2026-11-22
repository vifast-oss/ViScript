<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Xác minh tích hợp cuối Linux — main `84e8eb6` (v0.4.1)

**Nhánh**: `plan/w1-final-soak` từ `main` **`84e8eb6`**. Chỉ journal; không sửa code, không
merge/push/tag. Target riêng `/home/ps/ViScript/target/vs3-w1-final`;
`CARGO_BUILD_JOBS=3`, `VIS_SOAK_TMPDIR=/home/ps/tmp`.

## 1. `bin/vis-soak` 10/10 — PASS

- HEAD `84e8eb6`, bắt đầu `2026-11-22T19:39:33Z`, **759 s**, load trước 2,67 → sau 3,08.
- Log ngoài repo: `/home/ps/vislogs/final-linux-soak-84e8eb6.log`
  sha256 `d60ffdf9…fa8c92` (R7/VD-282).
- Kết quả từng vòng:
  - **gate-1/2/3** (`./vis-dev gate --fast`): PASS — **3660 passed / 4 skipped** mỗi lần.
  - **diff-1/2/3** (`VIS_DIFF_N=100000`, seed `20261115`): `tong=100000 … **lech=0**`
    (94 / 76 / 75 s).
  - **host-1/2** (`bin/vis-host-suite`): **54/54** ca xanh, `host_posix_real PASS`.
  - **cache-1/2** (`cargo test -p vis-cli --test cache_khong_che`): **5/5**.
- Không flake, không retry.

## 2. `bin/vis-wasm-suite` — 28/28

- Chạy **sau** soak (không đồng thời), exit 0, runtime **node**:
  `# tổng: 28 ca, 0 hỏng, runtime=node`.
- Log ngoài repo: `/home/ps/vislogs/final-wasm-suite-84e8eb6.log`
  sha256 `f853d0b9…57403e4`.

## 3. Phiên bản

- `./target/debug/vis --version` ⇒ **`vis 0.4.1`** (exit 0).
- Binary gate private `/home/ps/ViScript/target/vs3-w1-final/debug/vis --version` cũng `0.4.1`.

## 4. Kết luận

**PASS toàn bộ, không blocker.** Linux native tại `84e8eb6`: gate 3660/4 ×3, diff 100k lệch 0 ×3,
host 54/54 ×2, cache 5/5 ×2, wasm 28/28. Đủ điều kiện cho mốc `v0.4.1`.
