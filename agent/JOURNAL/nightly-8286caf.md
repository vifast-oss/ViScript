<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Nightly Linux — main `8286caf` (v0.4.1 + ST-297/299b/300 + docs)

**Nhánh**: `plan/w1-nightly` từ `main` **`8286caf`**; không sửa code, không merge/push/tag.
`CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`; không psvn/soak lớn (W4 đang soak Windows).

## 1. Kết quả (lần 2 — hợp lệ)

`./vis-dev nightly` ⇒ **PASS**, tổng **248 s**:

| Phần | Kết quả | Thời gian |
|---|---|---:|
| Gate `--fast` (fmt/clippy/R1..R7/toolchain/release/báo cáo/build/doc/readme/L0) | PASS — **3660 passed / 4 skipped** | ~118 s (L0 riêng 67,6 s) |
| Fuzz parser (2 phút) | **13.559.076 ca / 307.793.304 token / 0 vi phạm — PASS** | 120,0 s |
| Vòng tròn fmt | **475 tệp bất động, 0 lệch** (bỏ qua 78 ca lỗi cú pháp) | ~10 s |

Chi tiết L0: `Summary [67.636s] 3660 tests run: 3660 passed, 4 skipped`.
Fuzz: `112.992 ca/s`, `2.564.944 token/s`.

- Log ngoài repo: `/home/ps/vislogs/nightly-8286caf-lan2.log`
  sha256 `0e478fab…d7133211c`.

## 2. Lần 1 — T-INFRA (không phải sản phẩm, không retry mù)

Lần đầu đặt `CARGO_TARGET_DIR` riêng; `gate --fast` PASS **3660/4** nhưng bước fuzz đỏ vì
`bin/vis-dev` gọi cứng `target/release/vis-parse-fuzz` (không thấy binary trong target riêng):
`No such file or directory`. Đây là **lỗi môi trường/đường dẫn**, fuzz không chạy ca nào.
Chạy lại với target mặc định của worktree ⇒ PASS đầy đủ ở trên.
Log lần 1: `/home/ps/vislogs/nightly-8286caf-lan1-infra.log`
sha256 `77879fbd…6ce6fe7`.

## 3. Kết luận

Nightly Linux tại `8286caf`: **gate 3660/4, fuzz 0 vi phạm, fmt 0 lệch — PASS**. Không có
artifact lỗi fuzz; không sửa code. **Blocker**: không.
