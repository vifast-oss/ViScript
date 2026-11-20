// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (sqlite-tables)** — lõi **dùng chung** cho `sqlite.tables` (interp ↔ VM
//! byte-identical).
//!
//! Trả **tên** bảng và view người dùng trong **schema chính** của connection, sắp theo **byte**
//! (`ORDER BY name` — collation `BINARY` của SQLite so UTF-8 theo byte), bỏ đối tượng nội bộ có
//! tiền tố `sqlite_` (không phân biệt hoa thường như `LIKE`; `_` được escape nên chỉ đúng tiền
//! tố `sqlite_`, không nuốt tên kiểu `sqliteXtra`).
//!
//! Đi qua **đúng** đường `DbHost::db_query` hiện có (không thêm method host): cùng authorizer,
//! chính sách chỉ đọc (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler
//! timeout, cache schema/statement và bind params rỗng. `sqlite_master` là bảng hệ thống bình
//! thường với authorizer (chỉ `dbstat`/trang thô bị chặn) — probe xác nhận đọc được.
//!
//! **Bảng TEMP không xuất hiện**: chúng nằm ở schema `temp` (`sqlite_temp_master`), không phải
//! `sqlite_master` của schema chính; `tables` mô tả **schema chính** của connection (bảng tạm là
//! dữ liệu phiên, không thuộc tệp DB). Xem test `bang_tam_va_noi_bo` ở
//! `crates/vis-cli/tests/vd270_sqlite_tables.rs` — đây là hợp đồng **có chủ ý**, không phải sót.
//!
//! Mọi lỗi (handle thiếu/đã đóng/sai dạng, SQL/host lỗi, kết quả sai hình dạng) ⇒ `Err(String)`
//! **dữ liệu** — tầng std biến thành `Err(Error)`, **không** trap. DB rỗng ⇒ `Ok([])`.

use crate::builtin::DbHost;
use crate::value::Value;

/// Truy vấn hệ thống: một cột `name` TEXT, chỉ bảng/view người dùng của schema chính.
///
/// `name NOT LIKE 'sqlite\_%' ESCAPE '\'`: `LIKE` của SQLite **không phân biệt hoa thường** với
/// ASCII và `_` là wildcard một ký tự — escape để loại **đúng** tiền tố `sqlite_` (đối tượng nội
/// bộ), không loại oan tên người dùng như `sqliteXtra`. `ORDER BY name` dùng collation `BINARY`
/// mặc định ⇒ thứ tự **byte**, đúng hợp đồng.
const CAU: &str = "SELECT name FROM sqlite_master \
                   WHERE type IN ('table','view') \
                     AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' \
                   ORDER BY name";

/// Chạy [`CAU`] qua `host.db_query` và trả danh sách tên (đã sắp byte lần nữa cho chắc).
///
/// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
/// một cột, mỗi hàng đúng một ô `SqlValue.Text(str)`; lệch ⇒ `Err` dữ liệu (không panic).
pub fn chay(host: &mut dyn DbHost, h: &str) -> Result<Vec<String>, String> {
    let (cot, hang) = host.db_query(h, CAU, &[])?;
    if cot.len() != 1 {
        return Err(format!(
            "`sqlite.tables`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `name`)",
            cot.len()
        ));
    }
    let mut ten = Vec::with_capacity(hang.len());
    for (i, hang) in hang.into_iter().enumerate() {
        let [o] = hang.as_slice() else {
            return Err(format!(
                "`sqlite.tables`: hàng {} có {} ô (cần đúng 1)",
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
                            "`sqlite.tables`: hàng {} — `SqlValue.Text` không mang `str`",
                            i + 1
                        ))
                    }
                }
            }
            _ => {
                return Err(format!(
                    "`sqlite.tables`: hàng {} — tên bảng/view không phải TEXT",
                    i + 1
                ))
            }
        }
    }
    // SQL đã `ORDER BY name` (BINARY); sắp lại bằng `sort_unstable` (byte lexicographic) để hợp
    // đồng **không phụ thuộc** host/query — rẻ với số bảng thực tế và giữ đúng thứ tự byte.
    ten.sort_unstable();
    Ok(ten)
}
