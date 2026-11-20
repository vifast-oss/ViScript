// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (sqlite-table-exists)** — lõi **dùng chung** cho `sqlite.table_exists`
//! (interp ↔ VM byte-identical).
//!
//! Trả `true` **khi và chỉ khi** connection có **bảng hoặc view** tên **đúng** trong **schema
//! chính** (`main`). Truy vấn:
//!
//! ```sql
//! SELECT 1 FROM sqlite_master
//!  WHERE type IN ('table','view') AND name = ? LIMIT 1
//! ```
//!
//! `?` là **tên bind** (`SqlValue.Text`) — tên đối tượng là **dữ liệu**, không phải SQL: không
//! nối chuỗi, nên tên kiểu `t' OR 1=1 --`/`t; DROP TABLE t` chỉ là tên (không có đối tượng như
//! vậy ⇒ `Ok(false)`), không có tác dụng phụ. Hàm **chỉ đọc**, đi qua **đúng** đường
//! `DbHost::db_query` hiện có (không thêm method host): cùng authorizer, chính sách chỉ đọc
//! (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler timeout, cache
//! schema/statement. `sqlite_master` là bảng hệ thống bình thường với authorizer (chỉ
//! `dbstat`/trang thô bị chặn) — `sqlite.tables`/`columns`/`schema_sql` đã đi cùng đường đọc này.
//!
//! So khớp **phân biệt hoa/thường**: `name = ?` dùng collation `BINARY` mặc định của
//! `sqlite_master` (`CREATE TABLE Bang` ⇒ tra `Bang` = `true`, tra `bang`/`BANG` = `false` dù
//! SQLite vẫn cho tham chiếu bảng tên `bang` trong câu SQL khác). Đây là hợp đồng **có chủ ý**
//! của truy vấn, không phải sót.
//!
//! **Không có hàng ⇒ `Ok(false)`**. Đối tượng nội bộ `sqlite_%` **không** bị đặc cách ngoài
//! truy vấn: `sqlite_sequence` (sinh bởi `AUTOINCREMENT`, `type = 'table'`) ⇒ `true`;
//! `sqlite_master`/`sqlite_schema` không có hàng trong chính nó ⇒ `false`; index tự động
//! `sqlite_autoindex_%` có `type = 'index'` nên **không** tính là bảng/view ⇒ `false`. Trigger
//! cùng tên bảng cũng không tính (`type = 'trigger'`). Đối tượng TEMP nằm ở schema `temp`
//! (`sqlite_temp_master`), **không** thuộc schema chính ⇒ `false` — kể cả khi trùng tên bảng
//! chính (bảng chính vẫn `true`).
//!
//! Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến
//! thành `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng một cột, quá một
//! hàng, ô không phải `SqlValue.Integer(1)`) cũng là `Err` dữ liệu, không panic.

use crate::builtin::DbHost;
use crate::value::Value;

/// Truy vấn hệ thống: một cột hằng `1`, chỉ `table`/`view` của **schema chính** tra theo **tên**.
///
/// - `?` là **tên đối tượng** bind từ tham số — không nối chuỗi, injection vô hiệu.
/// - `type IN ('table','view')`: đúng hợp đồng "bảng **hoặc view**"; index/trigger không tính.
/// - `name = ?`: collation `BINARY` mặc định ⇒ so **byte**, **phân biệt hoa/thường**.
/// - `LIMIT 1`: tối đa một hàng (bảng/view chia sẻ không gian tên nên tên là duy nhất).
const CAU: &str = "SELECT 1 FROM sqlite_master WHERE type IN ('table','view') AND name = ? LIMIT 1";

