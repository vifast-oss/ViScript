// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (sqlite-columns)** — lõi **dùng chung** cho `sqlite.columns` (interp ↔ VM
//! byte-identical).
//!
//! Trả **tên cột** của một **bảng hoặc view** trong **schema chính** (`main`) của connection,
//! theo **thứ tự khai báo** (`cid` tăng dần), giữ nguyên hoa/thường như định nghĩa. Tên đối tượng
//! được **bind** làm tham số (`?`) — không bao giờ nối chuỗi SQL — nên tên chứa dấu nháy/`;`/
//! comment chỉ là tên, không có tác dụng SQL.
//!
//! Đi qua **đúng** đường `DbHost::db_query` hiện có (không thêm method host): cùng authorizer,
//! chính sách chỉ đọc (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler
//! timeout, cache schema/statement. `pragma_table_info` là table-valued function dựng sẵn của
//! SQLite; probe xác nhận authorizer cho đọc (chỉ `dbstat`/trang thô bị chặn). Tham số schema
//! **cố định** `'main'` nên bảng TEMP trùng tên **không** che bảng chính và không lọt vào kết quả.
//! **Hạn chế đã biết**: SQLite cho đối tượng schema trùng tên `pragma_table_info` che TVF dựng
//! sẵn; khi đó truy vấn lỗi ⇒ `Err(Error)` **dữ liệu** (không đọc nhầm/không trả dữ liệu sai).
//!
//! **Bảng/view không tồn tại ⇒ `Ok([])`** (pragma không trả hàng) — khác hẳn handle thiếu/đã
//! đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến thành `Err(Error)`,
//! **không** trap). Kết quả sai hình dạng (không đúng một cột `name`, ô không phải
//! `SqlValue.Text`) cũng là `Err` dữ liệu, không panic.

use crate::builtin::DbHost;
use crate::value::Value;

/// Truy vấn hệ thống: một cột `name` TEXT theo đúng thứ tự khai báo (`cid`).
///
/// - `?` là **tên bảng/view** bind từ tham số — không nối chuỗi, injection vô hiệu.
/// - `'main'` cố định: chỉ schema chính (không phải `temp`/schema gắn ngoài).
/// - `ORDER BY cid`: thứ tự khai báo, **không** sắp theo tên; view cũng có `cid` theo thứ tự
///   cột của câu `SELECT`.
const CAU: &str = "SELECT name FROM pragma_table_info(?, 'main') ORDER BY cid";

/// Chạy [`CAU`] qua `host.db_query` và trả danh sách tên cột **đúng thứ tự khai báo**.
///
/// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
/// một cột, mỗi hàng đúng một ô `SqlValue.Text(str)`; lệch ⇒ `Err` dữ liệu (không panic).
/// Bảng/view không tồn tại ⇒ pragma trả 0 hàng ⇒ `Ok(vec![])`.
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
            "`sqlite.columns`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `name`)",
            cot.len()
        ));
    }
    let mut ten = Vec::with_capacity(hang.len());
    for (i, hang) in hang.into_iter().enumerate() {
        let [o] = hang.as_slice() else {
            return Err(format!(
                "`sqlite.columns`: hàng {} có {} ô (cần đúng 1)",
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
                            "`sqlite.columns`: hàng {} — `SqlValue.Text` không mang `str`",
                            i + 1
                        ))
                    }
                }
            }
            _ => {
                return Err(format!(
                    "`sqlite.columns`: hàng {} — tên cột không phải TEXT",
                    i + 1
                ))
            }
        }
    }
    // **Không** sắp lại: `ORDER BY cid` là hợp đồng **thứ tự khai báo**. (Khác `sqlite.tables`,
    // nơi hợp đồng là thứ tự byte.)
    Ok(ten)
}
