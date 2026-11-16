<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# QUYẾT ĐỊNH AGENT 0 · Dòng ViScript · 2026-11-17, 07:45 JST (đợt 28)

**Căn cứ**: `BAO-CAO-DEM-AGENT1-CHO-AGENT0-2026-11-17.md`; `R27`; `SO-NO.md` (146 mục); yêu cầu người điều phối: Agent 2 chạy tiếp tới **10:00 JST** (Agent 0 thật online 10:00).
**Nguyên tắc**: chỉ làm **nợ đã có số trong `SO-NO.md`**, **không** cần trả lời 5 câu hỏi VS3, **không** mở tính năng VS3 (VD-148), **không** đụng Windows (VS3 mục 5). Mỗi hàng ≤ 1 h; **hard stop 09:45** không mở việc mới.

## §1. VD-156 (nối cuối `DECISIONS.md`)
```markdown
### VD-156 · Đợt 28: parity argv (#110) + một nguồn đếm khối (#129) (2026-11-17)
Cho phép sửa TỐI THIỂU hai nợ: (a) #110 — VM phải đưa `argv` (`list[str]`) vào **tham số không-`cap`** của `main` **như interp** (`interp.rs:221-234`), có ca đỏ-trước chạy cả hai đường và so byte; (b) #129 — gom danh sách **số dòng khối tài liệu** đang chép ở 4 tệp test về **một nguồn** duy nhất (một hằng/module dùng chung), không đổi số kỳ vọng. Không đổi ngữ nghĩa khác, không nới ngưỡng, không `unsafe`/crate mới; gate macOS PASS (L0 1175/0 hoặc số thật nếu thêm ca) là điều kiện commit. Không mở task VS3.
```

## §2. Hàng mới (thêm sau `ST-156`)
```markdown
| ST-157 | **Nợ #110**: VM đưa `argv` vào tham số không-`cap` của `main` (parity với interp) | ST-156 | 1 h | **READY** | ca đỏ-trước: chương trình đọc tham số `main` in `[a b]` — interp và `--vm` **cùng byte**; sửa ≤100 dòng; gate macOS PASS; log st157-* |
| ST-158 | **Nợ #129**: gom danh sách số dòng khối tài liệu về **một nguồn** (4 tệp test) | ST-156 | 0,75 h | **READY** | một hằng/nguồn duy nhất; 4 tệp tham chiếu (không chép số); số kỳ vọng không đổi; gate macOS PASS; log st158-* |
| ST-159 | **R28** (chốt phần sau R27) | ST-157, ST-158 | 0,75 h | **READY** | 28 báo cáo, **177** hàng; check_report PASS; gate macOS PASS; sổ đếm khớp; log st159-* |
```

## §3. Thứ tự & dừng
`ST-157` → `ST-158` → `ST-159`. **09:00**: nếu ST-158 chưa xong ⇒ dừng nó (BLOCKED + log), chuyển `ST-159`. **09:45**: không mở việc mới; hoàn tất trong 15 phút hoặc revert + báo. Nếu `ST-159` không kịp ⇒ để `READY`, báo rõ (không BLOCKED giả). Không retry cổng để lấy xanh; flake ⇒ `T-INFRA` + log, chạy lại **một** lần.
