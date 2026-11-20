// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (sqlite-primary-keys)** — lõi **dùng chung** cho `sqlite.primary_keys`
//! (interp ↔ VM byte-identical).
//!
//! Trả **tên cột khoá chính** của một **bảng** trong **schema chính** (`main`) của connection,
//! theo **thứ tự khoá** (`pk` 1..N — đúng thứ tự khai trong `PRIMARY KEY (…)`, **không** sắp
//! alphabet), giữ nguyên hoa/thường như định nghĩa. Tên bảng được **bind** làm tham số (`?`) —
//! không bao giờ nối chuỗi SQL — nên tên chứa dấu nháy/`;`/comment chỉ là tên, không có tác dụng
//! SQL.
//!
//! Đi qua **đúng** đường `DbHost::db_query` hiện có (không thêm method host): cùng authorizer,
//! chính sách chỉ đọc (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler
//! timeout, cache schema/statement. `pragma_table_info` là table-valued function dựng sẵn của
//! SQLite; probe xác nhận authorizer cho đọc (chỉ `dbstat`/trang thô bị chặn). Tham số schema
//! **cố định** `'main'` nên bảng TEMP trùng tên **không** che bảng chính và không lọt vào kết quả.
//! **Hạn chế đã biết**: SQLite cho đối tượng schema trùng tên `pragma_table_info` che TVF dựng
//! sẵn; khi đó truy vấn lỗi ⇒ `Err(Error)` **dữ liệu** (không đọc nhầm/không trả dữ liệu sai).
//!
//! **Bảng không có khoá chính, view, hoặc bảng không tồn tại ⇒ `Ok([])`** (pragma không trả
//! hàng `pk > 0`) — khác hẳn handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)`
//! **dữ liệu** (tầng std biến thành `Err(Error)`, **không** trap). Kết quả sai hình dạng (không
//! đúng một cột `name`, ô không phải `SqlValue.Text`) cũng là `Err` dữ liệu, không panic.

use crate::builtin::DbHost;
use crate::value::Value;

/// Truy vấn hệ thống: một cột `name` TEXT của các cột khoá chính, theo đúng thứ tự khoá.
///
/// - `?` là **tên bảng** bind từ tham số — không nối chuỗi, injection vô hiệu.
/// - `'main'` cố định: chỉ schema chính (không phải `temp`/schema gắn ngoài).
/// - `pk > 0` chỉ giữ cột thuộc khoá chính (`pk` = 1..N); `ORDER BY pk` là thứ tự khai trong
///   `PRIMARY KEY`, **không** phải thứ tự cột (`cid`) hay alphabet.
const CAU: &str = "SELECT name FROM pragma_table_info(?, 'main') WHERE pk > 0 ORDER BY pk";

/// Chạy [`CAU`] qua `host.db_query` và trả danh sách tên cột khoá chính **đúng thứ tự khoá**.
///
/// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
/// một cột, mỗi hàng đúng một ô `SqlValue.Text(str)`; lệch ⇒ `Err` dữ liệu (không panic).
/// Bảng không có khoá chính/view/không tồn tại ⇒ pragma trả 0 hàng ⇒ `Ok(vec![])`.
pub fn chay(host: &mut dyn DbHost, h: &str, table: &str) -> Result<Vec<String>, String> {
    // Tên đối tượng là **dữ liệu**, không phải SQL: bind `SqlValue.Text`. Host kiểm/giới hạn
    // param (≤ 1 MiB text) và bind qua rusqlite; không có đường nối chuỗi nào ở đây.
    let tham_so = [Value::Enum(
        "SqlValue".into(),
        "Text".into(),
        vec![Value::Str(table.to_string())],
    )];
    let (cot, hang) = host.db_query(h, CAU, &tham_so)?;
    if cot.len() != 1 {
        return Err(format!(
            "`sqlite.primary_keys`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `name`)",
            cot.len()
        ));
    }
    let mut ten = Vec::with_capacity(hang.len());
    for (i, hang) in hang.into_iter().enumerate() {
        let [o] = hang.as_slice() else {
            return Err(format!(
                "`sqlite.primary_keys`: hàng {} có {} ô (cần đúng 1)",
                i + 1,
                hang.len()
            ));
        };
        match o {
            Value::Enum(e, v, args) if e == "SqlValue" && v == "Text" && args.len() == 1 => {
                match &args[0] {
                    Value::Str(s) => ten.push(s.clone()),
                    _ => {
                        return Err(format!(
                            "`sqlite.primary_keys`: hàng {} — `SqlValue.Text` không mang `str`",
                            i + 1
                        ))
                    }
                }
            }
            _ => {
                return Err(format!(
                    "`sqlite.primary_keys`: hàng {} — tên cột không phải TEXT",
                    i + 1
                ))
            }
        }
    }
    // **Không** sắp lại: `ORDER BY pk` là hợp đồng **thứ tự khoá**, khác `sqlite.columns`
    // (`ORDER BY cid`) và `sqlite.tables`/`sqlite.indexes` (sắp byte).
    Ok(ten)
}
