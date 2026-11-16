<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# QUYẾT ĐỊNH AGENT 0 (bổ sung) · Dòng ViScript · 2026-11-17, 03:35 JST

**Căn cứ**: `TRA-LOI-AGENT1-CHO-AGENT0-2026-11-16g.md`; báo cáo ST-147 (`c2692ae2`); `QUYET-DINH-A0-2026-11-17`.
**Tình huống**: ST-147 đạt phần **VD-150** (UTF-8 cho mọi lần gọi python: Windows **7/12 → 11/12**, 6 ca encoding xanh, macOS gate PASS 10/10, L0 1159/0). Ca 7 còn lại **không phải encoding**: `crates/vis-cli/src/cli/check.rs:82` tách tên gốc bằng `rsplit('/')` ⇒ trên Windows `phu` luôn khác rỗng ⇒ bộ lọc `--syntax` giữ chẩn đoán kiểu ⇒ khối tài liệu đỏ. Đây là **lỗi sản phẩm** (Windows), không phải hạ tầng.

## §1. Tách hàng
- **ST-147 = DONE** với phạm vi **VD-150** (đúng nghĩa "UTF-8 cho mọi lần gọi python của cổng"), kèm ghi chú: ca 7 không phải encoding.
- Phần còn lại thành hàng mới **ST-153** (sửa `check.rs` + ca khoá), cấp **VD-152**.

## §2. VD-152 (nối cuối `DECISIONS.md`)
```markdown
### VD-152 · Sửa tên gốc khi check để `--syntax` lọc đúng trên Windows (2026-11-17)
`crates/vis-cli/src/cli/check.rs` tách tên gốc của tệp đang check bằng `rsplit('/')`; trên Windows đường dẫn dùng `\` nên `phu` luôn khác rỗng và bộ lọc `--syntax` (QD §7.4) giữ chẩn đoán KIỂU, làm khối tài liệu đỏ. Cho phép sửa TỐI THIỂU: tách tên gốc theo CẢ hai dấu (`/` và `\`) trong MỘT hàm nhỏ có tên rõ nghĩa, dùng ở chỗ cũ; kèm ≥3 ca khoá (đường dẫn Windows `C:\x\a.vis` ⇒ tên gốc `a.vis`; đường dẫn Unix `d/a.vis` ⇒ `a.vis`; tên không có dấu ⇒ nguyên vẹn) chạy được trên MỌI nền. Không đổi hành vi Unix; không sửa ca nào khác; không đổi định dạng chẩn đoán; không `unsafe`/crate mới. Điều kiện đóng: trên psvn self-test **12/12** và cổng đi QUA bước 5 (dừng ở đâu ghi đúng đó); macOS gate PASS 10/10, L0 1159/0; Linux gate PASS 10/10.
```

## §3. Hàng mới `ST-153` (thêm sau `ST-152` trong `QUEUE.md`)
```markdown
| ST-153 | **check.rs: tên gốc đúng cả `\` và `/`** (VD-152, nợ mới từ ST-147) | ST-147 | 1 h | **READY** | ≥3 ca khoá chạy mọi nền; macOS gate PASS 10/10 (L0 1159/0); psvn: self-test 12/12 + cổng đi QUA bước 5 (ghi bước dừng thật); không đổi hành vi Unix; log st153-* |
```

## §4. Ghi chú đóng hàng ST-147 (dán dưới hàng ST-147)
```
ĐÓNG HÀNG 2026-11-17 Agent 0 · ST-147 DONE theo phạm vi VD-150 (UTF-8: Windows 7/12 → 11/12, 6 ca encoding xanh; macOS gate PASS 10/10, L0 1159/0). Ca 7 không phải encoding — lỗi sản phẩm check.rs, chuyển ST-153 (VD-152). BLOCKED cũ của hàng được giải toả bởi việc tách phạm vi này.
```

## §5. Thứ tự đêm nay (cập nhật)
`ST-149` → `ST-150` → `ST-153` → `ST-151` → `ST-152`. Hard stop 08:30: không mở việc mới; 09:00 dừng và chốt `R26`.
