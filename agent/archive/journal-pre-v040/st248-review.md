<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-248 — review Agent 1 (Option field/index lift)

**Nguồn**: worktree `vd242-st248-fieldlift`, commit Agent 2 `26567bf` (baseline `f491bae`). **Người review**: Agent 1. **Ngày**: 2026-11-20.

## 1. Kết quả & căn cứ

| Mục | Bằng chứng | Verdict |
|---|---|---|
| Repro field/index trước/sau | baseline `5`/`7` (cả interp và VM) ⇒ sau `Some(5)`/`Some(7)` (cả hai engine); `st248_field_lift.rs` trước **4 FAILED**, sau **7/7** | **SHIP** |
| Căn cứ | VD-242 §3.3 (đích `Option[T]`, nguồn khớp `T` ⇒ nâng đúng một lớp; áp field/index); §3.1 (sai kiểu ⇒ từ chối; parity hai engine cùng sai không tính đạt) | **SHIP** |
| Phạm vi | Chỉ `codegen/than.rs`, `codegen.rs`, `interp.rs` + 1 test; dùng chung `$lift-opt`/`lift_opt`; **không** đổi opcode/bytecode/VERSION; R1 `codegen.rs` 1494, `interp.rs` 1488 (≤1500) | **SHIP** |
| Không phát sinh ngữ nghĩa mới | Không API freeze/cú pháp/kiểu/bytecode mới; không ratify `Unwrap → Unit`; không nới sandbox/hạ tiêu chí/dependency | **SHIP** |

## 2. Phát hiện ngoài phạm vi — stale cache khi đổi logic codegen (CẦN xử lý riêng)

Agent 2 báo rõ: `vis run --vm` **không** dùng `--no-cache` có thể nạp module VBC1 **cũ từ trước bản sửa**, vì `cache::PHIEN_BAN` là chuỗi cố định `concat!(CARGO_PKG_VERSION, "+vd241-coercion…")`, **không đổi theo logic codegen**. Test Rust và `vis test --vm` biên dịch mới nên không bị ảnh hưởng — nhưng người dùng có cache cũ sẽ vẫn thấy `5`/`7` thay vì `Some(5)`/`Some(7)`.

Đây là **hệ quả trực tiếp của ST-248**, nằm ngoài quyền tệp đã giao (`cache.rs` bị cấm trong ST-248, ST-246 đã xong). Phải xử lý trước khi coi đợt ổn định khép:
- Bump `PHIEN_BAN` trong `crates/vis-cli/src/cache.rs` kèm lý do ST-248/VD-129, **re-pin** khoá ghim `khoa_cache_ghim` (sẽ đổi hash), test lại.
- Ghi nợ/task riêng **ST-249** + **#187** trong sổ; không tự sửa trong ST-248.

`BLOCKED.md` §15 (field/index lift) sẽ đóng sau khi ST-248 merge; stale cache là vấn đề **mới**, theo dõi ở §16.

## 3. Kết luận

**SHIP ST-248.** Merge `26567bf`, đóng §15, mở ST-249/#187. Không tuyên bố “hợp đồng Option khép” trọn vẹn cho tới khi ST-249 xong (vì đường chạy cache cũ vẫn có thể trả kết quả sai).
