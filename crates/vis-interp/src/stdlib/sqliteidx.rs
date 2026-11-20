// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (sqlite-indexes)** — lõi **dùng chung** cho `sqlite.indexes` (interp ↔ VM
//! byte-identical).
//!
//! Trả **tên index do người dùng tạo** của một **bảng** trong **schema chính** (`main`) của
//! connection, sắp theo **byte** (`ORDER BY name`, collation `BINARY` mặc định), giữ nguyên
//! hoa/thường như đã khai. Index tự động của SQLite (`sqlite_autoindex_%` — sinh bởi `UNIQUE`/
//! `PRIMARY KEY`) và index của bảng khác **không** xuất hiện.
//!
//! Tên bảng được **bind** làm tham số (`?`) — không bao giờ nối chuỗi SQL — nên tên chứa dấu
//! nháy/`;`/comment chỉ là tên, không có tác dụng SQL. `tbl_name = ? COLLATE BINARY` so **byte**
//! (phân biệt hoa/thường) với tên đã khai trong `sqlite_master`, đúng như hợp đồng của truy vấn.
//!
//! Đi qua **đúng** đường `DbHost::db_query` hiện có (không thêm method host): cùng authorizer,
//! chính sách chỉ đọc (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler
//! timeout, cache schema/statement. `sqlite_master` của schema chính là bảng hệ thống bình thường
//! với authorizer (chỉ `dbstat`/trang thô bị chặn); `sqlite.tables`/`sqlite.columns` đã đi cùng
//! đường đọc này.
//!
//! **Bảng/view không tồn tại ⇒ `Ok([])`** (không có hàng `type='index'`); **view** không thể có
//! index nên cũng `Ok([])`. Bảng TEMP nằm ở schema `temp` (`sqlite_temp_master`), không thuộc
//! `sqlite_master` của schema chính nên index của nó không lọt vào kết quả. Khác hẳn handle
//! thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến thành
//! `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng một cột `name`, ô không phải
//! `SqlValue.Text`) cũng là `Err` dữ liệu, không panic.

use crate::builtin::DbHost;
use crate::value::Value;

/// Truy vấn hệ thống: một cột `name` TEXT, chỉ index người dùng của bảng trong schema chính.
///
/// - `?` là **tên bảng** bind từ tham số — không nối chuỗi, injection vô hiệu.
/// - `type = 'index'`: chỉ index; bảng/view/trigger không lọt.
/// - `tbl_name = ? COLLATE BINARY`: đúng bảng đích, so **byte** (phân biệt hoa/thường).
/// - `name NOT LIKE 'sqlite_autoindex_%'`: loại index tự động của SQLite. Tiền tố `sqlite_` là
///   **dành riêng** cho SQLite nên không thể là index người dùng.
/// - `ORDER BY name`: collation `BINARY` mặc định ⇒ thứ tự **byte**.
const CAU: &str = "SELECT name FROM sqlite_master \
                   WHERE type = 'index' \
                     AND tbl_name = ? COLLATE BINARY \
                     AND name NOT LIKE 'sqlite_autoindex_%' \
                   ORDER BY name";

/// Chạy [`CAU`] qua `host.db_query` và trả danh sách tên index (đã sắp byte lần nữa cho chắc).
///
/// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
/// một cột, mỗi hàng đúng một ô `SqlValue.Text(str)`; lệch ⇒ `Err` dữ liệu (không panic).
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
            "`sqlite.indexes`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `name`)",
            cot.len()
        ));
    }
    let mut ten = Vec::with_capacity(hang.len());
    for (i, hang) in hang.into_iter().enumerate() {
        let [o] = hang.as_slice() else {
            return Err(format!(
                "`sqlite.indexes`: hàng {} có {} ô (cần đúng 1)",
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
                            "`sqlite.indexes`: hàng {} — `SqlValue.Text` không mang `str`",
                            i + 1
                        ))
                    }
                }
            }
            _ => {
                return Err(format!(
                    "`sqlite.indexes`: hàng {} — tên index không phải TEXT",
                    i + 1
                ))
            }
        }
    }
    // SQL đã `ORDER BY name` (BINARY); sắp lại bằng `sort_unstable` (byte lexicographic) để hợp
    // đồng **không phụ thuộc** host/query — rẻ với số index thực tế và giữ đúng thứ tự byte.
    ten.sort_unstable();
    Ok(ten)
}
