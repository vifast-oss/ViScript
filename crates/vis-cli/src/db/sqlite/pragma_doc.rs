// SPDX-License-Identifier: Apache-2.0
//! **VD-270** — nhận dạng **hẹp** các pragma chỉ đọc mà SQLite đánh dấu **không** read-only, để
//! `query` mở đúng dạng **hằng** (không giá trị) mà không nới bất kỳ dạng ghi nào. Tách khỏi
//! `mod.rs` để giữ luật R1 (≤ 1.500 dòng/tệp).
//!
//! **`journal_mode`**: `PRAGMA journal_mode` (không giá trị) là truy vấn chỉ đọc nhưng
//! `sqlite3_stmt_readonly` trả **false** (cùng câu có thể đổi mode khi có giá trị), nên cần
//! [`la_pragma_journal_mode_chi_doc`].
//!
//! **`database_list`** (VD-270 sqlite-database-list): probe cho thấy `sqlite3_stmt_readonly` của
//! `PRAGMA database_list` là **true** — **không** cần ngoại lệ ở đây; chỉ authorizer cần mở mục
//! **đọc** `database_list` (không giá trị), xem `chinh_sach` trong `mod.rs`.

/// `PRAGMA journal_mode` **không giá trị**: sau `trim`, bỏ **một** `;` cuối, đúng **hai** token
/// `PRAGMA` + `journal_mode` (không phân biệt hoa/thường, cho phép khoảng trắng giữa). Mọi dạng
/// khác — `= value`, `main.journal_mode`, comment/token thừa — ⇒ `false` (rơi về kiểm
/// `sqlite3_stmt_readonly` như cũ; authorizer đã chặn ghi pragma).
pub(super) fn la_pragma_journal_mode_chi_doc(sql: &str) -> bool {
    let s = sql.trim();
    let s = s.strip_suffix(';').unwrap_or(s);
    let mut it = s.split_whitespace();
    matches!(
        (it.next(), it.next(), it.next()),
        (Some(a), Some(b), None)
            if a.eq_ignore_ascii_case("PRAGMA") && b.eq_ignore_ascii_case("journal_mode")
    )
}