/// Chạy [`CAU`] qua `host.db_query`; `true` ⇔ có ít nhất một hàng.
///
/// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
/// một cột; 0 hàng ⇒ `false`; đúng 1 hàng, đúng 1 ô `SqlValue.Integer(1)` ⇒ `true`; lệch ⇒ `Err`
/// dữ liệu (không panic).
pub fn chay(host: &mut dyn DbHost, h: &str, name: &str) -> Result<bool, String> {
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
            "`sqlite.table_exists`: truy vấn hệ thống trả {} cột (cần đúng 1)",
            cot.len()
        ));
    }
    match hang.as_slice() {
        // Không có bảng/view tên đó trong schema chính.
        [] => Ok(false),
        [hang] => {
            let [o] = hang.as_slice() else {
                return Err(format!(
                    "`sqlite.table_exists`: hàng có {} ô (cần đúng 1)",
                    hang.len()
                ));
            };
            match o {
                Value::Enum(e, v, args)
                    if e == "SqlValue"
                        && v == "Integer"
                        && matches!(args.as_slice(), [Value::Int(1)]) =>
                {
                    Ok(true)
                }
                Value::Enum(e, v, args) if e == "SqlValue" && v == "Integer" && args.len() == 1 => {
                    Err(format!(
                        "`sqlite.table_exists`: hằng truy vấn là `{}` (cần `1`)",
                        args[0].show()
                    ))
                }
                _ => Err(
                    "`sqlite.table_exists`: `SELECT 1` không trả `SqlValue.Integer(1)`".to_string(),
                ),
            }
        }
        _ => Err(format!(
            "`sqlite.table_exists`: truy vấn hệ thống trả {} hàng (cần tối đa 1)",
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
        hang: Result<Vec<Vec<Value>>, String>,
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
            self.hang.clone().map(|hang| (self.cot.clone(), hang))
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

    fn host(cot: &[&str], hang: Result<Vec<Vec<Value>>, String>) -> HostGia {
        HostGia {
            cot: cot.iter().map(|s| s.to_string()).collect(),
            hang,
            sql: None,
            tham_so: None,
        }
    }

    fn mot() -> Value {
        Value::Enum("SqlValue".into(), "Integer".into(), vec![Value::Int(1)])
    }

    #[test]
    fn tra_theo_ten_bind_va_co_hang_la_true() {
        let mut h = host(&["1"], Ok(vec![vec![mot()]]));
        assert!(chay(&mut h, "db:x", "t").expect("phải Ok"));
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
    fn khong_co_hang_la_false_va_loi_host_duoc_truyen_nguyen() {
        let mut h = host(&["1"], Ok(Vec::new()));
        assert!(!chay(&mut h, "db:x", "thieu").expect("phải Ok"));
        let mut h = host(&["1"], Err("handle đã đóng".to_string()));
        assert_eq!(
            chay(&mut h, "db:x", "t").expect_err("host lỗi ⇒ Err"),
            "handle đã đóng"
        );
    }

    #[test]
    fn hinh_dang_sai_la_err_du_lieu() {
        let mut h = host(&["1", "extra"], Ok(vec![vec![mot(), mot()]]));
        assert!(chay(&mut h, "db:x", "t").is_err(), "hai cột ⇒ Err");
        let mut h = host(&["1"], Ok(vec![vec![]]));
        assert!(chay(&mut h, "db:x", "t").is_err(), "hàng 0 ô ⇒ Err");
        let mut h = host(&["1"], Ok(vec![vec![Value::Int(1)]]));
        assert!(chay(&mut h, "db:x", "t").is_err(), "ô int trần ⇒ Err");
        let mut h = host(
            &["1"],
            Ok(vec![vec![Value::Enum(
                "SqlValue".into(),
                "Integer".into(),
                vec![Value::Int(0)],
            )]]),
        );
        assert!(chay(&mut h, "db:x", "t").is_err(), "hằng 0 ⇒ Err");
        let mut h = host(&["1"], Ok(vec![vec![mot()], vec![mot()]]));
        assert!(chay(&mut h, "db:x", "t").is_err(), "hai hàng ⇒ Err");
    }
}
