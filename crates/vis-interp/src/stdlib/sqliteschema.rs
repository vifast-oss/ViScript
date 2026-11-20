// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (sqlite-schema-sql)** — lõi **dùng chung** cho `sqlite.schema_sql` (interp ↔ VM
//! byte-identical).
//!
//! Trả **văn bản SQL khai báo** của một đối tượng schema trong **schema chính** (`main`) của
//! connection — `table`/`view`/`index`/`trigger` — tra theo **tên** trong `sqlite_master`. Tên
//! không phân biệt loại: cùng một danh sách tên của schema chính, nên tra `t` trả câu `CREATE`
//! tương ứng bất kể đối tượng là loại nào.
//!
//! So khớp tên là **phân biệt hoa/thường** (`=` trên cột `name` dùng collation `BINARY` mặc
//! định của `sqlite_master`): `CREATE TABLE Bang` lưu đúng `Bang`, nên tra `bang` ⇒ `Ok(None)`
//! dù SQLite vẫn cho tham chiếu bảng tên `bang` trong câu SQL khác. Đây là hợp đồng **có chủ ý**
//! của truy vấn, không phải sót.
//!
//! Không có tham số **loại** đối tượng: SQLite cho phép **trigger** trùng tên với bảng (hai
//! không gian tên khác nhau), nên nếu một tên có hai hàng trong `sqlite_master`, `LIMIT 1` trả
//! hàng mà truy vấn duyệt trước — hợp đồng **không hứa** loại nào; đây là hệ quả trực tiếp của
//! "tra theo tên", không phải lỗi. Bảng/view/index chia sẻ không gian tên nên không trùng nhau.
//!
//! Đi qua **đúng** đường `DbHost::db_query` hiện có (không thêm method host): cùng authorizer,
//! chính sách chỉ đọc (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler
//! timeout, cache schema/statement. `sqlite_master` của schema chính là bảng hệ thống bình thường
//! với authorizer (chỉ `dbstat`/trang thô bị chặn); `sqlite.tables`/`columns`/`indexes` đã đi
//! cùng đường đọc này.
//!
//! **Không có hàng ⇒ `Ok(None)`** (tên không tồn tại). Hàng có `sql = NULL` cũng ⇒ `Ok(None)`:
//! SQLite để `sql` NULL cho đối tượng **do nó tự sinh** như index tự động `sqlite_autoindex_%`
//! (từ `UNIQUE`/`PRIMARY KEY`) — không có văn bản `CREATE` để trả. Đối tượng nội bộ `sqlite_%`
//! **không** bị đặc cách ngoài những gì truy vấn trả: ví dụ `sqlite_sequence` (sinh bởi
//! `AUTOINCREMENT`) có `sql` thật nên trả đúng chuỗi SQLite lưu; `sqlite_master`/`sqlite_schema`
//! không có hàng trong chính nó nên tra ⇒ `Ok(None)`. TEMP object nằm ở schema `temp`
//! (`sqlite_temp_master`) nên **không** thuộc kết quả schema chính.
//!
//! Tên đối tượng là **dữ liệu**, không phải SQL: bind `SqlValue.Text` làm tham số `?` — không
//! bao giờ nối chuỗi, nên tên kiểu `t' OR 1=1 --`/`t; DROP TABLE t` chỉ là tên (không có đối
//! tượng như vậy ⇒ `Ok(None)`), không có tác dụng phụ. Hàm **chỉ đọc**.
//!
//! Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến
//! thành `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng một cột `sql`, quá một
//! hàng, ô không phải `SqlValue.Text`/`SqlValue.Null`) cũng là `Err` dữ liệu, không panic.

use crate::builtin::DbHost;
use crate::value::Value;

/// Truy vấn hệ thống: một cột `sql` (TEXT hoặc NULL), tra theo **tên** trong schema chính.
///
/// - `?` là **tên đối tượng** bind từ tham số — không nối chuỗi, injection vô hiệu.
/// - Không lọc `type`: mọi đối tượng schema (table/view/index/trigger) dùng chung một không gian
///   tên trong `sqlite_master`; tên quyết định đối tượng, không phải loại.
/// - `name = ?`: collation `BINARY` mặc định ⇒ so **byte**, **phân biệt hoa/thường**.
/// - `LIMIT 1`: tối đa một hàng (bình thường tên là duy nhất trong schema chính).
const CAU: &str = "SELECT sql FROM sqlite_master WHERE name = ? LIMIT 1";

