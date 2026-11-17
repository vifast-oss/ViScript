<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Đếm số lần gate đỏ do #146 (phục vụ R29, VD-175 §3)
**Quy ước**: mỗi lần gate `--fast` trên mac đỏ vì `dua_doi_symlink_lap_co_gioi_han` (`assert_eq!(ro_ri,0)`) được chạy lại **một lần** để phân loại (D2-07); ghi log riêng và đếm ở đây.

| Lần | HEAD | Log | Kết quả lần 1 | Lần 2 (phân loại) |
|---|---|---|---|---|
| 1 | `0df923c2` | `st161-gate-macos-lan2-t146.log` | đỏ `ro_ri=1` (`lap=50000 ok_trong=11878 ro_ri=1 chan=38121 vong_doi=16383`) | PASS 1182/0 |
| 2 | `f2d2e72d` | `st166-gate-macos.log` | đỏ `ro_ri=1/50 000` (65 passed; 1 failed) | PASS 1182/0 (`st166-gate-macos-lan2.log`) |

**Tổng trong đợt 29: 2 lần** (tính đến ST-167). Cả hai đều đã phân loại (b) — #146, BLOCKED-CHO-A0; **không** phải lỗi mã mới.
