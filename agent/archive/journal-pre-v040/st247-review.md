<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-247 — review Agent 1 (làm rõ miền global, VD-244 §3)

**Nguồn**: worktree `vd244-st247-global-domain`, commit Agent 2 `1309610`. **Người review**: Agent 1. **Ngày**: 2026-11-20.

## 1. Năm câu trả lời — verdict

| Câu | Kết quả | Bằng chứng |
|---|---|---|
| 1. Ghi chỉ số rất cao tự nới miền? | **Không** — `nglobals` là số ô có lệnh ghi, không phải `max+1`; ghi `BASE+1000` bị từ chối | `q1_*` |
| 2. Chặn vượt `u16`? | **Có** — operand `u16`, `LoadLocal(u16::MAX)` bị chặn kể cả `encode→decode→load_verified` | `q2_*` |
| 3. Slot thưa/chưa khởi tạo? | Module có lỗ hổng bị từ chối (ghi offset cao nhất tràn `nglobals`); đọc thẳng vào lỗ cũng bị chặn | `q3_*` |
| 4. Ghi trong nhánh không chạy / hàm chưa gọi? | Verifier **chấp nhận** (đếm tĩnh toàn module), runtime trả `Unit` **thành công**, không chẩn đoán — đã so `show == "()"`, không chỉ `catch_unwind` | `q4_*` |
| 5. Miền verify khớp runtime? | **Khớp có điều kiện** — đúng với codegen thật, **không khớp ca (4)** | `q5_*` |

**SHIP tư cách điều tra.** Không sửa `verify.rs`; 2 tệp test mới (`st247_global_domain.rs` bytecode + VM). `cargo test -p vis-bytecode -p vis-vm` 124/0, `vis test` 525/525 ×2.

## 2. Block phát hiện — báo A0, đã được A0 chốt hướng

Ca (4) là soundness gap: **hợp lệ chỉ số nhưng chưa khởi tạo ⇒ `Unit` thành công**. Không phải crafted-bytecode-only được phép giữ (VD-244 §3); A0 đã duyệt **ST-250: runtime kiểm tra khởi tạo và trap** (VD-245). §14 được cập nhật, nợ **#188** mở tới khi ST-250 đóng.

## 3. Kết luận

Merge `1309610`, ST-247 → **DONE (điều tra)**. Không đóng §14; mở ST-250. Không tag/release.