/// Chạy [`CAU`] qua `host.db_query` và trả văn bản SQL của đối tượng.
///
/// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**:
/// đúng một cột, tối đa một hàng, ô là `SqlValue.Text(str)` ⇒ `Some`, `SqlValue.Null` ⇒ `None`;
/// lệch ⇒ `Err` dữ liệu (không panic).
pub fn chay(host: &mut dyn DbHost, h: &str, name: &str) -> Result<Option<String>, String> {
    // Tên đối tượng là **dữ liệu**, không phải SQL: bind `SqlValue.Text`. Host kiểm/giới hạn
    // param (≤ 1 MiB text) và bind qua rusqlite; không có đường nối chuỗi nào ở đây.
    let tham_so = [Value::Enum(
        "SqlValue".into(),
        "Text".into(),
        vec![Value::Str(name.to_string())],
    )];
    let (cot, hang) = host.db_query(h, CAU, &tham_so)?;
    if cot.len() != 1 {
        return Err(format!(
            "`sqlite.schema_sql`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `sql`)",
            cot.len()
        ));
    }
    match hang.as_slice() {
        // Không có đối tượng tên đó trong schema chính.
        [] => Ok(None),
        [hang] => {
            let [o] = hang.as_slice() else {
                return Err(format!(
                    "`sqlite.schema_sql`: hàng có {} ô (cần đúng 1)",
                    hang.len()
                ));
            };
            match o {
                // `sql` NULL (index tự động, đối tượng SQLite tự sinh) ⇒ không có văn bản.
                Value::Enum(e, v, args) if e == "SqlValue" && v == "Null" && args.is_empty() => {
                    Ok(None)
                }
                Value::Enum(e, v, args) if e == "SqlValue" && v == "Text" && args.len() == 1 => {
                    match &args[0] {
                        Value::Str(s) => Ok(Some(s.clone())),
                        _ => {
                            Err("`sqlite.schema_sql`: `SqlValue.Text` không mang `str`".to_string())
                        }
                    }
                }
                _ => Err("`sqlite.schema_sql`: `sql` không phải TEXT/NULL".to_string()),
            }
        }
        _ => Err(format!(
            "`sqlite.schema_sql`: truy vấn hệ thống trả {} hàng (cần tối đa 1)",
            hang.len()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Host giả: trả đúng `(cột, hàng)` đã định; ghi lại SQL/params để kiểm hợp đồng.
    struct HostGia {
        cot: Vec<String>,
        hang: Vec<Vec<Value>>,
        sql: Option<String>,
        tham_so: Option<Vec<Value>>,
    }

    impl DbHost for HostGia {
        fn db_open(&mut self, _: &str, _: bool, _: i64) -> Result<String, String> {
            unreachable!()
        }
        fn db_close(&mut self, _: &str) -> Result<(), String> {
            unreachable!()
        }
        fn db_execute(&mut self, _: &str, _: &str, _: &[Value]) -> Result<i64, String> {
            unreachable!()
        }
        fn db_query(
            &mut self,
            _: &str,
            sql: &str,
            params: &[Value],
        ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
            self.sql = Some(sql.to_string());
            self.tham_so = Some(params.to_vec());
            Ok((self.cot.clone(), self.hang.clone()))
        }
        fn db_in_transaction(&mut self, _: &str) -> Result<bool, String> {
            unreachable!()
        }
        fn db_cleanup(&mut self) -> Result<u64, String> {
            unreachable!()
        }
        fn db_path_of(&mut self, _: &str) -> Result<String, String> {
            unreachable!()
        }
    }

    fn host(cot: &[&str], hang: Vec<Vec<Value>>) -> HostGia {
        HostGia {
            cot: cot.iter().map(|s| s.to_string()).collect(),
            hang,
            sql: None,
            tham_so: None,
        }
    }

    fn text(s: &str) -> Value {
        Value::Enum(
            "SqlValue".into(),
            "Text".into(),
            vec![Value::Str(s.to_string())],
        )
    }

    fn null() -> Value {
        Value::Enum("SqlValue".into(), "Null".into(), Vec::new())
    }

    #[test]
    fn tra_theo_ten_bind_va_tra_text() {
        let mut h = host(&["sql"], vec![vec![text("CREATE TABLE t (a TEXT)")]]);
        assert_eq!(
            chay(&mut h, "db:x", "t").expect("phải Ok"),
            Some("CREATE TABLE t (a TEXT)".to_string())
        );
        assert_eq!(
            h.sql.as_deref(),
            Some(CAU),
            "phải chạy đúng câu sqlite_master"
        );
        match &h.tham_so.as_deref() {
            Some([Value::Enum(e, v, args)]) if e == "SqlValue" && v == "Text" => {
                assert!(
                    matches!(args.as_slice(), [Value::Str(s)] if s == "t"),
                    "tên phải bind `SqlValue.Text(\"t\")`, được {args:?}"
                );
            }
            khac => panic!("param phải là `SqlValue.Text(tên)`, được {khac:?}"),
        }
    }

    #[test]
    fn khong_co_hang_hoac_null_deu_none() {
        let mut h = host(&["sql"], Vec::new());
        assert_eq!(chay(&mut h, "db:x", "thieu").expect("phải Ok"), None);
        let mut h = host(&["sql"], vec![vec![null()]]);
        assert_eq!(
            chay(&mut h, "db:x", "sqlite_autoindex_t_1").expect("phải Ok"),
            None
        );
    }

    #[test]
    fn hinh_dang_sai_la_err_du_lieu() {
        let mut h = host(&["sql", "extra"], vec![vec![text("x"), text("y")]]);
        assert!(chay(&mut h, "db:x", "t").is_err(), "hai cột ⇒ Err");
        let mut h = host(&["sql"], vec![vec![]]);
        assert!(chay(&mut h, "db:x", "t").is_err(), "hàng 0 ô ⇒ Err");
        let mut h = host(&["sql"], vec![vec![Value::Int(1)]]);
        assert!(chay(&mut h, "db:x", "t").is_err(), "ô int ⇒ Err");
        let mut h = host(&["sql"], vec![vec![text("a")], vec![text("b")]]);
        assert!(chay(&mut h, "db:x", "t").is_err(), "hai hàng ⇒ Err");
    }
}
