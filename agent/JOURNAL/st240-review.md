<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-240 — review Agent 1 (std/CLI hardening Đợt B)

**Nguồn**: worktree `vd242-st240-stdcli`, commit Agent 2 `b2666d8`. **Người review**: Agent 1. **Ngày**: 2026-11-20.

## 1. Các ca đã sửa (15) — verdict

Danh sách #168, #171, #172, #173, #174, #175, #177, #178, #179, #180, #182, #183, #184 (13 nợ số) + kiểm tích hợp #169/#170/Option/parity. Mỗi ca có repro đỏ→xanh trong báo cáo Agent 2; `st240_std_cli.rs` 11/11, unit LSP framing 4/4.

| Nhóm | Đối chiếu căn cứ | Verdict |
|---|---|---|
| JSON/sort: #168, #173, #175 | VD-242 §3.5: không im lặng sort, giữ dấu float, từ chối `01`; #169 đã đóng ST-238 | **SHIP** |
| Vị trí/chẩn đoán: #177, #178, #179, #180, #182, #183, #184 | VD-242 §4: sai tệp/dòng, CRLF, warning exit, tệp nêu đích danh không bỏ qua, EOF phân biệt; #181 đã đóng ST-238 | **SHIP** |
| #171 `ls --long` | VD-242 cho phép `–`; `modified` kẹp `0ns` để tránh in số âm/năm tràn | **SHIP có ghi chú**: muốn năm đúng 2446 cần sửa `ngoai.rs` (ngoài quyền) |
| #172 `percent` | VD-243: vượt miền ⇒ `VS0507` exit ≠ 0; non-finite chặn tại tạo giá trị; literal không biểu diễn được từ chối; biên hợp lệ giữ nguyên; trung gian lớn mà kết quả cuối hợp lệ không bị từ chối | **SHIP có ghi chú 2 điểm** (xem §2) |

**Không sửa/không đóng lại**: #176 (bác bỏ SPEC), #169 (ST-238), #170/Option/parity (ST-239). Đúng phân quyền.

## 2. Ghi chú cần A0 biết (không chặn merge)

1. **Thiếu hợp đồng miền `percent` (VD-243 §2.1)**: SPEC chép “tỉ lệ 0..1” nhưng `200pct` đã được chấp nhận từ trước. Agent 2 **không chốt miền mới**, dùng ranh giới biểu diễn hiện hành (`|ratio*100| ≤ 2^63`, nơi `show_percent` ép `as i64`). Đề nghị A0 làm rõ sau; không mở VD trong đợt này.
2. **Literal `percent` không biểu diễn được** bị từ chối lúc **đánh giá literal** (`lit_value`, cả interp + VM), không phải lexer/checker; **mã lỗi khác nhau giữa hai engine nhưng cùng loại lỗi** — phù hợp VD-243 §1 “giữ mã đúng tầng hiện hành”. Ghi nhận để theo dõi, không coi là parity lỗi.
3. **#177 VM**: bytecode `Span` chỉ có `start/end`, không có `file id`; CLI khôi phục module bằng điều kiện “tệp chính không chứa khoảng byte và đúng 1 module chứa”. Đúng repro, **không đoán khi mơ hồ**; sửa triệt để cần đổi bytecode format/codegen/verify (ngoài phạm vi). Ghi nợ theo dõi.

## 3. Kết luận

**SHIP 15/15.** Merge `b2666d8`; đóng 13 nợ số; đóng `BLOCKED.md` §12 (triển khai #172 có bằng chứng). Không tag, không fresh-clone/release verification cho tới khi §14/§16 xong.
